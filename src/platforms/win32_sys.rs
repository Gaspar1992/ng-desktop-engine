#![allow(unsafe_code)]

#[cfg(target_os = "windows")]
use super::windows_winui::WinUiControlBinding;
#[cfg(target_os = "windows")]
use crate::engine::{EventMask, coalesce_frame_events};
use crate::engine::{EventPayload, NativeEvent, NodeId};
#[cfg(target_os = "windows")]
use crate::ffi_dl::DynLib;
#[cfg(target_os = "windows")]
use std::collections::HashMap;
#[cfg(target_os = "windows")]
use std::ffi::{c_int, c_void};
#[cfg(target_os = "windows")]
use std::sync::{Mutex, OnceLock};

const BN_CLICKED: u16 = 0;
const EN_CHANGE: u16 = 0x0300;
const CBN_SELCHANGE: u16 = 1;

/// Translates a Win32 `WM_COMMAND` notification into a typed engine `NativeEvent`.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn translate_win32_control_command(
    node: NodeId,
    win32_class: &str,
    xaml_class: &str,
    notify_code: u16,
    text_value: &str,
    is_checked: bool,
    selected_index: i32,
) -> Option<NativeEvent> {
    match (win32_class, notify_code) {
        ("BUTTON", BN_CLICKED) => {
            if matches!(
                xaml_class,
                "Microsoft.UI.Xaml.Controls.CheckBox" | "Microsoft.UI.Xaml.Controls.ToggleSwitch"
            ) {
                Some(NativeEvent {
                    node,
                    event_name: String::from("toggled"),
                    payload: EventPayload::Bool(is_checked),
                })
            } else {
                Some(NativeEvent {
                    node,
                    event_name: String::from("clicked"),
                    payload: EventPayload::None,
                })
            }
        }
        ("EDIT", EN_CHANGE) => Some(NativeEvent {
            node,
            event_name: String::from("input"),
            payload: EventPayload::Text(text_value.to_string()),
        }),
        ("COMBOBOX", CBN_SELCHANGE) => Some(NativeEvent {
            node,
            event_name: String::from("changed"),
            payload: EventPayload::Int(i64::from(selected_index)),
        }),
        _ => None,
    }
}

/// Opaque Win32 `HWND` integer handle isolated behind the FFI boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Win32Handle(pub(super) usize);

#[must_use]
pub fn is_headless_env() -> bool {
    std::env::var("NG_DESKTOP_HEADLESS").is_ok_and(|raw| {
        matches!(
            raw.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes"
        )
    })
}

#[must_use]
pub fn to_wide_null(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

#[must_use]
pub fn decode_win32_size(lparam: isize) -> (i32, i32) {
    let bits = usize::from_ne_bytes(lparam.to_ne_bytes());
    let lo = u16::try_from(bits & 0xFFFF).unwrap_or(0);
    let hi = u16::try_from((bits >> 16) & 0xFFFF).unwrap_or(0);
    (
        i32::from(i16::from_ne_bytes(lo.to_ne_bytes())),
        i32::from(i16::from_ne_bytes(hi.to_ne_bytes())),
    )
}

#[must_use]
pub fn decode_win32_wheel_delta(wparam: usize) -> f64 {
    let hi = u16::try_from((wparam >> 16) & 0xFFFF).unwrap_or(0);
    let signed = i16::from_ne_bytes(hi.to_ne_bytes());
    f64::from(signed) / 120.0
}

#[must_use]
pub fn win32_show_cmd(visible: bool) -> i32 {
    if !visible {
        return 0; // SW_HIDE
    }
    let in_ci = std::env::var("CI").is_ok_and(|raw| {
        let trimmed = raw.trim();
        !trimmed.is_empty() && trimmed != "0" && !trimmed.eq_ignore_ascii_case("false")
    });
    if in_ci { 4 } else { 5 } // SW_SHOWNOACTIVATE vs SW_SHOW
}

pub const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
pub const DWMWCP_DEFAULT: i32 = 0;
pub const DWMWCP_DONOTROUND: i32 = 1;
pub const DWMWCP_ROUND: i32 = 2;
pub const DWMWCP_ROUNDSMALL: i32 = 3;

pub const DWMSBT_AUTO: i32 = 0;
pub const DWMSBT_NONE: i32 = 1;
pub const DWMSBT_MAINWINDOW: i32 = 2;
pub const DWMSBT_TRANSIENTWINDOW: i32 = 3;
pub const DWMSBT_TABBEDWINDOW: i32 = 4;

/// Resolves Windows 11 `DWMWA_WINDOW_CORNER_PREFERENCE` from inline CSS `border-radius`.
#[must_use]
pub fn resolve_dwm_corner_preference(style_css: &str) -> i32 {
    for decl in style_css.split(';') {
        let Some((key, val)) = decl.split_once(':') else {
            continue;
        };
        if key.trim().eq_ignore_ascii_case("border-radius") {
            let trimmed = val.trim();
            let numeric = trimmed.strip_suffix("px").map_or(trimmed, str::trim);
            if let Ok(radius) = numeric.parse::<f64>()
                && radius.is_finite()
                && radius >= 0.0
            {
                if radius == 0.0 {
                    return DWMWCP_DONOTROUND;
                }
                if radius <= 5.0 {
                    return DWMWCP_ROUNDSMALL;
                }
                return DWMWCP_ROUND;
            }
        }
    }
    DWMWCP_DEFAULT
}

/// Resolves Windows 11 `DWMWA_SYSTEMBACKDROP_TYPE` from a declarative backdrop name.
#[must_use]
pub fn resolve_dwm_backdrop_type(backdrop: &str) -> i32 {
    match backdrop.trim().to_ascii_lowercase().as_str() {
        "mica" | "mainwindow" => DWMSBT_MAINWINDOW,
        "acrylic" | "transient" => DWMSBT_TRANSIENTWINDOW,
        "tabbed" | "mica-alt" => DWMSBT_TABBEDWINDOW,
        "none" => DWMSBT_NONE,
        _ => DWMSBT_AUTO,
    }
}

#[cfg(target_os = "windows")]
const WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000;
#[cfg(target_os = "windows")]
const WS_POPUPWINDOW: u32 = 0x8088_0000;
#[cfg(target_os = "windows")]
const WS_CAPTION: u32 = 0x00C0_0000;
#[cfg(target_os = "windows")]
const WS_CHILD: u32 = 0x4000_0000;
#[cfg(target_os = "windows")]
const WS_VISIBLE: u32 = 0x1000_0000;
#[cfg(target_os = "windows")]
const WS_CLIPCHILDREN: u32 = 0x0200_0000;
#[cfg(target_os = "windows")]
const BS_AUTOCHECKBOX: u32 = 0x0000_0003;
#[cfg(target_os = "windows")]
const ES_AUTOHSCROLL: u32 = 0x0000_0080;
#[cfg(target_os = "windows")]
const CBS_DROPDOWNLIST: u32 = 0x0000_0003;

#[cfg(target_os = "windows")]
const WM_DESTROY: u32 = 0x0002;
#[cfg(target_os = "windows")]
const WM_SIZE: u32 = 0x0005;
#[cfg(target_os = "windows")]
const WM_SETFOCUS: u32 = 0x0007;
#[cfg(target_os = "windows")]
const WM_KILLFOCUS: u32 = 0x0008;
#[cfg(target_os = "windows")]
const WM_CLOSE: u32 = 0x0010;
#[cfg(target_os = "windows")]
const WM_KEYDOWN: u32 = 0x0100;
#[cfg(target_os = "windows")]
const WM_KEYUP: u32 = 0x0101;
#[cfg(target_os = "windows")]
const WM_COMMAND: u32 = 0x0111;
#[cfg(target_os = "windows")]
const WM_CONTEXTMENU: u32 = 0x007B;
#[cfg(target_os = "windows")]
const WM_MOUSEWHEEL: u32 = 0x020A;
#[cfg(target_os = "windows")]
const PM_REMOVE: u32 = 0x0001;
#[cfg(target_os = "windows")]
const SIZE_MAXIMIZED: usize = 2;
#[cfg(target_os = "windows")]
const BM_GETCHECK: u32 = 0x00F0;
#[cfg(target_os = "windows")]
const BST_CHECKED: isize = 1;
#[cfg(target_os = "windows")]
const CB_GETCURSEL: u32 = 0x0147;
#[cfg(target_os = "windows")]
const NG_WIN32_CLASS_NAME: &str = "NgDesktopWindowClass";

#[cfg(target_os = "windows")]
type WndProcFn = unsafe extern "system" fn(*mut c_void, u32, usize, isize) -> isize;
#[cfg(target_os = "windows")]
type RegisterClassExWFn = unsafe extern "system" fn(*const WndClassExW) -> u16;
#[cfg(target_os = "windows")]
type CreateWindowExWFn = unsafe extern "system" fn(
    u32,
    *const u16,
    *const u16,
    u32,
    c_int,
    c_int,
    c_int,
    c_int,
    *mut c_void,
    *mut c_void,
    *mut c_void,
    *mut c_void,
) -> *mut c_void;
#[cfg(target_os = "windows")]
type DestroyWindowFn = unsafe extern "system" fn(*mut c_void) -> c_int;
#[cfg(target_os = "windows")]
type ShowWindowFn = unsafe extern "system" fn(*mut c_void, c_int) -> c_int;
#[cfg(target_os = "windows")]
type SetWindowTextWFn = unsafe extern "system" fn(*mut c_void, *const u16) -> c_int;
#[cfg(target_os = "windows")]
type MoveWindowFn =
    unsafe extern "system" fn(*mut c_void, c_int, c_int, c_int, c_int, c_int) -> c_int;
#[cfg(target_os = "windows")]
type IsWindowFn = unsafe extern "system" fn(*mut c_void) -> c_int;
#[cfg(target_os = "windows")]
type PeekMessageWFn = unsafe extern "system" fn(*mut Win32Msg, *mut c_void, u32, u32, u32) -> c_int;
#[cfg(target_os = "windows")]
type TranslateMessageFn = unsafe extern "system" fn(*const Win32Msg) -> c_int;
#[cfg(target_os = "windows")]
type DispatchMessageWFn = unsafe extern "system" fn(*const Win32Msg) -> isize;
#[cfg(target_os = "windows")]
type EnableWindowFn = unsafe extern "system" fn(*mut c_void, c_int) -> c_int;
#[cfg(target_os = "windows")]
type GetWindowTextLengthWFn = unsafe extern "system" fn(*mut c_void) -> c_int;
#[cfg(target_os = "windows")]
type GetWindowTextWFn = unsafe extern "system" fn(*mut c_void, *mut u16, c_int) -> c_int;
#[cfg(target_os = "windows")]
type SendMessageWFn = unsafe extern "system" fn(*mut c_void, u32, usize, isize) -> isize;
#[cfg(target_os = "windows")]
type GetModuleHandleWFn = unsafe extern "system" fn(*const u16) -> *mut c_void;
#[cfg(target_os = "windows")]
type GetLastErrorFn = unsafe extern "system" fn() -> u32;
#[cfg(target_os = "windows")]
type DwmSetWindowAttributeFn =
    unsafe extern "system" fn(*mut c_void, u32, *const c_void, u32) -> i32;
#[cfg(target_os = "windows")]
const ERROR_CLASS_ALREADY_EXISTS: u32 = 1410;
#[cfg(target_os = "windows")]
const DWMWA_USE_IMMERSIVE_DARK_MODE_BEFORE_20H1: u32 = 19;
#[cfg(target_os = "windows")]
const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
#[cfg(target_os = "windows")]
const DWMWA_SYSTEMBACKDROP_TYPE: u32 = 38;

#[cfg(target_os = "windows")]
#[repr(C)]
struct WndClassExW {
    cb_size: u32,
    style: u32,
    lpfn_wnd_proc: WndProcFn,
    cb_cls_extra: c_int,
    cb_wnd_extra: c_int,
    h_instance: *mut c_void,
    h_icon: *mut c_void,
    h_cursor: *mut c_void,
    hbr_background: *mut c_void,
    lpsz_menu_name: *const u16,
    lpsz_class_name: *const u16,
    h_icon_sm: *mut c_void,
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct Win32Point {
    x: i32,
    y: i32,
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct Win32Msg {
    hwnd: *mut c_void,
    message: u32,
    w_param: usize,
    l_param: isize,
    time: u32,
    pt: Win32Point,
    l_private: u32,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone)]
struct Win32WindowRecord {
    node: NodeId,
    win32_class: &'static str,
    xaml_class: &'static str,
    event_mask: EventMask,
    prevent_close: bool,
    suppress_change: bool,
}

#[cfg(target_os = "windows")]
static WIN32_CLASS_REGISTERED: OnceLock<bool> = OnceLock::new();
#[cfg(target_os = "windows")]
static WIN32_DEF_WNDPROC: OnceLock<WndProcFn> = OnceLock::new();
#[cfg(target_os = "windows")]
static WIN32_GET_TEXT_LEN_W: OnceLock<GetWindowTextLengthWFn> = OnceLock::new();
#[cfg(target_os = "windows")]
static WIN32_GET_TEXT_W: OnceLock<GetWindowTextWFn> = OnceLock::new();
#[cfg(target_os = "windows")]
static WIN32_SEND_MSG_W: OnceLock<SendMessageWFn> = OnceLock::new();
#[cfg(target_os = "windows")]
static WIN32_HWND_MAP: OnceLock<Mutex<HashMap<usize, Win32WindowRecord>>> = OnceLock::new();
#[cfg(target_os = "windows")]
static WIN32_EVENTS: OnceLock<Mutex<Vec<NativeEvent>>> = OnceLock::new();

#[cfg(target_os = "windows")]
fn hwnd_map() -> &'static Mutex<HashMap<usize, Win32WindowRecord>> {
    WIN32_HWND_MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(target_os = "windows")]
fn event_queue() -> &'static Mutex<Vec<NativeEvent>> {
    WIN32_EVENTS.get_or_init(|| Mutex::new(Vec::new()))
}

#[cfg(target_os = "windows")]
fn push_win32_event(event: NativeEvent) {
    let mut guard = event_queue()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.push(event);
}

#[cfg(target_os = "windows")]
fn record_snapshot(hwnd_key: usize) -> Option<Win32WindowRecord> {
    let guard = hwnd_map()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.get(&hwnd_key).cloned()
}

#[cfg(target_os = "windows")]
fn read_hwnd_text(hwnd: *mut c_void) -> String {
    let (Some(get_len), Some(get_text)) = (WIN32_GET_TEXT_LEN_W.get(), WIN32_GET_TEXT_W.get())
    else {
        return String::new();
    };
    let len = unsafe { get_len(hwnd) };
    if len <= 0 {
        return String::new();
    }
    let cap = usize::try_from(len).unwrap_or(0) + 1;
    let mut buf = vec![0u16; cap];
    let copied = unsafe { get_text(hwnd, buf.as_mut_ptr(), c_int::try_from(cap).unwrap_or(0)) };
    let used = usize::try_from(copied.max(0)).unwrap_or(0).min(buf.len());
    String::from_utf16_lossy(&buf[..used])
}

#[cfg(target_os = "windows")]
fn handle_wm_command(wparam: usize, lparam: isize) {
    let child_key = usize::from_ne_bytes(lparam.to_ne_bytes());
    let Some(child) = record_snapshot(child_key) else {
        return;
    };
    if child.suppress_change {
        return;
    }
    let notify = u16::try_from((wparam >> 16) & 0xFFFF).unwrap_or(0);
    let child_hwnd = child_key as *mut c_void;
    let text_value = if child.win32_class == "EDIT" && notify == EN_CHANGE {
        read_hwnd_text(child_hwnd)
    } else {
        String::new()
    };
    let is_checked = if child.win32_class == "BUTTON" && notify == BN_CLICKED {
        WIN32_SEND_MSG_W
            .get()
            .is_some_and(|send| unsafe { send(child_hwnd, BM_GETCHECK, 0, 0) } == BST_CHECKED)
    } else {
        false
    };
    let selected_index = if child.win32_class == "COMBOBOX" && notify == CBN_SELCHANGE {
        WIN32_SEND_MSG_W.get().map_or(-1, |send| {
            i32::try_from(unsafe { send(child_hwnd, CB_GETCURSEL, 0, 0) }).unwrap_or(-1)
        })
    } else {
        -1
    };

    if let Some(ev) = translate_win32_control_command(
        child.node,
        child.win32_class,
        child.xaml_class,
        notify,
        &text_value,
        is_checked,
        selected_index,
    ) {
        push_win32_event(ev);
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn ng_win32_wndproc(
    hwnd: *mut c_void,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    let key = hwnd as usize;
    if msg == WM_DESTROY {
        hwnd_map()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&key);
        return 0;
    }

    if msg == WM_COMMAND {
        handle_wm_command(wparam, lparam);
    } else if let Some(rec) = record_snapshot(key) {
        match msg {
            WM_CLOSE => {
                if rec.prevent_close || rec.event_mask.contains(EventMask::CLOSE_REQUEST) {
                    push_win32_event(NativeEvent {
                        node: rec.node,
                        event_name: String::from("close-request"),
                        payload: EventPayload::Text(format!(
                            r#"{{"preventable":{}}}"#,
                            rec.prevent_close
                        )),
                    });
                }
                if rec.prevent_close {
                    return 0;
                }
            }
            WM_SIZE if rec.event_mask.contains(EventMask::WINDOW_STATE) => {
                let (w, h) = decode_win32_size(lparam);
                let maximized = wparam == SIZE_MAXIMIZED;
                push_win32_event(NativeEvent {
                    node: rec.node,
                    event_name: String::from("resize"),
                    payload: EventPayload::Text(format!(
                        r#"{{"width":{w},"height":{h},"maximized":{maximized}}}"#
                    )),
                });
            }
            WM_SETFOCUS | WM_KILLFOCUS if rec.event_mask.contains(EventMask::FOCUS) => {
                let active = msg == WM_SETFOCUS;
                push_win32_event(NativeEvent {
                    node: rec.node,
                    event_name: String::from(if active { "focus" } else { "blur" }),
                    payload: EventPayload::Bool(active),
                });
            }
            WM_MOUSEWHEEL if rec.event_mask.contains(EventMask::WHEEL) => {
                let delta = decode_win32_wheel_delta(wparam);
                push_win32_event(NativeEvent {
                    node: rec.node,
                    event_name: String::from("wheel"),
                    payload: EventPayload::Text(format!(r#"{{"deltaX":0,"deltaY":{delta}}}"#)),
                });
            }
            WM_CONTEXTMENU if rec.event_mask.contains(EventMask::CONTEXT_MENU) => {
                let (x, y) = decode_win32_size(lparam);
                push_win32_event(NativeEvent {
                    node: rec.node,
                    event_name: String::from("contextmenu"),
                    payload: EventPayload::Text(format!(r#"{{"x":{x},"y":{y},"button":3}}"#)),
                });
            }
            WM_KEYDOWN | WM_KEYUP if rec.event_mask.contains(EventMask::KEYBOARD) => {
                let name = if msg == WM_KEYDOWN {
                    "keydown"
                } else {
                    "keyup"
                };
                push_win32_event(NativeEvent {
                    node: rec.node,
                    event_name: String::from(name),
                    payload: EventPayload::Text(format!(r#"{{"keyval":{wparam}}}"#)),
                });
            }
            _ => {}
        }
    }

    WIN32_DEF_WNDPROC
        .get()
        .map_or(0, |def_proc| unsafe { def_proc(hwnd, msg, wparam, lparam) })
}

/// Dynamic `user32.dll` / `kernel32.dll` Win32 FFI driver for live Windows windows and controls.
#[cfg(target_os = "windows")]
pub(super) struct Win32Sys {
    h_instance: *mut c_void,
    create_window_ex_w: CreateWindowExWFn,
    destroy_window: DestroyWindowFn,
    show_window: ShowWindowFn,
    set_window_text_w: SetWindowTextWFn,
    move_window: MoveWindowFn,
    is_window: IsWindowFn,
    peek_message_w: PeekMessageWFn,
    translate_message: TranslateMessageFn,
    dispatch_message_w: DispatchMessageWFn,
    enable_window: EnableWindowFn,
    dwm_set_window_attribute: Option<DwmSetWindowAttributeFn>,
}

#[cfg(target_os = "windows")]
impl Win32Sys {
    pub(super) fn try_load() -> Option<Self> {
        let user32 = unsafe { DynLib::open(c"user32.dll") }?;
        let kernel32 = unsafe { DynLib::open(c"kernel32.dll") }?;
        let dwm_set_window_attribute: Option<DwmSetWindowAttributeFn> =
            unsafe { DynLib::open(c"dwmapi.dll") }
                .and_then(|lib| unsafe { lib.sym(c"DwmSetWindowAttribute") });

        let register_class_ex_w: RegisterClassExWFn = unsafe { user32.sym(c"RegisterClassExW") }?;
        let create_window_ex_w: CreateWindowExWFn = unsafe { user32.sym(c"CreateWindowExW") }?;
        let destroy_window: DestroyWindowFn = unsafe { user32.sym(c"DestroyWindow") }?;
        let show_window: ShowWindowFn = unsafe { user32.sym(c"ShowWindow") }?;
        let set_window_text_w: SetWindowTextWFn = unsafe { user32.sym(c"SetWindowTextW") }?;
        let move_window: MoveWindowFn = unsafe { user32.sym(c"MoveWindow") }?;
        let is_window: IsWindowFn = unsafe { user32.sym(c"IsWindow") }?;
        let peek_message_w: PeekMessageWFn = unsafe { user32.sym(c"PeekMessageW") }?;
        let translate_message: TranslateMessageFn = unsafe { user32.sym(c"TranslateMessage") }?;
        let dispatch_message_w: DispatchMessageWFn = unsafe { user32.sym(c"DispatchMessageW") }?;
        let def_window_proc_w: WndProcFn = unsafe { user32.sym(c"DefWindowProcW") }?;
        let enable_window: EnableWindowFn = unsafe { user32.sym(c"EnableWindow") }?;
        if let Some(f) = unsafe { user32.sym::<GetWindowTextLengthWFn>(c"GetWindowTextLengthW") } {
            let _ = WIN32_GET_TEXT_LEN_W.set(f);
        }
        if let Some(f) = unsafe { user32.sym::<GetWindowTextWFn>(c"GetWindowTextW") } {
            let _ = WIN32_GET_TEXT_W.set(f);
        }
        if let Some(f) = unsafe { user32.sym::<SendMessageWFn>(c"SendMessageW") } {
            let _ = WIN32_SEND_MSG_W.set(f);
        }
        let get_module_handle_w: GetModuleHandleWFn = unsafe { kernel32.sym(c"GetModuleHandleW") }?;
        let get_last_error: Option<GetLastErrorFn> = unsafe { kernel32.sym(c"GetLastError") };

        let _ = WIN32_DEF_WNDPROC.set(def_window_proc_w);
        let h_instance = unsafe { get_module_handle_w(std::ptr::null()) };
        if !Self::ensure_class_and_probe(
            register_class_ex_w,
            create_window_ex_w,
            destroy_window,
            get_last_error,
            h_instance,
        ) {
            return None;
        }

        Some(Self {
            h_instance,
            create_window_ex_w,
            destroy_window,
            show_window,
            set_window_text_w,
            move_window,
            is_window,
            peek_message_w,
            translate_message,
            dispatch_message_w,
            enable_window,
            dwm_set_window_attribute,
        })
    }

    fn ensure_class_and_probe(
        register_class_ex_w: RegisterClassExWFn,
        create_window_ex_w: CreateWindowExWFn,
        destroy_window: DestroyWindowFn,
        get_last_error: Option<GetLastErrorFn>,
        h_instance: *mut c_void,
    ) -> bool {
        let class_ok = *WIN32_CLASS_REGISTERED.get_or_init(|| {
            let class_w = to_wide_null(NG_WIN32_CLASS_NAME);
            let wc = WndClassExW {
                cb_size: u32::try_from(size_of::<WndClassExW>()).unwrap_or(80),
                style: 0,
                lpfn_wnd_proc: ng_win32_wndproc,
                cb_cls_extra: 0,
                cb_wnd_extra: 0,
                h_instance,
                h_icon: std::ptr::null_mut(),
                h_cursor: std::ptr::null_mut(),
                hbr_background: std::ptr::null_mut(),
                lpsz_menu_name: std::ptr::null(),
                lpsz_class_name: class_w.as_ptr(),
                h_icon_sm: std::ptr::null_mut(),
            };
            let atom = unsafe { register_class_ex_w(&raw const wc) };
            if atom == 0 {
                let err = get_last_error.map_or(0, |f| unsafe { f() });
                if err != ERROR_CLASS_ALREADY_EXISTS {
                    eprintln!(
                        "[Win32Sys] RegisterClassExW failed (error={err}); using headless mode"
                    );
                    return false;
                }
            }
            true
        });
        if !class_ok {
            return false;
        }

        // Probe whether this Windows session allows real HWND creation (Session 0 CI runners may deny GUI).
        let class_w = to_wide_null(NG_WIN32_CLASS_NAME);
        let title_w = to_wide_null("NgDesktop Probe");
        let probe_hwnd = unsafe {
            create_window_ex_w(
                0,
                class_w.as_ptr(),
                title_w.as_ptr(),
                WS_OVERLAPPEDWINDOW,
                0,
                0,
                64,
                64,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                h_instance,
                std::ptr::null_mut(),
            )
        };
        if probe_hwnd.is_null() {
            let err = get_last_error.map_or(0, |f| unsafe { f() });
            eprintln!(
                "[Win32Sys] CreateWindowExW probe returned NULL (error={err}); falling back to headless mode"
            );
            return false;
        }
        unsafe {
            let _ = destroy_window(probe_hwnd);
        }
        true
    }

    pub(super) fn create_toplevel(
        &self,
        node: NodeId,
        title: &str,
        is_dialog: bool,
        mask: EventMask,
        prevent_close: bool,
    ) -> Option<Win32Handle> {
        let class_w = to_wide_null(NG_WIN32_CLASS_NAME);
        let title_w = to_wide_null(title);
        let (style, width, height) = if is_dialog {
            (WS_POPUPWINDOW | WS_CAPTION | WS_CLIPCHILDREN, 460, 280)
        } else {
            (WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN, 1020, 720)
        };
        let hwnd = unsafe {
            (self.create_window_ex_w)(
                0,
                class_w.as_ptr(),
                title_w.as_ptr(),
                style,
                40,
                40,
                width,
                height,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                self.h_instance,
                std::ptr::null_mut(),
            )
        };
        if hwnd.is_null() {
            return None;
        }
        let handle = Win32Handle(hwnd as usize);
        hwnd_map()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(
                handle.0,
                Win32WindowRecord {
                    node,
                    win32_class: if is_dialog { "Dialog" } else { "Window" },
                    xaml_class: if is_dialog {
                        "Microsoft.UI.Xaml.Controls.ContentDialog"
                    } else {
                        "Microsoft.UI.Xaml.Window"
                    },
                    event_mask: mask,
                    prevent_close,
                    suppress_change: false,
                },
            );
        if crate::platforms::GpuRendererPreference::from_env().prefers_hardware_acceleration() {
            self.apply_dwm_theme(handle, false);
        }
        Some(handle)
    }

    pub(super) fn apply_dwm_theme(&self, handle: Win32Handle, dark: bool) {
        let Some(dwm_set) = self.dwm_set_window_attribute else {
            return;
        };
        let dark_val = c_int::from(dark);
        let hwnd = handle.0 as *mut c_void;
        let dword_size = u32::try_from(size_of::<c_int>()).unwrap_or(4);
        unsafe {
            let hr = dwm_set(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                (&raw const dark_val).cast::<c_void>(),
                dword_size,
            );
            if hr != 0 {
                let _ = dwm_set(
                    hwnd,
                    DWMWA_USE_IMMERSIVE_DARK_MODE_BEFORE_20H1,
                    (&raw const dark_val).cast::<c_void>(),
                    dword_size,
                );
            }
            if crate::platforms::GpuRendererPreference::from_env().prefers_hardware_acceleration() {
                let backdrop = DWMSBT_MAINWINDOW;
                let _ = dwm_set(
                    hwnd,
                    DWMWA_SYSTEMBACKDROP_TYPE,
                    (&raw const backdrop).cast::<c_void>(),
                    dword_size,
                );
            }
        }
    }

    pub(super) fn create_child_control(
        &self,
        parent: Win32Handle,
        node: NodeId,
        binding: &WinUiControlBinding,
        text: &str,
    ) -> Option<Win32Handle> {
        let is_container = matches!(
            binding.xaml_class,
            "Microsoft.UI.Xaml.Controls.StackPanel"
                | "Microsoft.UI.Xaml.Controls.Grid"
                | "Microsoft.UI.Xaml.Controls.ScrollViewer"
                | "Microsoft.UI.Xaml.Controls.SplitView"
                | "Microsoft.UI.Xaml.Controls.NavigationView"
                | "Microsoft.UI.Xaml.Controls.Expander"
                | "Microsoft.UI.Xaml.Controls.ItemsRepeater"
                | "Microsoft.UI.Xaml.Controls.ListView"
                | "Microsoft.UI.Xaml.Controls.TitleBar"
        );
        let native_class = match binding.win32_class {
            _ if is_container => NG_WIN32_CLASS_NAME,
            "BUTTON" | "EDIT" | "COMBOBOX" | "STATIC" => binding.win32_class,
            _ => NG_WIN32_CLASS_NAME,
        };
        let mut style = WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN;
        if binding.xaml_class == "Microsoft.UI.Xaml.Controls.CheckBox"
            || binding.xaml_class == "Microsoft.UI.Xaml.Controls.ToggleSwitch"
        {
            style |= BS_AUTOCHECKBOX;
        } else if native_class == "EDIT" {
            style |= ES_AUTOHSCROLL;
        } else if native_class == "COMBOBOX" {
            style |= CBS_DROPDOWNLIST;
        }

        let class_w = to_wide_null(native_class);
        let text_w = to_wide_null(text);
        let hwnd = unsafe {
            (self.create_window_ex_w)(
                0,
                class_w.as_ptr(),
                text_w.as_ptr(),
                style,
                12,
                12,
                280,
                32,
                parent.0 as *mut c_void,
                std::ptr::null_mut(),
                self.h_instance,
                std::ptr::null_mut(),
            )
        };
        if hwnd.is_null() {
            return None;
        }
        let handle = Win32Handle(hwnd as usize);
        hwnd_map()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(
                handle.0,
                Win32WindowRecord {
                    node,
                    win32_class: binding.win32_class,
                    xaml_class: binding.xaml_class,
                    event_mask: binding.event_mask,
                    prevent_close: false,
                    suppress_change: false,
                },
            );
        Some(handle)
    }

    pub(super) fn set_text(&self, handle: Win32Handle, text: &str) {
        {
            let mut guard = hwnd_map()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(rec) = guard.get_mut(&handle.0) {
                rec.suppress_change = true;
            }
        }
        let text_w = to_wide_null(text);
        unsafe {
            let _ = (self.set_window_text_w)(handle.0 as *mut c_void, text_w.as_ptr());
        }
        let mut guard = hwnd_map()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(rec) = guard.get_mut(&handle.0) {
            rec.suppress_change = false;
        }
    }

    pub(super) fn set_visible(&self, handle: Win32Handle, visible: bool) {
        unsafe {
            let _ = (self.show_window)(handle.0 as *mut c_void, win32_show_cmd(visible));
        }
    }

    pub(super) fn set_enabled(&self, handle: Win32Handle, enabled: bool) {
        unsafe {
            let _ = (self.enable_window)(handle.0 as *mut c_void, c_int::from(enabled));
        }
    }

    pub(super) fn set_bounds(&self, handle: Win32Handle, x: i32, y: i32, w: i32, h: i32) {
        unsafe {
            let _ = (self.move_window)(handle.0 as *mut c_void, x, y, w.max(40), h.max(24), 1);
        }
    }

    pub(super) fn set_event_mask(handle: Win32Handle, mask: EventMask) {
        let mut guard = hwnd_map()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(rec) = guard.get_mut(&handle.0) {
            rec.event_mask = rec.event_mask.union(mask);
        }
    }

    pub(super) fn set_prevent_close(handle: Win32Handle, prevent: bool) {
        let mut guard = hwnd_map()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(rec) = guard.get_mut(&handle.0) {
            rec.prevent_close = prevent;
            if prevent {
                rec.event_mask = rec.event_mask.union(EventMask::CLOSE_REQUEST);
            }
        }
    }

    pub(super) fn is_window_alive(&self, handle: Win32Handle) -> bool {
        unsafe { (self.is_window)(handle.0 as *mut c_void) != 0 }
    }

    pub(super) fn destroy_window(&self, handle: Win32Handle) {
        {
            let mut guard = hwnd_map()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            guard.remove(&handle.0);
        }
        unsafe {
            let _ = (self.destroy_window)(handle.0 as *mut c_void);
        }
        let mut guard = hwnd_map()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.retain(|&hwnd_key, _| self.is_window_alive(Win32Handle(hwnd_key)));
    }

    pub(super) fn pump_messages(&self) -> Vec<NativeEvent> {
        let mut msg = Win32Msg {
            hwnd: std::ptr::null_mut(),
            message: 0,
            w_param: 0,
            l_param: 0,
            time: 0,
            pt: Win32Point { x: 0, y: 0 },
            l_private: 0,
        };
        for _ in 0..256 {
            let has_msg = unsafe {
                (self.peek_message_w)(&raw mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE)
            };
            if has_msg == 0 {
                break;
            }
            unsafe {
                let _ = (self.translate_message)(&raw const msg);
                let _ = (self.dispatch_message_w)(&raw const msg);
            }
        }
        let mut guard = event_queue()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        coalesce_frame_events(std::mem::take(&mut *guard))
    }

    pub(super) fn apply_dwm_composition_attrs(
        &self,
        handle: Win32Handle,
        corner_pref: Option<i32>,
        backdrop: Option<i32>,
    ) {
        if !crate::platforms::GpuRendererPreference::from_env().prefers_hardware_acceleration() {
            return;
        }
        let Some(dwm_set) = self.dwm_set_window_attribute else {
            return;
        };
        let hwnd = handle.0 as *mut c_void;
        let dword_size = u32::try_from(size_of::<c_int>()).unwrap_or(4);
        if let Some(corner) = corner_pref {
            let val: c_int = corner;
            unsafe {
                let _ = dwm_set(
                    hwnd,
                    DWMWA_WINDOW_CORNER_PREFERENCE,
                    (&raw const val).cast::<c_void>(),
                    dword_size,
                );
            }
        }
        if let Some(backdrop_val) = backdrop {
            let val: c_int = backdrop_val;
            unsafe {
                let _ = dwm_set(
                    hwnd,
                    DWMWA_SYSTEMBACKDROP_TYPE,
                    (&raw const val).cast::<c_void>(),
                    dword_size,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_wide_strings_and_decodes_win32_lparam_and_wheel() {
        let wide = to_wide_null("NgNative");
        assert_eq!(wide.last(), Some(&0));
        assert_eq!(wide.len(), 9);

        let packed_size = isize::try_from((0x02D0_usize << 16) | 0x0500_usize).unwrap();
        assert_eq!(decode_win32_size(packed_size), (1280, 720));

        let wheel_up = usize::from(120_u16) << 16;
        assert!((decode_win32_wheel_delta(wheel_up) - 1.0).abs() < f64::EPSILON);

        let wheel_down = usize::from(u16::from_ne_bytes((-240_i16).to_ne_bytes())) << 16;
        assert!((decode_win32_wheel_delta(wheel_down) + 2.0).abs() < f64::EPSILON);

        assert_eq!(win32_show_cmd(false), 0);
        assert!(matches!(win32_show_cmd(true), 4 | 5));
    }

    #[test]
    fn resolves_dwm_corner_preference_and_backdrop_types() {
        assert_eq!(
            resolve_dwm_corner_preference("border-radius: 0px"),
            DWMWCP_DONOTROUND
        );
        assert_eq!(
            resolve_dwm_corner_preference("border-radius: 4px"),
            DWMWCP_ROUNDSMALL
        );
        assert_eq!(
            resolve_dwm_corner_preference("border-radius: 12px"),
            DWMWCP_ROUND
        );
        assert_eq!(resolve_dwm_corner_preference("color: red"), DWMWCP_DEFAULT);

        assert_eq!(resolve_dwm_backdrop_type("mica"), DWMSBT_MAINWINDOW);
        assert_eq!(resolve_dwm_backdrop_type("acrylic"), DWMSBT_TRANSIENTWINDOW);
        assert_eq!(resolve_dwm_backdrop_type("mica-alt"), DWMSBT_TABBEDWINDOW);
        assert_eq!(resolve_dwm_backdrop_type("none"), DWMSBT_NONE);
        assert_eq!(resolve_dwm_backdrop_type("auto"), DWMSBT_AUTO);
    }
}
