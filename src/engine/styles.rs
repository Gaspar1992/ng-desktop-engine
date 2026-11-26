use super::node::{EngineError, NodeId, StyleId};
use crate::desktop::DesktopTheme;
use std::collections::HashMap;
use std::fmt::Write as _;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StyleRule {
    pub class_name: String,
    pub background: Option<String>,
    pub foreground: Option<String>,
    pub padding: u16,
    pub spacing: u16,
    pub border_radius: Option<u16>,
    pub font_size: Option<u16>,
    pub font_weight: Option<u16>,
    pub min_width: Option<u16>,
    pub min_height: Option<u16>,
    pub orientation: Option<String>,
    pub hexpand: Option<bool>,
    pub vexpand: Option<bool>,
    pub halign: Option<String>,
    pub valign: Option<String>,
    pub min_children: Option<u16>,
    pub max_children: Option<u16>,
    pub homogeneous: Option<bool>,
    pub pseudo_class: Option<String>,
    pub media_theme: Option<String>,
    pub media_min_width: Option<u16>,
    pub media_max_width: Option<u16>,
    pub opacity: Option<u8>,
    pub transition_ms: Option<u16>,
    pub transition_prop: Option<String>,
}

impl StyleRule {
    /// Returns the canonical lookup key for `StyleRegistry`, keeping plain base rules as `class_name`
    /// while isolating pseudo-class (`:hover`/`:focus`/`:active`/`:disabled`) and `@media` variants.
    #[must_use]
    pub fn registry_key(&self) -> String {
        if self.pseudo_class.is_none()
            && self.media_theme.is_none()
            && self.media_min_width.is_none()
            && self.media_max_width.is_none()
        {
            return self.class_name.clone();
        }
        let mut key = self.class_name.clone();
        if let Some(pseudo) = &self.pseudo_class {
            let _ = write!(key, ":{pseudo}");
        }
        if let Some(theme) = &self.media_theme {
            let _ = write!(key, "@theme={theme}");
        }
        if let Some(min_w) = self.media_min_width {
            let _ = write!(key, "@minw={min_w}");
        }
        if let Some(max_w) = self.media_max_width {
            let _ = write!(key, "@maxw={max_w}");
        }
        key
    }

    /// Returns `true` when any `@media` constraints on this rule match the active desktop theme and viewport width.
    #[must_use]
    pub fn matches_context(&self, theme: DesktopTheme, viewport_width: i32) -> bool {
        if let Some(req_theme) = &self.media_theme
            && !req_theme.eq_ignore_ascii_case(theme.as_str())
        {
            return false;
        }
        if let Some(min_w) = self.media_min_width
            && viewport_width < i32::from(min_w)
        {
            return false;
        }
        if let Some(max_w) = self.media_max_width
            && viewport_width > i32::from(max_w)
        {
            return false;
        }
        true
    }

    /// Applies an optional extended `key=value` declaration (`radius`, `font-size`,
    /// `font-weight`, `min-width`, `min-height`, CSS Flexbox/Grid, pseudo/media/opacity/transition directives).
    /// Returns `false` when the key is unknown or the value is invalid.
    pub fn apply_extension(&mut self, key: &str, raw_value: &str) -> bool {
        let trimmed = raw_value.trim();
        match key {
            "orientation" | "flex-direction" => match trimmed {
                "horizontal" | "row" => self.orientation = Some(String::from("horizontal")),
                "vertical" | "column" => self.orientation = Some(String::from("vertical")),
                _ => return false,
            },
            "hexpand" => match Self::parse_bool(trimmed) {
                Some(b) => self.hexpand = Some(b),
                None => return false,
            },
            "vexpand" => match Self::parse_bool(trimmed) {
                Some(b) => self.vexpand = Some(b),
                None => return false,
            },
            "homogeneous" => match Self::parse_bool(trimmed) {
                Some(b) => self.homogeneous = Some(b),
                None => return false,
            },
            "halign" if matches!(trimmed, "fill" | "start" | "end" | "center" | "baseline") => {
                self.halign = Some(String::from(trimmed));
            }
            "valign" if matches!(trimmed, "fill" | "start" | "end" | "center" | "baseline") => {
                self.valign = Some(String::from(trimmed));
            }
            "pseudo" if matches!(trimmed, "hover" | "focus" | "active" | "disabled") => {
                self.pseudo_class = Some(String::from(trimmed));
            }
            "media-theme" if matches!(trimmed, "light" | "dark") => {
                self.media_theme = Some(String::from(trimmed));
            }
            "transition-prop"
                if matches!(
                    trimmed,
                    "all" | "background" | "color" | "opacity" | "padding" | "border-radius"
                ) =>
            {
                self.transition_prop = Some(String::from(trimmed));
            }
            "halign" | "valign" | "pseudo" | "media-theme" | "transition-prop" => return false,
            _ => return self.apply_numeric_extension(key, trimmed),
        }
        true
    }

    fn parse_bool(raw: &str) -> Option<bool> {
        match raw {
            "true" | "1" => Some(true),
            "false" | "0" => Some(false),
            _ => None,
        }
    }

    fn apply_numeric_extension(&mut self, key: &str, raw_value: &str) -> bool {
        let cleaned = raw_value
            .trim_end_matches("px")
            .trim_end_matches("ms")
            .trim();
        let Ok(value) = cleaned.parse::<u16>() else {
            return false;
        };
        match key {
            "radius" | "border-radius" => self.border_radius = Some(value),
            "font-size" => self.font_size = Some(value),
            "font-weight" => self.font_weight = Some(value),
            "min-width" => self.min_width = Some(value),
            "min-height" => self.min_height = Some(value),
            "min-children" | "min-per-row" => self.min_children = Some(value),
            "max-children" | "max-per-row" => self.max_children = Some(value),
            "media-min-width" => self.media_min_width = Some(value),
            "media-max-width" => self.media_max_width = Some(value),
            "opacity" if value <= 100 => {
                self.opacity = u8::try_from(value).ok();
            }
            "transition-ms" => self.transition_ms = Some(value),
            _ => return false,
        }
        true
    }
}

/// Game-engine style Resource Manager for UI styles.
pub struct StyleRegistry {
    rules_by_id: HashMap<StyleId, StyleRule>,
    ids_by_name: HashMap<String, StyleId>,
    node_styles: HashMap<NodeId, Vec<StyleId>>,
    next_id: u32,
}

impl Default for StyleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl StyleRegistry {
    pub fn new() -> Self {
        Self {
            rules_by_id: HashMap::new(),
            ids_by_name: HashMap::new(),
            node_styles: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn register_rule(&mut self, rule: StyleRule) -> StyleId {
        let key = rule.registry_key();
        if let Some(&existing_id) = self.ids_by_name.get(&key) {
            self.rules_by_id.insert(existing_id, rule);
            return existing_id;
        }

        let id = StyleId(self.next_id);
        self.next_id += 1;
        self.ids_by_name.insert(key, id);
        self.rules_by_id.insert(id, rule);
        id
    }

    pub fn attach_class_with_rule(
        &mut self,
        node: NodeId,
        class_name: &str,
    ) -> Result<(StyleId, Option<StyleRule>), EngineError> {
        let (style_id, registered_rule) = if let Some(&id) = self.ids_by_name.get(class_name) {
            (id, None)
        } else if self
            .rules_by_id
            .values()
            .any(|rule| rule.class_name == class_name)
        {
            let placeholder = StyleRule {
                class_name: class_name.to_string(),
                ..StyleRule::default()
            };
            let id = self.register_rule(placeholder.clone());
            (id, Some(placeholder))
        } else {
            return Err(EngineError::StyleNotFound(class_name.to_string()));
        };

        let list = self.node_styles.entry(node).or_default();
        if !list.contains(&style_id) {
            list.push(style_id);
        }
        Ok((style_id, registered_rule))
    }

    pub fn attach_class(&mut self, node: NodeId, class_name: &str) -> Result<StyleId, EngineError> {
        self.attach_class_with_rule(node, class_name)
            .map(|(style_id, _)| style_id)
    }

    pub fn detach_class(&mut self, node: NodeId, class_name: &str) -> Option<StyleId> {
        let style_id = *self.ids_by_name.get(class_name)?;
        if let Some(list) = self.node_styles.get_mut(&node) {
            list.retain(|id| *id != style_id);
        }
        Some(style_id)
    }

    pub fn get_rule(&self, id: StyleId) -> Option<&StyleRule> {
        self.rules_by_id.get(&id)
    }

    pub fn get_rule_by_name(&self, class_name: &str) -> Option<&StyleRule> {
        let id = self.ids_by_name.get(class_name)?;
        self.rules_by_id.get(id)
    }

    pub fn node_classes(&self, node: NodeId) -> Vec<&str> {
        self.node_styles
            .get(&node)
            .into_iter()
            .flatten()
            .filter_map(|id| self.get_rule(*id).map(|r| r.class_name.as_str()))
            .collect()
    }

    pub fn remove_node(&mut self, node: NodeId) {
        self.node_styles.remove(&node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deduplicates_styles_by_class_name_and_tracks_node_classes() {
        let mut reg = StyleRegistry::new();
        let node = NodeId {
            index: 0,
            generation: 1,
        };

        let id1 = reg.register_rule(StyleRule {
            class_name: String::from("card"),
            background: Some(String::from("#111111")),
            foreground: None,
            padding: 8,
            spacing: 4,
            ..StyleRule::default()
        });
        let id2 = reg.register_rule(StyleRule {
            class_name: String::from("card"),
            background: Some(String::from("#222222")),
            foreground: None,
            padding: 16,
            spacing: 8,
            ..StyleRule::default()
        });
        assert_eq!(id1, id2);
        assert_eq!(reg.get_rule(id1).unwrap().padding, 16);

        assert_eq!(
            reg.attach_class(node, "missing"),
            Err(EngineError::StyleNotFound(String::from("missing")))
        );
        assert_eq!(reg.attach_class(node, "card").unwrap(), id1);
        assert_eq!(reg.node_classes(node), vec!["card"]);

        assert_eq!(reg.detach_class(node, "card"), Some(id1));
        assert_eq!(reg.node_classes(node), Vec::<&str>::new());
        assert_eq!(reg.detach_class(node, "missing"), None);

        assert_eq!(reg.attach_class(node, "card").unwrap(), id1);
        reg.remove_node(node);
        assert_eq!(reg.node_classes(node), Vec::<&str>::new());
    }

    #[test]
    fn applies_typography_and_radius_extensions_and_rejects_invalid_ones() {
        let mut rule = StyleRule {
            class_name: String::from("card"),
            ..StyleRule::default()
        };
        assert!(rule.apply_extension("radius", "12px"));
        assert!(rule.apply_extension("font-size", "18"));
        assert!(rule.apply_extension("font-weight", "700"));
        assert!(rule.apply_extension("min-width", "240px"));
        assert!(rule.apply_extension("min-height", "120"));
        assert!(rule.apply_extension("orientation", "horizontal"));
        assert!(rule.apply_extension("hexpand", "true"));
        assert!(rule.apply_extension("vexpand", "false"));
        assert!(rule.apply_extension("halign", "center"));
        assert!(rule.apply_extension("valign", "start"));
        assert!(rule.apply_extension("min-children", "1"));
        assert!(rule.apply_extension("max-children", "12"));
        assert!(rule.apply_extension("homogeneous", "true"));
        assert!(!rule.apply_extension("orientation", "diagonal"));
        assert!(!rule.apply_extension("halign", "stretch"));
        assert!(!rule.apply_extension("font-weight", "bold"));
        assert!(!rule.apply_extension("letter-spacing", "2"));
        assert_eq!(rule.border_radius, Some(12));
        assert_eq!(rule.font_size, Some(18));
        assert_eq!(rule.font_weight, Some(700));
        assert_eq!(rule.min_width, Some(240));
        assert_eq!(rule.min_height, Some(120));
        assert_eq!(rule.orientation.as_deref(), Some("horizontal"));
        assert_eq!(rule.hexpand, Some(true));
        assert_eq!(rule.vexpand, Some(false));
        assert_eq!(rule.halign.as_deref(), Some("center"));
        assert_eq!(rule.valign.as_deref(), Some("start"));
        assert_eq!(rule.min_children, Some(1));
        assert_eq!(rule.max_children, Some(12));
        assert_eq!(rule.homogeneous, Some(true));
    }

    #[test]
    fn isolates_pseudo_and_media_style_variants_and_validates_context() {
        let mut reg = StyleRegistry::new();
        let base_id = reg.register_rule(StyleRule {
            class_name: String::from("btn"),
            background: Some(String::from("#111111")),
            ..StyleRule::default()
        });
        let hover_id = reg.register_rule(StyleRule {
            class_name: String::from("btn"),
            background: Some(String::from("#222222")),
            pseudo_class: Some(String::from("hover")),
            opacity: Some(90),
            transition_ms: Some(180),
            transition_prop: Some(String::from("background")),
            ..StyleRule::default()
        });
        let light_id = reg.register_rule(StyleRule {
            class_name: String::from("btn"),
            background: Some(String::from("#f8fafc")),
            media_theme: Some(String::from("light")),
            media_min_width: Some(640),
            ..StyleRule::default()
        });
        assert_ne!(base_id, hover_id);
        assert_ne!(base_id, light_id);
        assert_ne!(hover_id, light_id);

        let light_rule = reg.get_rule(light_id).unwrap();
        assert!(!light_rule.matches_context(DesktopTheme::Dark, 800));
        assert!(!light_rule.matches_context(DesktopTheme::Light, 500));
        assert!(light_rule.matches_context(DesktopTheme::Light, 800));

        let mut ext_rule = StyleRule {
            class_name: String::from("fade"),
            ..StyleRule::default()
        };
        assert!(ext_rule.apply_extension("pseudo", "focus"));
        assert!(ext_rule.apply_extension("media-theme", "dark"));
        assert!(ext_rule.apply_extension("media-min-width", "768px"));
        assert!(ext_rule.apply_extension("media-max-width", "1280px"));
        assert!(ext_rule.apply_extension("opacity", "75"));
        assert!(ext_rule.apply_extension("transition-ms", "220ms"));
        assert!(ext_rule.apply_extension("transition-prop", "opacity"));
        assert!(!ext_rule.apply_extension("opacity", "105"));
        assert!(!ext_rule.apply_extension("pseudo", "visited"));

        let node = NodeId {
            index: 1,
            generation: 1,
        };
        let attached = reg.attach_class(node, "btn").unwrap();
        assert_eq!(attached, base_id);

        let pseudo_only = reg.register_rule(StyleRule {
            class_name: String::from("hover-only"),
            background: Some(String::from("#334155")),
            pseudo_class: Some(String::from("hover")),
            ..StyleRule::default()
        });
        assert_ne!(pseudo_only.0, 0);
        assert!(reg.attach_class(node, "hover-only").is_ok());
        assert!(reg.node_classes(node).contains(&"hover-only"));
    }
}
