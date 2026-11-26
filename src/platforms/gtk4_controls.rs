#![allow(unsafe_code)]

use super::gtk4_callbacks::{
    on_gtk_context_menu_released, on_gtk_focus_enter, on_gtk_focus_leave, on_gtk_gesture_pressed,
    on_gtk_gesture_released, on_gtk_key_pressed, on_gtk_key_released, on_gtk_motion_enter,
    on_gtk_motion_leave, on_gtk_motion_move, on_gtk_wheel_scroll, on_gtk_window_close_request,
};
use super::gtk4_sys::{
    Gtk4Sys, GtkClickCallback, GtkCloseRequestCallback, GtkEdgeReachedCallback,
    GtkGestureClickCallback, GtkHandle, GtkKeyCallback, GtkListBoxRowCallback, GtkMotionCallback,
    GtkNotifyCallback, GtkScrollControllerCallback,
};
use std::ffi::{CStr, CString, c_void};

impl Gtk4Sys {
    pub fn set_label_text(&self, lbl: GtkHandle, text: &str) {
        if let Ok(c_str) = CString::new(text) {
            unsafe {
                (self.gtk_label_set_text)(lbl.0, c_str.as_ptr());
            }
        }
    }

    pub fn set_button_label(&self, btn: GtkHandle, text: &str) {
        if let Ok(c_str) = CString::new(text) {
            unsafe {
                (self.gtk_button_set_label)(btn.0, c_str.as_ptr());
            }
        }
    }

    pub fn set_entry_text(&self, entry: GtkHandle, text: &str) {
        if let Ok(c_str) = CString::new(text) {
            unsafe {
                (self.gtk_editable_set_text)(entry.0, c_str.as_ptr());
            }
        }
    }

    pub fn get_entry_text(&self, entry: GtkHandle) -> String {
        unsafe {
            let ptr = (self.gtk_editable_get_text)(entry.0);
            if ptr.is_null() {
                String::new()
            } else {
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        }
    }

    pub fn set_entry_placeholder(&self, entry: GtkHandle, placeholder: &str) {
        if let Ok(c_str) = CString::new(placeholder) {
            unsafe {
                (self.gtk_entry_set_placeholder_text)(entry.0, c_str.as_ptr());
            }
        }
    }

    pub fn set_switch_active(&self, sw: GtkHandle, active: bool) {
        unsafe {
            (self.gtk_switch_set_active)(sw.0, i32::from(active));
        }
    }

    pub fn get_switch_active(&self, sw: GtkHandle) -> bool {
        unsafe { (self.gtk_switch_get_active)(sw.0) != 0 }
    }

    pub fn set_check_button_label(&self, chk: GtkHandle, text: &str) {
        if let Ok(c_str) = CString::new(text) {
            unsafe {
                (self.gtk_check_button_set_label)(chk.0, c_str.as_ptr());
            }
        }
    }

    pub fn set_check_button_active(&self, chk: GtkHandle, active: bool) {
        unsafe {
            (self.gtk_check_button_set_active)(chk.0, i32::from(active));
        }
    }

    pub fn get_check_button_active(&self, chk: GtkHandle) -> bool {
        unsafe { (self.gtk_check_button_get_active)(chk.0) != 0 }
    }

    pub fn set_slider_value(&self, scale: GtkHandle, val: f64) {
        unsafe {
            (self.gtk_range_set_value)(scale.0, val);
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn get_slider_value(&self, scale: GtkHandle) -> i64 {
        unsafe { (self.gtk_range_get_value)(scale.0).round() as i64 }
    }

    pub fn set_drop_down_options(&self, dd: GtkHandle, raw_csv: &str) {
        let owned: Vec<CString> = raw_csv
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .filter_map(|s| CString::new(s).ok())
            .collect();
        let mut ptrs: Vec<*const std::ffi::c_char> = owned.iter().map(|c| c.as_ptr()).collect();
        ptrs.push(std::ptr::null());
        unsafe {
            let model = (self.gtk_string_list_new)(ptrs.as_ptr());
            if !model.is_null() {
                (self.gtk_drop_down_set_model)(dd.0, model);
                (self.g_object_unref)(model);
            }
        }
    }

    pub fn set_drop_down_selected(&self, dd: GtkHandle, idx: u32) {
        unsafe {
            (self.gtk_drop_down_set_selected)(dd.0, idx);
        }
    }

    pub fn get_drop_down_selected(&self, dd: GtkHandle) -> i64 {
        unsafe { i64::from((self.gtk_drop_down_get_selected)(dd.0)) }
    }

    pub fn add_css_class(&self, widget: GtkHandle, class_name: &str) {
        if let Ok(c_str) = CString::new(class_name) {
            unsafe {
                (self.gtk_widget_add_css_class)(widget.0, c_str.as_ptr());
            }
        }
    }

    pub fn remove_css_class(&self, widget: GtkHandle, class_name: &str) {
        if let Ok(c_str) = CString::new(class_name) {
            unsafe {
                (self.gtk_widget_remove_css_class)(widget.0, c_str.as_ptr());
            }
        }
    }

    pub fn set_window_child(&self, win: GtkHandle, child: GtkHandle) {
        unsafe {
            (self.gtk_window_set_child)(win.0, child.0);
            (self.gtk_window_present)(win.0);
        }
    }

    pub fn clear_window_child(&self, win: GtkHandle) {
        unsafe {
            (self.gtk_window_set_child)(win.0, std::ptr::null_mut());
        }
    }

    pub fn box_append(&self, bx: GtkHandle, child: GtkHandle) {
        unsafe {
            (self.gtk_box_append)(bx.0, child.0);
        }
    }

    pub fn box_insert_after(&self, bx: GtkHandle, child: GtkHandle, sibling: Option<GtkHandle>) {
        let sib_ptr = sibling.map_or(std::ptr::null_mut(), |h| h.0);
        unsafe {
            (self.gtk_box_insert_child_after)(bx.0, child.0, sib_ptr);
        }
    }

    pub fn box_reorder_after(&self, bx: GtkHandle, child: GtkHandle, sibling: Option<GtkHandle>) {
        let sib_ptr = sibling.map_or(std::ptr::null_mut(), |h| h.0);
        unsafe {
            (self.gtk_box_reorder_child_after)(bx.0, child.0, sib_ptr);
        }
    }

    pub fn box_remove(&self, bx: GtkHandle, child: GtkHandle) {
        unsafe {
            (self.gtk_box_remove)(bx.0, child.0);
        }
    }

    pub fn destroy_window(&self, win: GtkHandle) {
        unsafe {
            (self.gtk_window_destroy)(win.0);
        }
    }

    pub fn ref_sink_widget(&self, widget: GtkHandle) {
        if !widget.0.is_null() {
            unsafe {
                (self.g_object_ref_sink)(widget.0);
            }
        }
    }

    pub fn unref_widget(&self, widget: GtkHandle) {
        if !widget.0.is_null() {
            unsafe {
                (self.g_object_unref)(widget.0);
            }
        }
    }

    pub fn connect_clicked(
        &self,
        btn: GtkHandle,
        callback: GtkClickCallback,
        packed_node_id: usize,
    ) {
        self.connect_signal(btn, c"clicked", callback, packed_node_id);
    }

    pub fn connect_signal(
        &self,
        widget: GtkHandle,
        signal_name: &CStr,
        callback: GtkClickCallback,
        packed_node_id: usize,
    ) {
        unsafe {
            (self.g_signal_connect_data)(
                widget.0,
                signal_name.as_ptr(),
                callback as *const c_void,
                packed_node_id as *mut c_void,
                std::ptr::null_mut(),
                0,
            );
        }
    }

    pub fn connect_notify_signal(
        &self,
        widget: GtkHandle,
        signal_name: &CStr,
        callback: GtkNotifyCallback,
        packed_node_id: usize,
    ) {
        unsafe {
            (self.g_signal_connect_data)(
                widget.0,
                signal_name.as_ptr(),
                callback as *const c_void,
                packed_node_id as *mut c_void,
                std::ptr::null_mut(),
                0,
            );
        }
    }

    pub fn connect_edge_reached(
        &self,
        scroll: GtkHandle,
        callback: GtkEdgeReachedCallback,
        packed_node_id: usize,
    ) {
        unsafe {
            (self.g_signal_connect_data)(
                scroll.0,
                c"edge-reached".as_ptr(),
                callback as *const c_void,
                packed_node_id as *mut c_void,
                std::ptr::null_mut(),
                0,
            );
        }
    }

    pub fn connect_scroll_adjustment(
        &self,
        scroll: GtkHandle,
        callback: GtkClickCallback,
        packed_node_id: usize,
    ) {
        unsafe {
            let vadj = (self.gtk_scrolled_window_get_vadjustment)(scroll.0);
            if !vadj.is_null() {
                self.connect_signal(GtkHandle(vadj), c"value-changed", callback, packed_node_id);
            }
        }
    }

    pub fn connect_window_close_request(&self, win: GtkHandle, packed_node_id: usize) {
        let cb: GtkCloseRequestCallback = on_gtk_window_close_request;
        unsafe {
            self.connect_raw(win.0, c"close-request", cb as *const c_void, packed_node_id);
        }
    }

    pub fn connect_list_box_row_activated(
        &self,
        list_box: GtkHandle,
        callback: GtkListBoxRowCallback,
        packed_node_id: usize,
    ) {
        unsafe {
            self.connect_raw(
                list_box.0,
                c"row-activated",
                callback as *const c_void,
                packed_node_id,
            );
        }
    }

    #[must_use]
    pub fn scroll_metrics_json(&self, scroll: GtkHandle) -> Option<String> {
        unsafe {
            let vadj = (self.gtk_scrolled_window_get_vadjustment)(scroll.0);
            if vadj.is_null() {
                return None;
            }
            let top = (self.gtk_adjustment_get_value)(vadj);
            let height = (self.gtk_adjustment_get_upper)(vadj);
            let client = (self.gtk_adjustment_get_page_size)(vadj);
            let max_scroll = (height - client).max(0.0);
            let at_top = top <= 1.0;
            let at_bottom = max_scroll <= 1.0 || top + client >= height - 2.0;
            let progress = if max_scroll > 0.0 {
                (top / max_scroll).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let (client_width, _) = self.widget_size(scroll);
            Some(format!(
                r#"{{"scrollTop":{top:.1},"scrollHeight":{height:.1},"clientHeight":{client:.1},"clientWidth":{client_width},"atTop":{at_top},"atBottom":{at_bottom},"progress":{progress:.4}}}"#
            ))
        }
    }

    #[must_use]
    pub fn widget_size(&self, widget: GtkHandle) -> (i32, i32) {
        unsafe {
            (
                (self.gtk_widget_get_width)(widget.0),
                (self.gtk_widget_get_height)(widget.0),
            )
        }
    }

    #[must_use]
    pub fn window_flags(&self, win: GtkHandle) -> (bool, bool) {
        unsafe {
            (
                (self.gtk_window_is_maximized)(win.0) != 0,
                (self.gtk_window_is_active)(win.0) != 0,
            )
        }
    }

    #[must_use]
    pub fn current_gesture_button(&self, gesture: GtkHandle) -> u32 {
        unsafe { (self.gtk_gesture_single_get_current_button)(gesture.0) }
    }

    #[must_use]
    pub fn keyval_name(&self, keyval: u32) -> Option<String> {
        unsafe {
            let ptr = (self.gdk_keyval_name)(keyval);
            (!ptr.is_null()).then(|| CStr::from_ptr(ptr).to_string_lossy().into_owned())
        }
    }

    unsafe fn connect_raw(&self, obj: *mut c_void, sig: &CStr, cb: *const c_void, packed: usize) {
        unsafe {
            (self.g_signal_connect_data)(
                obj,
                sig.as_ptr(),
                cb,
                packed as *mut c_void,
                std::ptr::null_mut(),
                0,
            );
        }
    }

    pub fn attach_click_gesture(
        &self,
        widget: GtkHandle,
        packed: usize,
        _include_single_click: bool,
    ) {
        unsafe {
            let g = (self.gtk_gesture_click_new)();
            if g.is_null() {
                return;
            }
            (self.gtk_gesture_single_set_button)(g, 1);
            let press_cb: GtkGestureClickCallback = on_gtk_gesture_pressed;
            let rel_cb: GtkGestureClickCallback = on_gtk_gesture_released;
            self.connect_raw(g, c"pressed", press_cb as *const c_void, packed);
            self.connect_raw(g, c"released", rel_cb as *const c_void, packed);
            (self.gtk_widget_add_controller)(widget.0, g);
        }
    }

    pub fn attach_context_menu_gesture(&self, widget: GtkHandle, packed: usize) {
        unsafe {
            let g = (self.gtk_gesture_click_new)();
            if g.is_null() {
                return;
            }
            (self.gtk_gesture_single_set_button)(g, 3);
            let rel_cb: GtkGestureClickCallback = on_gtk_context_menu_released;
            self.connect_raw(g, c"released", rel_cb as *const c_void, packed);
            (self.gtk_widget_add_controller)(widget.0, g);
        }
    }

    pub fn attach_motion_controller(&self, widget: GtkHandle, packed: usize) {
        unsafe {
            let m = (self.gtk_event_controller_motion_new)();
            if m.is_null() {
                return;
            }
            let enter_cb: GtkMotionCallback = on_gtk_motion_enter;
            let move_cb: GtkMotionCallback = on_gtk_motion_move;
            let leave_cb: GtkClickCallback = on_gtk_motion_leave;
            self.connect_raw(m, c"enter", enter_cb as *const c_void, packed);
            self.connect_raw(m, c"motion", move_cb as *const c_void, packed);
            self.connect_raw(m, c"leave", leave_cb as *const c_void, packed);
            (self.gtk_widget_add_controller)(widget.0, m);
        }
    }

    pub fn attach_focus_controller(&self, widget: GtkHandle, packed: usize) {
        unsafe {
            let f = (self.gtk_event_controller_focus_new)();
            if f.is_null() {
                return;
            }
            let enter_cb: GtkClickCallback = on_gtk_focus_enter;
            let leave_cb: GtkClickCallback = on_gtk_focus_leave;
            self.connect_raw(f, c"enter", enter_cb as *const c_void, packed);
            self.connect_raw(f, c"leave", leave_cb as *const c_void, packed);
            (self.gtk_widget_add_controller)(widget.0, f);
        }
    }

    pub fn attach_key_controller(&self, widget: GtkHandle, packed: usize) {
        unsafe {
            let k = (self.gtk_event_controller_key_new)();
            if k.is_null() {
                return;
            }
            let press_cb: GtkKeyCallback = on_gtk_key_pressed;
            let rel_cb: GtkKeyCallback = on_gtk_key_released;
            self.connect_raw(k, c"key-pressed", press_cb as *const c_void, packed);
            self.connect_raw(k, c"key-released", rel_cb as *const c_void, packed);
            (self.gtk_widget_add_controller)(widget.0, k);
        }
    }

    pub fn attach_wheel_controller(&self, widget: GtkHandle, packed: usize) {
        unsafe {
            // GTK_EVENT_CONTROLLER_SCROLL_BOTH_AXES = 3 (vertical | horizontal)
            let s = (self.gtk_event_controller_scroll_new)(3);
            if s.is_null() {
                return;
            }
            let scroll_cb: GtkScrollControllerCallback = on_gtk_wheel_scroll;
            self.connect_raw(s, c"scroll", scroll_cb as *const c_void, packed);
            (self.gtk_widget_add_controller)(widget.0, s);
        }
    }

    pub fn open_window_count(&self) -> u32 {
        unsafe {
            let list = (self.gtk_window_get_toplevels)();
            if list.is_null() {
                0
            } else {
                (self.g_list_model_get_n_items)(list)
            }
        }
    }

    pub fn step_main_context(&self, may_block: bool) {
        unsafe {
            if may_block {
                (self.g_main_context_iteration)(std::ptr::null_mut(), 1);
            }
            while (self.g_main_context_iteration)(std::ptr::null_mut(), 0) != 0 {}
        }
    }
}
