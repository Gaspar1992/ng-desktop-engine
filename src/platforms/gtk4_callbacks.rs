#![allow(unsafe_code)]

use crate::engine::{EventPayload, NativeEvent, NodeId, coalesce_frame_events};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashSet;
use std::ffi::{c_int, c_uint, c_void};
#[cfg(target_os = "linux")]
use std::sync::OnceLock;

#[cfg(target_os = "linux")]
type ListBoxRowGetIndexFn = unsafe extern "C" fn(*mut c_void) -> c_int;

#[cfg(target_os = "linux")]
static LIST_BOX_ROW_GET_INDEX: OnceLock<ListBoxRowGetIndexFn> = OnceLock::new();

#[cfg(target_os = "linux")]
pub(super) fn set_list_box_row_index_fn(func: ListBoxRowGetIndexFn) {
    let _ = LIST_BOX_ROW_GET_INDEX.set(func);
}

thread_local! {
    static PENDING_EVENTS: RefCell<Vec<NativeEvent>> = const { RefCell::new(Vec::new()) };
    static PREVENT_CLOSE_WINDOWS: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());
}

pub fn set_window_prevent_close(packed: usize, prevent: bool) {
    PREVENT_CLOSE_WINDOWS.with(|set| {
        let mut guard = set.borrow_mut();
        if prevent {
            guard.insert(packed);
        } else {
            guard.remove(&packed);
        }
    });
}

#[must_use]
pub fn is_window_prevent_close(packed: usize) -> bool {
    PREVENT_CLOSE_WINDOWS.with(|set| set.borrow().contains(&packed))
}

pub const fn pack_node_id(id: NodeId) -> usize {
    (id.index << 32) | (id.generation as usize)
}

#[allow(clippy::cast_possible_truncation)]
pub const fn unpack_node_id(packed: usize) -> NodeId {
    NodeId {
        index: packed >> 32,
        generation: (packed & 0xFFFF_FFFF) as u32,
    }
}

fn enqueue_event(packed: usize, event_name: &str) {
    enqueue_event_with_payload(packed, event_name, EventPayload::None);
}

pub fn enqueue_event_with_payload(packed: usize, event_name: &str, payload: EventPayload) {
    let node = unpack_node_id(packed);
    PENDING_EVENTS.with(|queue| {
        queue.borrow_mut().push(NativeEvent {
            node,
            event_name: String::from(event_name),
            payload,
        });
    });
}

pub fn drain_pending_events() -> Vec<NativeEvent> {
    coalesce_frame_events(PENDING_EVENTS.with(|q| std::mem::take(&mut *q.borrow_mut())))
}

pub(super) unsafe extern "C" fn on_gtk_button_clicked(_btn: *mut c_void, user_data: *mut c_void) {
    enqueue_event(user_data as usize, "clicked");
}

pub(super) unsafe extern "C" fn on_gtk_entry_changed(_entry: *mut c_void, user_data: *mut c_void) {
    enqueue_event(user_data as usize, "input");
}

pub(super) unsafe extern "C" fn on_gtk_check_toggled(_chk: *mut c_void, user_data: *mut c_void) {
    enqueue_event(user_data as usize, "toggled");
}

pub(super) unsafe extern "C" fn on_gtk_scale_value_changed(
    _scale: *mut c_void,
    user_data: *mut c_void,
) {
    enqueue_event(user_data as usize, "value-changed");
}

pub(super) unsafe extern "C" fn on_gtk_switch_active_notify(
    _sw: *mut c_void,
    _pspec: *mut c_void,
    user_data: *mut c_void,
) {
    enqueue_event(user_data as usize, "toggled");
}

pub(super) unsafe extern "C" fn on_gtk_drop_down_selected_notify(
    _dd: *mut c_void,
    _pspec: *mut c_void,
    user_data: *mut c_void,
) {
    enqueue_event(user_data as usize, "value-changed");
}

pub(super) unsafe extern "C" fn on_gtk_scroll_edge_reached(
    _scroll: *mut c_void,
    pos: c_int,
    user_data: *mut c_void,
) {
    let packed = user_data as usize;
    let edge = match pos {
        0 => "left",
        1 => "right",
        2 => "top",
        _ => "bottom",
    };
    enqueue_event_with_payload(
        packed,
        "scroll-edge",
        EventPayload::Text(format!(r#"{{"edge":"{edge}"}}"#)),
    );
    if pos == 2 {
        enqueue_event(packed, "scroll-top");
    } else if pos == 3 {
        // GTK_POS_BOTTOM == 3: notifies both scroll-bottom handlers and `@defer (on viewport)`
        enqueue_event(packed, "scroll-bottom");
        enqueue_event(packed, "viewport-reached");
    }
}

pub(super) unsafe extern "C" fn on_gtk_scroll_value_changed(
    _adj: *mut c_void,
    user_data: *mut c_void,
) {
    enqueue_event(user_data as usize, "scroll");
}

pub(super) unsafe extern "C" fn on_gtk_entry_activate(_entry: *mut c_void, user_data: *mut c_void) {
    enqueue_event(user_data as usize, "activate");
}

pub(super) unsafe extern "C" fn on_gtk_search_stop(_entry: *mut c_void, user_data: *mut c_void) {
    enqueue_event(user_data as usize, "cancel");
}

pub(super) unsafe extern "C" fn on_gtk_gesture_pressed(
    _gesture: *mut c_void,
    n_press: c_int,
    x: f64,
    y: f64,
    user_data: *mut c_void,
) {
    enqueue_event_with_payload(
        user_data as usize,
        "mousedown",
        EventPayload::Text(format!(r#"{{"x":{x:.1},"y":{y:.1},"clicks":{n_press}}}"#)),
    );
}

pub(super) unsafe extern "C" fn on_gtk_gesture_released(
    _gesture: *mut c_void,
    n_press: c_int,
    x: f64,
    y: f64,
    user_data: *mut c_void,
) {
    let packed = user_data as usize;
    let payload = EventPayload::Text(format!(r#"{{"x":{x:.1},"y":{y:.1},"clicks":{n_press}}}"#));
    enqueue_event_with_payload(packed, "mouseup", payload.clone());
    if n_press == 1 {
        enqueue_event_with_payload(packed, "clicked", payload);
    } else if n_press >= 2 {
        enqueue_event_with_payload(packed, "dblclick", payload);
    }
}

pub(super) unsafe extern "C" fn on_gtk_context_menu_released(
    _gesture: *mut c_void,
    _n_press: c_int,
    x: f64,
    y: f64,
    user_data: *mut c_void,
) {
    enqueue_event_with_payload(
        user_data as usize,
        "contextmenu",
        EventPayload::Text(format!(r#"{{"x":{x:.1},"y":{y:.1},"button":2}}"#)),
    );
}

pub(super) unsafe extern "C" fn on_gtk_motion_enter(
    _ctrl: *mut c_void,
    x: f64,
    y: f64,
    user_data: *mut c_void,
) {
    enqueue_event_with_payload(
        user_data as usize,
        "mouseenter",
        EventPayload::Text(format!(r#"{{"x":{x:.1},"y":{y:.1}}}"#)),
    );
}

pub(super) unsafe extern "C" fn on_gtk_motion_leave(_ctrl: *mut c_void, user_data: *mut c_void) {
    enqueue_event(user_data as usize, "mouseleave");
}

pub(super) unsafe extern "C" fn on_gtk_motion_move(
    _ctrl: *mut c_void,
    x: f64,
    y: f64,
    user_data: *mut c_void,
) {
    enqueue_event_with_payload(
        user_data as usize,
        "mousemove",
        EventPayload::Text(format!(r#"{{"x":{x:.1},"y":{y:.1}}}"#)),
    );
}

pub(super) unsafe extern "C" fn on_gtk_focus_enter(_ctrl: *mut c_void, user_data: *mut c_void) {
    enqueue_event_with_payload(user_data as usize, "focus", EventPayload::Bool(true));
}

pub(super) unsafe extern "C" fn on_gtk_focus_leave(_ctrl: *mut c_void, user_data: *mut c_void) {
    enqueue_event_with_payload(user_data as usize, "blur", EventPayload::Bool(false));
}

pub(super) unsafe extern "C" fn on_gtk_wheel_scroll(
    _ctrl: *mut c_void,
    dx: f64,
    dy: f64,
    user_data: *mut c_void,
) -> c_int {
    enqueue_event_with_payload(
        user_data as usize,
        "wheel",
        EventPayload::Text(format!(r#"{{"deltaX":{dx:.2},"deltaY":{dy:.2}}}"#)),
    );
    0
}

fn format_keyval_name(keyval: u32) -> Cow<'static, str> {
    match keyval {
        0xff08 => Cow::Borrowed("Backspace"),
        0xff09 | 0xfe20 => Cow::Borrowed("Tab"),
        0xff0d | 0xff8d => Cow::Borrowed("Enter"),
        0xff1b => Cow::Borrowed("Escape"),
        0xff20 | 0x0020 => Cow::Borrowed("Space"),
        0xffff => Cow::Borrowed("Delete"),
        0xff50 => Cow::Borrowed("Home"),
        0xff51 => Cow::Borrowed("ArrowLeft"),
        0xff52 => Cow::Borrowed("ArrowUp"),
        0xff53 => Cow::Borrowed("ArrowRight"),
        0xff54 => Cow::Borrowed("ArrowDown"),
        0xff55 => Cow::Borrowed("PageUp"),
        0xff56 => Cow::Borrowed("PageDown"),
        0xff57 => Cow::Borrowed("End"),
        0xffbe..=0xffc9 => Cow::Owned(format!("F{}", keyval - 0xffbd)),
        0x0021..=0x007e => match u8::try_from(keyval) {
            Ok(b) if b != b'"' && b != b'\\' => Cow::Owned(String::from(char::from(b))),
            Ok(b'\\') => Cow::Borrowed("Backslash"),
            Ok(b'"') => Cow::Borrowed("Quote"),
            _ => Cow::Owned(format!("Key({keyval})")),
        },
        _ => Cow::Owned(format!("Key({keyval})")),
    }
}

fn push_key_event(event_name: &str, keyval: u32, keycode: u32, state: u32, packed: usize) {
    let key = format_keyval_name(keyval);
    let shift = (state & 1) != 0;
    let ctrl = (state & 4) != 0;
    let alt = (state & 8) != 0;
    let meta = (state & 0x1400_0000) != 0;
    let payload = format!(
        r#"{{"key":"{key}","keyval":{keyval},"keycode":{keycode},"ctrlKey":{ctrl},"shiftKey":{shift},"altKey":{alt},"metaKey":{meta}}}"#
    );
    enqueue_event_with_payload(packed, event_name, EventPayload::Text(payload));
}

pub(super) unsafe extern "C" fn on_gtk_key_pressed(
    _ctrl: *mut c_void,
    keyval: c_uint,
    keycode: c_uint,
    state: c_uint,
    user_data: *mut c_void,
) -> c_int {
    push_key_event("keydown", keyval, keycode, state, user_data as usize);
    0
}

pub(super) unsafe extern "C" fn on_gtk_key_released(
    _ctrl: *mut c_void,
    keyval: c_uint,
    keycode: c_uint,
    state: c_uint,
    user_data: *mut c_void,
) -> c_int {
    push_key_event("keyup", keyval, keycode, state, user_data as usize);
    0
}

pub(super) unsafe extern "C" fn on_gtk_window_close_request(
    _win: *mut c_void,
    user_data: *mut c_void,
) -> c_int {
    let packed = user_data as usize;
    let preventable = is_window_prevent_close(packed);
    enqueue_event_with_payload(
        packed,
        "close-request",
        EventPayload::Text(format!(r#"{{"preventable":{preventable}}}"#)),
    );
    c_int::from(preventable)
}

pub(super) unsafe extern "C" fn on_gtk_list_box_row_activated(
    _list_box: *mut c_void,
    row: *mut c_void,
    user_data: *mut c_void,
) {
    #[cfg(target_os = "linux")]
    let idx = if let (false, Some(get_idx)) = (row.is_null(), LIST_BOX_ROW_GET_INDEX.get()) {
        unsafe { i64::from(get_idx(row)) }
    } else {
        -1
    };
    #[cfg(not(target_os = "linux"))]
    let idx = {
        let _ = row;
        -1
    };
    enqueue_event_with_payload(user_data as usize, "row-activated", EventPayload::Int(idx));
}

pub(super) unsafe extern "C" fn on_gtk_popover_closed(
    _popover: *mut c_void,
    user_data: *mut c_void,
) {
    enqueue_event(user_data as usize, "closed");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_unpacks_and_drains_native_gtk_callback_events() {
        let _ = drain_pending_events();
        let original = NodeId {
            index: 42,
            generation: 7,
        };
        let packed = pack_node_id(original);
        assert_eq!(unpack_node_id(packed), original);

        unsafe {
            on_gtk_button_clicked(std::ptr::null_mut(), packed as *mut c_void);
            on_gtk_entry_changed(std::ptr::null_mut(), packed as *mut c_void);
            on_gtk_switch_active_notify(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                packed as *mut c_void,
            );
        }

        let events = drain_pending_events();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].node, original);
        assert_eq!(events[0].event_name, "clicked");
        assert_eq!(events[1].event_name, "input");
        assert_eq!(events[2].event_name, "toggled");
        assert_eq!(drain_pending_events(), Vec::<NativeEvent>::new());

        set_window_prevent_close(packed, false);
        let default_res =
            unsafe { on_gtk_window_close_request(std::ptr::null_mut(), packed as *mut c_void) };
        assert_eq!(default_res, 0);

        set_window_prevent_close(packed, true);
        let blocked_res =
            unsafe { on_gtk_window_close_request(std::ptr::null_mut(), packed as *mut c_void) };
        assert_eq!(blocked_res, 1);

        let close_events = drain_pending_events();
        assert_eq!(close_events.len(), 2);
        assert_eq!(close_events[0].event_name, "close-request");
        assert_eq!(
            close_events[0].payload,
            EventPayload::Text(String::from(r#"{"preventable":false}"#))
        );
        assert_eq!(
            close_events[1].payload,
            EventPayload::Text(String::from(r#"{"preventable":true}"#))
        );
        set_window_prevent_close(packed, false);
    }
}
