pub mod gtk4_callbacks;
mod gtk4_controls;
pub mod gtk4_styling;
pub mod gtk4_sys;
pub mod gtk4_widgets;
pub mod headless;
pub mod linux_gtk;
pub mod macos_appkit;
pub mod scene_state;
pub mod windows_winui;

use crate::engine::{NativeEvent, NodeId, RenderCommand};
pub use headless::HeadlessBackend;
pub use linux_gtk::GtkLinuxBackend;
pub use macos_appkit::MacOsAppKitBackend;
pub use scene_state::{NativeSceneNode, NativeSceneState};
pub use windows_winui::WinUiBackend;

/// Generic platform interface (Render Hardware Interface / OS Bridge).
/// Applies Dependency Inversion (DIP) and Open/Closed Principle (OCP):
/// `RenderEngine` depends only on this abstraction, never on a concrete OS toolkit.
pub trait PlatformBackend {
    fn backend_name(&self) -> &str;
    fn map_native_widget(&self, tag: &str) -> &'static str {
        match tag {
            "ng-window" | "adw-window" => "NativeWindow",
            "ng-header-bar" | "adw-header-bar" | "gtk-header-bar" => "NativeHeaderBar",
            "ng-scroll" | "gtk-scrolled-window" => "NativeScrollView",
            "ng-box" | "gtk-box" => "NativeBox",
            "ng-grid" | "gtk-grid" => "NativeGrid",
            "ng-separator" | "gtk-separator" => "NativeSeparator",
            "ng-label" | "gtk-label" => "NativeLabel",
            "ng-button" | "gtk-button" => "NativeButton",
            "ng-entry" | "gtk-entry" => "NativeEntry",
            "ng-switch" | "gtk-switch" => "NativeSwitch",
            "ng-checkbox" | "ng-check" | "gtk-check-button" => "NativeCheckButton",
            "ng-slider" | "ng-scale" | "gtk-scale" => "NativeSlider",
            "ng-select" | "ng-dropdown" | "gtk-drop-down" => "NativeDropDown",
            "ng-image" | "gtk-picture" | "ng-picture" => "NativeImage",
            "ng-level-bar" | "gtk-level-bar" => "NativeLevelBar",
            "ng-progress-bar" | "gtk-progress-bar" => "NativeProgressBar",
            "ng-spinner" | "gtk-spinner" => "NativeSpinner",
            "ng-flow" | "ng-flow-box" | "gtk-flow-box" => "NativeFlowBox",
            "ng-paned" | "gtk-paned" => "NativePaned",
            "ng-stack" | "gtk-stack" | "adw-navigation-view" => "NativeStack",
            "ng-expander" | "gtk-expander" => "NativeExpander",
            "ng-text-view" | "gtk-text-view" => "NativeTextView",
            "ng-password-entry" | "gtk-password-entry" => "NativePasswordEntry",
            "ng-search-entry" | "gtk-search-entry" => "NativeSearchEntry",
            "ng-spin-button" | "gtk-spin-button" => "NativeSpinButton",
            "ng-clamp" | "adw-clamp" => "NativeClamp",
            "ng-anchor" => "NativeAnchor",
            _ => "NativeWidget",
        }
    }
    fn flush_commands(&mut self, commands: &[RenderCommand]);
}

/// Polymorphic backend wrapper produced by the Platform Factory from `angular.json`.
pub enum AnyPlatformBackend {
    LinuxGtk(Box<GtkLinuxBackend>),
    WindowsWinUi(WinUiBackend),
    MacOsAppKit(MacOsAppKitBackend),
}

impl AnyPlatformBackend {
    /// Factory Method resolving the concrete OS adapter from `angular.json` `targetPlatform`.
    pub fn from_target(target_platform: &str) -> Result<Self, String> {
        match target_platform {
            "linux-gtk4" => Ok(Self::LinuxGtk(Box::default())),
            "windows-winui" => Ok(Self::WindowsWinUi(WinUiBackend::new())),
            "macos-appkit" => Ok(Self::MacOsAppKit(MacOsAppKitBackend::new())),
            other => Err(format!("Unsupported targetPlatform '{other}'")),
        }
    }

    pub fn is_native_display_active(&self) -> bool {
        match self {
            Self::LinuxGtk(b) => b.is_native_display_active(),
            Self::WindowsWinUi(_) | Self::MacOsAppKit(_) => false,
        }
    }

    pub fn open_window_count(&self) -> u32 {
        match self {
            Self::LinuxGtk(b) => b.open_window_count(),
            Self::WindowsWinUi(_) | Self::MacOsAppKit(_) => 0,
        }
    }

    pub fn step_event_loop(&self, may_block: bool) -> Vec<NodeId> {
        match self {
            Self::LinuxGtk(b) => b.step_event_loop(may_block),
            Self::WindowsWinUi(b) => b.step_native_events().into_iter().map(|e| e.node).collect(),
            Self::MacOsAppKit(b) => b.step_native_events().into_iter().map(|e| e.node).collect(),
        }
    }

    pub fn step_native_events(&self, may_block: bool) -> Vec<NativeEvent> {
        match self {
            Self::LinuxGtk(b) => b.step_native_events(may_block),
            Self::WindowsWinUi(b) => b.step_native_events(),
            Self::MacOsAppKit(b) => b.step_native_events(),
        }
    }
}

impl PlatformBackend for AnyPlatformBackend {
    fn backend_name(&self) -> &str {
        match self {
            Self::LinuxGtk(b) => b.backend_name(),
            Self::WindowsWinUi(b) => b.backend_name(),
            Self::MacOsAppKit(b) => b.backend_name(),
        }
    }

    fn map_native_widget(&self, tag: &str) -> &'static str {
        match self {
            Self::LinuxGtk(b) => b.map_native_widget(tag),
            Self::WindowsWinUi(b) => b.map_native_widget(tag),
            Self::MacOsAppKit(b) => b.map_native_widget(tag),
        }
    }

    fn flush_commands(&mut self, commands: &[RenderCommand]) {
        match self {
            Self::LinuxGtk(b) => b.flush_commands(commands),
            Self::WindowsWinUi(b) => b.flush_commands(commands),
            Self::MacOsAppKit(b) => b.flush_commands(commands),
        }
    }
}
