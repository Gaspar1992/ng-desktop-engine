#![allow(unsafe_code)]

#[cfg(target_os = "macos")]
use crate::engine::coalesce_frame_events;
use crate::engine::{EventMask, EventPayload, NativeEvent, NodeId};
#[cfg(target_os = "macos")]
use crate::ffi_dl::DynLib;
#[cfg(target_os = "macos")]
use std::collections::HashMap;
#[cfg(target_os = "macos")]
use std::ffi::{CStr, CString, c_char, c_int, c_void};
#[cfg(target_os = "macos")]
use std::sync::{Mutex, OnceLock};

/// Cocoa `NSPoint` FFI value matching AppKit's 64-bit `CGPoint` layout.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NsPoint {
    pub x: f64,
    pub y: f64,
}

/// Cocoa `NSSize` FFI value matching AppKit's 64-bit `CGSize` layout.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NsSize {
    pub width: f64,
    pub height: f64,
}

/// Cocoa `NSRect` FFI value matching AppKit's 64-bit `CGRect` layout.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NsRect {
    pub origin: NsPoint,
    pub size: NsSize,
}

/// Opaque AppKit Objective-C object (`NSWindow` / `NSPanel` / `NSView`) handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppKitHandle(pub(super) usize);

#[must_use]
pub const fn appkit_window_style_mask(is_panel: bool) -> u64 {
    // Titled (1) | Closable (2) | Miniaturizable (4, window only) | Resizable (8)
    if is_panel { 1 | 2 | 8 } else { 1 | 2 | 4 | 8 }
}

/// Translates an AppKit control target-action or `NSWindowDelegate` callback into a typed `NativeEvent`.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn translate_appkit_delegate_event(
    node: NodeId,
    objc_class: &str,
    event_mask: EventMask,
    prevent_close: bool,
    action: &str,
    text_value: &str,
    bool_value: bool,
    int_value: i64,
    size: (i32, i32),
) -> Option<NativeEvent> {
    match action {
        "windowShouldClose:" => {
            if prevent_close || event_mask.contains(EventMask::CLOSE_REQUEST) {
                Some(NativeEvent {
                    node,
                    event_name: String::from("close-request"),
                    payload: EventPayload::Text(format!(r#"{{"preventable":{prevent_close}}}"#)),
                })
            } else {
                None
            }
        }
        "windowDidResize:" if event_mask.contains(EventMask::WINDOW_STATE) => Some(NativeEvent {
            node,
            event_name: String::from("resize"),
            payload: EventPayload::Text(format!(
                r#"{{"width":{},"height":{},"maximized":false}}"#,
                size.0, size.1
            )),
        }),
        "action" | "controlTextDidChange:" | "textDidChange:" => match objc_class {
            "NSSwitch" | "NSButtonCheckbox" => Some(NativeEvent {
                node,
                event_name: String::from("toggled"),
                payload: EventPayload::Bool(bool_value),
            }),
            "NSButton" | "NSDisclosureButton" => Some(NativeEvent {
                node,
                event_name: String::from("clicked"),
                payload: EventPayload::None,
            }),
            "NSTextField"
            | "NSTextFieldEditable"
            | "NSSecureTextField"
            | "NSSearchField"
            | "NSTextView" => Some(NativeEvent {
                node,
                event_name: String::from("input"),
                payload: EventPayload::Text(text_value.to_string()),
            }),
            "NSSlider" | "NSStepper" => Some(NativeEvent {
                node,
                event_name: String::from("value-changed"),
                payload: EventPayload::Int(int_value),
            }),
            "NSPopUpButton" => Some(NativeEvent {
                node,
                event_name: String::from("changed"),
                payload: EventPayload::Int(int_value),
            }),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn pthread_main_np() -> c_int;
}

#[cfg(target_os = "macos")]
pub(super) fn can_activate_live_appkit() -> bool {
    if super::win32_sys::is_headless_env() {
        return false;
    }
    let force_live = std::env::var("NG_DESKTOP_FORCE_LIVE_APPKIT").is_ok_and(|raw| {
        matches!(
            raw.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes"
        )
    });
    let in_ci = std::env::var("CI").is_ok_and(|raw| {
        let trimmed = raw.trim();
        !trimmed.is_empty() && trimmed != "0" && !trimmed.eq_ignore_ascii_case("false")
    });
    if in_ci && !force_live {
        return false;
    }
    unsafe { pthread_main_np() == 1 }
}

#[cfg(target_os = "macos")]
type ObjcGetClassFn = unsafe extern "C" fn(*const c_char) -> *mut c_void;
#[cfg(target_os = "macos")]
type SelRegisterNameFn = unsafe extern "C" fn(*const c_char) -> *mut c_void;
#[cfg(target_os = "macos")]
type ObjcAllocateClassPairFn =
    unsafe extern "C" fn(*mut c_void, *const c_char, usize) -> *mut c_void;
#[cfg(target_os = "macos")]
type ObjcRegisterClassPairFn = unsafe extern "C" fn(*mut c_void);
#[cfg(target_os = "macos")]
type ClassAddMethodFn =
    unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_void, *const c_char) -> u8;

#[cfg(target_os = "macos")]
#[derive(Debug, Clone)]
struct AppKitNodeRecord {
    node: NodeId,
    objc_class: &'static str,
    event_mask: EventMask,
    prevent_close: bool,
    suppress_change: bool,
    last_size: (i32, i32),
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy)]
struct AppKitObjcDispatch {
    objc_msg_send: *mut c_void,
    sel_object: *mut c_void,
    sel_string_value: *mut c_void,
    sel_string: *mut c_void,
    sel_utf8_string: *mut c_void,
    sel_state: *mut c_void,
    sel_double_value: *mut c_void,
    sel_index_of_selected_item: *mut c_void,
}

#[cfg(target_os = "macos")]
unsafe impl Send for AppKitObjcDispatch {}
#[cfg(target_os = "macos")]
unsafe impl Sync for AppKitObjcDispatch {}

#[cfg(target_os = "macos")]
static NSAPP_READY: OnceLock<bool> = OnceLock::new();
#[cfg(target_os = "macos")]
static APPKIT_DISPATCH: OnceLock<AppKitObjcDispatch> = OnceLock::new();
#[cfg(target_os = "macos")]
static APPKIT_DELEGATE_SINGLETON: OnceLock<usize> = OnceLock::new();
#[cfg(target_os = "macos")]
static APPKIT_NODE_MAP: OnceLock<Mutex<HashMap<usize, AppKitNodeRecord>>> = OnceLock::new();
#[cfg(target_os = "macos")]
static APPKIT_EVENTS: OnceLock<Mutex<Vec<NativeEvent>>> = OnceLock::new();

#[cfg(target_os = "macos")]
fn node_map() -> &'static Mutex<HashMap<usize, AppKitNodeRecord>> {
    APPKIT_NODE_MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(target_os = "macos")]
fn event_queue() -> &'static Mutex<Vec<NativeEvent>> {
    APPKIT_EVENTS.get_or_init(|| Mutex::new(Vec::new()))
}

#[cfg(target_os = "macos")]
fn push_appkit_event(ev: NativeEvent) {
    let mut guard = event_queue()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.push(ev);
}

#[cfg(target_os = "macos")]
fn node_snapshot(handle_key: usize) -> Option<AppKitNodeRecord> {
    let guard = node_map()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.get(&handle_key).cloned()
}

#[cfg(target_os = "macos")]
unsafe fn read_control_text(
    dispatch: &AppKitObjcDispatch,
    sender: *mut c_void,
    is_text_view: bool,
) -> String {
    let msg_ptr0: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
        unsafe { std::mem::transmute_copy::<*mut c_void, _>(&dispatch.objc_msg_send) };
    let msg_cstr0: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *const c_char =
        unsafe { std::mem::transmute_copy::<*mut c_void, _>(&dispatch.objc_msg_send) };
    let sel = if is_text_view {
        dispatch.sel_string
    } else {
        dispatch.sel_string_value
    };
    let ns_str = unsafe { msg_ptr0(sender, sel) };
    if ns_str.is_null() {
        return String::new();
    }
    let utf8 = unsafe { msg_cstr0(ns_str, dispatch.sel_utf8_string) };
    if utf8.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(utf8) }
        .to_string_lossy()
        .into_owned()
}

#[cfg(target_os = "macos")]
unsafe fn dispatch_control_sender(sender: *mut c_void, action_name: &str) {
    if sender.is_null() {
        return;
    }
    let Some(rec) = node_snapshot(sender as usize) else {
        return;
    };
    if rec.suppress_change {
        return;
    }
    let Some(dispatch) = APPKIT_DISPATCH.get() else {
        return;
    };
    let msg_isize0: unsafe extern "C" fn(*mut c_void, *mut c_void) -> isize =
        unsafe { std::mem::transmute_copy::<*mut c_void, _>(&dispatch.objc_msg_send) };
    let msg_f64_0: unsafe extern "C" fn(*mut c_void, *mut c_void) -> f64 =
        unsafe { std::mem::transmute_copy::<*mut c_void, _>(&dispatch.objc_msg_send) };

    let bool_val = matches!(rec.objc_class, "NSSwitch" | "NSButtonCheckbox")
        && unsafe { msg_isize0(sender, dispatch.sel_state) } != 0;
    let int_val = match rec.objc_class {
        "NSSlider" | "NSStepper" => {
            let v = unsafe { msg_f64_0(sender, dispatch.sel_double_value) };
            #[allow(clippy::cast_possible_truncation)]
            {
                v.round() as i64
            }
        }
        "NSPopUpButton" => {
            i64::try_from(unsafe { msg_isize0(sender, dispatch.sel_index_of_selected_item) })
                .unwrap_or(-1)
        }
        _ => 0,
    };
    let text_val = match rec.objc_class {
        "NSTextField" | "NSTextFieldEditable" | "NSSecureTextField" | "NSSearchField" => unsafe {
            read_control_text(dispatch, sender, false)
        },
        "NSTextView" => unsafe { read_control_text(dispatch, sender, true) },
        _ => String::new(),
    };

    if let Some(ev) = translate_appkit_delegate_event(
        rec.node,
        rec.objc_class,
        rec.event_mask,
        rec.prevent_close,
        action_name,
        &text_val,
        bool_val,
        int_val,
        rec.last_size,
    ) {
        push_appkit_event(ev);
    }
}

#[cfg(target_os = "macos")]
unsafe fn dispatch_notification_text_change(notification: *mut c_void, action_name: &str) {
    if notification.is_null() {
        return;
    }
    let Some(dispatch) = APPKIT_DISPATCH.get() else {
        return;
    };
    let msg_ptr0: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
        unsafe { std::mem::transmute_copy::<*mut c_void, _>(&dispatch.objc_msg_send) };
    let sender = unsafe { msg_ptr0(notification, dispatch.sel_object) };
    unsafe { dispatch_control_sender(sender, action_name) };
}

#[cfg(target_os = "macos")]
unsafe extern "C" fn ng_appkit_on_control_action(
    _self_obj: *mut c_void,
    _sel: *mut c_void,
    sender: *mut c_void,
) {
    unsafe { dispatch_control_sender(sender, "action") };
}

#[cfg(target_os = "macos")]
unsafe extern "C" fn ng_appkit_control_text_did_change(
    _self_obj: *mut c_void,
    _sel: *mut c_void,
    notification: *mut c_void,
) {
    unsafe { dispatch_notification_text_change(notification, "controlTextDidChange:") };
}

#[cfg(target_os = "macos")]
unsafe extern "C" fn ng_appkit_text_did_change(
    _self_obj: *mut c_void,
    _sel: *mut c_void,
    notification: *mut c_void,
) {
    unsafe { dispatch_notification_text_change(notification, "textDidChange:") };
}

#[cfg(target_os = "macos")]
unsafe extern "C" fn ng_appkit_window_should_close(
    _self_obj: *mut c_void,
    _sel: *mut c_void,
    sender: *mut c_void,
) -> u8 {
    let Some(rec) = node_snapshot(sender as usize) else {
        return 1;
    };
    if let Some(ev) = translate_appkit_delegate_event(
        rec.node,
        rec.objc_class,
        rec.event_mask,
        rec.prevent_close,
        "windowShouldClose:",
        "",
        false,
        0,
        rec.last_size,
    ) {
        push_appkit_event(ev);
    }
    u8::from(!rec.prevent_close)
}

#[cfg(target_os = "macos")]
unsafe extern "C" fn ng_appkit_window_did_resize(
    _self_obj: *mut c_void,
    _sel: *mut c_void,
    notification: *mut c_void,
) {
    if notification.is_null() {
        return;
    }
    let Some(dispatch) = APPKIT_DISPATCH.get() else {
        return;
    };
    let msg_ptr0: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
        unsafe { std::mem::transmute_copy::<*mut c_void, _>(&dispatch.objc_msg_send) };
    let win = unsafe { msg_ptr0(notification, dispatch.sel_object) };
    let Some(rec) = node_snapshot(win as usize) else {
        return;
    };
    if let Some(ev) = translate_appkit_delegate_event(
        rec.node,
        rec.objc_class,
        rec.event_mask,
        rec.prevent_close,
        "windowDidResize:",
        "",
        false,
        0,
        rec.last_size,
    ) {
        push_appkit_event(ev);
    }
}

/// Dynamic Objective-C runtime bridge (`objc_msgSend`) for macOS AppKit windows and views.
#[cfg(target_os = "macos")]
pub(super) struct AppKitSys {
    objc_get_class: ObjcGetClassFn,
    sel_register_name: SelRegisterNameFn,
    objc_msg_send: *mut c_void,
    default_runloop_mode: *mut c_void,
    delegate_target: *mut c_void,
}

#[cfg(target_os = "macos")]
impl AppKitSys {
    pub(super) fn try_load() -> Option<Self> {
        let objc = unsafe {
            DynLib::open(c"/usr/lib/libobjc.A.dylib").or_else(|| DynLib::open(c"libobjc.A.dylib"))
        }?;
        let appkit =
            unsafe { DynLib::open(c"/System/Library/Frameworks/AppKit.framework/AppKit") }?;
        let objc_get_class: ObjcGetClassFn = unsafe { objc.sym(c"objc_getClass") }?;
        let sel_register_name: SelRegisterNameFn = unsafe { objc.sym(c"sel_registerName") }?;
        let objc_msg_send: *mut c_void = unsafe { objc.sym(c"objc_msgSend") }?;
        let mode_sym: Option<*const *mut c_void> = unsafe { appkit.sym(c"NSDefaultRunLoopMode") };
        let default_runloop_mode = mode_sym.map_or(std::ptr::null_mut(), |ptr| unsafe { *ptr });

        let _ = APPKIT_DISPATCH.get_or_init(|| unsafe {
            AppKitObjcDispatch {
                objc_msg_send,
                sel_object: sel_register_name(c"object".as_ptr()),
                sel_string_value: sel_register_name(c"stringValue".as_ptr()),
                sel_string: sel_register_name(c"string".as_ptr()),
                sel_utf8_string: sel_register_name(c"UTF8String".as_ptr()),
                sel_state: sel_register_name(c"state".as_ptr()),
                sel_double_value: sel_register_name(c"doubleValue".as_ptr()),
                sel_index_of_selected_item: sel_register_name(c"indexOfSelectedItem".as_ptr()),
            }
        });

        let delegate_target = *APPKIT_DELEGATE_SINGLETON.get_or_init(|| {
            Self::create_delegate_singleton(&objc, objc_get_class, sel_register_name, objc_msg_send)
        }) as *mut c_void;

        Some(Self {
            objc_get_class,
            sel_register_name,
            objc_msg_send,
            default_runloop_mode,
            delegate_target,
        })
    }

    fn create_delegate_singleton(
        objc: &DynLib,
        objc_get_class: ObjcGetClassFn,
        sel_register_name: SelRegisterNameFn,
        objc_msg_send: *mut c_void,
    ) -> usize {
        let Some(alloc_pair) =
            (unsafe { objc.sym::<ObjcAllocateClassPairFn>(c"objc_allocateClassPair") })
        else {
            return 0;
        };
        let Some(register_pair) =
            (unsafe { objc.sym::<ObjcRegisterClassPairFn>(c"objc_registerClassPair") })
        else {
            return 0;
        };
        let Some(add_method) = (unsafe { objc.sym::<ClassAddMethodFn>(c"class_addMethod") }) else {
            return 0;
        };

        let mut target_cls = unsafe { objc_get_class(c"NgAppKitEventTarget".as_ptr()) };
        if target_cls.is_null() {
            let ns_object = unsafe { objc_get_class(c"NSObject".as_ptr()) };
            if ns_object.is_null() {
                return 0;
            }
            target_cls = unsafe { alloc_pair(ns_object, c"NgAppKitEventTarget".as_ptr(), 0) };
            if target_cls.is_null() {
                return 0;
            }
            unsafe {
                let _ = add_method(
                    target_cls,
                    sel_register_name(c"onControlAction:".as_ptr()),
                    (ng_appkit_on_control_action as *const ()).cast::<c_void>(),
                    c"v@:@".as_ptr(),
                );
                let _ = add_method(
                    target_cls,
                    sel_register_name(c"controlTextDidChange:".as_ptr()),
                    (ng_appkit_control_text_did_change as *const ()).cast::<c_void>(),
                    c"v@:@".as_ptr(),
                );
                let _ = add_method(
                    target_cls,
                    sel_register_name(c"textDidChange:".as_ptr()),
                    (ng_appkit_text_did_change as *const ()).cast::<c_void>(),
                    c"v@:@".as_ptr(),
                );
                let _ = add_method(
                    target_cls,
                    sel_register_name(c"windowShouldClose:".as_ptr()),
                    (ng_appkit_window_should_close as *const ()).cast::<c_void>(),
                    c"c@:@".as_ptr(),
                );
                let _ = add_method(
                    target_cls,
                    sel_register_name(c"windowDidResize:".as_ptr()),
                    (ng_appkit_window_did_resize as *const ()).cast::<c_void>(),
                    c"v@:@".as_ptr(),
                );
                register_pair(target_cls);
            }
        }

        let send_ptr0: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
            unsafe { std::mem::transmute_copy::<*mut c_void, _>(&objc_msg_send) };
        let alloc = unsafe { send_ptr0(target_cls, sel_register_name(c"alloc".as_ptr())) };
        if alloc.is_null() {
            return 0;
        }
        let inst = unsafe { send_ptr0(alloc, sel_register_name(c"init".as_ptr())) };
        inst as usize
    }

    fn cls(&self, name: &CStr) -> *mut c_void {
        unsafe { (self.objc_get_class)(name.as_ptr()) }
    }

    fn sel(&self, name: &CStr) -> *mut c_void {
        unsafe { (self.sel_register_name)(name.as_ptr()) }
    }

    unsafe fn send_ptr0(&self, target: *mut c_void, selector: &CStr) -> *mut c_void {
        if target.is_null() {
            return std::ptr::null_mut();
        }
        let f: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
            unsafe { std::mem::transmute_copy::<*mut c_void, _>(&self.objc_msg_send) };
        unsafe { f(target, self.sel(selector)) }
    }

    unsafe fn send_void0(&self, target: *mut c_void, selector: &CStr) {
        if target.is_null() {
            return;
        }
        let f: unsafe extern "C" fn(*mut c_void, *mut c_void) =
            unsafe { std::mem::transmute_copy::<*mut c_void, _>(&self.objc_msg_send) };
        unsafe { f(target, self.sel(selector)) };
    }

    unsafe fn send_void1(&self, target: *mut c_void, selector: &CStr, arg: *mut c_void) {
        if target.is_null() {
            return;
        }
        let f: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
            unsafe { std::mem::transmute_copy::<*mut c_void, _>(&self.objc_msg_send) };
        unsafe { f(target, self.sel(selector), arg) };
    }

    unsafe fn send_void_bool(&self, target: *mut c_void, selector: &CStr, flag: bool) {
        if target.is_null() {
            return;
        }
        let f: unsafe extern "C" fn(*mut c_void, *mut c_void, u8) =
            unsafe { std::mem::transmute_copy::<*mut c_void, _>(&self.objc_msg_send) };
        unsafe { f(target, self.sel(selector), u8::from(flag)) };
    }

    fn make_nsstring(&self, text: &str) -> *mut c_void {
        let Ok(c_str) = CString::new(text) else {
            return std::ptr::null_mut();
        };
        let ns_string_cls = self.cls(c"NSString");
        if ns_string_cls.is_null() {
            return std::ptr::null_mut();
        }
        let f: unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_char) -> *mut c_void =
            unsafe { std::mem::transmute_copy::<*mut c_void, _>(&self.objc_msg_send) };
        unsafe {
            f(
                ns_string_cls,
                self.sel(c"stringWithUTF8String:"),
                c_str.as_ptr(),
            )
        }
    }

    pub(super) fn probe_nsstring_roundtrip(&self, text: &str) -> Option<String> {
        let ns_str = self.make_nsstring(text);
        if ns_str.is_null() {
            return None;
        }
        let f: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *const c_char =
            unsafe { std::mem::transmute_copy::<*mut c_void, _>(&self.objc_msg_send) };
        let utf8 = unsafe { f(ns_str, self.sel(c"UTF8String")) };
        if utf8.is_null() {
            return None;
        }
        Some(
            unsafe { CStr::from_ptr(utf8) }
                .to_string_lossy()
                .into_owned(),
        )
    }

    fn ensure_shared_application(&self) -> *mut c_void {
        let ns_app_cls = self.cls(c"NSApplication");
        let shared = unsafe { self.send_ptr0(ns_app_cls, c"sharedApplication") };
        let _ = NSAPP_READY.get_or_init(|| {
            if shared.is_null() {
                return false;
            }
            let set_policy: unsafe extern "C" fn(*mut c_void, *mut c_void, isize) -> u8 =
                unsafe { std::mem::transmute_copy::<*mut c_void, _>(&self.objc_msg_send) };
            let _ = unsafe { set_policy(shared, self.sel(c"setActivationPolicy:"), 0) };
            true
        });
        shared
    }

    pub(super) fn create_window(
        &self,
        node: NodeId,
        is_panel: bool,
        width: f64,
        height: f64,
    ) -> Option<AppKitHandle> {
        let _ = self.ensure_shared_application();
        let cls = self.cls(if is_panel { c"NSPanel" } else { c"NSWindow" });
        let alloc = unsafe { self.send_ptr0(cls, c"alloc") };
        if alloc.is_null() {
            return None;
        }
        let rect = NsRect {
            origin: NsPoint { x: 80.0, y: 80.0 },
            size: NsSize { width, height },
        };
        let init_win: unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            NsRect,
            u64,
            u64,
            u8,
        ) -> *mut c_void =
            unsafe { std::mem::transmute_copy::<*mut c_void, _>(&self.objc_msg_send) };
        let win = unsafe {
            init_win(
                alloc,
                self.sel(c"initWithContentRect:styleMask:backing:defer:"),
                rect,
                appkit_window_style_mask(is_panel),
                2, // NSBackingStoreBuffered
                0,
            )
        };
        if win.is_null() {
            return None;
        }
        unsafe {
            self.send_void_bool(win, c"setReleasedWhenClosed:", false);
            if !self.delegate_target.is_null() {
                self.send_void1(win, c"setDelegate:", self.delegate_target);
            }
        }
        let handle = AppKitHandle(win as usize);
        node_map()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(
                handle.0,
                AppKitNodeRecord {
                    node,
                    objc_class: if is_panel { "NSPanel" } else { "NSWindow" },
                    event_mask: EventMask::NONE,
                    prevent_close: false,
                    suppress_change: false,
                    #[allow(clippy::cast_possible_truncation)]
                    last_size: (width.round() as i32, height.round() as i32),
                },
            );
        Some(handle)
    }

    pub(super) fn create_view(
        &self,
        node: NodeId,
        objc_class: &'static str,
        rect: NsRect,
    ) -> Option<AppKitHandle> {
        let runtime_cls = match objc_class {
            "NSTextFieldEditable" => "NSTextField",
            "NSButtonCheckbox" | "NSDisclosureButton" => "NSButton",
            "NSBoxSeparator" => "NSBox",
            "NSPopover" => "NSView",
            other => other,
        };
        let c_cls = CString::new(runtime_cls).ok()?;
        let cls = self.cls(&c_cls);
        let alloc = unsafe { self.send_ptr0(cls, c"alloc") };
        if alloc.is_null() {
            return None;
        }
        let init_frame: unsafe extern "C" fn(*mut c_void, *mut c_void, NsRect) -> *mut c_void =
            unsafe { std::mem::transmute_copy::<*mut c_void, _>(&self.objc_msg_send) };
        let view = unsafe { init_frame(alloc, self.sel(c"initWithFrame:"), rect) };
        if view.is_null() {
            return None;
        }
        if !self.delegate_target.is_null() {
            if matches!(
                objc_class,
                "NSButton"
                    | "NSButtonCheckbox"
                    | "NSDisclosureButton"
                    | "NSSwitch"
                    | "NSSlider"
                    | "NSStepper"
                    | "NSPopUpButton"
                    | "NSTextFieldEditable"
                    | "NSSecureTextField"
                    | "NSSearchField"
            ) {
                unsafe {
                    self.send_void1(view, c"setTarget:", self.delegate_target);
                    self.send_void1(view, c"setAction:", self.sel(c"onControlAction:"));
                }
            }
            if matches!(
                objc_class,
                "NSTextFieldEditable" | "NSSecureTextField" | "NSSearchField" | "NSTextView"
            ) {
                unsafe {
                    self.send_void1(view, c"setDelegate:", self.delegate_target);
                }
            }
        }
        let handle = AppKitHandle(view as usize);
        node_map()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(
                handle.0,
                AppKitNodeRecord {
                    node,
                    objc_class,
                    event_mask: EventMask::NONE,
                    prevent_close: false,
                    suppress_change: false,
                    #[allow(clippy::cast_possible_truncation)]
                    last_size: (
                        rect.size.width.round() as i32,
                        rect.size.height.round() as i32,
                    ),
                },
            );
        Some(handle)
    }

    pub(super) fn set_frame(&self, handle: AppKitHandle, rect: NsRect) {
        let mut guard = node_map()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(rec) = guard.get_mut(&handle.0) {
            #[allow(clippy::cast_possible_truncation)]
            {
                rec.last_size = (
                    rect.size.width.round() as i32,
                    rect.size.height.round() as i32,
                );
            }
        }
        drop(guard);
        let f: unsafe extern "C" fn(*mut c_void, *mut c_void, NsRect) =
            unsafe { std::mem::transmute_copy::<*mut c_void, _>(&self.objc_msg_send) };
        unsafe { f(handle.0 as *mut c_void, self.sel(c"setFrame:"), rect) };
    }

    pub(super) fn set_event_mask(handle: AppKitHandle, mask: EventMask) {
        let mut guard = node_map()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(rec) = guard.get_mut(&handle.0) {
            rec.event_mask = rec.event_mask.union(mask);
        }
    }

    pub(super) fn set_prevent_close(handle: AppKitHandle, prevent: bool) {
        let mut guard = node_map()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(rec) = guard.get_mut(&handle.0) {
            rec.prevent_close = prevent;
            if prevent {
                rec.event_mask = rec.event_mask.union(EventMask::CLOSE_REQUEST);
            }
        }
    }

    pub(super) fn set_title(&self, handle: AppKitHandle, title: &str) {
        let ns = self.make_nsstring(title);
        if !ns.is_null() {
            unsafe { self.send_void1(handle.0 as *mut c_void, c"setTitle:", ns) };
        }
    }

    pub(super) fn set_string_value(&self, handle: AppKitHandle, text: &str) {
        let mut guard = node_map()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(rec) = guard.get_mut(&handle.0) {
            rec.suppress_change = true;
        }
        drop(guard);
        let ns = self.make_nsstring(text);
        if !ns.is_null() {
            unsafe { self.send_void1(handle.0 as *mut c_void, c"setStringValue:", ns) };
        }
        let mut guard = node_map()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(rec) = guard.get_mut(&handle.0) {
            rec.suppress_change = false;
        }
    }

    pub(super) fn add_subview(
        &self,
        parent: AppKitHandle,
        parent_is_window: bool,
        child: AppKitHandle,
    ) {
        let target = if parent_is_window {
            unsafe { self.send_ptr0(parent.0 as *mut c_void, c"contentView") }
        } else {
            parent.0 as *mut c_void
        };
        if !target.is_null() {
            unsafe { self.send_void1(target, c"addSubview:", child.0 as *mut c_void) };
        }
    }

    pub(super) fn remove_from_superview(&self, child: AppKitHandle) {
        unsafe { self.send_void0(child.0 as *mut c_void, c"removeFromSuperview") };
    }

    pub(super) fn set_hidden(&self, handle: AppKitHandle, hidden: bool) {
        unsafe { self.send_void_bool(handle.0 as *mut c_void, c"setHidden:", hidden) };
    }

    pub(super) fn present_window(&self, win: AppKitHandle) {
        unsafe {
            self.send_void1(
                win.0 as *mut c_void,
                c"makeKeyAndOrderFront:",
                std::ptr::null_mut(),
            );
        }
    }

    pub(super) fn close_window(&self, win: AppKitHandle) {
        unsafe { self.send_void0(win.0 as *mut c_void, c"close") };
    }

    pub(super) fn is_window_visible(&self, win: AppKitHandle) -> bool {
        let f: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u8 =
            unsafe { std::mem::transmute_copy::<*mut c_void, _>(&self.objc_msg_send) };
        unsafe { f(win.0 as *mut c_void, self.sel(c"isVisible")) != 0 }
    }

    pub(super) fn pump_events(&self) -> Vec<NativeEvent> {
        let shared = self.ensure_shared_application();
        if !shared.is_null() {
            let mode = if self.default_runloop_mode.is_null() {
                self.make_nsstring("kCFRunLoopDefaultMode")
            } else {
                self.default_runloop_mode
            };
            if !mode.is_null() {
                let next_event: unsafe extern "C" fn(
                    *mut c_void,
                    *mut c_void,
                    u64,
                    *mut c_void,
                    *mut c_void,
                    u8,
                ) -> *mut c_void =
                    unsafe { std::mem::transmute_copy::<*mut c_void, _>(&self.objc_msg_send) };
                let sel = self.sel(c"nextEventMatchingMask:untilDate:inMode:dequeue:");
                for _ in 0..64 {
                    let ev =
                        unsafe { next_event(shared, sel, u64::MAX, std::ptr::null_mut(), mode, 1) };
                    if ev.is_null() {
                        break;
                    }
                    unsafe { self.send_void1(shared, c"sendEvent:", ev) };
                }
            }
        }
        let mut guard = event_queue()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        coalesce_frame_events(std::mem::take(&mut *guard))
    }

    pub(super) fn release(&self, handle: AppKitHandle) {
        node_map()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&handle.0);
        unsafe { self.send_void0(handle.0 as *mut c_void, c"release") };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs_nsrect_and_appkit_style_masks() {
        let rect = NsRect {
            origin: NsPoint { x: 10.0, y: 20.0 },
            size: NsSize {
                width: 800.0,
                height: 600.0,
            },
        };
        assert_eq!(size_of::<NsRect>(), 32);
        assert!((rect.size.width - 800.0).abs() < f64::EPSILON);
        assert_eq!(appkit_window_style_mask(false), 15);
        assert_eq!(appkit_window_style_mask(true), 11);
    }

    #[test]
    fn translates_appkit_delegate_and_control_actions() {
        let node = NodeId {
            index: 3,
            generation: 1,
        };
        let click = translate_appkit_delegate_event(
            node,
            "NSButton",
            EventMask::CLICK,
            false,
            "action",
            "",
            false,
            0,
            (0, 0),
        )
        .unwrap();
        assert_eq!(click.event_name, "clicked");
        assert_eq!(click.payload, EventPayload::None);

        let toggle = translate_appkit_delegate_event(
            node,
            "NSSwitch",
            EventMask::VALUE_CHANGE,
            false,
            "action",
            "",
            true,
            0,
            (0, 0),
        )
        .unwrap();
        assert_eq!(toggle.event_name, "toggled");
        assert_eq!(toggle.payload, EventPayload::Bool(true));

        let close = translate_appkit_delegate_event(
            node,
            "NSWindow",
            EventMask::CLOSE_REQUEST,
            true,
            "windowShouldClose:",
            "",
            false,
            0,
            (1020, 720),
        )
        .unwrap();
        assert_eq!(close.event_name, "close-request");
        assert_eq!(
            close.payload,
            EventPayload::Text(String::from(r#"{"preventable":true}"#))
        );
    }
}
