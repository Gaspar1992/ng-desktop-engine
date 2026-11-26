//! Cross-platform native UI backend adapters (`Linux GTK4`, `Windows WinUI 3`, `macOS AppKit`,
//! and `HeadlessBackend`), shared Flexbox/Grid [`layout_solver`], and [`NativeSceneState`].

pub mod appkit_sys;
pub mod gtk4_callbacks;
mod gtk4_controls;
pub mod gtk4_styling;
pub mod gtk4_sys;
pub mod gtk4_widgets;
pub mod headless;
pub mod layout_solver;
pub mod linux_gtk;
pub mod macos_appkit;
pub mod scene_state;
pub mod win32_sys;
pub mod windows_winui;

use crate::config::AngularNativeConfig;
use crate::engine::{NativeEvent, NodeId, RenderCommand};
pub use headless::HeadlessBackend;
pub use layout_solver::{LayoutRect, compute_parent_relative_bounds, solve_scene_layout};
pub use linux_gtk::GtkLinuxBackend;
pub use macos_appkit::{AppKitWidgetBinding, MacOsAppKitBackend};
pub use scene_state::{NativeSceneNode, NativeSceneState};
use std::sync::OnceLock;
pub use windows_winui::{WinUiBackend, WinUiControlBinding};

/// Cross-platform GPU rendering preference resolved from `NG_DESKTOP_GPU_BACKEND`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuRendererPreference {
    Auto,
    Vulkan,
    Gl,
    Ngl,
    Metal,
    Dwm,
    Software,
}

impl GpuRendererPreference {
    #[must_use]
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "vulkan" => Self::Vulkan,
            "gl" => Self::Gl,
            "ngl" => Self::Ngl,
            "metal" => Self::Metal,
            "dwm" => Self::Dwm,
            "software" | "cairo" | "cpu" => Self::Software,
            _ => Self::Auto,
        }
    }

    #[must_use]
    pub fn from_env() -> Self {
        std::env::var("NG_DESKTOP_GPU_BACKEND").map_or(Self::Auto, |raw| Self::parse(&raw))
    }

    #[must_use]
    pub const fn gsk_renderer_value(self) -> Option<&'static str> {
        match self {
            Self::Vulkan => Some("vulkan"),
            Self::Gl => Some("gl"),
            Self::Ngl => Some("ngl"),
            Self::Software => Some("cairo"),
            Self::Auto | Self::Metal | Self::Dwm => None,
        }
    }

    #[must_use]
    pub const fn prefers_hardware_acceleration(self) -> bool {
        !matches!(self, Self::Software)
    }
}

static TRACE_COMMANDS: OnceLock<bool> = OnceLock::new();

fn parse_trace_flag(raw: &str) -> bool {
    matches!(raw.trim(), "1" | "true" | "TRUE" | "yes" | "YES")
}

/// Returns whether per-command render traces should be written to stdout.
#[must_use]
pub fn trace_commands_enabled() -> bool {
    *TRACE_COMMANDS.get_or_init(|| {
        std::env::var("NG_DESKTOP_TRACE_COMMANDS").is_ok_and(|v| parse_trace_flag(&v))
    })
}

pub(super) fn trace_flush_commands(
    backend_name: &str,
    frame: u64,
    commands: &[RenderCommand],
    resolve_widget: impl Fn(&str) -> &'static str,
) {
    if !trace_commands_enabled() {
        return;
    }
    println!(
        "[{backend_name} | Frame #{frame}] Flushing {} render commands:",
        commands.len()
    );
    for cmd in commands {
        if let RenderCommand::CreateWidget { id, tag } = cmd {
            println!(
                "  -> CreateWidget {{ id: {id:?}, tag: {tag:?} => native: {} }}",
                resolve_widget(tag)
            );
        } else {
            println!("  -> {cmd:?}");
        }
    }
}

/// Default cross-platform tag-to-widget mapping shared by generic and headless backends.
#[must_use]
pub const fn default_map_native_widget(tag: &str) -> &'static str {
    match tag.as_bytes() {
        b"ng-window" | b"adw-window" => "NativeWindow",
        b"ng-dialog" | b"gtk-dialog" | b"adw-dialog" => "NativeDialog",
        b"ng-header-bar" | b"adw-header-bar" | b"gtk-header-bar" => "NativeHeaderBar",
        b"ng-scroll" | b"gtk-scrolled-window" => "NativeScrollView",
        b"ng-box" | b"gtk-box" => "NativeBox",
        b"ng-grid" | b"gtk-grid" => "NativeGrid",
        b"ng-separator" | b"gtk-separator" => "NativeSeparator",
        b"ng-label" | b"gtk-label" => "NativeLabel",
        b"ng-button" | b"gtk-button" => "NativeButton",
        b"ng-entry" | b"gtk-entry" => "NativeEntry",
        b"ng-switch" | b"gtk-switch" => "NativeSwitch",
        b"ng-checkbox" | b"ng-check" | b"gtk-check-button" => "NativeCheckButton",
        b"ng-slider" | b"ng-scale" | b"gtk-scale" => "NativeSlider",
        b"ng-select" | b"ng-dropdown" | b"gtk-drop-down" => "NativeDropDown",
        b"ng-image" | b"gtk-picture" | b"ng-picture" => "NativeImage",
        b"ng-level-bar" | b"gtk-level-bar" => "NativeLevelBar",
        b"ng-progress-bar" | b"gtk-progress-bar" => "NativeProgressBar",
        b"ng-spinner" | b"gtk-spinner" => "NativeSpinner",
        b"ng-flow" | b"ng-flow-box" | b"gtk-flow-box" => "NativeFlowBox",
        b"ng-paned" | b"gtk-paned" => "NativePaned",
        b"ng-stack" | b"gtk-stack" | b"adw-navigation-view" => "NativeStack",
        b"ng-expander" | b"gtk-expander" => "NativeExpander",
        b"ng-text-view" | b"gtk-text-view" => "NativeTextView",
        b"ng-password-entry" | b"gtk-password-entry" => "NativePasswordEntry",
        b"ng-search-entry" | b"gtk-search-entry" => "NativeSearchEntry",
        b"ng-spin-button" | b"gtk-spin-button" => "NativeSpinButton",
        b"ng-clamp" | b"adw-clamp" => "NativeClamp",
        b"ng-list" | b"ng-table" | b"gtk-list-box" | b"gtk-list-view" => "NativeListView",
        b"ng-popover" | b"gtk-popover" | b"ng-menu" => "NativePopover",
        b"ng-anchor" => "NativeAnchor",
        _ => "NativeWidget",
    }
}

/// Generic platform interface (Render Hardware Interface / OS Bridge).
/// Applies Dependency Inversion (DIP) and Open/Closed Principle (OCP):
/// `RenderEngine` depends only on this abstraction, never on a concrete OS toolkit.
pub trait PlatformBackend {
    fn backend_name(&self) -> &str;
    fn map_native_widget(&self, tag: &str) -> &'static str {
        default_map_native_widget(tag)
    }
    fn flush_commands(&mut self, commands: &[RenderCommand]);
}

/// Polymorphic backend wrapper produced by the Platform Factory from `angular.json`.
pub enum AnyPlatformBackend {
    LinuxGtk(Box<GtkLinuxBackend>),
    WindowsWinUi(WinUiBackend),
    MacOsAppKit(MacOsAppKitBackend),
    Headless(HeadlessBackend),
}

impl AnyPlatformBackend {
    /// Creates an OS-independent headless backend for automated testing and snapshot runs.
    #[must_use]
    pub fn headless() -> Self {
        Self::Headless(HeadlessBackend::new())
    }

    /// Factory Method resolving the concrete OS adapter from `angular.json` `targetPlatform`.
    pub fn from_target(target_platform: &str) -> Result<Self, String> {
        match target_platform {
            "linux-gtk4" => Ok(Self::LinuxGtk(Box::default())),
            "windows-winui" => Ok(Self::WindowsWinUi(WinUiBackend::new())),
            "macos-appkit" => Ok(Self::MacOsAppKit(MacOsAppKitBackend::new())),
            "headless" => Ok(Self::headless()),
            other => Err(format!("Unsupported targetPlatform '{other}'")),
        }
    }

    /// Factory Method resolving the concrete OS adapter parameterized by workspace config.
    pub fn from_config(cfg: &AngularNativeConfig) -> Result<Self, String> {
        match cfg.target_platform.as_str() {
            "linux-gtk4" => Ok(Self::LinuxGtk(Box::new(GtkLinuxBackend::with_app_config(
                &cfg.project_name,
                &cfg.icon_search_dir,
            )))),
            "windows-winui" => Ok(Self::WindowsWinUi(WinUiBackend::new())),
            "macos-appkit" => Ok(Self::MacOsAppKit(MacOsAppKitBackend::new())),
            "headless" => Ok(Self::headless()),
            other => Err(format!("Unsupported targetPlatform '{other}'")),
        }
    }

    pub fn is_native_display_active(&self) -> bool {
        match self {
            Self::LinuxGtk(b) => b.is_native_display_active(),
            Self::WindowsWinUi(b) => b.is_native_display_active(),
            Self::MacOsAppKit(b) => b.is_native_display_active(),
            Self::Headless(_) => false,
        }
    }

    pub fn open_window_count(&self) -> u32 {
        match self {
            Self::LinuxGtk(b) => b.open_window_count(),
            Self::WindowsWinUi(b) => b.open_window_count(),
            Self::MacOsAppKit(b) => b.open_window_count(),
            Self::Headless(_) => 0,
        }
    }

    pub fn is_ffi_driver_loaded(&self) -> bool {
        match self {
            Self::LinuxGtk(b) => b.is_native_display_active(),
            Self::WindowsWinUi(b) => b.is_ffi_loaded(),
            Self::MacOsAppKit(b) => b.is_ffi_loaded(),
            Self::Headless(_) => false,
        }
    }

    pub fn step_event_loop(&self, may_block: bool) -> Vec<NodeId> {
        match self {
            Self::LinuxGtk(b) => b.step_event_loop(may_block),
            Self::WindowsWinUi(b) => b.step_native_events().into_iter().map(|e| e.node).collect(),
            Self::MacOsAppKit(b) => b.step_native_events().into_iter().map(|e| e.node).collect(),
            Self::Headless(_) => Vec::new(),
        }
    }

    pub fn step_native_events(&self, may_block: bool) -> Vec<NativeEvent> {
        match self {
            Self::LinuxGtk(b) => b.step_native_events(may_block),
            Self::WindowsWinUi(b) => b.step_native_events(),
            Self::MacOsAppKit(b) => b.step_native_events(),
            Self::Headless(_) => Vec::new(),
        }
    }
}

impl PlatformBackend for AnyPlatformBackend {
    fn backend_name(&self) -> &str {
        match self {
            Self::LinuxGtk(b) => b.backend_name(),
            Self::WindowsWinUi(b) => b.backend_name(),
            Self::MacOsAppKit(b) => b.backend_name(),
            Self::Headless(b) => b.backend_name(),
        }
    }

    fn map_native_widget(&self, tag: &str) -> &'static str {
        match self {
            Self::LinuxGtk(b) => b.map_native_widget(tag),
            Self::WindowsWinUi(b) => b.map_native_widget(tag),
            Self::MacOsAppKit(b) => b.map_native_widget(tag),
            Self::Headless(b) => b.map_native_widget(tag),
        }
    }

    fn flush_commands(&mut self, commands: &[RenderCommand]) {
        match self {
            Self::LinuxGtk(b) => b.flush_commands(commands),
            Self::WindowsWinUi(b) => b.flush_commands(commands),
            Self::MacOsAppKit(b) => b.flush_commands(commands),
            Self::Headless(b) => b.flush_commands(commands),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_tracing_is_disabled_by_default() {
        if std::env::var_os("NG_DESKTOP_TRACE_COMMANDS").is_none() {
            assert!(!trace_commands_enabled());
        }
        assert!(parse_trace_flag("1"));
        assert!(parse_trace_flag("true"));
        assert!(parse_trace_flag("YES"));
        assert!(!parse_trace_flag("0"));
        assert!(!parse_trace_flag("false"));
    }

    #[test]
    fn winui_and_appkit_exclude_anchor_and_recompute_layout_on_resize() {
        let mut winui = WinUiBackend::new_headless();
        let mut cocoa = MacOsAppKitBackend::new_headless();
        assert_eq!(
            winui.map_native_widget("ng-anchor"),
            "Microsoft.UI.Xaml.AnchorComment"
        );
        assert_eq!(cocoa.map_native_widget("ng-anchor"), "NSAnchorComment");

        let win = NodeId {
            index: 0,
            generation: 1,
        };
        let box_id = NodeId {
            index: 1,
            generation: 1,
        };
        let init_cmds = [
            RenderCommand::CreateWidget {
                id: win,
                tag: String::from("ng-window"),
            },
            RenderCommand::CreateWidget {
                id: box_id,
                tag: String::from("ng-box"),
            },
            RenderCommand::AppendChild {
                parent: win,
                child: box_id,
            },
        ];
        winui.flush_commands(&init_cmds);
        cocoa.flush_commands(&init_cmds);
        assert_eq!(winui.get_layout_rect(box_id).unwrap().width, 996);
        assert_eq!(cocoa.get_layout_rect(box_id).unwrap().width, 996);

        let resize_ev = NativeEvent {
            node: win,
            event_name: String::from("resize"),
            payload: crate::engine::EventPayload::Text(String::from(
                r#"{"width":1400,"height":900,"maximized":false}"#,
            )),
        };
        winui.queue_native_event(resize_ev.clone());
        cocoa.queue_native_event(resize_ev);
        let _ = winui.step_native_events();
        let _ = cocoa.step_native_events();

        winui.flush_commands(&[]);
        cocoa.flush_commands(&[]);
        assert_eq!(winui.get_layout_rect(box_id).unwrap().width, 1376);
        assert_eq!(cocoa.get_layout_rect(box_id).unwrap().width, 1376);
    }

    #[test]
    fn gpu_renderer_preference_parses_os_backends_and_software_fallback() {
        assert_eq!(
            GpuRendererPreference::parse(""),
            GpuRendererPreference::Auto
        );
        assert_eq!(
            GpuRendererPreference::parse("Vulkan"),
            GpuRendererPreference::Vulkan
        );
        assert_eq!(
            GpuRendererPreference::parse("gl"),
            GpuRendererPreference::Gl
        );
        assert_eq!(
            GpuRendererPreference::parse("ngl"),
            GpuRendererPreference::Ngl
        );
        assert_eq!(
            GpuRendererPreference::parse("metal"),
            GpuRendererPreference::Metal
        );
        assert_eq!(
            GpuRendererPreference::parse("dwm"),
            GpuRendererPreference::Dwm
        );
        assert_eq!(
            GpuRendererPreference::parse("cairo"),
            GpuRendererPreference::Software
        );
        assert_eq!(
            GpuRendererPreference::parse("software"),
            GpuRendererPreference::Software
        );

        assert_eq!(
            GpuRendererPreference::Vulkan.gsk_renderer_value(),
            Some("vulkan")
        );
        assert_eq!(GpuRendererPreference::Gl.gsk_renderer_value(), Some("gl"));
        assert_eq!(GpuRendererPreference::Ngl.gsk_renderer_value(), Some("ngl"));
        assert_eq!(
            GpuRendererPreference::Software.gsk_renderer_value(),
            Some("cairo")
        );
        assert_eq!(GpuRendererPreference::Auto.gsk_renderer_value(), None);
        assert!(GpuRendererPreference::Metal.prefers_hardware_acceleration());
        assert!(!GpuRendererPreference::Software.prefers_hardware_acceleration());
    }
}
