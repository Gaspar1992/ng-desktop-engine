/// Canonical native widget tag parsed once from `ng-*`, `gtk-*`, or `adw-*` aliases.
///
/// Centralizing alias parsing here ensures every platform backend (`GtkLinuxBackend`,
/// `WinUiBackend`, `MacOsAppKitBackend`, `default_map_native_widget`) matches exhaustively
/// on all 29 primitives plus `Anchor` and `Unknown`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WidgetTag {
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
    Unknown,
}

impl WidgetTag {
    #[must_use]
    pub fn from_tag(tag: &str) -> Self {
        match tag.as_bytes() {
            b"ng-window" | b"adw-window" => Self::Window,
            b"ng-dialog" | b"gtk-dialog" | b"adw-dialog" => Self::Dialog,
            b"ng-header-bar" | b"adw-header-bar" | b"gtk-header-bar" => Self::HeaderBar,
            b"ng-scroll" | b"gtk-scrolled-window" => Self::Scroll,
            b"ng-box" | b"gtk-box" => Self::Box,
            b"ng-grid" | b"gtk-grid" => Self::Grid,
            b"ng-separator" | b"gtk-separator" => Self::Separator,
            b"ng-label" | b"gtk-label" => Self::Label,
            b"ng-button" | b"gtk-button" => Self::Button,
            b"ng-entry" | b"gtk-entry" => Self::Entry,
            b"ng-switch" | b"gtk-switch" => Self::Switch,
            b"ng-checkbox" | b"ng-check" | b"gtk-check-button" => Self::CheckButton,
            b"ng-slider" | b"ng-scale" | b"gtk-scale" => Self::Slider,
            b"ng-select" | b"ng-dropdown" | b"gtk-drop-down" => Self::DropDown,
            b"ng-image" | b"gtk-picture" | b"ng-picture" => Self::Image,
            b"ng-level-bar" | b"gtk-level-bar" => Self::LevelBar,
            b"ng-spinner" | b"gtk-spinner" => Self::Spinner,
            b"ng-progress-bar" | b"gtk-progress-bar" => Self::ProgressBar,
            b"ng-flow" | b"ng-flow-box" | b"gtk-flow-box" => Self::FlowBox,
            b"ng-paned" | b"gtk-paned" => Self::Paned,
            b"ng-stack" | b"gtk-stack" | b"adw-navigation-view" => Self::Stack,
            b"ng-expander" | b"gtk-expander" => Self::Expander,
            b"ng-text-view" | b"gtk-text-view" => Self::TextView,
            b"ng-password-entry" | b"gtk-password-entry" => Self::PasswordEntry,
            b"ng-search-entry" | b"gtk-search-entry" => Self::SearchEntry,
            b"ng-spin-button" | b"gtk-spin-button" => Self::SpinButton,
            b"ng-clamp" | b"adw-clamp" => Self::Clamp,
            b"ng-list" | b"ng-table" | b"gtk-list-box" | b"gtk-list-view" => Self::ListView,
            b"ng-popover" | b"gtk-popover" | b"ng-menu" => Self::Popover,
            b"ng-anchor" | b"#comment" => Self::Anchor,
            _ => Self::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_tags_and_aliases_uniformly() {
        assert_eq!(WidgetTag::from_tag("ng-window"), WidgetTag::Window);
        assert_eq!(WidgetTag::from_tag("adw-window"), WidgetTag::Window);
        assert_eq!(WidgetTag::from_tag("ng-dialog"), WidgetTag::Dialog);
        assert_eq!(WidgetTag::from_tag("adw-dialog"), WidgetTag::Dialog);
        assert_eq!(WidgetTag::from_tag("ng-check"), WidgetTag::CheckButton);
        assert_eq!(WidgetTag::from_tag("ng-select"), WidgetTag::DropDown);
        assert_eq!(WidgetTag::from_tag("ng-menu"), WidgetTag::Popover);
        assert_eq!(WidgetTag::from_tag("#comment"), WidgetTag::Anchor);
        assert_eq!(WidgetTag::from_tag("ng-anchor"), WidgetTag::Anchor);
        assert_eq!(WidgetTag::from_tag("custom-el"), WidgetTag::Unknown);
    }
}
