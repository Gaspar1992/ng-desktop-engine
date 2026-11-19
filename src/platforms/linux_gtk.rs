#![allow(unsafe_code)]

use super::PlatformBackend;
use super::gtk4_callbacks::{
    drain_pending_events, on_gtk_button_clicked, on_gtk_check_toggled,
    on_gtk_drop_down_selected_notify, on_gtk_entry_changed, on_gtk_scale_value_changed,
    on_gtk_scroll_edge_reached, on_gtk_switch_active_notify, pack_node_id,
};
use super::gtk4_sys::{Gtk4Sys, GtkHandle};
use crate::engine::{EventPayload, NativeEvent, NodeId, RenderCommand, StyleId, StyleRule};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GtkWidgetType {
    Window,
    HeaderBar,
    Scroll,
    Box,
    Grid,
    Separator,
    Label,
    Button,
    Entry,
    Switch,
    CheckButton,
    Slider,
    DropDown,
    Image,
    LevelBar,
    Spinner,
    ProgressBar,
    FlowBox,
    Paned,
    Stack,
    Expander,
    TextView,
    PasswordEntry,
    SearchEntry,
    SpinButton,
    Clamp,
    Anchor,
}

impl GtkWidgetType {
    fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "ng-window" | "adw-window" => Some(Self::Window),
            "ng-header-bar" | "adw-header-bar" | "gtk-header-bar" => Some(Self::HeaderBar),
            "ng-scroll" | "gtk-scrolled-window" => Some(Self::Scroll),
            "ng-box" | "gtk-box" => Some(Self::Box),
            "ng-grid" | "gtk-grid" => Some(Self::Grid),
            "ng-separator" | "gtk-separator" => Some(Self::Separator),
            "ng-label" | "gtk-label" => Some(Self::Label),
            "ng-button" | "gtk-button" => Some(Self::Button),
            "ng-entry" | "gtk-entry" => Some(Self::Entry),
            "ng-switch" | "gtk-switch" => Some(Self::Switch),
            "ng-checkbox" | "ng-check" | "gtk-check-button" => Some(Self::CheckButton),
            "ng-slider" | "ng-scale" | "gtk-scale" => Some(Self::Slider),
            "ng-select" | "ng-dropdown" | "gtk-drop-down" => Some(Self::DropDown),
            "ng-image" | "gtk-picture" | "ng-picture" => Some(Self::Image),
            "ng-level-bar" | "gtk-level-bar" => Some(Self::LevelBar),
            "ng-spinner" | "gtk-spinner" => Some(Self::Spinner),
            "ng-progress-bar" | "gtk-progress-bar" => Some(Self::ProgressBar),
            "ng-flow" | "ng-flow-box" | "gtk-flow-box" => Some(Self::FlowBox),
            "ng-paned" | "gtk-paned" => Some(Self::Paned),
            "ng-stack" | "gtk-stack" | "adw-navigation-view" => Some(Self::Stack),
            "ng-expander" | "gtk-expander" => Some(Self::Expander),
            "ng-text-view" | "gtk-text-view" => Some(Self::TextView),
            "ng-password-entry" | "gtk-password-entry" => Some(Self::PasswordEntry),
            "ng-search-entry" | "gtk-search-entry" => Some(Self::SearchEntry),
            "ng-spin-button" | "gtk-spin-button" => Some(Self::SpinButton),
            "ng-clamp" | "adw-clamp" => Some(Self::Clamp),
            "ng-anchor" | "#comment" => Some(Self::Anchor),
            _ => None,
        }
    }
}

pub(super) struct NativeGtkWidget {
    pub(super) handle: Option<GtkHandle>,
    pub(super) kind: GtkWidgetType,
    pub(super) children: Vec<NodeId>,
    /// Indeterminate progress bars are pulsed from the event pump (GTK has no auto-pulse).
    pub(super) pulsing: bool,
    /// Tracks explicit width/height requests so updating one dimension preserves the other.
    pub(super) size_request: (i32, i32),
    /// Tracks whether native OS drag-and-drop (`GtkDropTarget`) is enabled on this widget.
    pub(super) drop_target: bool,
    /// Tracks programmatic scroll offset (`scroll-to="x,y"`) and anchor (`scroll-anchor="id"`).
    pub(super) scroll_position: Option<(i32, i32)>,
    pub(super) scroll_anchor: Option<String>,
}

/// Concrete Linux GTK4 + Libadwaita platform adapter backed by `libgtk-4.so.1`.
pub struct GtkLinuxBackend {
    pub flushed_frames: u64,
    app_id: String,
    icon_search_dir: String,
    pub(super) gtk: Option<Gtk4Sys>,
    pub(super) widgets: HashMap<NodeId, NativeGtkWidget>,
    pub(super) styles: HashMap<StyleId, StyleRule>,
    pub(super) header_titles: HashMap<NodeId, (String, String)>,
    pulse_tick: std::cell::Cell<u32>,
}

impl Default for GtkLinuxBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl GtkLinuxBackend {
    pub fn new() -> Self {
        Self::with_app_config("ng-desktop-app", "icons")
    }

    /// Initializes the GTK4 backend binding `g_set_prgname` and custom icon theme search paths
    /// from the resolved application configuration rather than hardcoded literals.
    pub fn with_app_config(app_id: &str, icon_search_dir: &str) -> Self {
        Self {
            flushed_frames: 0,
            app_id: String::from(app_id),
            icon_search_dir: String::from(icon_search_dir),
            gtk: Gtk4Sys::try_load_with_app_id(app_id),
            widgets: HashMap::new(),
            styles: HashMap::new(),
            header_titles: HashMap::new(),
            pulse_tick: std::cell::Cell::new(0),
        }
    }

    pub fn new_headless() -> Self {
        Self {
            flushed_frames: 0,
            app_id: String::from("ng-desktop-app"),
            icon_search_dir: String::from("icons"),
            gtk: None,
            widgets: HashMap::new(),
            styles: HashMap::new(),
            header_titles: HashMap::new(),
            pulse_tick: std::cell::Cell::new(0),
        }
    }

    #[must_use]
    pub fn app_id(&self) -> &str {
        &self.app_id
    }

    #[must_use]
    pub fn icon_search_dir(&self) -> &str {
        &self.icon_search_dir
    }

    pub fn is_native_display_active(&self) -> bool {
        self.gtk.is_some()
    }

    pub fn open_window_count(&self) -> u32 {
        self.gtk.as_ref().map_or(0, Gtk4Sys::open_window_count)
    }

    pub fn widget_count(&self) -> usize {
        self.widgets.len()
    }

    pub fn is_drop_target(&self, node: NodeId) -> bool {
        self.widgets.get(&node).is_some_and(|w| w.drop_target)
    }

    pub fn scroll_position(&self, node: NodeId) -> Option<(i32, i32)> {
        self.widgets.get(&node).and_then(|w| w.scroll_position)
    }

    pub fn scroll_anchor(&self, node: NodeId) -> Option<&str> {
        self.widgets
            .get(&node)
            .and_then(|w| w.scroll_anchor.as_deref())
    }

    pub fn step_event_loop(&self, may_block: bool) -> Vec<NodeId> {
        self.step_native_events(may_block)
            .into_iter()
            .map(|ev| ev.node)
            .collect()
    }

    pub fn step_native_events(&self, may_block: bool) -> Vec<NativeEvent> {
        if let Some(gtk) = &self.gtk {
            gtk.step_main_context(may_block);
        }
        let mut events = drain_pending_events();
        // ~60ms cadence at the 16ms pump rate: keeps indeterminate bars animating.
        let tick = self.pulse_tick.get().wrapping_add(1);
        self.pulse_tick.set(tick);
        if tick.is_multiple_of(4)
            && let Some(gtk) = &self.gtk
        {
            for widget in self.widgets.values().filter(|w| w.pulsing) {
                if let Some(handle) = widget.handle {
                    gtk.pulse_progress_bar(handle);
                }
            }
        }
        if let Some(gtk) = &self.gtk {
            for ev in &mut events {
                if ev.payload != EventPayload::None {
                    continue;
                }
                if let Some(widget) = self.widgets.get(&ev.node)
                    && let Some(handle) = widget.handle
                {
                    ev.payload = match widget.kind {
                        GtkWidgetType::Entry
                        | GtkWidgetType::SearchEntry
                        | GtkWidgetType::PasswordEntry => {
                            EventPayload::Text(gtk.get_entry_text(handle))
                        }
                        GtkWidgetType::Switch => EventPayload::Bool(gtk.get_switch_active(handle)),
                        GtkWidgetType::CheckButton => {
                            EventPayload::Bool(gtk.get_check_button_active(handle))
                        }
                        GtkWidgetType::Slider => EventPayload::Int(gtk.get_slider_value(handle)),
                        GtkWidgetType::SpinButton => {
                            EventPayload::Int(gtk.get_spin_button_value(handle))
                        }
                        GtkWidgetType::DropDown => {
                            EventPayload::Int(gtk.get_drop_down_selected(handle))
                        }
                        GtkWidgetType::Expander => {
                            EventPayload::Bool(gtk.get_expander_expanded(handle))
                        }
                        _ => EventPayload::None,
                    };
                }
            }
        }
        events
    }

    fn apply_command_to_gtk(&mut self, cmd: &RenderCommand) {
        match cmd {
            RenderCommand::RegisterStyle { style_id, rule } => {
                self.styles.insert(*style_id, rule.clone());
                let stylesheet = self.rebuild_css_stylesheet();
                if let Some(gtk) = &self.gtk {
                    gtk.load_css(&stylesheet);
                }
            }
            RenderCommand::CreateWidget { id, tag } => self.create_native_widget(*id, tag),
            RenderCommand::CreateComment { id, .. } => {
                self.widgets.insert(
                    *id,
                    NativeGtkWidget {
                        handle: None,
                        kind: GtkWidgetType::Anchor,
                        children: Vec::new(),
                        pulsing: false,
                        size_request: (-1, -1),
                        drop_target: false,
                        scroll_position: None,
                        scroll_anchor: None,
                    },
                );
            }
            RenderCommand::SetText { id, text } => {
                let (Some(gtk), Some(widget)) = (&self.gtk, self.widgets.get(id)) else {
                    return;
                };
                let Some(handle) = widget.handle else { return };
                match widget.kind {
                    GtkWidgetType::Window => gtk.set_window_title(handle, text),
                    GtkWidgetType::HeaderBar => {
                        let entry = self.header_titles.entry(*id).or_default();
                        entry.0.clone_from(text);
                        let (t, s) = (entry.0.clone(), entry.1.clone());
                        gtk.set_header_bar_title(handle, &t, &s);
                    }
                    GtkWidgetType::Label => gtk.set_label_text(handle, text),
                    GtkWidgetType::Button => gtk.set_button_label(handle, text),
                    GtkWidgetType::Entry
                    | GtkWidgetType::SearchEntry
                    | GtkWidgetType::PasswordEntry => gtk.set_entry_text(handle, text),
                    GtkWidgetType::TextView => gtk.set_text_view_text(handle, text),
                    GtkWidgetType::CheckButton => gtk.set_check_button_label(handle, text),
                    GtkWidgetType::Expander => gtk.set_expander_label(handle, text),
                    GtkWidgetType::Image => gtk.set_image_file(handle, text),
                    _ => {}
                }
            }
            RenderCommand::SetAttribute { node, key, value } => {
                self.apply_native_attribute(*node, key, value);
            }
            RenderCommand::SetWindowIcon { node, icon } => {
                let (Some(gtk), Some(widget)) = (&self.gtk, self.widgets.get(node)) else {
                    return;
                };
                if widget.kind == GtkWidgetType::Window
                    && let Some(handle) = widget.handle
                {
                    gtk.configure_window_icon(handle, &icon.app_id, &self.icon_search_dir);
                }
            }
            RenderCommand::ApplyStyleClass { node, style_id } => {
                let (Some(gtk), Some(widget), Some(rule)) =
                    (&self.gtk, self.widgets.get(node), self.styles.get(style_id))
                else {
                    return;
                };
                if let Some(handle) = widget.handle {
                    gtk.add_css_class(handle, &rule.class_name);
                }
            }
            RenderCommand::AppendChild { parent, child } => {
                self.apply_append_child(*parent, *child);
            }
            RenderCommand::InsertBefore {
                parent,
                child,
                ref_child,
            } => {
                self.apply_insert_before(*parent, *child, *ref_child);
            }
            RenderCommand::RemoveChild { parent, child } => {
                self.apply_remove_child(*parent, *child);
            }
            RenderCommand::DestroyWidget { id } => {
                if let (Some(gtk), Some(widget)) = (&self.gtk, self.widgets.remove(id))
                    && widget.kind == GtkWidgetType::Window
                    && let Some(handle) = widget.handle
                {
                    gtk.destroy_window(handle);
                }
            }
        }
    }

    fn insert_widget_record(&mut self, id: NodeId, handle: Option<GtkHandle>, kind: GtkWidgetType) {
        let size_request = if kind == GtkWidgetType::Image {
            (120, 120)
        } else {
            (-1, -1)
        };
        self.widgets.insert(
            id,
            NativeGtkWidget {
                handle,
                kind,
                children: Vec::new(),
                pulsing: false,
                size_request,
                drop_target: false,
                scroll_position: None,
                scroll_anchor: None,
            },
        );
    }

    fn create_native_widget(&mut self, id: NodeId, tag: &str) {
        let Some(gtk) = &self.gtk else {
            if let Some(kind) = GtkWidgetType::from_tag(tag) {
                self.insert_widget_record(id, None, kind);
            }
            return;
        };
        let (handle, kind) = match tag {
            "ng-window" | "adw-window" => (Some(gtk.create_window()), GtkWidgetType::Window),
            "ng-header-bar" | "adw-header-bar" | "gtk-header-bar" => {
                (Some(gtk.create_header_bar()), GtkWidgetType::HeaderBar)
            }
            "ng-scroll" | "gtk-scrolled-window" => {
                let scroll = gtk.create_scrolled_window();
                gtk.connect_edge_reached(scroll, on_gtk_scroll_edge_reached, pack_node_id(id));
                (Some(scroll), GtkWidgetType::Scroll)
            }
            "ng-box" | "gtk-box" => (Some(gtk.create_box(12)), GtkWidgetType::Box),
            "ng-grid" | "gtk-grid" => (Some(gtk.create_grid()), GtkWidgetType::Grid),
            "ng-separator" | "gtk-separator" => {
                (Some(gtk.create_separator()), GtkWidgetType::Separator)
            }
            "ng-label" | "gtk-label" => (Some(gtk.create_label()), GtkWidgetType::Label),
            "ng-button" | "gtk-button" => {
                let btn = gtk.create_button();
                gtk.connect_clicked(btn, on_gtk_button_clicked, pack_node_id(id));
                (Some(btn), GtkWidgetType::Button)
            }
            "ng-entry" | "gtk-entry" => {
                let entry = gtk.create_entry();
                gtk.connect_signal(entry, c"changed", on_gtk_entry_changed, pack_node_id(id));
                (Some(entry), GtkWidgetType::Entry)
            }
            "ng-switch" | "gtk-switch" => {
                let sw = gtk.create_switch();
                gtk.connect_notify_signal(
                    sw,
                    c"notify::active",
                    on_gtk_switch_active_notify,
                    pack_node_id(id),
                );
                (Some(sw), GtkWidgetType::Switch)
            }
            "ng-checkbox" | "ng-check" | "gtk-check-button" => {
                let chk = gtk.create_check_button();
                gtk.connect_signal(chk, c"toggled", on_gtk_check_toggled, pack_node_id(id));
                (Some(chk), GtkWidgetType::CheckButton)
            }
            "ng-slider" | "ng-scale" | "gtk-scale" => {
                let scale = gtk.create_slider();
                gtk.connect_signal(
                    scale,
                    c"value-changed",
                    on_gtk_scale_value_changed,
                    pack_node_id(id),
                );
                (Some(scale), GtkWidgetType::Slider)
            }
            "ng-select" | "ng-dropdown" | "gtk-drop-down" => {
                let dd = gtk.create_drop_down();
                gtk.connect_notify_signal(
                    dd,
                    c"notify::selected",
                    on_gtk_drop_down_selected_notify,
                    pack_node_id(id),
                );
                (Some(dd), GtkWidgetType::DropDown)
            }
            _ => match Self::create_extended_native_widget(gtk, id, tag) {
                Some(pair) => pair,
                None => return,
            },
        };
        self.insert_widget_record(id, handle, kind);
    }

    fn create_extended_native_widget(
        gtk: &Gtk4Sys,
        id: NodeId,
        tag: &str,
    ) -> Option<(Option<GtkHandle>, GtkWidgetType)> {
        let pair = match tag {
            "ng-image" | "gtk-picture" | "ng-picture" => {
                (Some(gtk.create_image()), GtkWidgetType::Image)
            }
            "ng-level-bar" | "gtk-level-bar" => {
                (Some(gtk.create_level_bar()), GtkWidgetType::LevelBar)
            }
            "ng-spinner" | "gtk-spinner" => (Some(gtk.create_spinner()), GtkWidgetType::Spinner),
            "ng-progress-bar" | "gtk-progress-bar" => {
                (Some(gtk.create_progress_bar()), GtkWidgetType::ProgressBar)
            }
            "ng-flow" | "ng-flow-box" | "gtk-flow-box" => {
                (Some(gtk.create_flow_box()), GtkWidgetType::FlowBox)
            }
            "ng-paned" | "gtk-paned" => (Some(gtk.create_paned()), GtkWidgetType::Paned),
            "ng-stack" | "gtk-stack" | "adw-navigation-view" => {
                (Some(gtk.create_stack()), GtkWidgetType::Stack)
            }
            "ng-expander" | "gtk-expander" => {
                let exp = gtk.create_expander();
                gtk.connect_notify_signal(
                    exp,
                    c"notify::expanded",
                    on_gtk_switch_active_notify,
                    pack_node_id(id),
                );
                (Some(exp), GtkWidgetType::Expander)
            }
            "ng-anchor" | "#comment" => (None, GtkWidgetType::Anchor),
            other => return Self::create_phase8_native_widget(gtk, id, other),
        };
        Some(pair)
    }

    fn create_phase8_native_widget(
        gtk: &Gtk4Sys,
        id: NodeId,
        tag: &str,
    ) -> Option<(Option<GtkHandle>, GtkWidgetType)> {
        let pair = match tag {
            "ng-text-view" | "gtk-text-view" => {
                let tv = gtk.create_text_view();
                let buf = gtk.text_view_buffer(tv);
                gtk.connect_signal(buf, c"changed", on_gtk_entry_changed, pack_node_id(id));
                (Some(tv), GtkWidgetType::TextView)
            }
            "ng-password-entry" | "gtk-password-entry" => {
                let pe = gtk.create_password_entry();
                gtk.connect_signal(pe, c"changed", on_gtk_entry_changed, pack_node_id(id));
                (Some(pe), GtkWidgetType::PasswordEntry)
            }
            "ng-search-entry" | "gtk-search-entry" => {
                let se = gtk.create_search_entry();
                gtk.connect_signal(se, c"changed", on_gtk_entry_changed, pack_node_id(id));
                (Some(se), GtkWidgetType::SearchEntry)
            }
            "ng-spin-button" | "gtk-spin-button" => {
                let sb = gtk.create_spin_button();
                gtk.connect_signal(
                    sb,
                    c"value-changed",
                    on_gtk_scale_value_changed,
                    pack_node_id(id),
                );
                (Some(sb), GtkWidgetType::SpinButton)
            }
            "ng-clamp" | "adw-clamp" => (Some(gtk.create_clamp()), GtkWidgetType::Clamp),
            _ => return None,
        };
        Some(pair)
    }

    fn apply_append_child(&mut self, parent: NodeId, child: NodeId) {
        let (Some(gtk), Some(parent_w), Some(child_w)) = (
            &self.gtk,
            self.widgets.get(&parent),
            self.widgets.get(&child),
        ) else {
            return;
        };
        let child_index = i32::try_from(parent_w.children.len()).unwrap_or(0);
        if let (Some(p_h), Some(c_h)) = (parent_w.handle, child_w.handle) {
            match parent_w.kind {
                GtkWidgetType::Window => {
                    if child_w.kind == GtkWidgetType::HeaderBar {
                        gtk.set_window_titlebar(p_h, c_h);
                    } else {
                        gtk.set_window_child(p_h, c_h);
                    }
                }
                GtkWidgetType::HeaderBar => gtk.header_bar_pack_end(p_h, c_h),
                GtkWidgetType::Scroll => gtk.set_scrolled_child(p_h, c_h),
                GtkWidgetType::Box => gtk.box_append(p_h, c_h),
                GtkWidgetType::FlowBox => gtk.flow_box_append(p_h, c_h),
                GtkWidgetType::Paned => {
                    if child_index == 0 {
                        gtk.paned_set_start_child(p_h, Some(c_h));
                    } else {
                        gtk.paned_set_end_child(p_h, Some(c_h));
                    }
                }
                GtkWidgetType::Stack => {
                    gtk.stack_add_named(p_h, c_h, &format!("page_{child_index}"));
                }
                GtkWidgetType::Expander => gtk.set_expander_child(p_h, Some(c_h)),
                GtkWidgetType::Clamp => gtk.set_clamp_child(p_h, Some(c_h)),
                GtkWidgetType::Grid => {
                    gtk.grid_attach(p_h, c_h, child_index % 2, child_index / 2);
                }
                _ => {}
            }
        }
        if let Some(pw) = self.widgets.get_mut(&parent) {
            pw.children.push(child);
        }
    }

    fn apply_insert_before(&mut self, parent: NodeId, child: NodeId, ref_child: Option<NodeId>) {
        let (Some(gtk), Some(parent_w), Some(child_w)) = (
            &self.gtk,
            self.widgets.get(&parent),
            self.widgets.get(&child),
        ) else {
            return;
        };

        let mut prev_real_sibling: Option<GtkHandle> = None;
        if let Some(ref_id) = ref_child
            && let Some(pos) = parent_w.children.iter().position(|c| *c == ref_id)
        {
            for prev_id in parent_w.children[..pos].iter().rev() {
                if let Some(prev_w) = self.widgets.get(prev_id)
                    && let Some(h) = prev_w.handle
                {
                    prev_real_sibling = Some(h);
                    break;
                }
            }
        }

        if parent_w.kind == GtkWidgetType::Box
            && let (Some(p_h), Some(c_h)) = (parent_w.handle, child_w.handle)
        {
            if ref_child.is_some() {
                gtk.box_insert_after(p_h, c_h, prev_real_sibling);
            } else {
                gtk.box_append(p_h, c_h);
            }
        }

        if let Some(pw) = self.widgets.get_mut(&parent) {
            pw.children.retain(|c| *c != child);
            if let Some(ref_id) = ref_child
                && let Some(pos) = pw.children.iter().position(|c| *c == ref_id)
            {
                pw.children.insert(pos, child);
            } else {
                pw.children.push(child);
            }
        }
    }

    fn apply_remove_child(&mut self, parent: NodeId, child: NodeId) {
        let (Some(gtk), Some(parent_w), Some(child_w)) = (
            &self.gtk,
            self.widgets.get(&parent),
            self.widgets.get(&child),
        ) else {
            return;
        };
        if let Some(p_h) = parent_w.handle {
            match parent_w.kind {
                GtkWidgetType::Box => {
                    if let Some(c_h) = child_w.handle {
                        gtk.box_remove(p_h, c_h);
                    }
                }
                GtkWidgetType::FlowBox => {
                    if let Some(c_h) = child_w.handle {
                        gtk.flow_box_remove(p_h, c_h);
                    }
                }
                GtkWidgetType::Stack => {
                    if let Some(c_h) = child_w.handle {
                        gtk.stack_remove(p_h, c_h);
                    }
                }
                GtkWidgetType::Paned => {
                    if parent_w.children.first() == Some(&child) {
                        gtk.paned_set_start_child(p_h, None);
                    } else {
                        gtk.paned_set_end_child(p_h, None);
                    }
                }
                GtkWidgetType::Expander => gtk.set_expander_child(p_h, None),
                GtkWidgetType::Clamp => gtk.set_clamp_child(p_h, None),
                _ => {}
            }
        }
        if let Some(pw) = self.widgets.get_mut(&parent) {
            pw.children.retain(|c| *c != child);
        }
    }
}

impl PlatformBackend for GtkLinuxBackend {
    fn backend_name(&self) -> &str {
        if self.gtk.is_some() {
            "Linux GTK4 Native (libgtk-4.so.1)"
        } else {
            "Linux GTK4 + Libadwaita"
        }
    }

    fn map_native_widget(&self, tag: &str) -> &'static str {
        match tag {
            "ng-window" | "adw-window" => "AdwApplicationWindow",
            "ng-header-bar" | "adw-header-bar" | "gtk-header-bar" => "AdwHeaderBar",
            "ng-scroll" | "gtk-scrolled-window" => "GtkScrolledWindow",
            "ng-box" | "gtk-box" => "GtkBox",
            "ng-grid" | "gtk-grid" => "GtkGrid",
            "ng-separator" | "gtk-separator" => "GtkSeparator",
            "ng-label" | "gtk-label" => "GtkLabel",
            "ng-button" | "gtk-button" => "GtkButton",
            "ng-entry" | "gtk-entry" => "GtkEntry",
            "ng-switch" | "gtk-switch" => "GtkSwitch",
            "ng-checkbox" | "ng-check" | "gtk-check-button" => "GtkCheckButton",
            "ng-slider" | "ng-scale" | "gtk-scale" => "GtkScale",
            "ng-select" | "ng-dropdown" | "gtk-drop-down" => "GtkDropDown",
            "ng-image" | "gtk-picture" | "ng-picture" => "GtkPicture",
            "ng-level-bar" | "gtk-level-bar" => "GtkLevelBar",
            "ng-spinner" | "gtk-spinner" => "GtkSpinner",
            "ng-progress-bar" | "gtk-progress-bar" => "GtkProgressBar",
            "ng-flow" | "ng-flow-box" | "gtk-flow-box" => "GtkFlowBox",
            "ng-paned" | "gtk-paned" => "GtkPaned",
            "ng-stack" | "gtk-stack" | "adw-navigation-view" => "GtkStack",
            "ng-expander" | "gtk-expander" => "GtkExpander",
            "ng-text-view" | "gtk-text-view" => "GtkTextView",
            "ng-password-entry" | "gtk-password-entry" => "GtkPasswordEntry",
            "ng-search-entry" | "gtk-search-entry" => "GtkSearchEntry",
            "ng-spin-button" | "gtk-spin-button" => "GtkSpinButton",
            "ng-clamp" | "adw-clamp" => "AdwClamp",
            "ng-anchor" | "#comment" => "GtkAnchorComment",
            _ => "GtkWidget",
        }
    }

    fn flush_commands(&mut self, commands: &[RenderCommand]) {
        if commands.is_empty() {
            return;
        }
        self.flushed_frames += 1;
        println!(
            "[{} | Frame #{}] Flushing {} render commands:",
            self.backend_name(),
            self.flushed_frames,
            commands.len()
        );
        for cmd in commands {
            if let RenderCommand::CreateWidget { id, tag } = cmd {
                println!(
                    "  -> CreateWidget {{ id: {id:?}, tag: {tag:?} => native: {} }}",
                    self.map_native_widget(tag)
                );
            } else {
                println!("  -> {cmd:?}");
            }
            self.apply_command_to_gtk(cmd);
        }
    }
}
