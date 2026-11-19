#![allow(unsafe_code)]

use super::host_bridge::HostBridge;
use crate::desktop::NativeHttpEngine;
#[cfg(target_os = "linux")]
use crate::ffi_dl::DynLib;
#[cfg(target_os = "linux")]
use std::ffi::c_uint;
use std::ffi::{CStr, CString, c_char, c_void};
use std::sync::{Arc, Mutex};

const G_TYPE_STRING: usize = 16 << 2;

pub type HttpEngineSlot = Arc<Mutex<Arc<Mutex<NativeHttpEngine>>>>;
pub type HostBridgeSlot = Arc<Mutex<Option<Arc<HostBridge>>>>;

#[cfg(target_os = "linux")]
#[derive(Clone, Copy)]
enum JscCallbackKind {
    Http,
    HttpConfig,
    HostService,
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
    let response_json = match data.kind {
        JscCallbackKind::Http | JscCallbackKind::HttpConfig => {
            if let Ok(slot_guard) = data.http_slot.lock()
                && let Ok(mut engine) = slot_guard.lock()
            {
                if matches!(data.kind, JscCallbackKind::HttpConfig) {
                    engine.apply_bridge_config_json(&payload)
                } else {
                    engine.execute_bridge_json(&payload)
                }
            } else {
                String::from(r#"{"status":500,"statusText":"Engine Lock Error","body":""}"#)
            }
        }
        JscCallbackKind::HostService => {
            let bridge = data.host_slot.lock().ok().and_then(|g| g.clone());
            bridge.map_or_else(
                || String::from(r#"{"ok":false,"error":"HostBridge not attached"}"#),
                |b| b.handle_service_call(&payload),
            )
        }
    };

    CString::new(response_json).map_or(std::ptr::null_mut(), |c_out| unsafe {
        (data.g_strdup)(c_out.as_ptr())
    })
}

/// Isolated dynamic FFI loader for Apple/WebKit `JavaScriptCore` (`libjavascriptcoregtk-6.0.so.1`).
/// Provides 4-tier JIT ES2024 execution with 0 MB binary bloat on Linux/GNOME and macOS.
pub struct JscSys {
    jsc_context_evaluate: unsafe extern "C" fn(*mut c_void, *const c_char, isize) -> *mut c_void,
    jsc_context_get_exception: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    jsc_context_clear_exception: unsafe extern "C" fn(*mut c_void),
    jsc_exception_to_string: Option<unsafe extern "C" fn(*mut c_void) -> *mut c_char>,
    jsc_value_to_string: unsafe extern "C" fn(*mut c_void) -> *mut c_char,
    g_free: unsafe extern "C" fn(*mut c_void),
    g_object_unref: unsafe extern "C" fn(*mut c_void),
    context: *mut c_void,
    http_slot: HttpEngineSlot,
    host_slot: HostBridgeSlot,
}

impl JscSys {
    #[cfg(target_os = "linux")]
    pub fn try_load() -> Option<Self> {
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
                jsc_context_evaluate: Self::load_fn(jsc_lib, c"jsc_context_evaluate")?,
                jsc_context_get_exception: Self::load_fn(jsc_lib, c"jsc_context_get_exception")?,
                jsc_context_clear_exception: Self::load_fn(
                    jsc_lib,
                    c"jsc_context_clear_exception",
                )?,
                jsc_exception_to_string: Self::load_fn(jsc_lib, c"jsc_exception_to_string"),
                jsc_value_to_string: Self::load_fn(jsc_lib, c"jsc_value_to_string")?,
                g_free: Self::load_fn(glib_lib, c"g_free")?,
                g_object_unref,
                context,
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

    #[cfg(not(target_os = "linux"))]
    pub fn try_load() -> Option<Self> {
        let _ = G_TYPE_STRING;
        None
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

    #[cfg(target_os = "linux")]
    unsafe fn open_lib(name: &CStr) -> Option<*mut c_void> {
        unsafe { DynLib::open(name).map(DynLib::as_ptr) }
    }

    #[cfg(target_os = "linux")]
    unsafe fn load_fn<F: Copy>(handle: *mut c_void, sym: &CStr) -> Option<F> {
        unsafe { DynLib::from_ptr(handle).sym(sym) }
    }

    unsafe fn format_exception(&self, exc_ptr: *mut c_void) -> String {
        if let Some(to_str) = self.jsc_exception_to_string {
            let err_c = unsafe { to_str(exc_ptr) };
            if !err_c.is_null() {
                let formatted = unsafe { CStr::from_ptr(err_c) }
                    .to_string_lossy()
                    .into_owned();
                unsafe { (self.g_free)(err_c.cast::<c_void>()) };
                if !formatted.trim().is_empty() {
                    return formatted;
                }
            }
        }
        String::from("JavaScriptCore runtime exception")
    }

    pub fn eval(&self, script: &str) -> Result<String, String> {
        let c_script = CString::new(script).map_err(|e| e.to_string())?;
        unsafe {
            let val_ptr = (self.jsc_context_evaluate)(self.context, c_script.as_ptr(), -1);
            let exc_ptr = (self.jsc_context_get_exception)(self.context);
            if !exc_ptr.is_null() {
                let message = self.format_exception(exc_ptr);
                (self.jsc_context_clear_exception)(self.context);
                if !val_ptr.is_null() {
                    (self.g_object_unref)(val_ptr);
                }
                return Err(message);
            }

            if val_ptr.is_null() {
                return Ok(String::from("undefined"));
            }

            let str_ptr = (self.jsc_value_to_string)(val_ptr);
            let output = if str_ptr.is_null() {
                String::new()
            } else {
                let s = CStr::from_ptr(str_ptr).to_string_lossy().into_owned();
                (self.g_free)(str_ptr.cast::<c_void>());
                s
            };
            (self.g_object_unref)(val_ptr);
            Ok(output)
        }
    }
}

impl Drop for JscSys {
    fn drop(&mut self) {
        if !self.context.is_null() {
            unsafe {
                (self.g_object_unref)(self.context);
            }
        }
    }
}
