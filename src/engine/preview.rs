use super::RenderEngine;
use super::node::{EngineError, NodeId};
use super::widget::WidgetKind;
use crate::desktop::http_codec::escape_json;
use crate::platforms::PlatformBackend;
use std::fmt::Write;

impl<B: PlatformBackend> RenderEngine<B> {
    pub fn print_tree(&self, id: NodeId, indent: usize) -> Result<(), EngineError> {
        let pad = "  ".repeat(indent);
        let classes = self.styles.node_classes(id);
        let class_str = if classes.is_empty() {
            String::new()
        } else {
            format!(" classes={classes:?}")
        };

        let widget = self.arena.get(id)?;
        println!(
            "{pad}{}",
            Self::format_tree_node_header(id, widget, &class_str)
        );

        for child_id in widget.child_ids() {
            if self.arena.get(child_id).is_ok() {
                self.print_tree(child_id, indent + 1)?;
            }
        }
        Ok(())
    }

    fn format_tree_node_header(id: NodeId, widget: &WidgetKind, class_str: &str) -> String {
        let idx = id.index;
        let generation = id.generation;
        match widget {
            WidgetKind::Window { title, icon, .. } => {
                let icon_str = icon.as_ref().map_or_else(String::new, |ic| {
                    format!(" icon=\"{}\" [{}]", ic.source_path, ic.format_resolutions())
                });
                format!("[Window #{idx}:{generation} title=\"{title}\"{icon_str}{class_str}]")
            }
            WidgetKind::Dialog {
                title,
                modal,
                transient_for,
                ..
            } => {
                let trans_str = transient_for
                    .as_ref()
                    .map_or_else(String::new, |t| format!(" transient-for=\"{t}\""));
                format!(
                    "[Dialog #{idx}:{generation} title=\"{title}\" modal={modal}{trans_str}{class_str}]"
                )
            }
            WidgetKind::HeaderBar {
                title,
                subtitle,
                show_title_buttons,
                ..
            } => {
                let sub_str = subtitle
                    .as_ref()
                    .map_or_else(String::new, |s| format!(" subtitle=\"{s}\""));
                format!(
                    "[HeaderBar #{idx}:{generation} title=\"{title}\"{sub_str} buttons={show_title_buttons}{class_str}]"
                )
            }
            WidgetKind::Scroll { min_height, .. } => {
                format!("[Scroll #{idx}:{generation} min_height={min_height}{class_str}]")
            }
            WidgetKind::Box {
                vertical, spacing, ..
            } => {
                let dir = if *vertical { "vertical" } else { "horizontal" };
                format!("[Box #{idx}:{generation} dir={dir} spacing={spacing}{class_str}]")
            }
            WidgetKind::Grid {
                row_spacing,
                column_spacing,
                ..
            } => format!(
                "[Grid #{idx}:{generation} row_spacing={row_spacing} column_spacing={column_spacing}{class_str}]"
            ),
            WidgetKind::Separator { vertical } => {
                let dir = if *vertical { "vertical" } else { "horizontal" };
                format!("[Separator #{idx}:{generation} dir={dir}{class_str}]")
            }
            WidgetKind::Button { label } => {
                format!("[Button #{idx}:{generation} label=\"{label}\"{class_str}]")
            }
            WidgetKind::Label { text } => {
                format!("[Label #{idx}:{generation} text=\"{text}\"{class_str}]")
            }
            WidgetKind::Entry { text, placeholder } => {
                format!(
                    "[Entry #{idx}:{generation} text=\"{text}\" placeholder=\"{placeholder}\"{class_str}]"
                )
            }
            WidgetKind::Switch { active } => {
                format!("[Switch #{idx}:{generation} active={active}{class_str}]")
            }
            WidgetKind::CheckButton { label, active } => {
                format!(
                    "[CheckButton #{idx}:{generation} label=\"{label}\" active={active}{class_str}]"
                )
            }
            WidgetKind::Slider { value, min, max } => {
                format!("[Slider #{idx}:{generation} value={value} range={min}..{max}{class_str}]")
            }
            WidgetKind::DropDown { options, selected } => {
                format!(
                    "[DropDown #{idx}:{generation} selected={selected} options={options:?}{class_str}]"
                )
            }
            other => Self::format_tree_display_header(id, other, class_str),
        }
    }

    /// Header lines for display-only / leaf widgets (split out to keep each match readable).
    fn format_tree_display_header(id: NodeId, widget: &WidgetKind, class_str: &str) -> String {
        let idx = id.index;
        let generation = id.generation;
        match widget {
            WidgetKind::Image { src, width, height } => {
                format!("[Image #{idx}:{generation} src=\"{src}\" {width}x{height}{class_str}]")
            }
            WidgetKind::LevelBar { value, min, max } => {
                format!(
                    "[LevelBar #{idx}:{generation} value={value} range={min}..{max}{class_str}]"
                )
            }
            WidgetKind::Spinner { active } => {
                format!("[Spinner #{idx}:{generation} active={active}{class_str}]")
            }
            WidgetKind::ProgressBar {
                fraction,
                indeterminate,
            } => {
                format!(
                    "[ProgressBar #{idx}:{generation} fraction={fraction} indeterminate={indeterminate}{class_str}]"
                )
            }
            WidgetKind::FlowBox {
                row_spacing,
                column_spacing,
                ..
            } => {
                format!(
                    "[FlowBox #{idx}:{generation} row_spacing={row_spacing} col_spacing={column_spacing}{class_str}]"
                )
            }
            WidgetKind::Paned {
                vertical, position, ..
            } => {
                let dir = if *vertical { "vertical" } else { "horizontal" };
                format!("[Paned #{idx}:{generation} dir={dir} pos={position}{class_str}]")
            }
            WidgetKind::Stack {
                visible_child,
                transition,
                ..
            } => format!(
                "[Stack #{idx}:{generation} visible=\"{visible_child}\" transition=\"{transition}\"{class_str}]"
            ),
            WidgetKind::Expander {
                label, expanded, ..
            } => format!(
                "[Expander #{idx}:{generation} label=\"{label}\" expanded={expanded}{class_str}]"
            ),
            WidgetKind::Clamp { max_size, .. } => {
                format!("[Clamp #{idx}:{generation} max_size={max_size}{class_str}]")
            }
            WidgetKind::TextView {
                text,
                editable,
                monospace,
            } => format!(
                "[TextView #{idx}:{generation} text=\"{text}\" editable={editable} monospace={monospace}{class_str}]"
            ),
            WidgetKind::PasswordEntry {
                placeholder,
                show_peek_icon,
                ..
            } => format!(
                "[PasswordEntry #{idx}:{generation} placeholder=\"{placeholder}\" peek={show_peek_icon}{class_str}]"
            ),
            WidgetKind::SearchEntry { text, placeholder } => format!(
                "[SearchEntry #{idx}:{generation} text=\"{text}\" placeholder=\"{placeholder}\"{class_str}]"
            ),
            WidgetKind::SpinButton {
                value,
                min,
                max,
                step,
            } => format!(
                "[SpinButton #{idx}:{generation} value={value} range={min}..{max} step={step}{class_str}]"
            ),
            WidgetKind::Anchor { comment } => {
                format!("[Anchor #{idx}:{generation} comment=\"{comment}\"]")
            }
            other => format!("[{other:?}]"),
        }
    }

    pub fn render_window_preview(&self, root_id: NodeId) -> Result<String, EngineError> {
        let root = self.arena.get(root_id)?;
        let (title, icon_opt, header_opt, child_opt) = match root {
            WidgetKind::Window {
                title,
                icon,
                header_bar,
                child,
            } => (title.as_str(), icon.as_ref(), *header_bar, *child),
            WidgetKind::Dialog {
                title,
                header_bar,
                child,
                ..
            } => (title.as_str(), None, *header_bar, *child),
            _ => return Err(EngineError::InvalidParent(root_id)),
        };

        let width: usize = 76;
        let border = "═".repeat(width);
        let mut out = String::new();
        let _ = writeln!(out, "╔{border}╗");
        let icon_badge = icon_opt.map_or_else(
            || String::from("● ● ●"),
            |ic| format!("[ICO:{}]", ic.format_resolutions()),
        );
        let header = format!(" {icon_badge}  {title} ({})", self.backend.backend_name());
        let _ = writeln!(out, "║{header:<width$}║");
        let _ = writeln!(out, "╠{border}╣");

        if let Some(hb_id) = header_opt
            && self.arena.get(hb_id).is_ok()
        {
            self.format_preview_node(hb_id, width, &mut out)?;
        }

        if let Some(child_id) = child_opt
            && self.arena.get(child_id).is_ok()
        {
            self.format_preview_node(child_id, width, &mut out)?;
        }

        let _ = write!(out, "╚{border}╝");
        Ok(out)
    }

    fn format_preview_node(
        &self,
        id: NodeId,
        width: usize,
        out: &mut String,
    ) -> Result<(), EngineError> {
        let widget = self.arena.get(id)?;
        let style_summary = self.describe_node_styles(id);

        if let Some(line) = Self::format_preview_line(widget, width, &style_summary) {
            let _ = writeln!(out, "║{line:<width$}║");
        }

        for child_id in widget.child_ids() {
            if self.arena.get(child_id).is_ok() {
                self.format_preview_node(child_id, width, out)?;
            }
        }
        Ok(())
    }

    fn format_preview_line(
        widget: &WidgetKind,
        width: usize,
        style_summary: &str,
    ) -> Option<String> {
        match widget {
            WidgetKind::HeaderBar {
                title, subtitle, ..
            } => {
                let sub = subtitle
                    .as_deref()
                    .map_or_else(String::new, |s| format!(" — {s}"));
                Some(format!("  [HeaderBar: {title}{sub}]"))
            }
            WidgetKind::Box {
                vertical, spacing, ..
            } => {
                if style_summary.is_empty() && *vertical && *spacing == 0 {
                    return None;
                }
                let dir = if *vertical { "vertical" } else { "horizontal" };
                let meta = if style_summary.is_empty() {
                    format!("dir:{dir}, gap:{spacing}px")
                } else {
                    format!("{style_summary} dir:{dir}, gap:{spacing}px")
                };
                Some(format!("  [Box {meta}]"))
            }
            WidgetKind::Grid {
                row_spacing,
                column_spacing,
                ..
            } => Some(format!(
                "  [Grid row-gap:{row_spacing}px col-gap:{column_spacing}px]"
            )),
            WidgetKind::FlowBox {
                row_spacing,
                column_spacing,
                ..
            } => Some(format!(
                "  [FlowBox row-gap:{row_spacing}px col-gap:{column_spacing}px]"
            )),
            WidgetKind::Paned {
                vertical, position, ..
            } => {
                let dir = if *vertical { "vertical" } else { "horizontal" };
                Some(format!("  [Paned dir:{dir}, pos:{position}px]"))
            }
            WidgetKind::Stack {
                visible_child,
                transition,
                ..
            } => Some(format!(
                "  [Stack active:\"{visible_child}\", transition:{transition}]"
            )),
            WidgetKind::Expander {
                label, expanded, ..
            } => {
                let arrow = if *expanded { "▼" } else { "▶" };
                Some(format!("  [Expander {arrow} {label}]"))
            }
            WidgetKind::Clamp { max_size, .. } => Some(format!("  [Clamp max-width:{max_size}px]")),
            WidgetKind::Separator { .. } => {
                let sep = "─".repeat(width.saturating_sub(4));
                Some(format!("  {sep}"))
            }
            other => Self::format_leaf_preview_line(other, style_summary),
        }
    }

    fn format_leaf_preview_line(widget: &WidgetKind, style_summary: &str) -> Option<String> {
        match widget {
            WidgetKind::Label { text } => Some(if style_summary.is_empty() {
                format!("  {text}")
            } else {
                format!("  {text}  {style_summary}")
            }),
            WidgetKind::Button { label } => Some(if style_summary.is_empty() {
                format!("  [ {label} ]")
            } else {
                format!("  [ {label} ]  {style_summary}")
            }),
            WidgetKind::Entry { text, placeholder } => {
                let display = if text.is_empty() { placeholder } else { text };
                Some(format!("  [Entry: \"{display}\"]"))
            }
            WidgetKind::TextView { text, .. } => Some(format!("  [TextView: \"{text}\"]")),
            WidgetKind::PasswordEntry {
                text, placeholder, ..
            } => {
                let display = if text.is_empty() {
                    placeholder.clone()
                } else {
                    "•".repeat(text.chars().count())
                };
                Some(format!("  [PasswordEntry: \"{display}\"]"))
            }
            WidgetKind::SearchEntry { text, placeholder } => {
                let display = if text.is_empty() { placeholder } else { text };
                Some(format!("  [SearchEntry: \"{display}\"]"))
            }
            WidgetKind::SpinButton {
                value,
                min,
                max,
                step,
            } => Some(format!(
                "  [SpinButton: {value} ({min}..{max}, step:{step})]"
            )),
            WidgetKind::Switch { active } => {
                let state = if *active { "ON" } else { "OFF" };
                Some(format!("  [Switch: {state}]"))
            }
            WidgetKind::CheckButton { label, active } => {
                let mark = if *active { "x" } else { " " };
                Some(format!("  [{mark}] {label}"))
            }
            WidgetKind::Slider { value, min, max } => {
                Some(format!("  [Slider: {value} ({min}..{max})]"))
            }
            WidgetKind::DropDown { options, selected } => {
                let current = options.get(*selected).map_or("", String::as_str);
                Some(format!("  [Select: \"{current}\" ({selected})]"))
            }
            WidgetKind::Image { src, width, height } => Some(format!(
                "  [Image: \"{src}\" ({width}x{height})]{style_summary}"
            )),
            WidgetKind::Spinner { active } => {
                let state = if *active { "spinning" } else { "stopped" };
                Some(format!("  [Spinner: {state}]"))
            }
            WidgetKind::ProgressBar {
                fraction,
                indeterminate,
            } => Some(if *indeterminate {
                String::from("  [ProgressBar: indeterminate]")
            } else {
                format!("  [ProgressBar: {:.0}%]", fraction * 100.0)
            }),
            WidgetKind::LevelBar { value, max, .. } => {
                Some(format!("  [LevelBar: {value:.0}/{max:.0}]{style_summary}"))
            }
            _ => None,
        }
    }

    fn describe_node_styles(&self, id: NodeId) -> String {
        let classes = self.styles.node_classes(id);
        if classes.is_empty() {
            return String::new();
        }
        let mut details = Vec::new();
        for class_name in classes {
            if let Some(rule) = self.styles.get_rule_by_name(class_name) {
                let bg = rule.background.as_deref().unwrap_or("transparent");
                let fg = rule.foreground.as_deref().unwrap_or("inherit");
                let mut extras = String::new();
                if let Some(radius) = rule.border_radius {
                    let _ = write!(extras, ", radius:{radius}px");
                }
                if let Some(size) = rule.font_size {
                    let _ = write!(extras, ", font-size:{size}px");
                }
                if let Some(weight) = rule.font_weight {
                    let _ = write!(extras, ", font-weight:{weight}");
                }
                details.push(format!(
                    ".{class_name}(bg:{bg}, fg:{fg}, pad:{}px{extras})",
                    rule.padding
                ));
            } else {
                details.push(format!(".{class_name}"));
            }
        }
        format!("{{{}}}", details.join(" "))
    }

    pub fn snapshot_tree_json(&self, root_id: NodeId) -> Result<String, EngineError> {
        let mut out = String::with_capacity(512);
        self.write_snapshot_node(root_id, &mut out)?;
        Ok(out)
    }

    pub fn query_nodes_json(
        &self,
        root_id: NodeId,
        id_or_testid: Option<&str>,
        tag: Option<&str>,
        text_contains: Option<&str>,
    ) -> Result<String, EngineError> {
        let _ = self.arena.get(root_id)?;
        let mut matches = Vec::new();
        self.collect_query_matches(root_id, id_or_testid, tag, text_contains, &mut matches);
        Ok(format!("[{}]", matches.join(",")))
    }

    #[must_use]
    pub fn find_node_by_attr(&self, root_id: NodeId, target: &str) -> Option<NodeId> {
        let widget = self.arena.get(root_id).ok()?;
        if let Some(attrs) = self.attributes.get(&root_id)
            && (attrs.get("id").map(String::as_str) == Some(target)
                || attrs.get("data-testid").map(String::as_str) == Some(target))
        {
            return Some(root_id);
        }
        for child_id in widget.child_ids() {
            if let Some(found) = self.find_node_by_attr(child_id, target) {
                return Some(found);
            }
        }
        None
    }

    fn write_snapshot_node(&self, id: NodeId, out: &mut String) -> Result<(), EngineError> {
        let widget = self.arena.get(id)?;
        let attrs = self.attributes.get(&id);
        let visible = attrs.and_then(|a| a.get("visible")).map(String::as_str) != Some("false");
        let _ = write!(
            out,
            r#"{{"node":"{}:{}","tag":"{}","text":"#,
            id.index,
            id.generation,
            widget.tag_name()
        );
        if let Some(text) = widget.primary_text() {
            let _ = write!(out, "\"{}\"", escape_json(text));
        } else {
            out.push_str("null");
        }
        let _ = write!(out, ",\"visible\":{visible},\"classes\":[");
        for (idx, cls) in self.styles.node_classes(id).iter().enumerate() {
            if idx > 0 {
                out.push(',');
            }
            let _ = write!(out, "\"{}\"", escape_json(cls));
        }
        out.push_str("],\"attrs\":{");
        if let Some(attr_map) = attrs {
            let mut pairs: Vec<(&String, &String)> = attr_map.iter().collect();
            pairs.sort_by(|a, b| a.0.cmp(b.0));
            for (idx, (k, v)) in pairs.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                let _ = write!(out, "\"{}\":\"{}\"", escape_json(k), escape_json(v));
            }
        }
        out.push_str("},\"children\":[");
        let mut first_child = true;
        for child_id in widget.child_ids() {
            if self.arena.get(child_id).is_ok() {
                if !first_child {
                    out.push(',');
                }
                first_child = false;
                self.write_snapshot_node(child_id, out)?;
            }
        }
        out.push_str("]}");
        Ok(())
    }

    fn collect_query_matches(
        &self,
        id: NodeId,
        id_or_testid: Option<&str>,
        tag: Option<&str>,
        text_contains: Option<&str>,
        out: &mut Vec<String>,
    ) {
        let Ok(widget) = self.arena.get(id) else {
            return;
        };
        let attrs = self.attributes.get(&id);
        let attr_id = attrs.and_then(|a| a.get("id")).map(String::as_str);
        let attr_testid = attrs.and_then(|a| a.get("data-testid")).map(String::as_str);
        let text_opt = widget.primary_text();
        let matches_id = id_or_testid
            .is_none_or(|needle| attr_id == Some(needle) || attr_testid == Some(needle));
        let matches_tag = tag.is_none_or(|expected| widget.tag_name() == expected);
        let matches_text =
            text_contains.is_none_or(|needle| text_opt.is_some_and(|txt| txt.contains(needle)));

        if matches_id && matches_tag && matches_text {
            let visible = attrs.and_then(|a| a.get("visible")).map(String::as_str) != Some("false");
            let id_json = attr_id.map_or_else(
                || String::from("null"),
                |val| format!("\"{}\"", escape_json(val)),
            );
            let testid_json = attr_testid.map_or_else(
                || String::from("null"),
                |val| format!("\"{}\"", escape_json(val)),
            );
            let text_json = text_opt.map_or_else(
                || String::from("null"),
                |val| format!("\"{}\"", escape_json(val)),
            );
            out.push(format!(
                r#"{{"node":"{}:{}","tag":"{}","id":{id_json},"testId":{testid_json},"text":{text_json},"visible":{visible}}}"#,
                id.index,
                id.generation,
                widget.tag_name(),
            ));
        }

        for child_id in widget.child_ids() {
            self.collect_query_matches(child_id, id_or_testid, tag, text_contains, out);
        }
    }
}
