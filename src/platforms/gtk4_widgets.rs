#![allow(unsafe_code)]

use super::gtk4_sys::{GTK_ORIENTATION_HORIZONTAL, GTK_ORIENTATION_VERTICAL, Gtk4Sys, GtkHandle};
use std::ffi::{CString, c_int};

impl Gtk4Sys {
    pub fn load_css(&self, css: &str) {
        if self.css_provider.is_null() {
            return;
        }
        if let Ok(c_css) = CString::new(css) {
            unsafe {
                (self.gtk_css_provider_load_from_string)(self.css_provider, c_css.as_ptr());
            }
        }
    }

    pub fn configure_window_icon(&self, win: GtkHandle, app_id: &str, search_dir: &str) {
        if let Ok(c_dir) = CString::new(search_dir)
            && !self.icon_theme.is_null()
        {
            unsafe {
                (self.gtk_icon_theme_add_search_path)(self.icon_theme, c_dir.as_ptr());
            }
        }
        if let Ok(c_app) = CString::new(app_id) {
            unsafe {
                (self.g_set_prgname)(c_app.as_ptr());
                (self.g_set_application_name)(c_app.as_ptr());
                (self.gtk_window_set_default_icon_name)(c_app.as_ptr());
                (self.gtk_window_set_icon_name)(win.0, c_app.as_ptr());
            }
        }
    }

    pub fn create_window(&self) -> GtkHandle {
        unsafe {
            let win = (self.gtk_window_new)();
            (self.gtk_window_set_default_size)(win, 1020, 720);
            GtkHandle(win)
        }
    }

    pub fn create_header_bar(&self) -> GtkHandle {
        unsafe {
            let hb = (self.gtk_header_bar_new)();
            (self.gtk_header_bar_set_show_title_buttons)(hb, 1);
            GtkHandle(hb)
        }
    }

    pub fn create_scrolled_window(&self) -> GtkHandle {
        unsafe { GtkHandle((self.gtk_scrolled_window_new)()) }
    }

    pub fn create_box(&self, spacing: i32) -> GtkHandle {
        unsafe {
            let bx = (self.gtk_box_new)(GTK_ORIENTATION_VERTICAL, spacing);
            self.set_uniform_margin(GtkHandle(bx), 20);
            GtkHandle(bx)
        }
    }

    pub fn create_grid(&self) -> GtkHandle {
        unsafe { GtkHandle((self.gtk_grid_new)()) }
    }

    pub fn create_separator(&self) -> GtkHandle {
        unsafe { GtkHandle((self.gtk_separator_new)(GTK_ORIENTATION_HORIZONTAL)) }
    }

    pub fn create_label(&self) -> GtkHandle {
        unsafe {
            let lbl = (self.gtk_label_new)(c"".as_ptr());
            (self.gtk_label_set_wrap)(lbl, 1);
            (self.gtk_label_set_max_width_chars)(lbl, 60);
            GtkHandle(lbl)
        }
    }

    pub fn create_button(&self) -> GtkHandle {
        unsafe { GtkHandle((self.gtk_button_new_with_label)(c"".as_ptr())) }
    }

    pub fn create_entry(&self) -> GtkHandle {
        unsafe { GtkHandle((self.gtk_entry_new)()) }
    }

    pub fn create_switch(&self) -> GtkHandle {
        unsafe { GtkHandle((self.gtk_switch_new)()) }
    }

    pub fn create_check_button(&self) -> GtkHandle {
        unsafe { GtkHandle((self.gtk_check_button_new_with_label)(c"".as_ptr())) }
    }

    pub fn create_slider(&self) -> GtkHandle {
        unsafe {
            GtkHandle((self.gtk_scale_new_with_range)(
                GTK_ORIENTATION_HORIZONTAL,
                0.0,
                100.0,
                1.0,
            ))
        }
    }

    pub fn create_drop_down(&self) -> GtkHandle {
        let null_list: [*const std::ffi::c_char; 1] = [std::ptr::null()];
        unsafe { GtkHandle((self.gtk_drop_down_new_from_strings)(null_list.as_ptr())) }
    }

    pub fn create_image(&self) -> GtkHandle {
        unsafe {
            let pic = (self.gtk_picture_new)();
            (self.gtk_picture_set_can_shrink)(pic, 1);
            (self.gtk_picture_set_keep_aspect_ratio)(pic, 1);
            (self.gtk_widget_set_size_request)(pic, 120, 120);
            GtkHandle(pic)
        }
    }

    pub fn set_image_file(&self, pic: GtkHandle, path: &str) {
        if let Ok(c_path) = CString::new(path) {
            unsafe {
                (self.gtk_picture_set_filename)(pic.0, c_path.as_ptr());
            }
        }
    }

    pub fn set_size_request(&self, widget: GtkHandle, width: i32, height: i32) {
        unsafe {
            (self.gtk_widget_set_size_request)(widget.0, width, height);
        }
    }

    pub fn set_widget_visible(&self, widget: GtkHandle, visible: bool) {
        unsafe {
            (self.gtk_widget_set_visible)(widget.0, i32::from(visible));
        }
    }

    pub fn create_level_bar(&self) -> GtkHandle {
        unsafe {
            let bar = (self.gtk_level_bar_new)();
            (self.gtk_level_bar_set_max_value)(bar, 100.0);
            GtkHandle(bar)
        }
    }

    pub fn set_level_bar_value(&self, bar: GtkHandle, val: f64) {
        unsafe {
            (self.gtk_level_bar_set_value)(bar.0, val);
        }
    }

    pub fn set_level_bar_max(&self, bar: GtkHandle, max: f64) {
        unsafe {
            (self.gtk_level_bar_set_max_value)(bar.0, max);
        }
    }

    pub fn create_spinner(&self) -> GtkHandle {
        unsafe {
            let spinner = (self.gtk_spinner_new)();
            (self.gtk_spinner_set_spinning)(spinner, 1);
            GtkHandle(spinner)
        }
    }

    pub fn set_spinner_active(&self, spinner: GtkHandle, active: bool) {
        unsafe {
            (self.gtk_spinner_set_spinning)(spinner.0, i32::from(active));
        }
    }

    pub fn create_progress_bar(&self) -> GtkHandle {
        unsafe { GtkHandle((self.gtk_progress_bar_new)()) }
    }

    pub fn set_progress_fraction(&self, bar: GtkHandle, fraction: f64) {
        unsafe {
            (self.gtk_progress_bar_set_fraction)(bar.0, fraction);
        }
    }

    pub fn pulse_progress_bar(&self, bar: GtkHandle) {
        unsafe {
            (self.gtk_progress_bar_pulse)(bar.0);
        }
    }

    pub fn create_flow_box(&self) -> GtkHandle {
        unsafe {
            let flow = (self.gtk_flow_box_new)();
            (self.gtk_flow_box_set_selection_mode)(flow, 0); // GTK_SELECTION_NONE
            (self.gtk_flow_box_set_homogeneous)(flow, 0);
            GtkHandle(flow)
        }
    }

    pub fn flow_box_append(&self, flow: GtkHandle, child: GtkHandle) {
        unsafe {
            (self.gtk_flow_box_append)(flow.0, child.0);
        }
    }

    pub fn flow_box_insert(&self, flow: GtkHandle, child: GtkHandle, position: i32) {
        unsafe {
            (self.gtk_flow_box_insert)(flow.0, child.0, position);
        }
    }

    pub fn flow_box_remove(&self, flow: GtkHandle, child: GtkHandle) {
        unsafe {
            (self.gtk_flow_box_remove)(flow.0, child.0);
        }
    }

    /// Reorders an existing `GtkFlowBox` child safely by retaining a strong reference across removal.
    pub fn flow_box_reorder(
        &self,
        flow: GtkHandle,
        child: GtkHandle,
        position: i32,
        has_ref: bool,
    ) {
        unsafe {
            (self.g_object_ref)(child.0);
            (self.gtk_flow_box_remove)(flow.0, child.0);
            if has_ref {
                (self.gtk_flow_box_insert)(flow.0, child.0, position);
            } else {
                (self.gtk_flow_box_append)(flow.0, child.0);
            }
            (self.g_object_unref)(child.0);
        }
    }

    pub fn set_flow_box_row_spacing(&self, flow: GtkHandle, spacing: u32) {
        unsafe {
            (self.gtk_flow_box_set_row_spacing)(flow.0, spacing);
        }
    }

    pub fn set_flow_box_column_spacing(&self, flow: GtkHandle, spacing: u32) {
        unsafe {
            (self.gtk_flow_box_set_column_spacing)(flow.0, spacing);
        }
    }

    pub fn set_flow_box_min_children_per_line(&self, flow: GtkHandle, min: u32) {
        unsafe {
            (self.gtk_flow_box_set_min_children_per_line)(flow.0, min);
        }
    }

    pub fn set_flow_box_max_children_per_line(&self, flow: GtkHandle, max: u32) {
        unsafe {
            (self.gtk_flow_box_set_max_children_per_line)(flow.0, max);
        }
    }

    pub fn set_flow_box_homogeneous(&self, flow: GtkHandle, homogeneous: bool) {
        unsafe {
            (self.gtk_flow_box_set_homogeneous)(flow.0, i32::from(homogeneous));
        }
    }

    pub fn create_paned(&self) -> GtkHandle {
        unsafe { GtkHandle((self.gtk_paned_new)(GTK_ORIENTATION_HORIZONTAL)) }
    }

    pub fn paned_set_start_child(&self, paned: GtkHandle, child: Option<GtkHandle>) {
        let ptr = child.map_or(std::ptr::null_mut(), |h| h.0);
        unsafe {
            (self.gtk_paned_set_start_child)(paned.0, ptr);
        }
    }

    pub fn paned_set_end_child(&self, paned: GtkHandle, child: Option<GtkHandle>) {
        let ptr = child.map_or(std::ptr::null_mut(), |h| h.0);
        unsafe {
            (self.gtk_paned_set_end_child)(paned.0, ptr);
        }
    }

    pub fn set_paned_position(&self, paned: GtkHandle, pos: i32) {
        unsafe {
            (self.gtk_paned_set_position)(paned.0, pos);
        }
    }

    pub fn create_stack(&self) -> GtkHandle {
        unsafe { GtkHandle((self.gtk_stack_new)()) }
    }

    pub fn stack_add_named(&self, stack: GtkHandle, child: GtkHandle, name: &str) {
        if let Ok(c_name) = CString::new(name) {
            unsafe {
                (self.gtk_stack_add_named)(stack.0, child.0, c_name.as_ptr());
            }
        }
    }

    pub fn stack_remove(&self, stack: GtkHandle, child: GtkHandle) {
        unsafe {
            (self.gtk_stack_remove)(stack.0, child.0);
        }
    }

    pub fn set_stack_visible_child_name(&self, stack: GtkHandle, name: &str) {
        if let Ok(c_name) = CString::new(name) {
            unsafe {
                (self.gtk_stack_set_visible_child_name)(stack.0, c_name.as_ptr());
            }
        }
    }

    pub fn set_stack_transition(&self, stack: GtkHandle, transition: &str) {
        let code = match transition.trim() {
            "none" => 0,
            "slide-right" => 2,
            "slide-left" => 3,
            "slide-up" => 4,
            "slide-down" => 5,
            "slide-left-right" => 6,
            "slide-up-down" => 7,
            _ => 1, // crossfade
        };
        unsafe {
            (self.gtk_stack_set_transition_type)(stack.0, code);
        }
    }

    pub fn create_expander(&self) -> GtkHandle {
        unsafe { GtkHandle((self.gtk_expander_new)(c"".as_ptr())) }
    }

    pub fn set_expander_label(&self, exp: GtkHandle, label: &str) {
        if let Ok(c_label) = CString::new(label) {
            unsafe {
                (self.gtk_expander_set_label)(exp.0, c_label.as_ptr());
            }
        }
    }

    pub fn set_expander_expanded(&self, exp: GtkHandle, expanded: bool) {
        unsafe {
            (self.gtk_expander_set_expanded)(exp.0, i32::from(expanded));
        }
    }

    pub fn get_expander_expanded(&self, exp: GtkHandle) -> bool {
        unsafe { (self.gtk_expander_get_expanded)(exp.0) != 0 }
    }

    pub fn set_expander_child(&self, exp: GtkHandle, child: Option<GtkHandle>) {
        let ptr = child.map_or(std::ptr::null_mut(), |h| h.0);
        unsafe {
            (self.gtk_expander_set_child)(exp.0, ptr);
        }
    }

    pub fn create_text_view(&self) -> GtkHandle {
        unsafe {
            let tv = (self.gtk_text_view_new)();
            (self.gtk_text_view_set_wrap_mode)(tv, 3); // GTK_WRAP_WORD_CHAR
            GtkHandle(tv)
        }
    }

    pub fn text_view_buffer(&self, tv: GtkHandle) -> GtkHandle {
        unsafe { GtkHandle((self.gtk_text_view_get_buffer)(tv.0)) }
    }

    pub fn set_text_view_text(&self, tv: GtkHandle, text: &str) {
        if let Ok(c_str) = CString::new(text) {
            unsafe {
                let buf = (self.gtk_text_view_get_buffer)(tv.0);
                if !buf.is_null() {
                    (self.gtk_text_buffer_set_text)(buf, c_str.as_ptr(), -1);
                }
            }
        }
    }

    pub fn set_text_view_editable(&self, tv: GtkHandle, editable: bool) {
        unsafe {
            (self.gtk_text_view_set_editable)(tv.0, i32::from(editable));
        }
    }

    pub fn set_text_view_monospace(&self, tv: GtkHandle, monospace: bool) {
        unsafe {
            (self.gtk_text_view_set_monospace)(tv.0, i32::from(monospace));
        }
    }

    pub fn create_password_entry(&self) -> GtkHandle {
        unsafe {
            let pe = (self.gtk_password_entry_new)();
            (self.gtk_password_entry_set_show_peek_icon)(pe, 1);
            GtkHandle(pe)
        }
    }

    pub fn set_password_peek_icon(&self, pe: GtkHandle, show: bool) {
        unsafe {
            (self.gtk_password_entry_set_show_peek_icon)(pe.0, i32::from(show));
        }
    }

    pub fn create_search_entry(&self) -> GtkHandle {
        unsafe { GtkHandle((self.gtk_search_entry_new)()) }
    }

    pub fn set_search_entry_placeholder(&self, se: GtkHandle, placeholder: &str) {
        if let Ok(c_str) = CString::new(placeholder) {
            unsafe {
                (self.gtk_search_entry_set_placeholder_text)(se.0, c_str.as_ptr());
            }
        }
    }

    pub fn create_spin_button(&self) -> GtkHandle {
        unsafe { GtkHandle((self.gtk_spin_button_new_with_range)(0.0, 100.0, 1.0)) }
    }

    pub fn set_spin_button_value(&self, sb: GtkHandle, val: f64) {
        unsafe {
            (self.gtk_spin_button_set_value)(sb.0, val);
        }
    }

    pub fn get_spin_button_value(&self, sb: GtkHandle) -> i64 {
        unsafe { i64::from((self.gtk_spin_button_get_value_as_int)(sb.0)) }
    }

    pub fn set_spin_button_range(&self, sb: GtkHandle, min: f64, max: f64) {
        unsafe {
            (self.gtk_spin_button_set_range)(sb.0, min, max);
        }
    }

    pub fn set_spin_button_step(&self, sb: GtkHandle, step: f64) {
        unsafe {
            (self.gtk_spin_button_set_increments)(sb.0, step, step * 10.0);
        }
    }

    pub fn create_clamp(&self) -> GtkHandle {
        if let Some(adw_clamp_new) = self.adw_clamp_new {
            unsafe {
                let clamp = adw_clamp_new();
                if let Some(set_max) = self.adw_clamp_set_maximum_size {
                    set_max(clamp, 800);
                }
                return GtkHandle(clamp);
            }
        }
        self.create_box(0)
    }

    pub fn set_clamp_max_size(&self, clamp: GtkHandle, max_size: i32) {
        if let Some(set_max) = self.adw_clamp_set_maximum_size {
            unsafe {
                set_max(clamp.0, max_size);
            }
        }
    }

    pub fn set_clamp_child(&self, clamp: GtkHandle, child: Option<GtkHandle>) {
        if let Some(set_child) = self.adw_clamp_set_child {
            let ptr = child.map_or(std::ptr::null_mut(), |h| h.0);
            unsafe {
                set_child(clamp.0, ptr);
            }
        } else if let Some(ch) = child {
            self.box_append(clamp, ch);
        }
    }

    pub fn set_window_title(&self, win: GtkHandle, title: &str) {
        if let Ok(c_str) = CString::new(title) {
            unsafe {
                (self.gtk_window_set_title)(win.0, c_str.as_ptr());
            }
        }
    }

    pub fn set_window_titlebar(&self, win: GtkHandle, header_bar: GtkHandle) {
        unsafe {
            let handle = (self.gtk_window_handle_new)();
            (self.gtk_widget_set_hexpand)(handle, 1);
            (self.gtk_widget_set_hexpand)(header_bar.0, 1);
            (self.gtk_window_handle_set_child)(handle, header_bar.0);
            (self.gtk_window_set_titlebar)(win.0, handle);
        }
    }

    pub fn clear_window_titlebar(&self, win: GtkHandle) {
        unsafe {
            (self.gtk_window_set_titlebar)(win.0, std::ptr::null_mut());
        }
    }

    pub fn set_header_bar_title(&self, hb: GtkHandle, title: &str, subtitle: &str) {
        let c_title = CString::new(title).ok();
        let c_sub = CString::new(subtitle).ok();
        let title_ptr = c_title.as_ref().map_or(std::ptr::null(), |s| s.as_ptr());
        let sub_ptr = c_sub.as_ref().map_or(std::ptr::null(), |s| s.as_ptr());

        if let Some(adw_title_new) = self.adw_window_title_new {
            unsafe {
                let win_title = adw_title_new(title_ptr, sub_ptr);
                (self.gtk_widget_set_can_target)(win_title, 0);
                (self.gtk_header_bar_set_title_widget)(hb.0, win_title);
            }
        } else {
            unsafe {
                let title_box = (self.gtk_box_new)(GTK_ORIENTATION_VERTICAL, 0);
                self.set_halign(GtkHandle(title_box), "center");
                self.set_valign(GtkHandle(title_box), "center");
                let title_lbl = (self.gtk_label_new)(title_ptr);
                self.add_css_class(GtkHandle(title_lbl), "title");
                (self.gtk_widget_set_can_target)(title_lbl, 0);
                (self.gtk_box_append)(title_box, title_lbl);
                if !subtitle.trim().is_empty() {
                    let sub_lbl = (self.gtk_label_new)(sub_ptr);
                    self.add_css_class(GtkHandle(sub_lbl), "subtitle");
                    (self.gtk_widget_set_can_target)(sub_lbl, 0);
                    (self.gtk_box_append)(title_box, sub_lbl);
                }
                (self.gtk_widget_set_can_target)(title_box, 0);
                (self.gtk_header_bar_set_title_widget)(hb.0, title_box);
            }
        }
    }

    pub fn set_header_bar_show_title_buttons(&self, hb: GtkHandle, show: bool) {
        unsafe {
            (self.gtk_header_bar_set_show_title_buttons)(hb.0, i32::from(show));
        }
    }

    pub fn header_bar_pack_end(&self, hb: GtkHandle, child: GtkHandle) {
        unsafe {
            (self.gtk_header_bar_pack_end)(hb.0, child.0);
        }
    }

    pub fn header_bar_remove(&self, hb: GtkHandle, child: GtkHandle) {
        unsafe {
            (self.gtk_header_bar_remove)(hb.0, child.0);
        }
    }

    pub fn set_scrolled_child(&self, scroll: GtkHandle, child: GtkHandle) {
        unsafe {
            (self.gtk_scrolled_window_set_child)(scroll.0, child.0);
        }
    }

    pub fn clear_scrolled_child(&self, scroll: GtkHandle) {
        unsafe {
            (self.gtk_scrolled_window_set_child)(scroll.0, std::ptr::null_mut());
        }
    }

    pub fn set_scrolled_min_height(&self, scroll: GtkHandle, min_height: i32) {
        unsafe {
            (self.gtk_scrolled_window_set_min_content_height)(scroll.0, min_height);
        }
    }

    pub fn set_orientation(&self, widget: GtkHandle, vertical: bool) {
        let orient = if vertical {
            GTK_ORIENTATION_VERTICAL
        } else {
            GTK_ORIENTATION_HORIZONTAL
        };
        unsafe {
            (self.gtk_orientable_set_orientation)(widget.0, orient);
        }
    }

    pub fn set_box_spacing(&self, bx: GtkHandle, spacing: i32) {
        unsafe {
            (self.gtk_box_set_spacing)(bx.0, spacing);
        }
    }

    pub fn set_grid_row_spacing(&self, grid: GtkHandle, row_spacing: u32) {
        unsafe {
            (self.gtk_grid_set_row_spacing)(grid.0, row_spacing);
        }
    }

    pub fn set_grid_column_spacing(&self, grid: GtkHandle, col_spacing: u32) {
        unsafe {
            (self.gtk_grid_set_column_spacing)(grid.0, col_spacing);
        }
    }

    pub fn grid_attach(&self, grid: GtkHandle, child: GtkHandle, col: i32, row: i32) {
        unsafe {
            (self.gtk_grid_attach)(grid.0, child.0, col, row, 1, 1);
        }
    }

    pub fn grid_remove(&self, grid: GtkHandle, child: GtkHandle) {
        unsafe {
            (self.gtk_grid_remove)(grid.0, child.0);
        }
    }

    pub fn set_hexpand(&self, widget: GtkHandle, expand: bool) {
        unsafe {
            (self.gtk_widget_set_hexpand)(widget.0, i32::from(expand));
        }
    }

    pub fn set_vexpand(&self, widget: GtkHandle, expand: bool) {
        unsafe {
            (self.gtk_widget_set_vexpand)(widget.0, i32::from(expand));
        }
    }

    fn parse_gtk_align(align: &str) -> c_int {
        match align.trim() {
            "start" => 1,
            "end" => 2,
            "center" => 3,
            "baseline" => 4,
            _ => 0,
        }
    }

    pub fn set_halign(&self, widget: GtkHandle, align: &str) {
        unsafe {
            (self.gtk_widget_set_halign)(widget.0, Self::parse_gtk_align(align));
        }
    }

    pub fn set_valign(&self, widget: GtkHandle, align: &str) {
        unsafe {
            (self.gtk_widget_set_valign)(widget.0, Self::parse_gtk_align(align));
        }
    }

    pub fn set_uniform_margin(&self, widget: GtkHandle, margin: i32) {
        unsafe {
            (self.gtk_widget_set_margin_top)(widget.0, margin);
            (self.gtk_widget_set_margin_bottom)(widget.0, margin);
            (self.gtk_widget_set_margin_start)(widget.0, margin);
            (self.gtk_widget_set_margin_end)(widget.0, margin);
        }
    }

    pub fn set_margin_side(&self, widget: GtkHandle, side: &str, margin: i32) {
        unsafe {
            match side {
                "margin-top" => (self.gtk_widget_set_margin_top)(widget.0, margin),
                "margin-bottom" => (self.gtk_widget_set_margin_bottom)(widget.0, margin),
                "margin-start" => (self.gtk_widget_set_margin_start)(widget.0, margin),
                "margin-end" => (self.gtk_widget_set_margin_end)(widget.0, margin),
                _ => {}
            }
        }
    }
}
