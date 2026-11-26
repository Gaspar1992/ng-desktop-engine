#![allow(unsafe_code)]

use super::PlatformBackend;
use super::gtk4_callbacks::{
    drain_pending_events, on_gtk_button_clicked, on_gtk_check_toggled,
    on_gtk_drop_down_selected_notify, on_gtk_entry_activate, on_gtk_entry_changed,
    on_gtk_list_box_row_activated, on_gtk_popover_closed, on_gtk_scale_value_changed,
    on_gtk_scroll_edge_reached, on_gtk_scroll_value_changed, on_gtk_search_stop,
    on_gtk_switch_active_notify, pack_node_id, set_window_prevent_close,
};
use super::gtk4_sys::{Gtk4Sys, GtkHandle};
use crate::desktop::DesktopTheme;
use crate::engine::{
    EventMask, EventPayload, NativeEvent, NodeId, RenderCommand, StyleId, StyleRule,
};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GtkWidgetType {
    Window,
    Dialog,
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
    ListView,
    Popover,
    Anchor,
}

impl GtkWidgetType {
    pub(super) const fn is_toplevel(self) -> bool {
        matches!(self, Self::Window | Self::Dialog)
    }

    fn from_tag(tag: &str) -> Option<Self> {
        match super::WidgetTag::from_tag(tag) {
            super::WidgetTag::Window => Some(Self::Window),
            super::WidgetTag::Dialog => Some(Self::Dialog),
            super::WidgetTag::HeaderBar => Some(Self::HeaderBar),
            super::WidgetTag::Scroll => Some(Self::Scroll),
            super::WidgetTag::Box => Some(Self::Box),
            super::WidgetTag::Grid => Some(Self::Grid),
            super::WidgetTag::Separator => Some(Self::Separator),
            super::WidgetTag::Label => Some(Self::Label),
            super::WidgetTag::Button => Some(Self::Button),
            super::WidgetTag::Entry => Some(Self::Entry),
            super::WidgetTag::Switch => Some(Self::Switch),
            super::WidgetTag::CheckButton => Some(Self::CheckButton),
            super::WidgetTag::Slider => Some(Self::Slider),
            super::WidgetTag::DropDown => Some(Self::DropDown),
            super::WidgetTag::Image => Some(Self::Image),
            super::WidgetTag::LevelBar => Some(Self::LevelBar),
            super::WidgetTag::Spinner => Some(Self::Spinner),
            super::WidgetTag::ProgressBar => Some(Self::ProgressBar),
            super::WidgetTag::FlowBox => Some(Self::FlowBox),
            super::WidgetTag::Paned => Some(Self::Paned),
            super::WidgetTag::Stack => Some(Self::Stack),
            super::WidgetTag::Expander => Some(Self::Expander),
            super::WidgetTag::TextView => Some(Self::TextView),
            super::WidgetTag::PasswordEntry => Some(Self::PasswordEntry),
            super::WidgetTag::SearchEntry => Some(Self::SearchEntry),
            super::WidgetTag::SpinButton => Some(Self::SpinButton),
            super::WidgetTag::Clamp => Some(Self::Clamp),
            super::WidgetTag::ListView => Some(Self::ListView),
            super::WidgetTag::Popover => Some(Self::Popover),
            super::WidgetTag::Anchor => Some(Self::Anchor),
            super::WidgetTag::Unknown => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct DialogConfig {
    pub(super) modal: bool,
    pub(super) transient_for: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct PopoverConfig {
    pub(super) open: bool,
    pub(super) anchor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WidgetVisibility {
    Visible,
    Hidden,
}

impl WidgetVisibility {
    pub(super) const fn from_bool(visible: bool) -> Self {
        if visible { Self::Visible } else { Self::Hidden }
    }

    pub(super) const fn is_visible(self) -> bool {
        matches!(self, Self::Visible)
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
    /// Tracks which optional GTK4 `EventControllers` are currently attached to this widget.
    pub(super) attached_events: EventMask,
    /// When true on a toplevel (`Window`/`Dialog`), `close-request` notifies JS and prevents OS destruction until `close-window`.
    pub(super) prevent_close: bool,
    /// Dialog modality and transient parent metadata.
    pub(super) dialog: DialogConfig,
    /// Explicit column count for `GtkGrid` parents (`columns` / `max-children`), and optional child cell (`col`/`row`).
    pub(super) grid_columns: i32,
    pub(super) grid_col: Option<i32>,
    pub(super) grid_row: Option<i32>,
    /// Popover open state and optional anchor ID metadata.
    pub(super) popover: PopoverConfig,
    /// Tracks explicit visibility flag (notably to defer presenting hidden dialogs until visible=true).
    pub(super) visibility: WidgetVisibility,
    /// Tracks `(min, max)` numeric bounds for `Slider`, `SpinButton`, and `LevelBar`.
    pub(super) value_range: (f64, f64),
}

/// Concrete Linux GTK4 + Libadwaita platform adapter backed by `libgtk-4.so.1`.
pub struct GtkLinuxBackend {
    pub flushed_frames: u64,
    app_id: String,
    icon_search_dir: String,
    pub(super) gtk: Option<Gtk4Sys>,
    pub(super) widgets: HashMap<NodeId, NativeGtkWidget>,
    pub(super) styles: HashMap<StyleId, StyleRule>,
    pub(super) node_classes: HashMap<NodeId, Vec<String>>,
    pub(super) header_titles: HashMap<NodeId, (String, String)>,
    pub(super) dom_id_to_node: HashMap<String, NodeId>,
    pub(super) parent_of: HashMap<NodeId, NodeId>,
    css_dirty: bool,
    css_reload_count: u64,
    pub(super) active_theme: DesktopTheme,
    pub(super) viewport_width: i32,
    pending_viewport_width: std::cell::Cell<Option<i32>>,
    pulse_tick: std::cell::Cell<u32>,
    window_states: std::cell::RefCell<HashMap<NodeId, (i32, i32, bool, bool)>>,
}

impl Drop for GtkLinuxBackend {
    fn drop(&mut self) {
        let drained: Vec<(NodeId, NativeGtkWidget)> = self.widgets.drain().collect();
        for &(id, ref widget) in &drained {
            if widget.kind.is_toplevel() {
                set_window_prevent_close(pack_node_id(id), false);
            }
        }
        let Some(gtk) = &self.gtk else {
            return;
        };
        for (_, widget) in &drained {
            if widget.kind == GtkWidgetType::Popover
                && let Some(handle) = widget.handle
            {
                gtk.popover_detach(handle);
            }
        }
        let has_open_windows = gtk.open_window_count() > 0;
        if has_open_windows {
            for (_, widget) in &drained {
                if widget.kind.is_toplevel()
                    && let Some(handle) = widget.handle
                {
                    gtk.destroy_window(handle);
                }
            }
        }
        for (_, widget) in &drained {
            if !widget.kind.is_toplevel()
                && let Some(handle) = widget.handle
            {
                gtk.unref_widget(handle);
            }
        }
    }
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
            node_classes: HashMap::new(),
            header_titles: HashMap::new(),
            dom_id_to_node: HashMap::new(),
            parent_of: HashMap::new(),
            css_dirty: false,
            css_reload_count: 0,
            active_theme: DesktopTheme::Dark,
            viewport_width: 1020,
            pending_viewport_width: std::cell::Cell::new(None),
            pulse_tick: std::cell::Cell::new(0),
            window_states: std::cell::RefCell::new(HashMap::new()),
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
            node_classes: HashMap::new(),
            header_titles: HashMap::new(),
            dom_id_to_node: HashMap::new(),
            parent_of: HashMap::new(),
            css_dirty: false,
            css_reload_count: 0,
            active_theme: DesktopTheme::Dark,
            viewport_width: 1020,
            pending_viewport_width: std::cell::Cell::new(None),
            pulse_tick: std::cell::Cell::new(0),
            window_states: std::cell::RefCell::new(HashMap::new()),
        }
    }

    #[must_use]
    pub const fn active_theme(&self) -> DesktopTheme {
        self.active_theme
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

    /// Returns the ordered child `NodeId` slice tracked for a container widget.
    #[must_use]
    pub fn widget_children(&self, id: NodeId) -> Option<&[NodeId]> {
        self.widgets.get(&id).map(|w| w.children.as_slice())
    }

    /// Returns how many times the GTK CSS stylesheet has been rebuilt and reloaded.
    #[must_use]
    pub const fn css_reload_count(&self) -> u64 {
        self.css_reload_count
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

    #[must_use]
    pub fn attached_event_mask(&self, node: NodeId) -> EventMask {
        self.widgets
            .get(&node)
            .map_or(EventMask::NONE, |w| w.attached_events)
    }

    #[must_use]
    pub fn is_prevent_close(&self, node: NodeId) -> bool {
        self.widgets.get(&node).is_some_and(|w| w.prevent_close)
    }

    #[must_use]
    pub fn is_modal(&self, node: NodeId) -> bool {
        self.widgets.get(&node).is_some_and(|w| w.dialog.modal)
    }

    #[must_use]
    pub fn transient_for(&self, node: NodeId) -> Option<&str> {
        self.widgets
            .get(&node)
            .and_then(|w| w.dialog.transient_for.as_deref())
    }

    #[must_use]
    pub fn is_popover_open(&self, node: NodeId) -> bool {
        self.widgets.get(&node).is_some_and(|w| w.popover.open)
    }

    #[must_use]
    pub fn popover_anchor(&self, node: NodeId) -> Option<&str> {
        self.widgets
            .get(&node)
            .and_then(|w| w.popover.anchor.as_deref())
    }

    #[must_use]
    pub fn toplevel_count(&self) -> usize {
        self.widgets
            .values()
            .filter(|w| w.kind.is_toplevel())
            .count()
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
        events.retain(|ev| self.widgets.contains_key(&ev.node));
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
                        GtkWidgetType::TextView => {
                            EventPayload::Text(gtk.get_text_view_text(handle))
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
                        GtkWidgetType::Scroll if ev.event_name == "scroll" => gtk
                            .scroll_metrics_json(handle)
                            .map_or(EventPayload::None, EventPayload::Text),
                        _ => EventPayload::None,
                    };
                }
            }
        }
        self.poll_window_lifecycle_events(&mut events);
        events
    }

    fn poll_window_lifecycle_events(&self, events: &mut Vec<NativeEvent>) {
        let Some(gtk) = &self.gtk else { return };
        if gtk.open_window_count() == 0 {
            self.window_states.borrow_mut().clear();
            return;
        }
        let mut prev_states = self.window_states.borrow_mut();
        for (&id, widget) in &self.widgets {
            if !widget.kind.is_toplevel() {
                continue;
            }
            let Some(handle) = widget.handle else {
                continue;
            };
            let (w, h) = gtk.widget_size(handle);
            if w <= 0 || h <= 0 {
                continue;
            }
            let (maximized, active) = gtk.window_flags(handle);
            let prev = prev_states.get(&id).copied();
            if prev.is_none_or(|(pw, ph, pm, _)| pw != w || ph != h || pm != maximized) {
                self.pending_viewport_width.set(Some(w));
                events.push(NativeEvent {
                    node: id,
                    event_name: String::from("resize"),
                    payload: EventPayload::Text(format!(
                        r#"{{"width":{w},"height":{h},"maximized":{maximized}}}"#
                    )),
                });
            }
            if prev.is_some_and(|(_, _, pm, _)| pm != maximized) {
                events.push(NativeEvent {
                    node: id,
                    event_name: String::from("maximize"),
                    payload: EventPayload::Bool(maximized),
                });
            }
            if prev.is_some_and(|(_, _, _, pa)| pa != active) {
                events.push(NativeEvent {
                    node: id,
                    event_name: String::from(if active { "focus" } else { "blur" }),
                    payload: EventPayload::Bool(active),
                });
            }
            prev_states.insert(id, (w, h, maximized, active));
        }
    }

    fn apply_gtk_set_text(&mut self, id: NodeId, text: &str) {
        let (Some(gtk), Some(widget)) = (&self.gtk, self.widgets.get(&id)) else {
            return;
        };
        let Some(handle) = widget.handle else { return };
        match widget.kind {
            GtkWidgetType::Window | GtkWidgetType::Dialog => gtk.set_window_title(handle, text),
            GtkWidgetType::HeaderBar => {
                let entry = self.header_titles.entry(id).or_default();
                entry.0 = text.to_string();
                let (t, s) = (entry.0.clone(), entry.1.clone());
                gtk.set_header_bar_title(handle, &t, &s);
            }
            GtkWidgetType::Label => gtk.set_label_text(handle, text),
            GtkWidgetType::Button => gtk.set_button_label(handle, text),
            GtkWidgetType::Entry | GtkWidgetType::SearchEntry | GtkWidgetType::PasswordEntry => {
                gtk.set_entry_text(handle, text);
            }
            GtkWidgetType::TextView => gtk.set_text_view_text(handle, text),
            GtkWidgetType::CheckButton => gtk.set_check_button_label(handle, text),
            GtkWidgetType::Expander => gtk.set_expander_label(handle, text),
            GtkWidgetType::Image => gtk.set_image_file(handle, text),
            _ => {}
        }
    }

    fn apply_command_to_gtk(&mut self, cmd: &RenderCommand) {
        match cmd {
            RenderCommand::RegisterStyle { style_id, rule } => {
                self.styles.insert(*style_id, rule.clone());
                self.css_dirty = true;
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
                        attached_events: EventMask::NONE,
                        prevent_close: false,
                        dialog: DialogConfig::default(),
                        grid_columns: 2,
                        grid_col: None,
                        grid_row: None,
                        popover: PopoverConfig::default(),
                        visibility: WidgetVisibility::Visible,
                        value_range: (0.0, 100.0),
                    },
                );
            }
            RenderCommand::SetText { id, text } => self.apply_gtk_set_text(*id, text),
            RenderCommand::SetAttribute { node, key, value } => {
                self.apply_gtk_set_attribute(*node, key, value);
            }
            RenderCommand::SetWindowIcon { node, icon } => {
                let (Some(gtk), Some(widget)) = (&self.gtk, self.widgets.get(node)) else {
                    return;
                };
                if widget.kind.is_toplevel()
                    && let Some(handle) = widget.handle
                {
                    gtk.configure_window_icon(handle, &icon.app_id, &self.icon_search_dir);
                }
            }
            RenderCommand::ApplyStyleClass { node, style_id } => {
                self.apply_style_class_command(*node, *style_id, true);
            }
            RenderCommand::RemoveStyleClass { node, style_id } => {
                self.apply_style_class_command(*node, *style_id, false);
            }
            RenderCommand::SetTheme { theme } => {
                if self.active_theme != *theme {
                    self.active_theme = *theme;
                    self.css_dirty = true;
                    if let Some(gtk) = &self.gtk {
                        gtk.set_color_scheme(*theme == DesktopTheme::Dark);
                    }
                    self.reapply_all_node_style_layouts();
                } else if let Some(gtk) = &self.gtk {
                    gtk.set_color_scheme(*theme == DesktopTheme::Dark);
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
                self.apply_gtk_destroy_widget(*id);
            }
        }
    }

    fn apply_gtk_set_attribute(&mut self, node: NodeId, key: &str, value: &str) {
        if key == "width"
            && self
                .widgets
                .get(&node)
                .is_some_and(|w| w.kind.is_toplevel())
            && let Some(px) = Self::parse_px_i32(value)
            && px > 0
            && px != self.viewport_width
        {
            self.set_viewport_width(px);
        }
        self.apply_native_attribute(node, key, value);
    }

    fn apply_style_class_command(&mut self, node: NodeId, style_id: StyleId, add: bool) {
        let class_name_opt = self.styles.get(&style_id).map(|r| r.class_name.clone());
        let Some(class_name) = class_name_opt else {
            return;
        };
        if let (Some(gtk), Some(widget)) = (&self.gtk, self.widgets.get(&node))
            && let Some(handle) = widget.handle
        {
            if add {
                gtk.add_css_class(handle, &class_name);
            } else {
                gtk.remove_css_class(handle, &class_name);
            }
        }
        if add {
            let entry = self.node_classes.entry(node).or_default();
            if !entry.contains(&class_name) {
                entry.push(class_name.clone());
            }
            self.reapply_node_class_layouts(node, &class_name);
        } else if let Some(classes) = self.node_classes.get_mut(&node) {
            classes.retain(|c| c != &class_name);
        }
    }

    fn set_viewport_width(&mut self, width: i32) {
        if width > 0 && width != self.viewport_width {
            self.viewport_width = width;
            if self
                .styles
                .values()
                .any(|r| r.media_min_width.is_some() || r.media_max_width.is_some())
            {
                self.css_dirty = true;
            }
            self.reapply_all_node_style_layouts();
        }
    }

    fn reapply_node_class_layouts(&mut self, node: NodeId, class_name: &str) {
        let mut matching: Vec<(StyleId, StyleRule)> = self
            .styles
            .iter()
            .filter(|(_, r)| {
                r.class_name == class_name
                    && r.pseudo_class.is_none()
                    && r.matches_context(self.active_theme, self.viewport_width)
            })
            .map(|(&id, r)| (id, r.clone()))
            .collect();
        matching.sort_by_key(|(id, _)| id.0);
        for (_, rule) in matching {
            self.apply_style_rule_layout(node, &rule);
        }
    }

    fn reapply_all_node_style_layouts(&mut self) {
        let pairs: Vec<(NodeId, Vec<String>)> = self
            .node_classes
            .iter()
            .map(|(&node, classes)| (node, classes.clone()))
            .collect();
        for (node, classes) in pairs {
            for class_name in classes {
                self.reapply_node_class_layouts(node, &class_name);
            }
        }
    }

    fn apply_gtk_destroy_widget(&mut self, id: NodeId) {
        self.header_titles.remove(&id);
        self.window_states.borrow_mut().remove(&id);
        self.parent_of.remove(&id);
        self.node_classes.remove(&id);
        self.dom_id_to_node.retain(|_, node| *node != id);
        set_window_prevent_close(pack_node_id(id), false);
        if let Some(widget) = self.widgets.remove(&id)
            && let (Some(gtk), Some(handle)) = (&self.gtk, widget.handle)
        {
            if widget.kind == GtkWidgetType::Popover {
                gtk.popover_detach(handle);
            }
            if widget.kind.is_toplevel() {
                gtk.destroy_window(handle);
            } else {
                gtk.unref_widget(handle);
            }
        }
    }

    fn insert_widget_record(&mut self, id: NodeId, handle: Option<GtkHandle>, kind: GtkWidgetType) {
        if !kind.is_toplevel()
            && let (Some(gtk), Some(h)) = (&self.gtk, handle)
        {
            gtk.ref_sink_widget(h);
        }
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
                attached_events: EventMask::NONE,
                prevent_close: false,
                dialog: DialogConfig {
                    modal: kind == GtkWidgetType::Dialog,
                    transient_for: None,
                },
                grid_columns: 2,
                grid_col: None,
                grid_row: None,
                popover: PopoverConfig::default(),
                visibility: WidgetVisibility::Visible,
                value_range: (0.0, 100.0),
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
                let packed = pack_node_id(id);
                gtk.connect_edge_reached(scroll, on_gtk_scroll_edge_reached, packed);
                gtk.connect_scroll_adjustment(scroll, on_gtk_scroll_value_changed, packed);
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
                let packed = pack_node_id(id);
                gtk.connect_signal(entry, c"changed", on_gtk_entry_changed, packed);
                gtk.connect_signal(entry, c"activate", on_gtk_entry_activate, packed);
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
                let packed = pack_node_id(id);
                gtk.connect_signal(pe, c"changed", on_gtk_entry_changed, packed);
                gtk.connect_signal(pe, c"activate", on_gtk_entry_activate, packed);
                (Some(pe), GtkWidgetType::PasswordEntry)
            }
            "ng-search-entry" | "gtk-search-entry" => {
                let se = gtk.create_search_entry();
                let packed = pack_node_id(id);
                gtk.connect_signal(se, c"changed", on_gtk_entry_changed, packed);
                gtk.connect_signal(se, c"activate", on_gtk_entry_activate, packed);
                gtk.connect_signal(se, c"stop-search", on_gtk_search_stop, packed);
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
            "ng-dialog" | "gtk-dialog" | "adw-dialog" => {
                (Some(gtk.create_dialog()), GtkWidgetType::Dialog)
            }
            "ng-list" | "ng-table" | "gtk-list-box" | "gtk-list-view" => {
                let list_box = gtk.create_list_box();
                gtk.connect_list_box_row_activated(
                    list_box,
                    on_gtk_list_box_row_activated,
                    pack_node_id(id),
                );
                (Some(list_box), GtkWidgetType::ListView)
            }
            "ng-popover" | "gtk-popover" | "ng-menu" => {
                let pop = gtk.create_popover();
                gtk.connect_signal(pop, c"closed", on_gtk_popover_closed, pack_node_id(id));
                (Some(pop), GtkWidgetType::Popover)
            }
            _ => return None,
        };
        Some(pair)
    }

    pub(super) fn sync_popover_anchor(
        &self,
        popover_node: NodeId,
        fallback_parent: Option<NodeId>,
    ) {
        let (Some(gtk), Some(pop_w)) = (&self.gtk, self.widgets.get(&popover_node)) else {
            return;
        };
        if pop_w.kind != GtkWidgetType::Popover {
            return;
        }
        let Some(pop_h) = pop_w.handle else { return };
        let anchor_handle = pop_w
            .popover
            .anchor
            .as_deref()
            .and_then(|dom_id| self.dom_id_to_node.get(dom_id))
            .filter(|&&nid| nid != popover_node)
            .and_then(|nid| self.widgets.get(nid))
            .and_then(|w| w.handle)
            .or_else(|| {
                fallback_parent
                    .or_else(|| self.parent_of.get(&popover_node).copied())
                    .filter(|&nid| nid != popover_node)
                    .and_then(|nid| self.widgets.get(&nid))
                    .and_then(|w| w.handle)
            });
        if let Some(target_h) = anchor_handle {
            gtk.popover_attach(pop_h, target_h);
            if pop_w.popover.open {
                gtk.popover_popup(pop_h);
            }
        }
    }

    pub(super) fn resolve_transient_window_handle(
        &self,
        dialog_node: NodeId,
        explicit_target: Option<&str>,
    ) -> Option<GtkHandle> {
        if let Some(target_dom) = explicit_target
            && !target_dom.trim().is_empty()
            && let Some(&target_node) = self.dom_id_to_node.get(target_dom.trim())
            && target_node != dialog_node
            && let Some(w) = self.widgets.get(&target_node)
            && w.kind.is_toplevel()
        {
            return w.handle;
        }
        let mut cursor = self.parent_of.get(&dialog_node).copied();
        let mut steps = 0;
        while let Some(curr) = cursor {
            if steps > 64 {
                break;
            }
            steps += 1;
            if curr != dialog_node
                && let Some(w) = self.widgets.get(&curr)
                && w.kind.is_toplevel()
                && let Some(h) = w.handle
            {
                return Some(h);
            }
            cursor = self.parent_of.get(&curr).copied();
        }
        None
    }

    fn sync_dialog_transient_and_present(&self, dialog_node: NodeId) {
        let (Some(gtk), Some(dialog_w)) = (&self.gtk, self.widgets.get(&dialog_node)) else {
            return;
        };
        if dialog_w.kind != GtkWidgetType::Dialog {
            return;
        }
        let Some(d_h) = dialog_w.handle else {
            return;
        };
        if let Some(parent_h) = self
            .resolve_transient_window_handle(dialog_node, dialog_w.dialog.transient_for.as_deref())
        {
            gtk.set_window_transient_for(d_h, Some(parent_h));
        }
        if dialog_w.visibility.is_visible() {
            gtk.present_window(d_h);
        }
    }

    fn sync_paned_children(&self, paned_id: NodeId) {
        let (Some(gtk), Some(paned_w)) = (&self.gtk, self.widgets.get(&paned_id)) else {
            return;
        };
        let Some(p_h) = paned_w.handle else { return };
        let mut real_handles = paned_w
            .children
            .iter()
            .filter_map(|cid| self.widgets.get(cid).and_then(|w| w.handle));
        let start = real_handles.next();
        let end = real_handles.next();
        gtk.paned_set_start_child(p_h, None);
        gtk.paned_set_end_child(p_h, None);
        gtk.paned_set_start_child(p_h, start);
        gtk.paned_set_end_child(p_h, end);
    }

    fn resolve_grid_cell(&self, parent: NodeId, child: NodeId, fallback_idx: i32) -> (i32, i32) {
        let cols = self
            .widgets
            .get(&parent)
            .map_or(2, |w| w.grid_columns)
            .max(1);
        let child_w = self.widgets.get(&child);
        let col = child_w
            .and_then(|w| w.grid_col)
            .unwrap_or(fallback_idx % cols);
        let row = child_w
            .and_then(|w| w.grid_row)
            .unwrap_or(fallback_idx / cols);
        (col, row)
    }

    fn apply_append_child(&mut self, parent: NodeId, child: NodeId) {
        self.parent_of.insert(child, parent);
        let (Some(parent_w), Some(child_w)) = (self.widgets.get(&parent), self.widgets.get(&child))
        else {
            return;
        };
        let parent_kind = parent_w.kind;
        if child_w.kind.is_toplevel() {
            if let Some(pw) = self.widgets.get_mut(&parent)
                && !pw.children.contains(&child)
            {
                pw.children.push(child);
            }
            self.sync_dialog_transient_and_present(child);
            return;
        }
        if child_w.kind == GtkWidgetType::Popover {
            if let Some(pw) = self.widgets.get_mut(&parent)
                && !pw.children.contains(&child)
            {
                pw.children.push(child);
            }
            self.sync_popover_anchor(child, Some(parent));
            return;
        }
        if parent_kind == GtkWidgetType::Paned {
            if let Some(pw) = self.widgets.get_mut(&parent) {
                pw.children.push(child);
            }
            self.sync_paned_children(parent);
            return;
        }
        let child_index = i32::try_from(parent_w.children.len()).unwrap_or(0);
        let grid_cell = if parent_kind == GtkWidgetType::Grid {
            Some(self.resolve_grid_cell(parent, child, child_index))
        } else {
            None
        };
        let (Some(parent_w), Some(child_w)) = (self.widgets.get(&parent), self.widgets.get(&child))
        else {
            return;
        };
        if let (Some(gtk), Some(p_h), Some(c_h)) = (&self.gtk, parent_w.handle, child_w.handle) {
            match parent_kind {
                kind if kind.is_toplevel() => {
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
                GtkWidgetType::ListView => gtk.list_box_append(p_h, c_h),
                GtkWidgetType::Popover => gtk.set_popover_child(p_h, Some(c_h)),
                GtkWidgetType::Stack => {
                    gtk.stack_add_named(p_h, c_h, &format!("page_{child_index}"));
                }
                GtkWidgetType::Expander => gtk.set_expander_child(p_h, Some(c_h)),
                GtkWidgetType::Clamp => gtk.set_clamp_child(p_h, Some(c_h)),
                GtkWidgetType::Grid => {
                    let (col, row) = grid_cell.unwrap_or((child_index % 2, child_index / 2));
                    gtk.grid_attach(p_h, c_h, col, row);
                }
                _ => {}
            }
        }
        if let Some(pw) = self.widgets.get_mut(&parent) {
            pw.children.push(child);
        }
    }

    fn apply_insert_before(&mut self, parent: NodeId, child: NodeId, ref_child: Option<NodeId>) {
        self.parent_of.insert(child, parent);
        let (Some(parent_w), Some(child_w)) = (self.widgets.get(&parent), self.widgets.get(&child))
        else {
            return;
        };
        let parent_kind = parent_w.kind;
        if child_w.kind.is_toplevel() {
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
            self.sync_dialog_transient_and_present(child);
            return;
        }
        if child_w.kind == GtkWidgetType::Popover {
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
            self.sync_popover_anchor(child, Some(parent));
            return;
        }

        let mut prev_real_sibling: Option<GtkHandle> = None;
        let mut real_insert_idx: i32 = 0;
        let mut total_real_children: i32 = 0;
        let mut found_ref = false;
        for sibling_id in &parent_w.children {
            if Some(*sibling_id) == ref_child {
                found_ref = true;
            }
            if *sibling_id == child {
                continue;
            }
            if let Some(sib_w) = self.widgets.get(sibling_id)
                && let Some(h) = sib_w.handle
            {
                if !found_ref {
                    prev_real_sibling = Some(h);
                    real_insert_idx += 1;
                }
                total_real_children += 1;
            }
        }
        let target_idx = if ref_child.is_some() && found_ref {
            real_insert_idx
        } else {
            total_real_children
        };

        let grid_cell = if parent_kind == GtkWidgetType::Grid {
            Some(self.resolve_grid_cell(parent, child, target_idx))
        } else {
            None
        };
        let (Some(parent_w), Some(child_w)) = (self.widgets.get(&parent), self.widgets.get(&child))
        else {
            return;
        };

        if parent_kind != GtkWidgetType::Paned
            && let (Some(gtk), Some(c_h), Some(p_h)) = (&self.gtk, child_w.handle, parent_w.handle)
        {
            Self::attach_gtk_child_at(
                gtk,
                parent_kind,
                child_w.kind,
                p_h,
                c_h,
                ref_child.is_some(),
                prev_real_sibling,
                target_idx,
                grid_cell,
                parent_w.children.contains(&child),
            );
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
        if parent_kind == GtkWidgetType::Paned {
            self.sync_paned_children(parent);
        }
    }

    #[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
    fn attach_gtk_child_at(
        gtk: &Gtk4Sys,
        parent_kind: GtkWidgetType,
        child_kind: GtkWidgetType,
        p_h: GtkHandle,
        c_h: GtkHandle,
        has_ref: bool,
        prev_real_sibling: Option<GtkHandle>,
        target_idx: i32,
        grid_cell: Option<(i32, i32)>,
        is_existing_child: bool,
    ) {
        match parent_kind {
            kind if kind.is_toplevel() => {
                if child_kind == GtkWidgetType::HeaderBar {
                    gtk.set_window_titlebar(p_h, c_h);
                } else {
                    gtk.set_window_child(p_h, c_h);
                }
            }
            GtkWidgetType::HeaderBar => gtk.header_bar_pack_end(p_h, c_h),
            GtkWidgetType::Scroll => gtk.set_scrolled_child(p_h, c_h),
            GtkWidgetType::Clamp => gtk.set_clamp_child(p_h, Some(c_h)),
            GtkWidgetType::Expander => gtk.set_expander_child(p_h, Some(c_h)),
            GtkWidgetType::Box => {
                if is_existing_child {
                    gtk.box_reorder_after(p_h, c_h, prev_real_sibling);
                } else if has_ref {
                    gtk.box_insert_after(p_h, c_h, prev_real_sibling);
                } else {
                    gtk.box_append(p_h, c_h);
                }
            }
            GtkWidgetType::FlowBox => {
                if is_existing_child {
                    gtk.flow_box_reorder(p_h, c_h, target_idx, has_ref);
                } else if has_ref {
                    gtk.flow_box_insert(p_h, c_h, target_idx);
                } else {
                    gtk.flow_box_append(p_h, c_h);
                }
            }
            GtkWidgetType::ListView => {
                if is_existing_child {
                    gtk.list_box_reorder(p_h, c_h, target_idx, has_ref);
                } else if has_ref {
                    gtk.list_box_insert(p_h, c_h, target_idx);
                } else {
                    gtk.list_box_append(p_h, c_h);
                }
            }
            GtkWidgetType::Popover => gtk.set_popover_child(p_h, Some(c_h)),
            GtkWidgetType::Grid => {
                let (col, row) = grid_cell.unwrap_or((target_idx % 2, target_idx / 2));
                gtk.grid_attach(p_h, c_h, col, row);
            }
            GtkWidgetType::Stack => gtk.stack_add_named(p_h, c_h, &format!("page_{target_idx}")),
            _ => {}
        }
    }

    fn apply_remove_child(&mut self, parent: NodeId, child: NodeId) {
        self.parent_of.remove(&child);
        let mut is_paned = false;
        if let (Some(gtk), Some(parent_w), Some(child_w)) = (
            &self.gtk,
            self.widgets.get(&parent),
            self.widgets.get(&child),
        ) && let Some(p_h) = parent_w.handle
        {
            if child_w.kind == GtkWidgetType::Popover {
                if let Some(c_h) = child_w.handle {
                    gtk.popover_detach(c_h);
                }
            } else if parent_w.kind == GtkWidgetType::Paned {
                is_paned = true;
            } else if !child_w.kind.is_toplevel() && child_w.handle.is_some() {
                match parent_w.kind {
                    kind if kind.is_toplevel() => {
                        if child_w.kind == GtkWidgetType::HeaderBar {
                            gtk.clear_window_titlebar(p_h);
                        } else {
                            gtk.clear_window_child(p_h);
                        }
                    }
                    GtkWidgetType::HeaderBar => {
                        if let Some(c_h) = child_w.handle {
                            gtk.header_bar_remove(p_h, c_h);
                        }
                    }
                    GtkWidgetType::Scroll => gtk.clear_scrolled_child(p_h),
                    GtkWidgetType::Box => {
                        if let Some(c_h) = child_w.handle {
                            gtk.box_remove(p_h, c_h);
                        }
                    }
                    GtkWidgetType::Grid => {
                        if let Some(c_h) = child_w.handle {
                            gtk.grid_remove(p_h, c_h);
                        }
                    }
                    GtkWidgetType::FlowBox => {
                        if let Some(c_h) = child_w.handle {
                            gtk.flow_box_remove(p_h, c_h);
                        }
                    }
                    GtkWidgetType::ListView => {
                        if let Some(c_h) = child_w.handle {
                            gtk.list_box_remove(p_h, c_h);
                        }
                    }
                    GtkWidgetType::Popover => gtk.set_popover_child(p_h, None),
                    GtkWidgetType::Stack => {
                        if let Some(c_h) = child_w.handle {
                            gtk.stack_remove(p_h, c_h);
                        }
                    }
                    GtkWidgetType::Expander => gtk.set_expander_child(p_h, None),
                    GtkWidgetType::Clamp => gtk.set_clamp_child(p_h, None),
                    _ => {}
                }
            }
        }
        if let Some(pw) = self.widgets.get_mut(&parent) {
            pw.children.retain(|c| *c != child);
        }
        if is_paned {
            self.sync_paned_children(parent);
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
        match super::WidgetTag::from_tag(tag) {
            super::WidgetTag::Window => "AdwApplicationWindow",
            super::WidgetTag::Dialog => "AdwDialog",
            super::WidgetTag::HeaderBar => "AdwHeaderBar",
            super::WidgetTag::Scroll => "GtkScrolledWindow",
            super::WidgetTag::Box => "GtkBox",
            super::WidgetTag::Grid => "GtkGrid",
            super::WidgetTag::Separator => "GtkSeparator",
            super::WidgetTag::Label => "GtkLabel",
            super::WidgetTag::Button => "GtkButton",
            super::WidgetTag::Entry => "GtkEntry",
            super::WidgetTag::Switch => "GtkSwitch",
            super::WidgetTag::CheckButton => "GtkCheckButton",
            super::WidgetTag::Slider => "GtkScale",
            super::WidgetTag::DropDown => "GtkDropDown",
            super::WidgetTag::Image => "GtkPicture",
            super::WidgetTag::LevelBar => "GtkLevelBar",
            super::WidgetTag::Spinner => "GtkSpinner",
            super::WidgetTag::ProgressBar => "GtkProgressBar",
            super::WidgetTag::FlowBox => "GtkFlowBox",
            super::WidgetTag::Paned => "GtkPaned",
            super::WidgetTag::Stack => "GtkStack",
            super::WidgetTag::Expander => "GtkExpander",
            super::WidgetTag::TextView => "GtkTextView",
            super::WidgetTag::PasswordEntry => "GtkPasswordEntry",
            super::WidgetTag::SearchEntry => "GtkSearchEntry",
            super::WidgetTag::SpinButton => "GtkSpinButton",
            super::WidgetTag::Clamp => "AdwClamp",
            super::WidgetTag::ListView => "GtkListBox",
            super::WidgetTag::Popover => "GtkPopover",
            super::WidgetTag::Anchor => "GtkAnchorComment",
            super::WidgetTag::Unknown => "GtkWidget",
        }
    }

    fn is_native_display_active(&self) -> bool {
        Self::is_native_display_active(self)
    }

    fn open_window_count(&self) -> u32 {
        Self::open_window_count(self)
    }

    fn is_ffi_driver_loaded(&self) -> bool {
        Self::is_native_display_active(self)
    }

    fn step_native_events(&self, may_block: bool) -> Vec<NativeEvent> {
        Self::step_native_events(self, may_block)
    }

    fn flush_commands(&mut self, commands: &[RenderCommand]) {
        if let Some(new_w) = self.pending_viewport_width.take() {
            self.set_viewport_width(new_w);
        }
        if commands.is_empty() && !self.css_dirty {
            return;
        }
        if !commands.is_empty() {
            self.flushed_frames += 1;
            super::trace_flush_commands(
                self.backend_name(),
                self.flushed_frames,
                commands,
                |tag| self.map_native_widget(tag),
            );
            for cmd in commands {
                self.apply_command_to_gtk(cmd);
            }
        }
        if self.css_dirty {
            let stylesheet = self.rebuild_css_stylesheet();
            if let Some(gtk) = &self.gtk {
                gtk.load_css(&stylesheet);
            }
            self.css_dirty = false;
            self.css_reload_count += 1;
        }
    }
}
