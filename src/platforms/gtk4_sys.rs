#![allow(unsafe_code)]

#[cfg(target_os = "linux")]
use crate::ffi_dl::DynLib;
#[cfg(target_os = "linux")]
use std::ffi::CStr;
use std::ffi::{c_char, c_int, c_uint, c_ulong, c_void};

/// Opaque native GTK4 widget/object pointer encapsulated inside the FFI boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GtkHandle(pub(super) *mut c_void);

pub const GTK_ORIENTATION_HORIZONTAL: c_int = 0;
pub const GTK_ORIENTATION_VERTICAL: c_int = 1;
#[cfg(target_os = "linux")]
const GTK_STYLE_PROVIDER_PRIORITY_APPLICATION: c_uint = 600;

pub type GtkClickCallback = unsafe extern "C" fn(*mut c_void, *mut c_void);
pub type GtkNotifyCallback = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void);
pub type GtkEdgeReachedCallback = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void);

/// Dynamically loaded `GTK4` / `GLib` / `GObject` / `GIO` function table for Linux.
/// Isolates all C FFI symbol loading (`dlopen` / `dlsym`) from widget operations.
pub struct Gtk4Sys {
    pub(super) gtk_css_provider_load_from_string: unsafe extern "C" fn(*mut c_void, *const c_char),
    pub(super) gtk_icon_theme_add_search_path: unsafe extern "C" fn(*mut c_void, *const c_char),
    pub(super) gtk_window_new: unsafe extern "C" fn() -> *mut c_void,
    pub(super) gtk_window_set_title: unsafe extern "C" fn(*mut c_void, *const c_char),
    pub(super) gtk_window_set_titlebar: unsafe extern "C" fn(*mut c_void, *mut c_void),
    pub(super) gtk_window_handle_new: unsafe extern "C" fn() -> *mut c_void,
    pub(super) gtk_window_handle_set_child: unsafe extern "C" fn(*mut c_void, *mut c_void),
    pub(super) gtk_window_set_icon_name: unsafe extern "C" fn(*mut c_void, *const c_char),
    pub(super) gtk_window_set_default_icon_name: unsafe extern "C" fn(*const c_char),
    pub(super) gtk_window_set_default_size: unsafe extern "C" fn(*mut c_void, c_int, c_int),
    pub(super) gtk_window_set_child: unsafe extern "C" fn(*mut c_void, *mut c_void),
    pub(super) gtk_window_present: unsafe extern "C" fn(*mut c_void),
    pub(super) gtk_window_destroy: unsafe extern "C" fn(*mut c_void),
    pub(super) gtk_window_get_toplevels: unsafe extern "C" fn() -> *mut c_void,
    pub(super) gtk_header_bar_new: unsafe extern "C" fn() -> *mut c_void,
    pub(super) gtk_header_bar_set_show_title_buttons: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_header_bar_set_title_widget: unsafe extern "C" fn(*mut c_void, *mut c_void),
    pub(super) gtk_header_bar_pack_end: unsafe extern "C" fn(*mut c_void, *mut c_void),
    pub(super) adw_window_title_new:
        Option<unsafe extern "C" fn(*const c_char, *const c_char) -> *mut c_void>,
    pub(super) gtk_scrolled_window_new: unsafe extern "C" fn() -> *mut c_void,
    pub(super) gtk_scrolled_window_set_child: unsafe extern "C" fn(*mut c_void, *mut c_void),
    pub(super) gtk_scrolled_window_set_min_content_height: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_box_new: unsafe extern "C" fn(c_int, c_int) -> *mut c_void,
    pub(super) gtk_box_append: unsafe extern "C" fn(*mut c_void, *mut c_void),
    pub(super) gtk_box_remove: unsafe extern "C" fn(*mut c_void, *mut c_void),
    pub(super) gtk_box_insert_child_after:
        unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void),
    pub(super) gtk_box_set_spacing: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_grid_new: unsafe extern "C" fn() -> *mut c_void,
    pub(super) gtk_grid_attach:
        unsafe extern "C" fn(*mut c_void, *mut c_void, c_int, c_int, c_int, c_int),
    pub(super) gtk_grid_set_row_spacing: unsafe extern "C" fn(*mut c_void, c_uint),
    pub(super) gtk_grid_set_column_spacing: unsafe extern "C" fn(*mut c_void, c_uint),
    pub(super) gtk_separator_new: unsafe extern "C" fn(c_int) -> *mut c_void,
    pub(super) gtk_orientable_set_orientation: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_label_new: unsafe extern "C" fn(*const c_char) -> *mut c_void,
    pub(super) gtk_label_set_text: unsafe extern "C" fn(*mut c_void, *const c_char),
    pub(super) gtk_label_set_wrap: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_label_set_max_width_chars: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_button_new_with_label: unsafe extern "C" fn(*const c_char) -> *mut c_void,
    pub(super) gtk_button_set_label: unsafe extern "C" fn(*mut c_void, *const c_char),
    pub(super) gtk_entry_new: unsafe extern "C" fn() -> *mut c_void,
    pub(super) gtk_entry_set_placeholder_text: unsafe extern "C" fn(*mut c_void, *const c_char),
    pub(super) gtk_editable_set_text: unsafe extern "C" fn(*mut c_void, *const c_char),
    pub(super) gtk_editable_get_text: unsafe extern "C" fn(*mut c_void) -> *const c_char,
    pub(super) gtk_switch_new: unsafe extern "C" fn() -> *mut c_void,
    pub(super) gtk_switch_set_active: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_switch_get_active: unsafe extern "C" fn(*mut c_void) -> c_int,
    pub(super) gtk_check_button_new_with_label: unsafe extern "C" fn(*const c_char) -> *mut c_void,
    pub(super) gtk_check_button_set_label: unsafe extern "C" fn(*mut c_void, *const c_char),
    pub(super) gtk_check_button_set_active: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_check_button_get_active: unsafe extern "C" fn(*mut c_void) -> c_int,
    pub(super) gtk_scale_new_with_range: unsafe extern "C" fn(c_int, f64, f64, f64) -> *mut c_void,
    pub(super) gtk_range_set_value: unsafe extern "C" fn(*mut c_void, f64),
    pub(super) gtk_range_get_value: unsafe extern "C" fn(*mut c_void) -> f64,
    pub(super) gtk_drop_down_new_from_strings:
        unsafe extern "C" fn(*const *const c_char) -> *mut c_void,
    pub(super) gtk_drop_down_set_selected: unsafe extern "C" fn(*mut c_void, c_uint),
    pub(super) gtk_drop_down_get_selected: unsafe extern "C" fn(*mut c_void) -> c_uint,
    pub(super) gtk_drop_down_set_model: unsafe extern "C" fn(*mut c_void, *mut c_void),
    pub(super) gtk_string_list_new: unsafe extern "C" fn(*const *const c_char) -> *mut c_void,
    pub(super) gtk_widget_add_css_class: unsafe extern "C" fn(*mut c_void, *const c_char),
    pub(super) gtk_widget_remove_css_class: unsafe extern "C" fn(*mut c_void, *const c_char),
    pub(super) gtk_widget_set_can_target: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_widget_set_hexpand: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_widget_set_vexpand: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_widget_set_halign: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_widget_set_valign: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_widget_set_margin_top: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_widget_set_margin_bottom: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_widget_set_margin_start: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_widget_set_margin_end: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_widget_set_size_request: unsafe extern "C" fn(*mut c_void, c_int, c_int),
    pub(super) gtk_widget_set_visible: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_picture_new: unsafe extern "C" fn() -> *mut c_void,
    pub(super) gtk_picture_set_filename: unsafe extern "C" fn(*mut c_void, *const c_char),
    pub(super) gtk_picture_set_can_shrink: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_picture_set_keep_aspect_ratio: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_level_bar_new: unsafe extern "C" fn() -> *mut c_void,
    pub(super) gtk_level_bar_set_value: unsafe extern "C" fn(*mut c_void, f64),
    pub(super) gtk_level_bar_set_max_value: unsafe extern "C" fn(*mut c_void, f64),
    pub(super) gtk_spinner_new: unsafe extern "C" fn() -> *mut c_void,
    pub(super) gtk_spinner_set_spinning: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_progress_bar_new: unsafe extern "C" fn() -> *mut c_void,
    pub(super) gtk_progress_bar_set_fraction: unsafe extern "C" fn(*mut c_void, f64),
    pub(super) gtk_progress_bar_pulse: unsafe extern "C" fn(*mut c_void),
    pub(super) gtk_flow_box_new: unsafe extern "C" fn() -> *mut c_void,
    pub(super) gtk_flow_box_append: unsafe extern "C" fn(*mut c_void, *mut c_void),
    pub(super) gtk_flow_box_insert: unsafe extern "C" fn(*mut c_void, *mut c_void, c_int),
    pub(super) gtk_flow_box_set_selection_mode: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_flow_box_set_min_children_per_line: unsafe extern "C" fn(*mut c_void, c_uint),
    pub(super) gtk_flow_box_set_max_children_per_line: unsafe extern "C" fn(*mut c_void, c_uint),
    pub(super) gtk_flow_box_set_row_spacing: unsafe extern "C" fn(*mut c_void, c_uint),
    pub(super) gtk_flow_box_set_column_spacing: unsafe extern "C" fn(*mut c_void, c_uint),
    pub(super) gtk_flow_box_set_homogeneous: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_flow_box_remove: unsafe extern "C" fn(*mut c_void, *mut c_void),
    pub(super) gtk_paned_new: unsafe extern "C" fn(c_int) -> *mut c_void,
    pub(super) gtk_paned_set_start_child: unsafe extern "C" fn(*mut c_void, *mut c_void),
    pub(super) gtk_paned_set_end_child: unsafe extern "C" fn(*mut c_void, *mut c_void),
    pub(super) gtk_paned_set_position: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_stack_new: unsafe extern "C" fn() -> *mut c_void,
    pub(super) gtk_stack_add_named:
        unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_char) -> *mut c_void,
    pub(super) gtk_stack_remove: unsafe extern "C" fn(*mut c_void, *mut c_void),
    pub(super) gtk_stack_set_visible_child_name: unsafe extern "C" fn(*mut c_void, *const c_char),
    pub(super) gtk_stack_set_transition_type: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_expander_new: unsafe extern "C" fn(*const c_char) -> *mut c_void,
    pub(super) gtk_expander_set_label: unsafe extern "C" fn(*mut c_void, *const c_char),
    pub(super) gtk_expander_set_expanded: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_expander_get_expanded: unsafe extern "C" fn(*mut c_void) -> c_int,
    pub(super) gtk_expander_set_child: unsafe extern "C" fn(*mut c_void, *mut c_void),
    pub(super) gtk_text_view_new: unsafe extern "C" fn() -> *mut c_void,
    pub(super) gtk_text_view_get_buffer: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    pub(super) gtk_text_view_set_editable: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_text_view_set_monospace: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_text_view_set_wrap_mode: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_text_buffer_set_text: unsafe extern "C" fn(*mut c_void, *const c_char, c_int),
    pub(super) gtk_password_entry_new: unsafe extern "C" fn() -> *mut c_void,
    pub(super) gtk_password_entry_set_show_peek_icon: unsafe extern "C" fn(*mut c_void, c_int),
    pub(super) gtk_search_entry_new: unsafe extern "C" fn() -> *mut c_void,
    pub(super) gtk_search_entry_set_placeholder_text:
        unsafe extern "C" fn(*mut c_void, *const c_char),
    pub(super) gtk_spin_button_new_with_range: unsafe extern "C" fn(f64, f64, f64) -> *mut c_void,
    pub(super) gtk_spin_button_set_value: unsafe extern "C" fn(*mut c_void, f64),
    pub(super) gtk_spin_button_get_value_as_int: unsafe extern "C" fn(*mut c_void) -> c_int,
    pub(super) gtk_spin_button_set_range: unsafe extern "C" fn(*mut c_void, f64, f64),
    pub(super) gtk_spin_button_set_increments: unsafe extern "C" fn(*mut c_void, f64, f64),
    pub(super) adw_clamp_new: Option<unsafe extern "C" fn() -> *mut c_void>,
    pub(super) adw_clamp_set_maximum_size: Option<unsafe extern "C" fn(*mut c_void, c_int)>,
    pub(super) adw_clamp_set_child: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    pub(super) g_main_context_iteration: unsafe extern "C" fn(*mut c_void, c_int) -> c_int,
    pub(super) g_set_prgname: unsafe extern "C" fn(*const c_char),
    pub(super) g_set_application_name: unsafe extern "C" fn(*const c_char),
    pub(super) g_signal_connect_data: unsafe extern "C" fn(
        *mut c_void,
        *const c_char,
        *const c_void,
        *mut c_void,
        *mut c_void,
        c_uint,
    ) -> c_ulong,
    pub(super) g_object_unref: unsafe extern "C" fn(*mut c_void),
    pub(super) g_list_model_get_n_items: unsafe extern "C" fn(*mut c_void) -> c_uint,
    pub(super) css_provider: *mut c_void,
    pub(super) icon_theme: *mut c_void,
}

impl Gtk4Sys {
    #[cfg(target_os = "linux")]
    pub fn try_load() -> Option<Self> {
        Self::try_load_with_app_id("ng-desktop-app")
    }

    #[cfg(target_os = "linux")]
    #[allow(clippy::too_many_lines)]
    pub fn try_load_with_app_id(app_id: &str) -> Option<Self> {
        unsafe {
            let gtk_lib = Self::open_lib(c"libgtk-4.so.1")?;
            let gobject_lib = Self::open_lib(c"libgobject-2.0.so.0")?;
            let gio_lib = Self::open_lib(c"libgio-2.0.so.0")?;
            let glib_lib = Self::open_lib(c"libglib-2.0.so.0")?;

            let g_set_prgname: unsafe extern "C" fn(*const c_char) =
                Self::load_fn(glib_lib, c"g_set_prgname")?;
            let prg_cstr = std::ffi::CString::new(app_id)
                .unwrap_or_else(|_| std::ffi::CString::from(c"ng-desktop-app"));
            g_set_prgname(prg_cstr.as_ptr());

            let gtk_init_check: unsafe extern "C" fn() -> c_int =
                Self::load_fn(gtk_lib, c"gtk_init_check")?;
            if gtk_init_check() == 0 {
                return None;
            }

            let (css_provider, icon_theme) = Self::init_display_resources(gtk_lib)?;

            let adw_lib = Self::open_lib(c"libadwaita-1.so.0");
            if let Some(lib) = adw_lib
                && let Some(adw_init) = Self::load_fn::<unsafe extern "C" fn()>(lib, c"adw_init")
            {
                adw_init();
            }
            let adw_window_title_new =
                adw_lib.and_then(|lib| Self::load_fn(lib, c"adw_window_title_new"));
            let adw_clamp_new = adw_lib.and_then(|lib| Self::load_fn(lib, c"adw_clamp_new"));
            let adw_clamp_set_maximum_size =
                adw_lib.and_then(|lib| Self::load_fn(lib, c"adw_clamp_set_maximum_size"));
            let adw_clamp_set_child =
                adw_lib.and_then(|lib| Self::load_fn(lib, c"adw_clamp_set_child"));

            Some(Self {
                gtk_css_provider_load_from_string: Self::load_fn(
                    gtk_lib,
                    c"gtk_css_provider_load_from_string",
                )?,
                gtk_icon_theme_add_search_path: Self::load_fn(
                    gtk_lib,
                    c"gtk_icon_theme_add_search_path",
                )?,
                gtk_window_new: Self::load_fn(gtk_lib, c"gtk_window_new")?,
                gtk_window_set_title: Self::load_fn(gtk_lib, c"gtk_window_set_title")?,
                gtk_window_set_titlebar: Self::load_fn(gtk_lib, c"gtk_window_set_titlebar")?,
                gtk_window_handle_new: Self::load_fn(gtk_lib, c"gtk_window_handle_new")?,
                gtk_window_handle_set_child: Self::load_fn(
                    gtk_lib,
                    c"gtk_window_handle_set_child",
                )?,
                gtk_window_set_icon_name: Self::load_fn(gtk_lib, c"gtk_window_set_icon_name")?,
                gtk_window_set_default_icon_name: Self::load_fn(
                    gtk_lib,
                    c"gtk_window_set_default_icon_name",
                )?,
                gtk_window_set_default_size: Self::load_fn(
                    gtk_lib,
                    c"gtk_window_set_default_size",
                )?,
                gtk_window_set_child: Self::load_fn(gtk_lib, c"gtk_window_set_child")?,
                gtk_window_present: Self::load_fn(gtk_lib, c"gtk_window_present")?,
                gtk_window_destroy: Self::load_fn(gtk_lib, c"gtk_window_destroy")?,
                gtk_window_get_toplevels: Self::load_fn(gtk_lib, c"gtk_window_get_toplevels")?,
                gtk_header_bar_new: Self::load_fn(gtk_lib, c"gtk_header_bar_new")?,
                gtk_header_bar_set_show_title_buttons: Self::load_fn(
                    gtk_lib,
                    c"gtk_header_bar_set_show_title_buttons",
                )?,
                gtk_header_bar_set_title_widget: Self::load_fn(
                    gtk_lib,
                    c"gtk_header_bar_set_title_widget",
                )?,
                gtk_header_bar_pack_end: Self::load_fn(gtk_lib, c"gtk_header_bar_pack_end")?,
                adw_window_title_new,
                gtk_scrolled_window_new: Self::load_fn(gtk_lib, c"gtk_scrolled_window_new")?,
                gtk_scrolled_window_set_child: Self::load_fn(
                    gtk_lib,
                    c"gtk_scrolled_window_set_child",
                )?,
                gtk_scrolled_window_set_min_content_height: Self::load_fn(
                    gtk_lib,
                    c"gtk_scrolled_window_set_min_content_height",
                )?,
                gtk_box_new: Self::load_fn(gtk_lib, c"gtk_box_new")?,
                gtk_box_append: Self::load_fn(gtk_lib, c"gtk_box_append")?,
                gtk_box_remove: Self::load_fn(gtk_lib, c"gtk_box_remove")?,
                gtk_box_insert_child_after: Self::load_fn(gtk_lib, c"gtk_box_insert_child_after")?,
                gtk_box_set_spacing: Self::load_fn(gtk_lib, c"gtk_box_set_spacing")?,
                gtk_grid_new: Self::load_fn(gtk_lib, c"gtk_grid_new")?,
                gtk_grid_attach: Self::load_fn(gtk_lib, c"gtk_grid_attach")?,
                gtk_grid_set_row_spacing: Self::load_fn(gtk_lib, c"gtk_grid_set_row_spacing")?,
                gtk_grid_set_column_spacing: Self::load_fn(
                    gtk_lib,
                    c"gtk_grid_set_column_spacing",
                )?,
                gtk_separator_new: Self::load_fn(gtk_lib, c"gtk_separator_new")?,
                gtk_orientable_set_orientation: Self::load_fn(
                    gtk_lib,
                    c"gtk_orientable_set_orientation",
                )?,
                gtk_label_new: Self::load_fn(gtk_lib, c"gtk_label_new")?,
                gtk_label_set_text: Self::load_fn(gtk_lib, c"gtk_label_set_text")?,
                gtk_label_set_wrap: Self::load_fn(gtk_lib, c"gtk_label_set_wrap")?,
                gtk_label_set_max_width_chars: Self::load_fn(
                    gtk_lib,
                    c"gtk_label_set_max_width_chars",
                )?,
                gtk_button_new_with_label: Self::load_fn(gtk_lib, c"gtk_button_new_with_label")?,
                gtk_button_set_label: Self::load_fn(gtk_lib, c"gtk_button_set_label")?,
                gtk_entry_new: Self::load_fn(gtk_lib, c"gtk_entry_new")?,
                gtk_entry_set_placeholder_text: Self::load_fn(
                    gtk_lib,
                    c"gtk_entry_set_placeholder_text",
                )?,
                gtk_editable_set_text: Self::load_fn(gtk_lib, c"gtk_editable_set_text")?,
                gtk_editable_get_text: Self::load_fn(gtk_lib, c"gtk_editable_get_text")?,
                gtk_switch_new: Self::load_fn(gtk_lib, c"gtk_switch_new")?,
                gtk_switch_set_active: Self::load_fn(gtk_lib, c"gtk_switch_set_active")?,
                gtk_switch_get_active: Self::load_fn(gtk_lib, c"gtk_switch_get_active")?,
                gtk_check_button_new_with_label: Self::load_fn(
                    gtk_lib,
                    c"gtk_check_button_new_with_label",
                )?,
                gtk_check_button_set_label: Self::load_fn(gtk_lib, c"gtk_check_button_set_label")?,
                gtk_check_button_set_active: Self::load_fn(
                    gtk_lib,
                    c"gtk_check_button_set_active",
                )?,
                gtk_check_button_get_active: Self::load_fn(
                    gtk_lib,
                    c"gtk_check_button_get_active",
                )?,
                gtk_scale_new_with_range: Self::load_fn(gtk_lib, c"gtk_scale_new_with_range")?,
                gtk_range_set_value: Self::load_fn(gtk_lib, c"gtk_range_set_value")?,
                gtk_range_get_value: Self::load_fn(gtk_lib, c"gtk_range_get_value")?,
                gtk_drop_down_new_from_strings: Self::load_fn(
                    gtk_lib,
                    c"gtk_drop_down_new_from_strings",
                )?,
                gtk_drop_down_set_selected: Self::load_fn(gtk_lib, c"gtk_drop_down_set_selected")?,
                gtk_drop_down_get_selected: Self::load_fn(gtk_lib, c"gtk_drop_down_get_selected")?,
                gtk_drop_down_set_model: Self::load_fn(gtk_lib, c"gtk_drop_down_set_model")?,
                gtk_string_list_new: Self::load_fn(gtk_lib, c"gtk_string_list_new")?,
                gtk_widget_add_css_class: Self::load_fn(gtk_lib, c"gtk_widget_add_css_class")?,
                gtk_widget_remove_css_class: Self::load_fn(
                    gtk_lib,
                    c"gtk_widget_remove_css_class",
                )?,
                gtk_widget_set_can_target: Self::load_fn(gtk_lib, c"gtk_widget_set_can_target")?,
                gtk_widget_set_hexpand: Self::load_fn(gtk_lib, c"gtk_widget_set_hexpand")?,
                gtk_widget_set_vexpand: Self::load_fn(gtk_lib, c"gtk_widget_set_vexpand")?,
                gtk_widget_set_halign: Self::load_fn(gtk_lib, c"gtk_widget_set_halign")?,
                gtk_widget_set_valign: Self::load_fn(gtk_lib, c"gtk_widget_set_valign")?,
                gtk_widget_set_margin_top: Self::load_fn(gtk_lib, c"gtk_widget_set_margin_top")?,
                gtk_widget_set_margin_bottom: Self::load_fn(
                    gtk_lib,
                    c"gtk_widget_set_margin_bottom",
                )?,
                gtk_widget_set_margin_start: Self::load_fn(
                    gtk_lib,
                    c"gtk_widget_set_margin_start",
                )?,
                gtk_widget_set_margin_end: Self::load_fn(gtk_lib, c"gtk_widget_set_margin_end")?,
                gtk_widget_set_size_request: Self::load_fn(
                    gtk_lib,
                    c"gtk_widget_set_size_request",
                )?,
                gtk_widget_set_visible: Self::load_fn(gtk_lib, c"gtk_widget_set_visible")?,
                gtk_picture_new: Self::load_fn(gtk_lib, c"gtk_picture_new")?,
                gtk_picture_set_filename: Self::load_fn(gtk_lib, c"gtk_picture_set_filename")?,
                gtk_picture_set_can_shrink: Self::load_fn(gtk_lib, c"gtk_picture_set_can_shrink")?,
                gtk_picture_set_keep_aspect_ratio: Self::load_fn(
                    gtk_lib,
                    c"gtk_picture_set_keep_aspect_ratio",
                )?,
                gtk_level_bar_new: Self::load_fn(gtk_lib, c"gtk_level_bar_new")?,
                gtk_level_bar_set_value: Self::load_fn(gtk_lib, c"gtk_level_bar_set_value")?,
                gtk_level_bar_set_max_value: Self::load_fn(
                    gtk_lib,
                    c"gtk_level_bar_set_max_value",
                )?,
                gtk_spinner_new: Self::load_fn(gtk_lib, c"gtk_spinner_new")?,
                gtk_spinner_set_spinning: Self::load_fn(gtk_lib, c"gtk_spinner_set_spinning")?,
                gtk_progress_bar_new: Self::load_fn(gtk_lib, c"gtk_progress_bar_new")?,
                gtk_progress_bar_set_fraction: Self::load_fn(
                    gtk_lib,
                    c"gtk_progress_bar_set_fraction",
                )?,
                gtk_progress_bar_pulse: Self::load_fn(gtk_lib, c"gtk_progress_bar_pulse")?,
                gtk_flow_box_new: Self::load_fn(gtk_lib, c"gtk_flow_box_new")?,
                gtk_flow_box_append: Self::load_fn(gtk_lib, c"gtk_flow_box_append")?,
                gtk_flow_box_insert: Self::load_fn(gtk_lib, c"gtk_flow_box_insert")?,
                gtk_flow_box_set_selection_mode: Self::load_fn(
                    gtk_lib,
                    c"gtk_flow_box_set_selection_mode",
                )?,
                gtk_flow_box_set_min_children_per_line: Self::load_fn(
                    gtk_lib,
                    c"gtk_flow_box_set_min_children_per_line",
                )?,
                gtk_flow_box_set_max_children_per_line: Self::load_fn(
                    gtk_lib,
                    c"gtk_flow_box_set_max_children_per_line",
                )?,
                gtk_flow_box_set_row_spacing: Self::load_fn(
                    gtk_lib,
                    c"gtk_flow_box_set_row_spacing",
                )?,
                gtk_flow_box_set_column_spacing: Self::load_fn(
                    gtk_lib,
                    c"gtk_flow_box_set_column_spacing",
                )?,
                gtk_flow_box_set_homogeneous: Self::load_fn(
                    gtk_lib,
                    c"gtk_flow_box_set_homogeneous",
                )?,
                gtk_flow_box_remove: Self::load_fn(gtk_lib, c"gtk_flow_box_remove")?,
                gtk_paned_new: Self::load_fn(gtk_lib, c"gtk_paned_new")?,
                gtk_paned_set_start_child: Self::load_fn(gtk_lib, c"gtk_paned_set_start_child")?,
                gtk_paned_set_end_child: Self::load_fn(gtk_lib, c"gtk_paned_set_end_child")?,
                gtk_paned_set_position: Self::load_fn(gtk_lib, c"gtk_paned_set_position")?,
                gtk_stack_new: Self::load_fn(gtk_lib, c"gtk_stack_new")?,
                gtk_stack_add_named: Self::load_fn(gtk_lib, c"gtk_stack_add_named")?,
                gtk_stack_remove: Self::load_fn(gtk_lib, c"gtk_stack_remove")?,
                gtk_stack_set_visible_child_name: Self::load_fn(
                    gtk_lib,
                    c"gtk_stack_set_visible_child_name",
                )?,
                gtk_stack_set_transition_type: Self::load_fn(
                    gtk_lib,
                    c"gtk_stack_set_transition_type",
                )?,
                gtk_expander_new: Self::load_fn(gtk_lib, c"gtk_expander_new")?,
                gtk_expander_set_label: Self::load_fn(gtk_lib, c"gtk_expander_set_label")?,
                gtk_expander_set_expanded: Self::load_fn(gtk_lib, c"gtk_expander_set_expanded")?,
                gtk_expander_get_expanded: Self::load_fn(gtk_lib, c"gtk_expander_get_expanded")?,
                gtk_expander_set_child: Self::load_fn(gtk_lib, c"gtk_expander_set_child")?,
                gtk_text_view_new: Self::load_fn(gtk_lib, c"gtk_text_view_new")?,
                gtk_text_view_get_buffer: Self::load_fn(gtk_lib, c"gtk_text_view_get_buffer")?,
                gtk_text_view_set_editable: Self::load_fn(gtk_lib, c"gtk_text_view_set_editable")?,
                gtk_text_view_set_monospace: Self::load_fn(
                    gtk_lib,
                    c"gtk_text_view_set_monospace",
                )?,
                gtk_text_view_set_wrap_mode: Self::load_fn(
                    gtk_lib,
                    c"gtk_text_view_set_wrap_mode",
                )?,
                gtk_text_buffer_set_text: Self::load_fn(gtk_lib, c"gtk_text_buffer_set_text")?,
                gtk_password_entry_new: Self::load_fn(gtk_lib, c"gtk_password_entry_new")?,
                gtk_password_entry_set_show_peek_icon: Self::load_fn(
                    gtk_lib,
                    c"gtk_password_entry_set_show_peek_icon",
                )?,
                gtk_search_entry_new: Self::load_fn(gtk_lib, c"gtk_search_entry_new")?,
                gtk_search_entry_set_placeholder_text: Self::load_fn(
                    gtk_lib,
                    c"gtk_search_entry_set_placeholder_text",
                )?,
                gtk_spin_button_new_with_range: Self::load_fn(
                    gtk_lib,
                    c"gtk_spin_button_new_with_range",
                )?,
                gtk_spin_button_set_value: Self::load_fn(gtk_lib, c"gtk_spin_button_set_value")?,
                gtk_spin_button_get_value_as_int: Self::load_fn(
                    gtk_lib,
                    c"gtk_spin_button_get_value_as_int",
                )?,
                gtk_spin_button_set_range: Self::load_fn(gtk_lib, c"gtk_spin_button_set_range")?,
                gtk_spin_button_set_increments: Self::load_fn(
                    gtk_lib,
                    c"gtk_spin_button_set_increments",
                )?,
                adw_clamp_new,
                adw_clamp_set_maximum_size,
                adw_clamp_set_child,
                g_main_context_iteration: Self::load_fn(glib_lib, c"g_main_context_iteration")?,
                g_set_prgname,
                g_set_application_name: Self::load_fn(glib_lib, c"g_set_application_name")?,
                g_signal_connect_data: Self::load_fn(gobject_lib, c"g_signal_connect_data")?,
                g_object_unref: Self::load_fn(gobject_lib, c"g_object_unref")?,
                g_list_model_get_n_items: Self::load_fn(gio_lib, c"g_list_model_get_n_items")?,
                css_provider,
                icon_theme,
            })
        }
    }

    #[cfg(not(target_os = "linux"))]
    #[must_use]
    pub const fn try_load() -> Option<Self> {
        None
    }

    #[cfg(not(target_os = "linux"))]
    #[must_use]
    pub const fn try_load_with_app_id(_app_id: &str) -> Option<Self> {
        None
    }

    #[cfg(target_os = "linux")]
    unsafe fn init_display_resources(gtk_lib: *mut c_void) -> Option<(*mut c_void, *mut c_void)> {
        unsafe {
            let gdk_display_get_default: unsafe extern "C" fn() -> *mut c_void =
                Self::load_fn(gtk_lib, c"gdk_display_get_default")?;
            let gtk_css_provider_new: unsafe extern "C" fn() -> *mut c_void =
                Self::load_fn(gtk_lib, c"gtk_css_provider_new")?;
            let gtk_style_context_add_provider_for_display: unsafe extern "C" fn(
                *mut c_void,
                *mut c_void,
                c_uint,
            ) = Self::load_fn(gtk_lib, c"gtk_style_context_add_provider_for_display")?;
            let gtk_icon_theme_get_for_display: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
                Self::load_fn(gtk_lib, c"gtk_icon_theme_get_for_display")?;

            let display = gdk_display_get_default();
            let css_provider = gtk_css_provider_new();
            let icon_theme = if display.is_null() {
                std::ptr::null_mut()
            } else {
                if !css_provider.is_null() {
                    gtk_style_context_add_provider_for_display(
                        display,
                        css_provider,
                        GTK_STYLE_PROVIDER_PRIORITY_APPLICATION,
                    );
                }
                gtk_icon_theme_get_for_display(display)
            };
            Some((css_provider, icon_theme))
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
}
