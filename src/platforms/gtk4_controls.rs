#![allow(unsafe_code)]

use super::gtk4_sys::{
    Gtk4Sys, GtkClickCallback, GtkEdgeReachedCallback, GtkHandle, GtkNotifyCallback,
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
