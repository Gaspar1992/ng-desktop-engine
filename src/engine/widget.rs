use super::icons::WindowIconAsset;
use super::node::{EngineError, NodeId};

#[derive(Debug)]
pub enum WidgetKind {
    Window {
        title: String,
        icon: Option<WindowIconAsset>,
        header_bar: Option<NodeId>,
        child: Option<NodeId>,
    },
    Dialog {
        title: String,
        modal: bool,
        transient_for: Option<String>,
        header_bar: Option<NodeId>,
        child: Option<NodeId>,
    },
    HeaderBar {
        title: String,
        subtitle: Option<String>,
        show_title_buttons: bool,
        children: Vec<NodeId>,
    },
    Scroll {
        min_height: u32,
        child: Option<NodeId>,
    },
    Box {
        vertical: bool,
        spacing: u32,
        children: Vec<NodeId>,
    },
    Grid {
        row_spacing: u32,
        column_spacing: u32,
        children: Vec<NodeId>,
    },
    Separator {
        vertical: bool,
    },
    Button {
        label: String,
    },
    Label {
        text: String,
    },
    Entry {
        text: String,
        placeholder: String,
    },
    Switch {
        active: bool,
    },
    CheckButton {
        label: String,
        active: bool,
    },
    Slider {
        value: i64,
        min: i64,
        max: i64,
    },
    DropDown {
        options: Vec<String>,
        selected: usize,
    },
    Image {
        src: String,
        width: u32,
        height: u32,
    },
    LevelBar {
        value: f64,
        min: f64,
        max: f64,
    },
    FlowBox {
        row_spacing: u32,
        column_spacing: u32,
        min_children_per_line: u32,
        max_children_per_line: u32,
        homogeneous: bool,
        children: Vec<NodeId>,
    },
    Spinner {
        active: bool,
    },
    ProgressBar {
        fraction: f64,
        indeterminate: bool,
    },
    Paned {
        vertical: bool,
        position: u32,
        start_child: Option<NodeId>,
        end_child: Option<NodeId>,
    },
    Stack {
        visible_child: String,
        transition: String,
        children: Vec<NodeId>,
    },
    Expander {
        label: String,
        expanded: bool,
        child: Option<NodeId>,
    },
    TextView {
        text: String,
        editable: bool,
        monospace: bool,
    },
    PasswordEntry {
        text: String,
        placeholder: String,
        show_peek_icon: bool,
    },
    SpinButton {
        value: i64,
        min: i64,
        max: i64,
        step: i64,
    },
    SearchEntry {
        text: String,
        placeholder: String,
    },
    Clamp {
        max_size: u32,
        child: Option<NodeId>,
    },
    ListView {
        selection_mode: String,
        show_separators: bool,
        children: Vec<NodeId>,
    },
    Popover {
        open: bool,
        position: String,
        autohide: bool,
        has_arrow: bool,
        child: Option<NodeId>,
    },
    Anchor {
        comment: String,
    },
}

impl WidgetKind {
    pub fn from_tag(tag: &str) -> Result<Self, EngineError> {
        match tag {
            "ng-window" | "adw-window" => Ok(Self::Window {
                title: String::from("Untitled"),
                icon: None,
                header_bar: None,
                child: None,
            }),
            "ng-header-bar" | "adw-header-bar" | "gtk-header-bar" => Ok(Self::HeaderBar {
                title: String::new(),
                subtitle: None,
                show_title_buttons: true,
                children: Vec::new(),
            }),
            "ng-scroll" | "gtk-scrolled-window" => Ok(Self::Scroll {
                min_height: 0,
                child: None,
            }),
            "ng-box" | "gtk-box" => Ok(Self::Box {
                vertical: true,
                spacing: 0,
                children: Vec::new(),
            }),
            "ng-grid" | "gtk-grid" => Ok(Self::Grid {
                row_spacing: 0,
                column_spacing: 0,
                children: Vec::new(),
            }),
            "ng-separator" | "gtk-separator" => Ok(Self::Separator { vertical: false }),
            "ng-button" | "gtk-button" => Ok(Self::Button {
                label: String::new(),
            }),
            "ng-label" | "gtk-label" => Ok(Self::Label {
                text: String::new(),
            }),
            "ng-entry" | "gtk-entry" => Ok(Self::Entry {
                text: String::new(),
                placeholder: String::new(),
            }),
            "ng-switch" | "gtk-switch" => Ok(Self::Switch { active: false }),
            "ng-checkbox" | "ng-check" | "gtk-check-button" => Ok(Self::CheckButton {
                label: String::new(),
                active: false,
            }),
            "ng-slider" | "ng-scale" | "gtk-scale" => Ok(Self::Slider {
                value: 0,
                min: 0,
                max: 100,
            }),
            "ng-select" | "ng-dropdown" | "gtk-drop-down" => Ok(Self::DropDown {
                options: Vec::new(),
                selected: 0,
            }),
            "ng-image" | "gtk-picture" | "ng-picture" => Ok(Self::Image {
                src: String::new(),
                width: 0,
                height: 0,
            }),
            "ng-level-bar" | "gtk-level-bar" => Ok(Self::LevelBar {
                value: 0.0,
                min: 0.0,
                max: 100.0,
            }),
            "ng-flow" | "ng-flow-box" | "gtk-flow-box" => Ok(Self::FlowBox {
                row_spacing: 0,
                column_spacing: 0,
                min_children_per_line: 0,
                max_children_per_line: 0,
                homogeneous: false,
                children: Vec::new(),
            }),
            "ng-spinner" | "gtk-spinner" => Ok(Self::Spinner { active: true }),
            "ng-progress-bar" | "gtk-progress-bar" => Ok(Self::ProgressBar {
                fraction: 0.0,
                indeterminate: false,
            }),
            other => Self::from_extended_tag(other),
        }
    }

    fn from_extended_tag(tag: &str) -> Result<Self, EngineError> {
        match tag {
            "ng-paned" | "gtk-paned" => Ok(Self::Paned {
                vertical: false,
                position: 200,
                start_child: None,
                end_child: None,
            }),
            "ng-stack" | "gtk-stack" | "adw-navigation-view" => Ok(Self::Stack {
                visible_child: String::new(),
                transition: String::from("crossfade"),
                children: Vec::new(),
            }),
            "ng-expander" | "gtk-expander" => Ok(Self::Expander {
                label: String::new(),
                expanded: false,
                child: None,
            }),
            "ng-text-view" | "gtk-text-view" => Ok(Self::TextView {
                text: String::new(),
                editable: true,
                monospace: false,
            }),
            "ng-password-entry" | "gtk-password-entry" => Ok(Self::PasswordEntry {
                text: String::new(),
                placeholder: String::new(),
                show_peek_icon: true,
            }),
            "ng-spin-button" | "gtk-spin-button" => Ok(Self::SpinButton {
                value: 0,
                min: 0,
                max: 100,
                step: 1,
            }),
            "ng-search-entry" | "gtk-search-entry" => Ok(Self::SearchEntry {
                text: String::new(),
                placeholder: String::new(),
            }),
            "ng-clamp" | "adw-clamp" => Ok(Self::Clamp {
                max_size: 800,
                child: None,
            }),
            "ng-dialog" | "gtk-dialog" | "adw-dialog" => Ok(Self::Dialog {
                title: String::from("Dialog"),
                modal: true,
                transient_for: None,
                header_bar: None,
                child: None,
            }),
            "ng-list" | "ng-table" | "gtk-list-box" | "gtk-list-view" => Ok(Self::ListView {
                selection_mode: String::from("single"),
                show_separators: false,
                children: Vec::new(),
            }),
            "ng-popover" | "gtk-popover" | "ng-menu" => Ok(Self::Popover {
                open: false,
                position: String::from("bottom"),
                autohide: true,
                has_arrow: true,
                child: None,
            }),
            "ng-anchor" | "#comment" => Ok(Self::Anchor {
                comment: String::new(),
            }),
            other => Err(EngineError::UnknownTag(other.to_string())),
        }
    }

    pub fn new_comment(comment: impl Into<String>) -> Self {
        Self::Anchor {
            comment: comment.into(),
        }
    }

    pub fn is_header_bar(&self) -> bool {
        matches!(self, Self::HeaderBar { .. })
    }

    /// Returns all direct child `NodeId`s held by container widgets in traversal order.
    #[must_use]
    pub fn child_ids(&self) -> Vec<NodeId> {
        match self {
            Self::Window {
                header_bar, child, ..
            }
            | Self::Dialog {
                header_bar, child, ..
            } => header_bar.iter().chain(child.iter()).copied().collect(),
            Self::Paned {
                start_child,
                end_child,
                ..
            } => start_child
                .iter()
                .chain(end_child.iter())
                .copied()
                .collect(),
            Self::Scroll { child, .. }
            | Self::Expander { child, .. }
            | Self::Clamp { child, .. }
            | Self::Popover { child, .. } => child.iter().copied().collect(),
            Self::HeaderBar { children, .. }
            | Self::Box { children, .. }
            | Self::Grid { children, .. }
            | Self::FlowBox { children, .. }
            | Self::Stack { children, .. }
            | Self::ListView { children, .. } => children.clone(),
            Self::Separator { .. }
            | Self::Button { .. }
            | Self::Label { .. }
            | Self::Image { .. }
            | Self::LevelBar { .. }
            | Self::Spinner { .. }
            | Self::ProgressBar { .. }
            | Self::Entry { .. }
            | Self::TextView { .. }
            | Self::PasswordEntry { .. }
            | Self::SearchEntry { .. }
            | Self::SpinButton { .. }
            | Self::Switch { .. }
            | Self::CheckButton { .. }
            | Self::Slider { .. }
            | Self::DropDown { .. }
            | Self::Anchor { .. } => Vec::new(),
        }
    }

    pub fn set_text(&mut self, id: NodeId, text: impl Into<String>) -> Result<(), EngineError> {
        match self {
            Self::Window { title, .. }
            | Self::Dialog { title, .. }
            | Self::HeaderBar { title, .. } => *title = text.into(),
            Self::Button { label }
            | Self::CheckButton { label, .. }
            | Self::Expander { label, .. } => *label = text.into(),
            Self::Label { text: label_text }
            | Self::Entry {
                text: label_text, ..
            }
            | Self::TextView {
                text: label_text, ..
            }
            | Self::PasswordEntry {
                text: label_text, ..
            }
            | Self::SearchEntry {
                text: label_text, ..
            }
            | Self::Image {
                src: label_text, ..
            } => {
                *label_text = text.into();
            }
            Self::Anchor { comment } => *comment = text.into(),
            Self::Box { .. }
            | Self::Scroll { .. }
            | Self::Grid { .. }
            | Self::FlowBox { .. }
            | Self::Paned { .. }
            | Self::Stack { .. }
            | Self::Clamp { .. }
            | Self::ListView { .. }
            | Self::Popover { .. }
            | Self::Separator { .. }
            | Self::Switch { .. }
            | Self::Slider { .. }
            | Self::SpinButton { .. }
            | Self::DropDown { .. }
            | Self::LevelBar { .. }
            | Self::Spinner { .. }
            | Self::ProgressBar { .. } => {
                return Err(EngineError::InvalidParent(id));
            }
        }
        Ok(())
    }

    pub fn set_icon(&mut self, id: NodeId, icon_asset: WindowIconAsset) -> Result<(), EngineError> {
        match self {
            Self::Window { icon, .. } => {
                *icon = Some(icon_asset);
                Ok(())
            }
            _ => Err(EngineError::InvalidParent(id)),
        }
    }

    pub fn add_child(
        &mut self,
        parent_id: NodeId,
        child_id: NodeId,
        is_header_bar: bool,
    ) -> Result<(), EngineError> {
        match self {
            Self::Window {
                header_bar, child, ..
            }
            | Self::Dialog {
                header_bar, child, ..
            } => {
                if is_header_bar {
                    *header_bar = Some(child_id);
                } else {
                    *child = Some(child_id);
                }
                Ok(())
            }
            Self::Scroll { child, .. }
            | Self::Expander { child, .. }
            | Self::Clamp { child, .. }
            | Self::Popover { child, .. } => {
                *child = Some(child_id);
                Ok(())
            }
            Self::Paned {
                start_child,
                end_child,
                ..
            } => {
                if start_child.is_none() || *start_child == Some(child_id) {
                    *start_child = Some(child_id);
                } else {
                    *end_child = Some(child_id);
                }
                Ok(())
            }
            Self::Box { children, .. }
            | Self::Grid { children, .. }
            | Self::FlowBox { children, .. }
            | Self::Stack { children, .. }
            | Self::HeaderBar { children, .. }
            | Self::ListView { children, .. } => {
                if !children.contains(&child_id) {
                    children.push(child_id);
                }
                Ok(())
            }
            Self::Button { .. }
            | Self::Label { .. }
            | Self::Image { .. }
            | Self::LevelBar { .. }
            | Self::Spinner { .. }
            | Self::ProgressBar { .. }
            | Self::Separator { .. }
            | Self::Entry { .. }
            | Self::TextView { .. }
            | Self::PasswordEntry { .. }
            | Self::SpinButton { .. }
            | Self::SearchEntry { .. }
            | Self::Switch { .. }
            | Self::CheckButton { .. }
            | Self::Slider { .. }
            | Self::DropDown { .. }
            | Self::Anchor { .. } => Err(EngineError::InvalidParent(parent_id)),
        }
    }

    pub fn insert_before(
        &mut self,
        parent_id: NodeId,
        new_child: NodeId,
        ref_child: Option<NodeId>,
        is_header_bar: bool,
    ) -> Result<(), EngineError> {
        match self {
            Self::Box { children, .. }
            | Self::Grid { children, .. }
            | Self::FlowBox { children, .. }
            | Self::Stack { children, .. }
            | Self::HeaderBar { children, .. }
            | Self::ListView { children, .. } => {
                children.retain(|c| *c != new_child);
                if let Some(ref_id) = ref_child {
                    let pos = children
                        .iter()
                        .position(|c| *c == ref_id)
                        .ok_or(EngineError::InvalidNode(ref_id))?;
                    children.insert(pos, new_child);
                } else {
                    children.push(new_child);
                }
                Ok(())
            }
            Self::Window {
                header_bar, child, ..
            }
            | Self::Dialog {
                header_bar, child, ..
            } => {
                if is_header_bar {
                    *header_bar = Some(new_child);
                } else {
                    *child = Some(new_child);
                }
                Ok(())
            }
            Self::Scroll { child, .. }
            | Self::Expander { child, .. }
            | Self::Clamp { child, .. }
            | Self::Popover { child, .. } => {
                *child = Some(new_child);
                Ok(())
            }
            Self::Paned {
                start_child,
                end_child,
                ..
            } => {
                if (ref_child.is_some() && ref_child == *end_child) || start_child.is_none() {
                    *start_child = Some(new_child);
                } else {
                    *end_child = Some(new_child);
                }
                Ok(())
            }
            Self::Button { .. }
            | Self::Label { .. }
            | Self::Image { .. }
            | Self::LevelBar { .. }
            | Self::Spinner { .. }
            | Self::ProgressBar { .. }
            | Self::Separator { .. }
            | Self::Entry { .. }
            | Self::TextView { .. }
            | Self::PasswordEntry { .. }
            | Self::SpinButton { .. }
            | Self::SearchEntry { .. }
            | Self::Switch { .. }
            | Self::CheckButton { .. }
            | Self::Slider { .. }
            | Self::DropDown { .. }
            | Self::Anchor { .. } => Err(EngineError::InvalidParent(parent_id)),
        }
    }

    pub fn remove_child(&mut self, parent_id: NodeId, child_id: NodeId) -> Result<(), EngineError> {
        match self {
            Self::Window {
                header_bar, child, ..
            }
            | Self::Dialog {
                header_bar, child, ..
            } => {
                if *header_bar == Some(child_id) {
                    *header_bar = None;
                    Ok(())
                } else if *child == Some(child_id) {
                    *child = None;
                    Ok(())
                } else {
                    Err(EngineError::InvalidNode(child_id))
                }
            }
            Self::Scroll { child, .. }
            | Self::Expander { child, .. }
            | Self::Clamp { child, .. }
            | Self::Popover { child, .. } => {
                if *child == Some(child_id) {
                    *child = None;
                    Ok(())
                } else {
                    Err(EngineError::InvalidNode(child_id))
                }
            }
            Self::Paned {
                start_child,
                end_child,
                ..
            } => {
                if *start_child == Some(child_id) {
                    *start_child = None;
                    Ok(())
                } else if *end_child == Some(child_id) {
                    *end_child = None;
                    Ok(())
                } else {
                    Err(EngineError::InvalidNode(child_id))
                }
            }
            Self::Box { children, .. }
            | Self::Grid { children, .. }
            | Self::FlowBox { children, .. }
            | Self::Stack { children, .. }
            | Self::HeaderBar { children, .. }
            | Self::ListView { children, .. } => {
                let before = children.len();
                children.retain(|c| *c != child_id);
                if children.len() == before {
                    Err(EngineError::InvalidNode(child_id))
                } else {
                    Ok(())
                }
            }
            Self::Button { .. }
            | Self::Label { .. }
            | Self::Image { .. }
            | Self::LevelBar { .. }
            | Self::Spinner { .. }
            | Self::ProgressBar { .. }
            | Self::Separator { .. }
            | Self::Entry { .. }
            | Self::TextView { .. }
            | Self::PasswordEntry { .. }
            | Self::SpinButton { .. }
            | Self::SearchEntry { .. }
            | Self::Switch { .. }
            | Self::CheckButton { .. }
            | Self::Slider { .. }
            | Self::DropDown { .. }
            | Self::Anchor { .. } => Err(EngineError::InvalidParent(parent_id)),
        }
    }

    #[must_use]
    pub const fn tag_name(&self) -> &'static str {
        match self {
            Self::Window { .. } => "ng-window",
            Self::Dialog { .. } => "ng-dialog",
            Self::HeaderBar { .. } => "ng-header-bar",
            Self::Scroll { .. } => "ng-scroll",
            Self::Box { .. } => "ng-box",
            Self::Grid { .. } => "ng-grid",
            Self::Separator { .. } => "ng-separator",
            Self::Button { .. } => "ng-button",
            Self::Label { .. } => "ng-label",
            Self::Entry { .. } => "ng-entry",
            Self::Switch { .. } => "ng-switch",
            Self::CheckButton { .. } => "ng-checkbox",
            Self::Slider { .. } => "ng-slider",
            Self::DropDown { .. } => "ng-select",
            Self::Image { .. } => "ng-image",
            Self::LevelBar { .. } => "ng-level-bar",
            Self::FlowBox { .. } => "ng-flow",
            Self::Spinner { .. } => "ng-spinner",
            Self::ProgressBar { .. } => "ng-progress-bar",
            Self::Paned { .. } => "ng-paned",
            Self::Stack { .. } => "ng-stack",
            Self::Expander { .. } => "ng-expander",
            Self::TextView { .. } => "ng-text-view",
            Self::PasswordEntry { .. } => "ng-password-entry",
            Self::SpinButton { .. } => "ng-spin-button",
            Self::SearchEntry { .. } => "ng-search-entry",
            Self::Clamp { .. } => "ng-clamp",
            Self::ListView { .. } => "ng-list",
            Self::Popover { .. } => "ng-popover",
            Self::Anchor { .. } => "ng-anchor",
        }
    }

    #[must_use]
    pub fn primary_text(&self) -> Option<&str> {
        match self {
            Self::Window { title, .. }
            | Self::Dialog { title, .. }
            | Self::HeaderBar { title, .. } => Some(title.as_str()),
            Self::Label { text }
            | Self::Entry { text, .. }
            | Self::SearchEntry { text, .. }
            | Self::PasswordEntry { text, .. }
            | Self::TextView { text, .. } => Some(text.as_str()),
            Self::Button { label }
            | Self::CheckButton { label, .. }
            | Self::Expander { label, .. } => Some(label.as_str()),
            Self::Image { src, .. } => Some(src.as_str()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_supported_widget_tags_and_rejects_unknown() {
        for tag in [
            "ng-window",
            "ng-header-bar",
            "ng-scroll",
            "ng-box",
            "ng-grid",
            "ng-separator",
            "ng-button",
            "ng-label",
            "ng-entry",
            "ng-switch",
            "ng-checkbox",
            "ng-slider",
            "ng-select",
            "ng-image",
            "ng-level-bar",
            "ng-spinner",
            "ng-progress-bar",
            "ng-flow",
            "ng-flow-box",
            "gtk-flow-box",
            "ng-paned",
            "ng-stack",
            "ng-expander",
            "ng-text-view",
            "ng-password-entry",
            "ng-spin-button",
            "ng-search-entry",
            "ng-clamp",
            "ng-dialog",
            "gtk-dialog",
            "adw-dialog",
            "ng-list",
            "ng-table",
            "gtk-list-box",
            "gtk-list-view",
            "ng-popover",
            "gtk-popover",
            "ng-menu",
            "ng-anchor",
        ] {
            assert!(WidgetKind::from_tag(tag).is_ok(), "Failed to parse {tag}");
        }
        assert_eq!(
            WidgetKind::from_tag("ng-unknown").unwrap_err(),
            EngineError::UnknownTag(String::from("ng-unknown"))
        );
    }

    #[test]
    fn updates_layout_and_interactive_form_attributes() {
        let mut bx = WidgetKind::from_tag("ng-box").unwrap();
        bx.set_attribute("orientation", "horizontal");
        bx.set_attribute("spacing", "24px");
        assert!(matches!(
            bx,
            WidgetKind::Box {
                vertical: false,
                spacing: 24,
                ..
            }
        ));

        let mut entry = WidgetKind::from_tag("ng-entry").unwrap();
        entry.set_attribute("placeholder", "Email");
        entry.set_attribute("value", "dev@angular.io");
        assert!(matches!(
            entry,
            WidgetKind::Entry { ref text, ref placeholder }
            if text == "dev@angular.io" && placeholder == "Email"
        ));

        let mut sw = WidgetKind::from_tag("ng-switch").unwrap();
        sw.set_attribute("active", "true");
        assert!(matches!(sw, WidgetKind::Switch { active: true }));
    }

    #[test]
    fn feedback_primitives_parse_clamp_and_stay_leaf_nodes() {
        let mut spinner = WidgetKind::from_tag("ng-spinner").unwrap();
        assert!(matches!(spinner, WidgetKind::Spinner { active: true }));
        spinner.set_attribute("active", "false");
        assert!(matches!(spinner, WidgetKind::Spinner { active: false }));

        let mut bar = WidgetKind::from_tag("ng-progress-bar").unwrap();
        bar.set_attribute("value", "1.7");
        bar.set_attribute("indeterminate", "true");
        assert!(matches!(
            bar,
            WidgetKind::ProgressBar {
                fraction,
                indeterminate: true
            } if (fraction - 1.0).abs() < f64::EPSILON
        ));

        // `ng-progress-bar` is no longer an alias of the level meter.
        assert!(matches!(
            WidgetKind::from_tag("ng-level-bar").unwrap(),
            WidgetKind::LevelBar { .. }
        ));

        let id = NodeId {
            index: 1,
            generation: 0,
        };
        assert_eq!(
            spinner.add_child(id, id, false).unwrap_err(),
            EngineError::InvalidParent(id)
        );
    }

    #[test]
    fn enforces_parent_child_hierarchy_invariants() {
        let parent_id = NodeId {
            index: 0,
            generation: 1,
        };
        let c1 = NodeId {
            index: 1,
            generation: 1,
        };
        let c2 = NodeId {
            index: 2,
            generation: 1,
        };

        let mut btn = WidgetKind::from_tag("ng-button").unwrap();
        assert_eq!(
            btn.add_child(parent_id, c1, false).unwrap_err(),
            EngineError::InvalidParent(parent_id)
        );

        let mut bx = WidgetKind::from_tag("ng-box").unwrap();
        bx.add_child(parent_id, c1, false).unwrap();
        bx.insert_before(parent_id, c2, Some(c1), false).unwrap();
        if let WidgetKind::Box { children, .. } = &bx {
            assert_eq!(children, &vec![c2, c1]);
        }
        bx.remove_child(parent_id, c2).unwrap();
        if let WidgetKind::Box { children, .. } = &bx {
            assert_eq!(children, &vec![c1]);
        }
    }

    #[test]
    fn phase8_container_primitives_manage_children_and_attributes() {
        let p_id = NodeId {
            index: 10,
            generation: 0,
        };
        let c1 = NodeId {
            index: 11,
            generation: 0,
        };
        let c2 = NodeId {
            index: 12,
            generation: 0,
        };

        let mut paned = WidgetKind::from_tag("ng-paned").unwrap();
        paned.set_attribute("orientation", "vertical");
        paned.set_attribute("position", "320px");
        paned.add_child(p_id, c1, false).unwrap();
        paned.add_child(p_id, c2, false).unwrap();
        assert!(matches!(
            paned,
            WidgetKind::Paned {
                vertical: true,
                position: 320,
                start_child: Some(s),
                end_child: Some(e),
            } if s == c1 && e == c2
        ));
        paned.remove_child(p_id, c1).unwrap();
        assert!(matches!(
            paned,
            WidgetKind::Paned {
                start_child: None,
                end_child: Some(e),
                ..
            } if e == c2
        ));

        let mut stack = WidgetKind::from_tag("ng-stack").unwrap();
        stack.set_attribute("visible-child", "details");
        stack.set_attribute("transition", "slide-left");
        stack.add_child(p_id, c1, false).unwrap();
        stack.add_child(p_id, c2, false).unwrap();
        assert!(matches!(
            stack,
            WidgetKind::Stack {
                ref visible_child,
                ref transition,
                ref children,
            } if visible_child == "details" && transition == "slide-left" && children == &vec![c1, c2]
        ));

        let mut expander = WidgetKind::from_tag("ng-expander").unwrap();
        expander.set_attribute("label", "Advanced");
        expander.set_attribute("expanded", "true");
        expander.add_child(p_id, c1, false).unwrap();
        assert!(matches!(
            expander,
            WidgetKind::Expander {
                ref label,
                expanded: true,
                child: Some(ch),
            } if label == "Advanced" && ch == c1
        ));
    }

    #[test]
    fn phase8_extended_inputs_and_clamp_parse_attributes() {
        let p_id = NodeId {
            index: 20,
            generation: 0,
        };
        let c1 = NodeId {
            index: 21,
            generation: 0,
        };

        let mut clamp = WidgetKind::from_tag("ng-clamp").unwrap();
        clamp.set_attribute("max-size", "960px");
        clamp.add_child(p_id, c1, false).unwrap();
        assert!(matches!(
            clamp,
            WidgetKind::Clamp {
                max_size: 960,
                child: Some(ch),
            } if ch == c1
        ));

        let mut spin = WidgetKind::from_tag("ng-spin-button").unwrap();
        spin.set_attribute("min", "0");
        spin.set_attribute("max", "500");
        spin.set_attribute("step", "5");
        spin.set_attribute("value", "25");
        assert!(matches!(
            spin,
            WidgetKind::SpinButton {
                value: 25,
                min: 0,
                max: 500,
                step: 5,
            }
        ));

        let mut search = WidgetKind::from_tag("ng-search-entry").unwrap();
        search.set_attribute("placeholder", "Search items");
        search.set_attribute("value", "widget");
        assert!(matches!(
            search,
            WidgetKind::SearchEntry { ref text, ref placeholder }
            if text == "widget" && placeholder == "Search items"
        ));

        let mut pass = WidgetKind::from_tag("ng-password-entry").unwrap();
        pass.set_attribute("show-peek-icon", "false");
        pass.set_text(p_id, "secret").unwrap();
        assert!(matches!(
            pass,
            WidgetKind::PasswordEntry { ref text, show_peek_icon: false, .. }
            if text == "secret"
        ));

        let mut tv = WidgetKind::from_tag("ng-text-view").unwrap();
        tv.set_attribute("editable", "false");
        tv.set_attribute("monospace", "true");
        tv.set_text(p_id, "Activity log").unwrap();
        assert!(matches!(
            tv,
            WidgetKind::TextView { ref text, editable: false, monospace: true }
            if text == "Activity log"
        ));
    }

    #[test]
    fn list_view_and_popover_manage_children_and_attributes() {
        let p_id = NodeId {
            index: 30,
            generation: 0,
        };
        let row1 = NodeId {
            index: 31,
            generation: 0,
        };
        let row2 = NodeId {
            index: 32,
            generation: 0,
        };

        let mut list = WidgetKind::from_tag("ng-list").unwrap();
        list.set_attribute("selection-mode", "multiple");
        list.set_attribute("show-separators", "true");
        list.add_child(p_id, row1, false).unwrap();
        list.insert_before(p_id, row2, Some(row1), false).unwrap();
        assert!(matches!(
            list,
            WidgetKind::ListView {
                ref selection_mode,
                show_separators: true,
                ref children,
            } if selection_mode == "multiple" && children == &vec![row2, row1]
        ));
        list.remove_child(p_id, row2).unwrap();
        assert_eq!(list.child_ids(), vec![row1]);
        assert_eq!(list.tag_name(), "ng-list");

        let mut pop = WidgetKind::from_tag("ng-popover").unwrap();
        pop.set_attribute("open", "true");
        pop.set_attribute("position", "top");
        pop.set_attribute("autohide", "false");
        pop.set_attribute("has-arrow", "false");
        pop.add_child(p_id, row1, false).unwrap();
        assert!(matches!(
            pop,
            WidgetKind::Popover {
                open: true,
                ref position,
                autohide: false,
                has_arrow: false,
                child: Some(c),
            } if position == "top" && c == row1
        ));
        pop.remove_child(p_id, row1).unwrap();
        assert_eq!(pop.child_ids(), Vec::<NodeId>::new());
        assert_eq!(pop.tag_name(), "ng-popover");
    }
}
