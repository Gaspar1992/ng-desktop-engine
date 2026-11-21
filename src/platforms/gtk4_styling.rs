use super::gtk4_sys::{Gtk4Sys, GtkHandle};
use super::linux_gtk::{GtkLinuxBackend, GtkWidgetType};
use crate::engine::{NodeId, StyleRule};
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
    let _ = writeln!(css, "}}");
    css
}

impl GtkLinuxBackend {
    pub(super) fn rebuild_css_stylesheet(&self) -> String {
        let mut css = String::from(
            "window { background: #1e1e1e; color: #ffffff; }\n\
             headerbar, windowhandle { min-height: 46px; }\n",
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
            _ => false,
        }
    }

    pub(super) fn apply_native_attribute(&mut self, node: NodeId, key: &str, value: &str) {
        if let Some(w) = self.widgets.get_mut(&node) {
            match key {
                "drop-target" => {
                    let trimmed = value.trim();
                    w.drop_target = !trimmed.eq_ignore_ascii_case("false") && trimmed != "0";
                }
                "scroll-to" => {
                    w.scroll_position =
                        super::scene_state::NativeSceneState::parse_scroll_coords(value);
                }
                "scroll-anchor" => {
                    w.scroll_anchor = Some(value.to_string());
                }
                _ => {}
            }
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
        if matches!(key, "width" | "height")
            && let Some(px) = Self::parse_px_i32(value)
        {
            if let Some(w) = self.widgets.get_mut(&node) {
                if key == "width" {
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
            "title" if widget_kind == GtkWidgetType::Window => {
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
            "min-height" if widget_kind == GtkWidgetType::Scroll => {
                if let Some(px) = Self::parse_px_i32(value) {
                    gtk.set_scrolled_min_height(handle, px);
                }
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
            "hexpand" => gtk.set_hexpand(handle, value == "true" || value == "1"),
            "vexpand" => gtk.set_vexpand(handle, value == "true" || value == "1"),
            "visible" => gtk.set_widget_visible(handle, value != "false" && value != "0"),
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
            ..StyleRule::default()
        };
        let hero_css = render_gtk_css_rule(&hero);
        assert!(hero_css.starts_with(".hero {"));
        assert!(hero_css.contains("color: #78aeed;"));
        assert!(hero_css.contains("border-radius: 12px;"));
        assert!(hero_css.contains("font-size: 22px;"));
        assert!(hero_css.contains("font-weight: 800;"));
        assert!(!hero_css.contains("padding"));
    }
}
