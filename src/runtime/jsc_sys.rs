#![allow(unsafe_code)]

use super::host_bridge::HostBridge;
use crate::desktop::NativeHttpEngine;
use crate::ffi_dl::DynLib;
use std::collections::HashMap;
#[cfg(target_os = "linux")]
use std::ffi::c_uint;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::sync::{Arc, Mutex, OnceLock};

#[cfg(target_os = "linux")]
const G_TYPE_STRING: usize = 16 << 2;

pub type HttpEngineSlot = Arc<Mutex<Arc<Mutex<NativeHttpEngine>>>>;
pub type HostBridgeSlot = Arc<Mutex<Option<Arc<HostBridge>>>>;

#[derive(Clone, Copy)]
pub(super) enum JscCallbackKind {
    Http,
    HttpConfig,
    HostService,
}

pub(super) fn dispatch_host_bridge_call(
    kind: JscCallbackKind,
    payload: &str,
    http_slot: &HttpEngineSlot,
    host_slot: &HostBridgeSlot,
) -> String {
    match kind {
        JscCallbackKind::Http => {
            if let Some(bridge) = host_slot.lock().ok().and_then(|g| g.clone())
                && !bridge.allows_network()
            {
                return String::from(
                    r#"{"status":403,"statusText":"Forbidden","body":"Permission denied: network"}"#,
                );
            }
            let Ok(engine_arc) = http_slot.lock().map(|guard| Arc::clone(&*guard)) else {
                return String::from(
                    r#"{"status":500,"statusText":"Engine Lock Error","body":""}"#,
                );
            };
            NativeHttpEngine::execute_bridge_detached(&engine_arc, payload)
        }
        JscCallbackKind::HttpConfig => {
            let Ok(engine_arc) = http_slot.lock().map(|guard| Arc::clone(&*guard)) else {
                return String::from(
                    r#"{"status":500,"statusText":"Engine Lock Error","body":""}"#,
                );
            };
            let Ok(mut engine) = engine_arc.lock() else {
                return String::from(
                    r#"{"status":500,"statusText":"Engine Lock Error","body":""}"#,
                );
            };
            engine.apply_bridge_config_json(payload)
        }
        JscCallbackKind::HostService => {
            let bridge = host_slot.lock().ok().and_then(|g| g.clone());
            bridge.map_or_else(
                || String::from(r#"{"ok":false,"error":"HostBridge not attached"}"#),
                |b| b.handle_service_call(payload),
            )
        }
    }
}

#[cfg(target_os = "linux")]
struct JscBridgeData {
    kind: JscCallbackKind,
    http_slot: HttpEngineSlot,
    host_slot: HostBridgeSlot,
    g_strdup: unsafe extern "C" fn(*const c_char) -> *mut c_char,
}

#[cfg(target_os = "linux")]
unsafe extern "C" fn drop_bridge_data(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr.cast::<JscBridgeData>()));
        }
    }
}

#[cfg(target_os = "linux")]
unsafe extern "C" fn host_bridge_trampoline(
    payload_ptr: *const c_char,
    user_data: *mut c_void,
) -> *mut c_char {
    if payload_ptr.is_null() || user_data.is_null() {
        return std::ptr::null_mut();
    }
    let payload = unsafe { CStr::from_ptr(payload_ptr) }.to_string_lossy();
    let data = unsafe { &*user_data.cast::<JscBridgeData>() };
    let response_json =
        dispatch_host_bridge_call(data.kind, &payload, &data.http_slot, &data.host_slot);

    CString::new(response_json).map_or(std::ptr::null_mut(), |c_out| unsafe {
        (data.g_strdup)(c_out.as_ptr())
    })
}

type JsContextRef = *mut c_void;
type JsGlobalContextRef = *mut c_void;
type JsObjectRef = *mut c_void;
type JsValueRef = *mut c_void;
type JsStringRef = *mut c_void;
type JsClassRef = *mut c_void;
type JsPropertyAttributes = u32;
type JsObjectCallAsFunctionCallback = unsafe extern "C" fn(
    ctx: JsContextRef,
    function: JsObjectRef,
    this_object: JsObjectRef,
    argument_count: usize,
    arguments: *const JsValueRef,
    exception: *mut JsValueRef,
) -> JsValueRef;

#[allow(clippy::struct_field_names)]
#[derive(Clone, Copy)]
struct JscCApiFns {
    js_global_context_create: unsafe extern "C" fn(JsClassRef) -> JsGlobalContextRef,
    js_global_context_release: unsafe extern "C" fn(JsGlobalContextRef),
    js_context_get_global_context: unsafe extern "C" fn(JsContextRef) -> JsGlobalContextRef,
    js_context_get_global_object: unsafe extern "C" fn(JsContextRef) -> JsObjectRef,
    js_evaluate_script: unsafe extern "C" fn(
        JsContextRef,
        JsStringRef,
        JsObjectRef,
        JsStringRef,
        c_int,
        *mut JsValueRef,
    ) -> JsValueRef,
    js_object_make_function_with_callback: unsafe extern "C" fn(
        JsContextRef,
        JsStringRef,
        JsObjectCallAsFunctionCallback,
    ) -> JsObjectRef,
    js_object_set_property: unsafe extern "C" fn(
        JsContextRef,
        JsObjectRef,
        JsStringRef,
        JsValueRef,
        JsPropertyAttributes,
        *mut JsValueRef,
    ),
    js_string_create_with_utf8_cstring: unsafe extern "C" fn(*const c_char) -> JsStringRef,
    js_string_get_maximum_utf8_cstring_size: unsafe extern "C" fn(JsStringRef) -> usize,
    js_string_get_utf8_cstring: unsafe extern "C" fn(JsStringRef, *mut c_char, usize) -> usize,
    js_string_release: unsafe extern "C" fn(JsStringRef),
    js_value_make_string: unsafe extern "C" fn(JsContextRef, JsStringRef) -> JsValueRef,
    js_value_make_undefined: unsafe extern "C" fn(JsContextRef) -> JsValueRef,
    js_value_to_string_copy:
        unsafe extern "C" fn(JsContextRef, JsValueRef, *mut JsValueRef) -> JsStringRef,
}

struct JscCApiContextState {
    fns: JscCApiFns,
    http_slot: HttpEngineSlot,
    host_slot: HostBridgeSlot,
}

fn c_api_registry() -> &'static Mutex<HashMap<usize, Arc<JscCApiContextState>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<usize, Arc<JscCApiContextState>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

unsafe fn js_value_to_rust_string(
    fns: &JscCApiFns,
    ctx: JsContextRef,
    val: JsValueRef,
) -> Option<String> {
    if val.is_null() {
        return None;
    }
    let mut exc: JsValueRef = std::ptr::null_mut();
    let js_str = unsafe { (fns.js_value_to_string_copy)(ctx, val, &raw mut exc) };
    if js_str.is_null() {
        return None;
    }
    let max_len = unsafe { (fns.js_string_get_maximum_utf8_cstring_size)(js_str) };
    if max_len == 0 {
        unsafe { (fns.js_string_release)(js_str) };
        return Some(String::new());
    }
    let mut buf = vec![0u8; max_len];
    let written = unsafe {
        (fns.js_string_get_utf8_cstring)(js_str, buf.as_mut_ptr().cast::<c_char>(), max_len)
    };
    unsafe { (fns.js_string_release)(js_str) };
    if written == 0 {
        return Some(String::new());
    }
    let c_len = buf[..written.min(buf.len())]
        .iter()
        .position(|&b| b == 0)
        .unwrap_or_else(|| written.saturating_sub(1));
    Some(String::from_utf8_lossy(&buf[..c_len]).into_owned())
}

unsafe fn rust_str_to_js_string(fns: &JscCApiFns, text: &str) -> Option<JsStringRef> {
    let c_text = CString::new(text).ok()?;
    let js_str = unsafe { (fns.js_string_create_with_utf8_cstring)(c_text.as_ptr()) };
    if js_str.is_null() { None } else { Some(js_str) }
}

unsafe fn lookup_c_api_state(ctx: JsContextRef) -> Option<Arc<JscCApiContextState>> {
    let direct_key = ctx as usize;
    if let Ok(guard) = c_api_registry().lock() {
        if let Some(state) = guard.get(&direct_key) {
            return Some(Arc::clone(state));
        }
        // Fallback: resolve `JSContextGetGlobalContext(ctx)` using any registered `fns` instance.
        if let Some(any_state) = guard.values().next() {
            let get_global = any_state.fns.js_context_get_global_context;
            let global_ctx = unsafe { get_global(ctx) };
            if let Some(state) = guard.get(&(global_ctx as usize)) {
                return Some(Arc::clone(state));
            }
        }
    }
    None
}

unsafe fn c_api_dispatch(
    kind: JscCallbackKind,
    ctx: JsContextRef,
    argument_count: usize,
    arguments: *const JsValueRef,
) -> JsValueRef {
    let Some(state) = (unsafe { lookup_c_api_state(ctx) }) else {
        return std::ptr::null_mut();
    };
    let payload = if argument_count >= 1 && !arguments.is_null() {
        let first_arg = unsafe { *arguments };
        unsafe { js_value_to_rust_string(&state.fns, ctx, first_arg) }.unwrap_or_default()
    } else {
        String::new()
    };
    let response_json =
        dispatch_host_bridge_call(kind, &payload, &state.http_slot, &state.host_slot);

    unsafe { rust_str_to_js_string(&state.fns, &response_json) }.map_or_else(
        || unsafe { (state.fns.js_value_make_undefined)(ctx) },
        |js_str| {
            let js_val = unsafe { (state.fns.js_value_make_string)(ctx, js_str) };
            unsafe { (state.fns.js_string_release)(js_str) };
            js_val
        },
    )
}

unsafe extern "C" fn c_api_http_trampoline(
    ctx: JsContextRef,
    _function: JsObjectRef,
    _this_object: JsObjectRef,
    argument_count: usize,
    arguments: *const JsValueRef,
    _exception: *mut JsValueRef,
) -> JsValueRef {
    unsafe { c_api_dispatch(JscCallbackKind::Http, ctx, argument_count, arguments) }
}

unsafe extern "C" fn c_api_http_config_trampoline(
    ctx: JsContextRef,
    _function: JsObjectRef,
    _this_object: JsObjectRef,
    argument_count: usize,
    arguments: *const JsValueRef,
    _exception: *mut JsValueRef,
) -> JsValueRef {
    unsafe { c_api_dispatch(JscCallbackKind::HttpConfig, ctx, argument_count, arguments) }
}

unsafe extern "C" fn c_api_host_service_trampoline(
    ctx: JsContextRef,
    _function: JsObjectRef,
    _this_object: JsObjectRef,
    argument_count: usize,
    arguments: *const JsValueRef,
    _exception: *mut JsValueRef,
) -> JsValueRef {
    unsafe { c_api_dispatch(JscCallbackKind::HostService, ctx, argument_count, arguments) }
}

enum JscBackend {
    #[cfg(target_os = "linux")]
    GObject {
        jsc_context_evaluate:
            unsafe extern "C" fn(*mut c_void, *const c_char, isize) -> *mut c_void,
        jsc_context_get_exception: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
        jsc_context_clear_exception: unsafe extern "C" fn(*mut c_void),
        jsc_exception_to_string: Option<unsafe extern "C" fn(*mut c_void) -> *mut c_char>,
        jsc_value_to_string: unsafe extern "C" fn(*mut c_void) -> *mut c_char,
        g_free: unsafe extern "C" fn(*mut c_void),
        g_object_unref: unsafe extern "C" fn(*mut c_void),
        context: *mut c_void,
    },
    CApi {
        fns: JscCApiFns,
        context: JsGlobalContextRef,
    },
}

/// Isolated dynamic FFI loader for Apple/WebKit `JavaScriptCore`:
/// uses `GObject` `libjavascriptcoregtk` on Linux (when available) and the standard
/// Apple/WebKit `JavaScriptCore` C API on macOS (`JavaScriptCore.framework`) or fallback.
/// Provides 4-tier JIT ES2024 execution with 0 MB binary bloat on Linux/GNOME and macOS.
pub struct JscSys {
    backend: JscBackend,
    http_slot: HttpEngineSlot,
    host_slot: HostBridgeSlot,
}

impl JscSys {
    pub fn try_load() -> Option<Self> {
        #[cfg(target_os = "linux")]
        if let Some(gobject) = Self::try_load_gobject() {
            return Some(gobject);
        }
        Self::try_load_c_api()
    }

    /// Loads `JavaScriptCore` using the portable Apple/WebKit C API (`JSGlobalContextCreate`,
    /// `JSEvaluateScript`, `JSObjectMakeFunctionWithCallback`).
    /// Active on macOS `JavaScriptCore.framework` and directly testable on Linux against
    /// `libjavascriptcoregtk-6.0.so.1`.
    pub fn try_load_c_api() -> Option<Self> {
        let candidates: &[&CStr] = if cfg!(target_os = "macos") {
            &[
                c"/System/Library/Frameworks/JavaScriptCore.framework/JavaScriptCore",
                c"/System/Library/Frameworks/JavaScriptCore.framework/Versions/A/JavaScriptCore",
                c"JavaScriptCore.framework/JavaScriptCore",
            ]
        } else if cfg!(target_os = "linux") {
            &[
                c"libjavascriptcoregtk-6.0.so.1",
                c"libjavascriptcoregtk-4.1.so.0",
                c"libjavascriptcoregtk-4.0.so.18",
            ]
        } else {
            &[c"JavaScriptCore.dll"]
        };

        unsafe {
            let mut loaded_lib = None;
            for candidate in candidates {
                if let Some(lib) = Self::open_lib(candidate) {
                    loaded_lib = Some(lib);
                    break;
                }
            }
            let jsc_lib = loaded_lib?;

            let fns = JscCApiFns {
                js_global_context_create: Self::load_fn(jsc_lib, c"JSGlobalContextCreate")?,
                js_global_context_release: Self::load_fn(jsc_lib, c"JSGlobalContextRelease")?,
                js_context_get_global_context: Self::load_fn(
                    jsc_lib,
                    c"JSContextGetGlobalContext",
                )?,
                js_context_get_global_object: Self::load_fn(jsc_lib, c"JSContextGetGlobalObject")?,
                js_evaluate_script: Self::load_fn(jsc_lib, c"JSEvaluateScript")?,
                js_object_make_function_with_callback: Self::load_fn(
                    jsc_lib,
                    c"JSObjectMakeFunctionWithCallback",
                )?,
                js_object_set_property: Self::load_fn(jsc_lib, c"JSObjectSetProperty")?,
                js_string_create_with_utf8_cstring: Self::load_fn(
                    jsc_lib,
                    c"JSStringCreateWithUTF8CString",
                )?,
                js_string_get_maximum_utf8_cstring_size: Self::load_fn(
                    jsc_lib,
                    c"JSStringGetMaximumUTF8CStringSize",
                )?,
                js_string_get_utf8_cstring: Self::load_fn(jsc_lib, c"JSStringGetUTF8CString")?,
                js_string_release: Self::load_fn(jsc_lib, c"JSStringRelease")?,
                js_value_make_string: Self::load_fn(jsc_lib, c"JSValueMakeString")?,
                js_value_make_undefined: Self::load_fn(jsc_lib, c"JSValueMakeUndefined")?,
                js_value_to_string_copy: Self::load_fn(jsc_lib, c"JSValueToStringCopy")?,
            };

            let context = (fns.js_global_context_create)(std::ptr::null_mut());
            if context.is_null() {
                return None;
            }

            let http_slot: HttpEngineSlot =
                Arc::new(Mutex::new(Arc::new(Mutex::new(NativeHttpEngine::new()))));
            let host_slot: HostBridgeSlot = Arc::new(Mutex::new(None));
            let state = Arc::new(JscCApiContextState {
                fns,
                http_slot: Arc::clone(&http_slot),
                host_slot: Arc::clone(&host_slot),
            });
            if let Ok(mut guard) = c_api_registry().lock() {
                guard.insert(context as usize, state);
            }

            let global_obj = (fns.js_context_get_global_object)(context);
            let registrations: [(&str, JsObjectCallAsFunctionCallback); 3] = [
                ("__NG_NATIVE_HOST_HTTP__", c_api_http_trampoline),
                (
                    "__NG_NATIVE_HOST_HTTP_CONFIG__",
                    c_api_http_config_trampoline,
                ),
                ("__NG_NATIVE_HOST_SERVICE__", c_api_host_service_trampoline),
            ];
            for (name, callback) in registrations {
                if let Some(js_name) = rust_str_to_js_string(&fns, name) {
                    let fn_obj =
                        (fns.js_object_make_function_with_callback)(context, js_name, callback);
                    if !fn_obj.is_null() {
                        let mut exc: JsValueRef = std::ptr::null_mut();
                        (fns.js_object_set_property)(
                            context,
                            global_obj,
                            js_name,
                            fn_obj,
                            0,
                            &raw mut exc,
                        );
                    }
                    (fns.js_string_release)(js_name);
                }
            }

            Some(Self {
                backend: JscBackend::CApi { fns, context },
                http_slot,
                host_slot,
            })
        }
    }

    #[cfg(target_os = "linux")]
    fn try_load_gobject() -> Option<Self> {
        unsafe {
            let jsc_lib = Self::open_lib(c"libjavascriptcoregtk-6.0.so.1")
                .or_else(|| Self::open_lib(c"libjavascriptcoregtk-4.1.so.0"))?;
            let glib_lib = Self::open_lib(c"libglib-2.0.so.0")?;
            let gobject_lib = Self::open_lib(c"libgobject-2.0.so.0")?;

            let jsc_context_new: unsafe extern "C" fn() -> *mut c_void =
                Self::load_fn(jsc_lib, c"jsc_context_new")?;
            let context = jsc_context_new();
            if context.is_null() {
                return None;
            }

            let http_slot: HttpEngineSlot =
                Arc::new(Mutex::new(Arc::new(Mutex::new(NativeHttpEngine::new()))));
            let host_slot: HostBridgeSlot = Arc::new(Mutex::new(None));
            let g_object_unref: unsafe extern "C" fn(*mut c_void) =
                Self::load_fn(gobject_lib, c"g_object_unref")?;

            Self::bind_host_callbacks(
                jsc_lib,
                glib_lib,
                g_object_unref,
                context,
                &http_slot,
                &host_slot,
            );

            Some(Self {
                backend: JscBackend::GObject {
                    jsc_context_evaluate: Self::load_fn(jsc_lib, c"jsc_context_evaluate")?,
                    jsc_context_get_exception: Self::load_fn(
                        jsc_lib,
                        c"jsc_context_get_exception",
                    )?,
                    jsc_context_clear_exception: Self::load_fn(
                        jsc_lib,
                        c"jsc_context_clear_exception",
                    )?,
                    jsc_exception_to_string: Self::load_fn(jsc_lib, c"jsc_exception_to_string"),
                    jsc_value_to_string: Self::load_fn(jsc_lib, c"jsc_value_to_string")?,
                    g_free: Self::load_fn(glib_lib, c"g_free")?,
                    g_object_unref,
                    context,
                },
                http_slot,
                host_slot,
            })
        }
    }

    #[cfg(target_os = "linux")]
    unsafe fn bind_host_callbacks(
        jsc_lib: *mut c_void,
        glib_lib: *mut c_void,
        g_object_unref: unsafe extern "C" fn(*mut c_void),
        context: *mut c_void,
        http_slot: &HttpEngineSlot,
        host_slot: &HostBridgeSlot,
    ) {
        type NewFnV = unsafe extern "C" fn(
            *mut c_void,
            *const c_char,
            *mut c_void,
            *mut c_void,
            unsafe extern "C" fn(*mut c_void),
            usize,
            c_uint,
            *const usize,
        ) -> *mut c_void;
        type SetVal = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_void);
        type Strdup = unsafe extern "C" fn(*const c_char) -> *mut c_char;

        let (Some(new_fn), Some(set_val), Some(g_strdup)) = (
            unsafe { Self::load_fn::<NewFnV>(jsc_lib, c"jsc_value_new_functionv") },
            unsafe { Self::load_fn::<SetVal>(jsc_lib, c"jsc_context_set_value") },
            unsafe { Self::load_fn::<Strdup>(glib_lib, c"g_strdup") },
        ) else {
            return;
        };

        let param_types = [G_TYPE_STRING];
        let registrations: [(&CStr, JscCallbackKind); 3] = [
            (c"__NG_NATIVE_HOST_HTTP__", JscCallbackKind::Http),
            (
                c"__NG_NATIVE_HOST_HTTP_CONFIG__",
                JscCallbackKind::HttpConfig,
            ),
            (c"__NG_NATIVE_HOST_SERVICE__", JscCallbackKind::HostService),
        ];

        for (name, kind) in registrations {
            let user_data = Box::into_raw(Box::new(JscBridgeData {
                kind,
                http_slot: Arc::clone(http_slot),
                host_slot: Arc::clone(host_slot),
                g_strdup,
            }))
            .cast::<c_void>();
            unsafe {
                let fn_val = new_fn(
                    context,
                    name.as_ptr(),
                    host_bridge_trampoline as *const () as *mut c_void,
                    user_data,
                    drop_bridge_data,
                    G_TYPE_STRING,
                    1,
                    param_types.as_ptr(),
                );
                if !fn_val.is_null() {
                    set_val(context, name.as_ptr(), fn_val);
                    g_object_unref(fn_val);
                }
            }
        }
    }

    /// Binds a shared host `NativeHttpEngine` so JS HTTP calls execute on `DesktopServices`.
    pub fn set_http_engine(&self, engine: Arc<Mutex<NativeHttpEngine>>) {
        if let Ok(mut guard) = self.http_slot.lock() {
            *guard = engine;
        }
    }

    /// Binds a shared `HostBridge` so `__NG_NATIVE_HOST_SERVICE__` calls execute natively.
    pub fn set_host_bridge(&self, bridge: Arc<HostBridge>) {
        if let Ok(mut guard) = self.host_slot.lock() {
            *guard = Some(bridge);
        }
    }

    unsafe fn open_lib(name: &CStr) -> Option<*mut c_void> {
        unsafe { DynLib::open(name).map(DynLib::as_ptr) }
    }

    unsafe fn load_fn<F: Copy>(handle: *mut c_void, sym: &CStr) -> Option<F> {
        unsafe { DynLib::from_ptr(handle).sym(sym) }
    }

    pub fn eval(&self, script: &str) -> Result<String, String> {
        match &self.backend {
            #[cfg(target_os = "linux")]
            JscBackend::GObject {
                jsc_context_evaluate,
                jsc_context_get_exception,
                jsc_context_clear_exception,
                jsc_exception_to_string,
                jsc_value_to_string,
                g_free,
                g_object_unref,
                context,
            } => {
                let c_script = CString::new(script).map_err(|e| e.to_string())?;
                unsafe {
                    let val_ptr = (jsc_context_evaluate)(*context, c_script.as_ptr(), -1);
                    let exc_ptr = (jsc_context_get_exception)(*context);
                    if !exc_ptr.is_null() {
                        let message = jsc_exception_to_string
                            .and_then(|to_str| {
                                let err_c = to_str(exc_ptr);
                                if err_c.is_null() {
                                    None
                                } else {
                                    let s = CStr::from_ptr(err_c).to_string_lossy().into_owned();
                                    (g_free)(err_c.cast::<c_void>());
                                    Some(s)
                                }
                            })
                            .filter(|s| !s.trim().is_empty())
                            .unwrap_or_else(|| String::from("JavaScriptCore runtime exception"));
                        (jsc_context_clear_exception)(*context);
                        if !val_ptr.is_null() {
                            (g_object_unref)(val_ptr);
                        }
                        return Err(message);
                    }

                    if val_ptr.is_null() {
                        return Ok(String::from("undefined"));
                    }

                    let str_ptr = (jsc_value_to_string)(val_ptr);
                    let output = if str_ptr.is_null() {
                        String::new()
                    } else {
                        let s = CStr::from_ptr(str_ptr).to_string_lossy().into_owned();
                        (g_free)(str_ptr.cast::<c_void>());
                        s
                    };
                    (g_object_unref)(val_ptr);
                    Ok(output)
                }
            }
            JscBackend::CApi { fns, context } => unsafe {
                let js_script = rust_str_to_js_string(fns, script)
                    .ok_or_else(|| String::from("Failed to allocate JSString for script"))?;
                let mut exc: JsValueRef = std::ptr::null_mut();
                let val_ref = (fns.js_evaluate_script)(
                    *context,
                    js_script,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    1,
                    &raw mut exc,
                );
                (fns.js_string_release)(js_script);

                if !exc.is_null() {
                    let message = js_value_to_rust_string(fns, *context, exc)
                        .filter(|s| !s.trim().is_empty())
                        .unwrap_or_else(|| String::from("JavaScriptCore runtime exception"));
                    return Err(message);
                }
                if val_ref.is_null() {
                    return Ok(String::from("undefined"));
                }
                Ok(js_value_to_rust_string(fns, *context, val_ref)
                    .unwrap_or_else(|| String::from("undefined")))
            },
        }
    }
}

impl Drop for JscSys {
    fn drop(&mut self) {
        match &self.backend {
            #[cfg(target_os = "linux")]
            JscBackend::GObject {
                g_object_unref,
                context,
                ..
            } => {
                if !context.is_null() {
                    unsafe {
                        (g_object_unref)(*context);
                    }
                }
            }
            JscBackend::CApi { fns, context } => {
                if !context.is_null() {
                    if let Ok(mut guard) = c_api_registry().lock() {
                        guard.remove(&(*context as usize));
                    }
                    unsafe {
                        (fns.js_global_context_release)(*context);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppPermissions;
    use crate::desktop::DesktopServices;
    #[allow(unused_imports)]
    use std::path::PathBuf as _;

    #[test]
    fn jsc_c_api_evaluates_es2024_and_host_callbacks() {
        let Some(sys) = JscSys::try_load_c_api() else {
            return;
        };
        let http = Arc::new(Mutex::new(NativeHttpEngine::new()));
        http.lock().unwrap().register_http_endpoint(
            "GET",
            "https://capi.ng-native.test/ping",
            200,
            r#"{"pong":true}"#,
        );
        sys.set_http_engine(http);

        let desktop =
            DesktopServices::with_app_config("capi-test", AppPermissions::allow_all_sandboxed());
        sys.set_host_bridge(Arc::new(HostBridge::new(&desktop)));

        let out = sys
            .eval(
                r#"
                const httpRes = JSON.parse(__NG_NATIVE_HOST_HTTP__(JSON.stringify({
                    method: "GET",
                    url: "https://capi.ng-native.test/ping"
                })));
                const sysRes = JSON.parse(__NG_NATIVE_HOST_SERVICE__(JSON.stringify({
                    action: "systemEnv"
                })));
                `${httpRes.status}:${JSON.parse(httpRes.body).pong}:${sysRes.ok}`
                "#,
            )
            .unwrap();
        assert_eq!(out, "200:true:true");

        let err = sys.eval("function () { broken syntax !!!");
        assert!(err.is_err());
    }

    #[test]
    fn jsc_http_callback_enforces_network_permission_without_holding_slot_lock() {
        let http_engine = Arc::new(Mutex::new(NativeHttpEngine::new()));
        http_engine.lock().unwrap().register_http_endpoint(
            "GET",
            "https://capi.ng-native.test/blocked",
            200,
            r#"{"ok":true}"#,
        );
        let http_slot: HttpEngineSlot = Arc::new(Mutex::new(http_engine));

        let blocked_desktop = DesktopServices::with_app_config(
            "capi-net-blocked",
            AppPermissions {
                network: false,
                full_system_access: false,
                ..AppPermissions::minimal()
            },
        );
        let blocked_host: HostBridgeSlot = Arc::new(Mutex::new(Some(Arc::new(HostBridge::new(
            &blocked_desktop,
        )))));
        let denied = dispatch_host_bridge_call(
            JscCallbackKind::Http,
            r#"{"method":"GET","url":"https://capi.ng-native.test/blocked"}"#,
            &http_slot,
            &blocked_host,
        );
        assert!(denied.contains("403"), "{denied}");
        assert!(denied.contains("Permission denied: network"), "{denied}");

        let allowed_desktop = DesktopServices::with_app_config(
            "capi-net-allowed",
            AppPermissions::allow_all_sandboxed(),
        );
        let allowed_host: HostBridgeSlot = Arc::new(Mutex::new(Some(Arc::new(HostBridge::new(
            &allowed_desktop,
        )))));
        let ok = dispatch_host_bridge_call(
            JscCallbackKind::Http,
            r#"{"method":"GET","url":"https://capi.ng-native.test/blocked"}"#,
            &http_slot,
            &allowed_host,
        );
        assert!(ok.contains(r#""status":200"#), "{ok}");
    }
}
