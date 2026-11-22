#![allow(unsafe_code)]

#[cfg(target_os = "macos")]
use crate::ffi_dl::DynLib;
#[cfg(target_os = "macos")]
use std::ffi::{CStr, CString, c_char, c_int, c_void};
#[cfg(target_os = "macos")]
use std::sync::OnceLock;

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
static NSAPP_READY: OnceLock<bool> = OnceLock::new();

/// Dynamic Objective-C runtime bridge (`objc_msgSend`) for macOS AppKit windows and views.
#[cfg(target_os = "macos")]
pub(super) struct AppKitSys {
    objc_get_class: ObjcGetClassFn,
    sel_register_name: SelRegisterNameFn,
    objc_msg_send: *mut c_void,
    default_runloop_mode: *mut c_void,
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

        Some(Self {
            objc_get_class,
            sel_register_name,
            objc_msg_send,
            default_runloop_mode,
        })
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
        }
        Some(AppKitHandle(win as usize))
    }

    pub(super) fn create_view(&self, objc_class: &str, rect: NsRect) -> Option<AppKitHandle> {
        let c_cls = CString::new(objc_class).ok()?;
        let cls = self.cls(&c_cls);
        let alloc = unsafe { self.send_ptr0(cls, c"alloc") };
        if alloc.is_null() {
            return None;
        }
        let init_frame: unsafe extern "C" fn(*mut c_void, *mut c_void, NsRect) -> *mut c_void =
            unsafe { std::mem::transmute_copy::<*mut c_void, _>(&self.objc_msg_send) };
        let view = unsafe { init_frame(alloc, self.sel(c"initWithFrame:"), rect) };
        if view.is_null() {
            None
        } else {
            Some(AppKitHandle(view as usize))
        }
    }

    pub(super) fn set_title(&self, handle: AppKitHandle, title: &str) {
        let ns = self.make_nsstring(title);
        if !ns.is_null() {
            unsafe { self.send_void1(handle.0 as *mut c_void, c"setTitle:", ns) };
        }
    }

    pub(super) fn set_string_value(&self, handle: AppKitHandle, text: &str) {
        let ns = self.make_nsstring(text);
        if !ns.is_null() {
            unsafe { self.send_void1(handle.0 as *mut c_void, c"setStringValue:", ns) };
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

    pub(super) fn pump_events(&self) {
        let shared = self.ensure_shared_application();
        if shared.is_null() {
            return;
        }
        let mode = if self.default_runloop_mode.is_null() {
            self.make_nsstring("kCFRunLoopDefaultMode")
        } else {
            self.default_runloop_mode
        };
        if mode.is_null() {
            return;
        }
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
            let ev = unsafe { next_event(shared, sel, u64::MAX, std::ptr::null_mut(), mode, 1) };
            if ev.is_null() {
                break;
            }
            unsafe { self.send_void1(shared, c"sendEvent:", ev) };
        }
    }

    pub(super) fn release(&self, handle: AppKitHandle) {
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
}
