use super::gtk4_callbacks::{pack_node_id, set_window_prevent_close};
use super::gtk4_sys::{Gtk4Sys, GtkHandle};
use super::linux_gtk::{GtkLinuxBackend, GtkWidgetType, WidgetVisibility};
use super::scene_state::NativeSceneState;
use crate::engine::{EventMask, NodeId, StyleRule};
use std::fmt::Write as _;

/// Translates a single engine `StyleRule` into a GTK4 CSS class block (pure, unit-testable).
pub(super) fn render_gtk_css_rule(rule: &StyleRule) -> String {
    let mut css = String::new();
    let _ = writeln!(css, ".{} {{", rule.class_name);
    if let Some(bg) = &rule.background {
        let _ = writeln!(css, "  background: {bg};");
    }
    if let Some(fg) = &rule.foreground {
        let _ = writeln!(css, "  color: {fg};");
    }
    if rule.padding > 0 {
        let _ = writeln!(css, "  padding: {}px;", rule.padding);
    }
    if let Some(radius) = rule.border_radius {
        let _ = writeln!(css, "  border-radius: {radius}px;");
    }
    if let Some(size) = rule.font_size {
        let _ = writeln!(css, "  font-size: {size}px;");
    }
    if let Some(weight) = rule.font_weight {
        let _ = writeln!(css, "  font-weight: {weight};");
    }
    if let Some(mw) = rule.min_width {
        let _ = writeln!(css, "  min-width: {mw}px;");
    }
    if let Some(mh) = rule.min_height {
        let _ = writeln!(css, "  min-height: {mh}px;");
    }
    let _ = writeln!(css, "}}");
    css
}

impl GtkLinuxBackend {
    pub(super) fn rebuild_css_stylesheet(&self) -> String {
        let mut css = String::from(
            "window { background: #0f172a; color: #f8fafc; }\n\
             headerbar, windowhandle { min-height: 46px; }\n\
             searchentry, entry, spinbutton, textview { border-radius: 8px; padding: 6px 10px; }\n\
             button { border-radius: 8px; padding: 6px 12px; font-weight: 600; }\n\
             levelbar block.filled { background: #38bdf8; border-radius: 4px; }\n\
             levelbar block.empty { background: #1e293b; border-radius: 4px; }\n\
             listbox { background: transparent; }\n\
             popover > contents { background: #1e293b; color: #f8fafc; border-radius: 10px; padding: 8px; }\n",
        );
        let mut ordered: Vec<_> = self.styles.iter().collect();
        ordered.sort_by_key(|(style_id, _)| style_id.0);
        for (_, rule) in ordered {
            css.push_str(&render_gtk_css_rule(rule));
        }
        css
    }

    pub(super) fn parse_px_i32(raw: &str) -> Option<i32> {
        raw.trim().trim_end_matches("px").trim().parse::<i32>().ok()
    }

    fn apply_container_attribute(
        gtk: &Gtk4Sys,
        kind: GtkWidgetType,
        handle: GtkHandle,
        key: &str,
        value: &str,
    ) -> bool {
        match key {
            "spacing" | "gap" if matches!(kind, GtkWidgetType::Grid | GtkWidgetType::FlowBox) => {
                if let Some(px) = Self::parse_px_i32(value)
                    && let Ok(u) = u32::try_from(px)
                {
                    if kind == GtkWidgetType::Grid {
                        gtk.set_grid_row_spacing(handle, u);
                        gtk.set_grid_column_spacing(handle, u);
                    } else {
                        gtk.set_flow_box_row_spacing(handle, u);
                        gtk.set_flow_box_column_spacing(handle, u);
                    }
                }
                true
            }
            "row-spacing" if matches!(kind, GtkWidgetType::Grid | GtkWidgetType::FlowBox) => {
                if let Some(px) = Self::parse_px_i32(value)
                    && let Ok(u) = u32::try_from(px)
                {
                    if kind == GtkWidgetType::Grid {
                        gtk.set_grid_row_spacing(handle, u);
                    } else {
                        gtk.set_flow_box_row_spacing(handle, u);
                    }
                }
                true
            }
            "column-spacing" if matches!(kind, GtkWidgetType::Grid | GtkWidgetType::FlowBox) => {
                if let Some(px) = Self::parse_px_i32(value)
                    && let Ok(u) = u32::try_from(px)
                {
                    if kind == GtkWidgetType::Grid {
                        gtk.set_grid_column_spacing(handle, u);
                    } else {
                        gtk.set_flow_box_column_spacing(handle, u);
                    }
                }
                true
            }
            "min-children" | "min-per-row" if kind == GtkWidgetType::FlowBox => {
                if let Ok(u) = value.trim().parse::<u32>() {
                    gtk.set_flow_box_min_children_per_line(handle, u);
                }
                true
            }
            "max-children" | "max-per-row" if kind == GtkWidgetType::FlowBox => {
                if let Ok(u) = value.trim().parse::<u32>() {
                    gtk.set_flow_box_max_children_per_line(handle, u);
                }
                true
            }
            "homogeneous" if kind == GtkWidgetType::FlowBox => {
                gtk.set_flow_box_homogeneous(handle, value != "false" && value != "0");
                true
            }
            _ => Self::apply_phase8_container_attribute(gtk, kind, handle, key, value),
        }
    }

    fn apply_phase8_container_attribute(
        gtk: &Gtk4Sys,
        kind: GtkWidgetType,
        handle: GtkHandle,
        key: &str,
        value: &str,
    ) -> bool {
        match (kind, key) {
            (GtkWidgetType::Paned, "orientation") => {
                gtk.set_orientation(handle, value == "vertical");
                true
            }
            (GtkWidgetType::Paned, "position") => {
                if let Some(px) = Self::parse_px_i32(value) {
                    gtk.set_paned_position(handle, px);
                }
                true
            }
            (GtkWidgetType::Stack, "visible-child" | "active-page") => {
                gtk.set_stack_visible_child_name(handle, value);
                true
            }
            (GtkWidgetType::Stack, "transition" | "transition-type") => {
                gtk.set_stack_transition(handle, value);
                true
            }
            (GtkWidgetType::Expander, "label" | "title") => {
                gtk.set_expander_label(handle, value);
                true
            }
            (GtkWidgetType::Expander, "expanded" | "open") => {
                gtk.set_expander_expanded(handle, value == "true" || value == "1");
                true
            }
            (GtkWidgetType::Clamp, "max-size" | "maximum-size") => {
                if let Some(px) = Self::parse_px_i32(value) {
                    gtk.set_clamp_max_size(handle, px);
                }
                true
            }
            (GtkWidgetType::ListView, "selection-mode") => {
                gtk.set_list_box_selection_mode(handle, value);
                true
            }
            (GtkWidgetType::ListView, "show-separators" | "separators") => {
                gtk.set_list_box_show_separators(handle, value == "true" || value == "1");
                true
            }
            (GtkWidgetType::Popover, "position" | "placement") => {
                gtk.set_popover_position(handle, value);
                true
            }
            (GtkWidgetType::Popover, "autohide") => {
                gtk.set_popover_autohide(handle, value != "false" && value != "0");
                true
            }
            (GtkWidgetType::Popover, "has-arrow" | "arrow") => {
                gtk.set_popover_has_arrow(handle, value != "false" && value != "0");
                true
            }
            _ => false,
        }
    }

    fn apply_popover_metadata_attribute(&mut self, node: NodeId, key: &str, value: &str) -> bool {
        if key != "open" && key != "visible" && key != "anchor" && key != "anchor-id" {
            return false;
        }
        let is_popover = self
            .widgets
            .get(&node)
            .is_some_and(|w| w.kind == GtkWidgetType::Popover);
        if !is_popover {
            return false;
        }
        match key {
            "open" | "visible" => {
                let is_open = NativeSceneState::parse_truthy_attr(value);
                if let Some(w) = self.widgets.get_mut(&node) {
                    w.popover.open = is_open;
                    w.visibility = WidgetVisibility::from_bool(is_open);
                    if let (Some(gtk), Some(handle)) = (&self.gtk, w.handle) {
                        if is_open {
                            gtk.popover_popup(handle);
                        } else {
                            gtk.popover_popdown(handle);
                        }
                    }
                }
                true
            }
            "anchor" | "anchor-id" => {
                let trimmed = value.trim();
                let anchor_opt = (!trimmed.is_empty()).then(|| trimmed.to_string());
                if let Some(w) = self.widgets.get_mut(&node) {
                    w.popover.anchor = anchor_opt;
                }
                self.sync_popover_anchor(node, None);
                true
            }
            _ => false,
        }
    }

    fn apply_metadata_attribute(&mut self, node: NodeId, key: &str, value: &str) -> bool {
        if self.apply_popover_metadata_attribute(node, key, value) {
            return true;
        }
        if key == "id" {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                self.dom_id_to_node.insert(trimmed.to_string(), node);
            }
            return true;
        }
        if key == "event-mask" {
            self.enable_widget_events(node, EventMask::parse_attr(value));
            return true;
        }
        if key == "prevent-close" {
            let prevent = NativeSceneState::parse_truthy_attr(value);
            if let Some(w) = self.widgets.get_mut(&node) {
                w.prevent_close = prevent;
            }
            set_window_prevent_close(pack_node_id(node), prevent);
            if prevent {
                self.enable_widget_events(node, EventMask::CLOSE_REQUEST);
            }
            return true;
        }
        if key == "close-window" {
            if NativeSceneState::parse_truthy_attr(value)
                && let Some(w) = self.widgets.get_mut(&node)
                && w.kind.is_toplevel()
            {
                w.prevent_close = false;
                set_window_prevent_close(pack_node_id(node), false);
                if let Some(handle) = w.handle.take()
                    && let Some(gtk) = &self.gtk
                {
                    gtk.destroy_window(handle);
                }
            }
            return true;
        }
        if key == "modal" {
            let modal = NativeSceneState::parse_truthy_attr(value);
            if let Some(w) = self.widgets.get_mut(&node) {
                w.dialog.modal = modal;
                if let (Some(gtk), Some(handle)) = (&self.gtk, w.handle)
                    && w.kind == GtkWidgetType::Dialog
                {
                    gtk.set_window_modal(handle, modal);
                }
            }
            return true;
        }
        if key == "transient-for" {
            let trimmed = value.trim();
            let target_opt = (!trimmed.is_empty()).then(|| trimmed.to_string());
            if let Some(w) = self.widgets.get_mut(&node) {
                w.dialog.transient_for.clone_from(&target_opt);
            }
            if let Some(target_handle) =
                self.resolve_transient_window_handle(node, target_opt.as_deref())
                && let (Some(gtk), Some(w)) = (&self.gtk, self.widgets.get(&node))
                && w.kind == GtkWidgetType::Dialog
                && let Some(handle) = w.handle
            {
                gtk.set_window_transient_for(handle, Some(target_handle));
            }
            return true;
        }
        if let Some(w) = self.widgets.get_mut(&node) {
            match key {
                "visible" => {
                    w.visibility =
                        WidgetVisibility::from_bool(NativeSceneState::parse_truthy_attr(value));
                }
                "drop-target" => {
                    let trimmed = value.trim();
                    w.drop_target = !trimmed.eq_ignore_ascii_case("false") && trimmed != "0";
                }
                "scroll-to" => {
                    w.scroll_position = NativeSceneState::parse_scroll_coords(value);
                }
                "scroll-anchor" => {
                    w.scroll_anchor = Some(value.to_string());
                }
                "columns" | "max-children" if w.kind == GtkWidgetType::Grid => {
                    if let Ok(cols) = value.trim().parse::<i32>() {
                        w.grid_columns = cols.max(1);
                    }
                }
                "col" | "column" => {
                    w.grid_col = value.trim().parse::<i32>().ok().filter(|&c| c >= 0);
                }
                "row" => {
                    w.grid_row = value.trim().parse::<i32>().ok().filter(|&r| r >= 0);
                }
                _ => {}
            }
        }
        false
    }

    pub(super) fn enable_widget_events(&mut self, node: NodeId, requested: EventMask) {
        let Some(widget) = self.widgets.get_mut(&node) else {
            return;
        };
        let delta = EventMask::newly_enabled(widget.attached_events, requested);
        widget.attached_events = widget.attached_events.union(requested);
        if delta.is_empty() {
            return;
        }
        let (Some(gtk), Some(handle)) = (&self.gtk, widget.handle) else {
            return;
        };
        let kind = widget.kind;
        let packed = pack_node_id(node);
        if delta.contains(EventMask::CLICK) {
            gtk.attach_click_gesture(handle, packed, kind != GtkWidgetType::Button);
        }
        if delta.contains(EventMask::CONTEXT_MENU) {
            gtk.attach_context_menu_gesture(handle, packed);
        }
        if delta.contains(EventMask::POINTER_MOTION) {
            gtk.attach_motion_controller(handle, packed);
        }
        if delta.contains(EventMask::FOCUS) {
            gtk.attach_focus_controller(handle, packed);
        }
        if delta.contains(EventMask::KEYBOARD) {
            gtk.attach_key_controller(handle, packed);
        }
        if delta.contains(EventMask::WHEEL) {
            gtk.attach_wheel_controller(handle, packed);
        }
        if delta.contains(EventMask::CLOSE_REQUEST) && kind.is_toplevel() {
            gtk.connect_window_close_request(handle, packed);
        }
    }

    pub(super) fn apply_native_attribute(&mut self, node: NodeId, key: &str, value: &str) {
        if self.apply_metadata_attribute(node, key, value) {
            return;
        }
        let (Some(gtk), Some(widget)) = (&self.gtk, self.widgets.get(&node)) else {
            return;
        };
        let widget_kind = widget.kind;
        let Some(handle) = widget.handle else { return };
        if key == "indeterminate" && widget_kind == GtkWidgetType::ProgressBar {
            let pulsing = value != "false" && value != "0";
            if let Some(w) = self.widgets.get_mut(&node) {
                w.pulsing = pulsing;
            }
            return;
        }
        if matches!(key, "width" | "height" | "min-width" | "min-height")
            && let Some(px) = Self::parse_px_i32(value)
        {
            if key == "min-height" && widget_kind == GtkWidgetType::Scroll {
                gtk.set_scrolled_min_height(handle, px);
                return;
            }
            if let Some(w) = self.widgets.get_mut(&node) {
                if matches!(key, "width" | "min-width") {
                    w.size_request.0 = px;
                } else {
                    w.size_request.1 = px;
                }
                gtk.set_size_request(handle, w.size_request.0, w.size_request.1);
            }
            return;
        }
        if Self::apply_container_attribute(gtk, widget_kind, handle, key, value) {
            return;
        }
        match key {
            "title" if widget_kind.is_toplevel() => {
                gtk.set_window_title(handle, value);
            }
            "title" if widget_kind == GtkWidgetType::HeaderBar => {
                let entry = self.header_titles.entry(node).or_default();
                entry.0 = value.to_string();
                let (t, s) = (entry.0.clone(), entry.1.clone());
                gtk.set_header_bar_title(handle, &t, &s);
            }
            "subtitle" if widget_kind == GtkWidgetType::HeaderBar => {
                let entry = self.header_titles.entry(node).or_default();
                entry.1 = value.to_string();
                let (t, s) = (entry.0.clone(), entry.1.clone());
                gtk.set_header_bar_title(handle, &t, &s);
            }
            "show-title-buttons" if widget_kind == GtkWidgetType::HeaderBar => {
                gtk.set_header_bar_show_title_buttons(handle, value != "false" && value != "0");
            }
            "orientation"
                if matches!(widget_kind, GtkWidgetType::Box | GtkWidgetType::Separator) =>
            {
                gtk.set_orientation(handle, value != "horizontal");
            }
            "spacing" | "gap" if widget_kind == GtkWidgetType::Box => {
                if let Some(px) = Self::parse_px_i32(value) {
                    gtk.set_box_spacing(handle, px);
                }
            }
            "hexpand" | "vexpand" | "visible" => {
                Self::apply_visibility_and_expand(gtk, widget_kind, handle, key, value);
            }
            "halign" => gtk.set_halign(handle, value),
            "valign" => gtk.set_valign(handle, value),
            "margin" => {
                if let Some(px) = Self::parse_px_i32(value) {
                    gtk.set_uniform_margin(handle, px);
                }
            }
            "margin-top" | "margin-bottom" | "margin-start" | "margin-end" => {
                if let Some(px) = Self::parse_px_i32(value) {
                    gtk.set_margin_side(handle, key, px);
                }
            }
            _ => Self::apply_interactive_attribute(gtk, widget_kind, handle, key, value),
        }
    }

    fn apply_visibility_and_expand(
        gtk: &Gtk4Sys,
        kind: GtkWidgetType,
        handle: GtkHandle,
        key: &str,
        value: &str,
    ) {
        match key {
            "hexpand" => gtk.set_hexpand(handle, value == "true" || value == "1"),
            "vexpand" => gtk.set_vexpand(handle, value == "true" || value == "1"),
            "visible" => {
                let is_vis = value != "false" && value != "0";
                gtk.set_widget_visible(handle, is_vis);
                if kind == GtkWidgetType::Spinner {
                    gtk.set_spinner_active(handle, is_vis);
                } else if kind.is_toplevel() && is_vis {
                    gtk.present_window(handle);
                }
            }
            _ => {}
        }
    }

    fn apply_interactive_attribute(
        gtk: &Gtk4Sys,
        kind: GtkWidgetType,
        handle: GtkHandle,
        key: &str,
        value: &str,
    ) {
        match (kind, key) {
            (GtkWidgetType::Entry, "value" | "text") => gtk.set_entry_text(handle, value),
            (GtkWidgetType::Entry, "placeholder") => gtk.set_entry_placeholder(handle, value),
            (GtkWidgetType::Switch, "active" | "checked" | "value") => {
                gtk.set_switch_active(handle, value == "true" || value == "1");
            }
            (GtkWidgetType::CheckButton, "active" | "checked" | "value") => {
                gtk.set_check_button_active(handle, value == "true" || value == "1");
            }
            (GtkWidgetType::CheckButton, "label") => gtk.set_check_button_label(handle, value),
            (GtkWidgetType::Slider, "value") => {
                if let Ok(v) = value.trim().parse::<f64>() {
                    gtk.set_slider_value(handle, v);
                }
            }
            (GtkWidgetType::DropDown, "options") => {
                gtk.set_drop_down_options(handle, value);
            }
            (GtkWidgetType::DropDown, "selected" | "value") => {
                if let Ok(idx) = value.trim().parse::<u32>() {
                    gtk.set_drop_down_selected(handle, idx);
                }
            }
            (GtkWidgetType::Image, "src") => gtk.set_image_file(handle, value),
            (GtkWidgetType::Spinner, "active") => {
                gtk.set_spinner_active(handle, value != "false" && value != "0");
            }
            (GtkWidgetType::ProgressBar, "value") => {
                if let Ok(v) = value.trim().parse::<f64>() {
                    gtk.set_progress_fraction(handle, v.clamp(0.0, 1.0));
                }
            }
            (GtkWidgetType::LevelBar, "value") => {
                if let Ok(v) = value.trim().parse::<f64>() {
                    gtk.set_level_bar_value(handle, v);
                }
            }
            (GtkWidgetType::LevelBar, "max") => {
                if let Ok(m) = value.trim().parse::<f64>() {
                    gtk.set_level_bar_max(handle, m);
                }
            }
            _ => Self::apply_extended_input_attribute(gtk, kind, handle, key, value),
        }
    }

    fn apply_extended_input_attribute(
        gtk: &Gtk4Sys,
        kind: GtkWidgetType,
        handle: GtkHandle,
        key: &str,
        value: &str,
    ) {
        match (kind, key) {
            (GtkWidgetType::TextView, "value" | "text") => gtk.set_text_view_text(handle, value),
            (GtkWidgetType::TextView, "editable") => {
                gtk.set_text_view_editable(handle, value != "false" && value != "0");
            }
            (GtkWidgetType::TextView, "monospace") => {
                gtk.set_text_view_monospace(handle, value == "true" || value == "1");
            }
            (GtkWidgetType::PasswordEntry | GtkWidgetType::SearchEntry, "value" | "text") => {
                gtk.set_entry_text(handle, value);
            }
            (GtkWidgetType::PasswordEntry, "show-peek-icon" | "peek") => {
                gtk.set_password_peek_icon(handle, value != "false" && value != "0");
            }
            (GtkWidgetType::SearchEntry, "placeholder") => {
                gtk.set_search_entry_placeholder(handle, value);
            }
            (GtkWidgetType::SpinButton, "value") => {
                if let Ok(v) = value.trim().parse::<f64>() {
                    gtk.set_spin_button_value(handle, v);
                }
            }
            (GtkWidgetType::SpinButton, "max") => {
                if let Ok(m) = value.trim().parse::<f64>() {
                    gtk.set_spin_button_range(handle, 0.0, m);
                }
            }
            (GtkWidgetType::SpinButton, "step") => {
                if let Ok(s) = value.trim().parse::<f64>() {
                    gtk.set_spin_button_step(handle, s.max(1.0));
                }
            }
            _ => {}
        }
    }
    /// Applies CSS Flexbox/Grid-derived layout properties attached to a registered `StyleRule`.
    pub(super) fn apply_style_rule_layout(&mut self, node: NodeId, rule: &StyleRule) {
        if (rule.min_width.is_some() || rule.min_height.is_some())
            && let (Some(gtk), Some(w)) = (&self.gtk, self.widgets.get_mut(&node))
            && let Some(handle) = w.handle
        {
            if let Some(mw) = rule.min_width
                && w.size_request.0 < 0
            {
                w.size_request.0 = i32::from(mw);
            }
            if let Some(mh) = rule.min_height
                && w.size_request.1 < 0
            {
                w.size_request.1 = i32::from(mh);
            }
            gtk.set_size_request(handle, w.size_request.0, w.size_request.1);
        }
        let (Some(gtk), Some(widget)) = (&self.gtk, self.widgets.get(&node)) else {
            return;
        };
        let Some(handle) = widget.handle else { return };
        let kind = widget.kind;
        if let Some(orient) = &rule.orientation
            && matches!(
                kind,
                GtkWidgetType::Box | GtkWidgetType::Paned | GtkWidgetType::Separator
            )
        {
            gtk.set_orientation(handle, orient != "horizontal");
        }
        Self::apply_style_rule_spacing_and_align(gtk, kind, handle, rule);
    }

    fn apply_style_rule_spacing_and_align(
        gtk: &Gtk4Sys,
        kind: GtkWidgetType,
        handle: GtkHandle,
        rule: &StyleRule,
    ) {
        if rule.spacing > 0 {
            let sp = i32::from(rule.spacing);
            match kind {
                GtkWidgetType::Box => gtk.set_box_spacing(handle, sp),
                GtkWidgetType::FlowBox => {
                    let u = u32::from(rule.spacing);
                    gtk.set_flow_box_row_spacing(handle, u);
                    gtk.set_flow_box_column_spacing(handle, u);
                }
                GtkWidgetType::Grid => {
                    let u = u32::from(rule.spacing);
                    gtk.set_grid_row_spacing(handle, u);
                    gtk.set_grid_column_spacing(handle, u);
                }
                _ => {}
            }
        }
        if let Some(hexp) = rule.hexpand {
            gtk.set_hexpand(handle, hexp);
        }
        if let Some(vexp) = rule.vexpand {
            gtk.set_vexpand(handle, vexp);
        }
        if let Some(hal) = &rule.halign {
            gtk.set_halign(handle, hal);
        }
        if let Some(val) = &rule.valign {
            gtk.set_valign(handle, val);
        }
        if kind == GtkWidgetType::FlowBox {
            if let Some(min_c) = rule.min_children {
                gtk.set_flow_box_min_children_per_line(handle, u32::from(min_c));
            }
            if let Some(max_c) = rule.max_children {
                gtk.set_flow_box_max_children_per_line(handle, u32::from(max_c));
            }
            if let Some(hom) = rule.homogeneous {
                gtk.set_flow_box_homogeneous(handle, hom);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_typography_and_radius_only_when_declared() {
        let plain = StyleRule {
            class_name: String::from("plain"),
            padding: 8,
            ..StyleRule::default()
        };
        let plain_css = render_gtk_css_rule(&plain);
        assert!(plain_css.contains("padding: 8px;"));
        assert!(!plain_css.contains("border-radius"));

        let hero = StyleRule {
            class_name: String::from("hero"),
            foreground: Some(String::from("#78aeed")),
            border_radius: Some(12),
            font_size: Some(22),
            font_weight: Some(800),
            min_width: Some(240),
            min_height: Some(180),
            ..StyleRule::default()
        };
        let hero_css = render_gtk_css_rule(&hero);
        assert!(hero_css.starts_with(".hero {"));
        assert!(hero_css.contains("color: #78aeed;"));
        assert!(hero_css.contains("border-radius: 12px;"));
        assert!(hero_css.contains("font-size: 22px;"));
        assert!(hero_css.contains("font-weight: 800;"));
        assert!(hero_css.contains("min-width: 240px;"));
        assert!(hero_css.contains("min-height: 180px;"));
        assert!(!hero_css.contains("padding"));
    }
}
