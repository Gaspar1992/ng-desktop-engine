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
mod retained_backend;
pub mod scene_state;
pub mod widget_tag;
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
use std::sync::atomic::{AtomicU8, Ordering};
pub use widget_tag::WidgetTag;
pub use windows_winui::{WinUiBackend, WinUiControlBinding};

static CONFIGURED_GPU_PREF: AtomicU8 = AtomicU8::new(0);

/// Cross-platform GPU rendering preference resolved from `NG_DESKTOP_GPU_BACKEND` or `angular.json` `gpu`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GpuRendererPreference {
    Auto = 0,
    Vulkan = 1,
    Gl = 2,
    Ngl = 3,
    Metal = 4,
    Dwm = 5,
    Software = 6,
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
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Vulkan => "vulkan",
            Self::Gl => "gl",
            Self::Ngl => "ngl",
            Self::Metal => "metal",
            Self::Dwm => "dwm",
            Self::Software => "software",
        }
    }

    const fn from_u8(code: u8) -> Self {
        match code {
            1 => Self::Vulkan,
            2 => Self::Gl,
            3 => Self::Ngl,
            4 => Self::Metal,
            5 => Self::Dwm,
            6 => Self::Software,
            _ => Self::Auto,
        }
    }

    pub fn set_configured(pref: Self) {
        CONFIGURED_GPU_PREF.store(pref as u8, Ordering::Relaxed);
    }

    #[cfg(test)]
    pub(crate) fn reset_configured() {
        CONFIGURED_GPU_PREF.store(0, Ordering::Relaxed);
    }

    #[must_use]
    pub fn from_env() -> Self {
        if let Ok(raw) = std::env::var("NG_DESKTOP_GPU_BACKEND") {
            let trimmed = raw.trim();
            if !trimmed.is_empty() {
                return Self::parse(trimmed);
            }
        }
        Self::from_u8(CONFIGURED_GPU_PREF.load(Ordering::Relaxed))
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
pub fn default_map_native_widget(tag: &str) -> &'static str {
    match WidgetTag::from_tag(tag) {
        WidgetTag::Window => "NativeWindow",
        WidgetTag::Dialog => "NativeDialog",
        WidgetTag::HeaderBar => "NativeHeaderBar",
        WidgetTag::Scroll => "NativeScrollView",
        WidgetTag::Box => "NativeBox",
        WidgetTag::Grid => "NativeGrid",
        WidgetTag::Separator => "NativeSeparator",
        WidgetTag::Label => "NativeLabel",
        WidgetTag::Button => "NativeButton",
        WidgetTag::Entry => "NativeEntry",
        WidgetTag::Switch => "NativeSwitch",
        WidgetTag::CheckButton => "NativeCheckButton",
        WidgetTag::Slider => "NativeSlider",
        WidgetTag::DropDown => "NativeDropDown",
        WidgetTag::Image => "NativeImage",
        WidgetTag::LevelBar => "NativeLevelBar",
        WidgetTag::ProgressBar => "NativeProgressBar",
        WidgetTag::Spinner => "NativeSpinner",
        WidgetTag::FlowBox => "NativeFlowBox",
        WidgetTag::Paned => "NativePaned",
        WidgetTag::Stack => "NativeStack",
        WidgetTag::Expander => "NativeExpander",
        WidgetTag::TextView => "NativeTextView",
        WidgetTag::PasswordEntry => "NativePasswordEntry",
        WidgetTag::SearchEntry => "NativeSearchEntry",
        WidgetTag::SpinButton => "NativeSpinButton",
        WidgetTag::Clamp => "NativeClamp",
        WidgetTag::ListView => "NativeListView",
        WidgetTag::Popover => "NativePopover",
        WidgetTag::Anchor => "NativeAnchor",
        WidgetTag::Unknown => "NativeWidget",
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
    fn is_native_display_active(&self) -> bool {
        false
    }
    fn open_window_count(&self) -> u32 {
        0
    }
    fn is_ffi_driver_loaded(&self) -> bool {
        false
    }
    fn step_native_events(&self, _may_block: bool) -> Vec<NativeEvent> {
        Vec::new()
    }
    fn gpu_telemetry_summary(&self) -> String {
        let pref = GpuRendererPreference::from_env();
        format!(
            "preference={} hardware_accel={}",
            pref.as_str(),
            pref.prefers_hardware_acceleration()
        )
    }
}

macro_rules! dispatch_backend {
    ($self:expr, $b:ident => $body:expr) => {
        match $self {
            Self::LinuxGtk($b) => $body,
            Self::WindowsWinUi($b) => $body,
            Self::MacOsAppKit($b) => $body,
            Self::Headless($b) => $body,
        }
    };
}

/// Polymorphic backend wrapper produced by the Platform Factory from `angular.json`.
pub enum AnyPlatformBackend {
    LinuxGtk(Box<GtkLinuxBackend>),
    WindowsWinUi(Box<WinUiBackend>),
    MacOsAppKit(Box<MacOsAppKitBackend>),
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
            "windows-winui" => Ok(Self::WindowsWinUi(Box::default())),
            "macos-appkit" => Ok(Self::MacOsAppKit(Box::default())),
            "headless" => Ok(Self::headless()),
            other => Err(format!("Unsupported targetPlatform '{other}'")),
        }
    }

    /// Factory Method resolving the concrete OS adapter parameterized by workspace config.
    pub fn from_config(cfg: &AngularNativeConfig) -> Result<Self, String> {
        GpuRendererPreference::set_configured(GpuRendererPreference::parse(&cfg.gpu_backend));
        match cfg.target_platform.as_str() {
            "linux-gtk4" => Ok(Self::LinuxGtk(Box::new(GtkLinuxBackend::with_app_config(
                &cfg.project_name,
                &cfg.icon_search_dir,
            )))),
            "windows-winui" => Ok(Self::WindowsWinUi(Box::default())),
            "macos-appkit" => Ok(Self::MacOsAppKit(Box::default())),
            "headless" => Ok(Self::headless()),
            other => Err(format!("Unsupported targetPlatform '{other}'")),
        }
    }

    pub fn is_native_display_active(&self) -> bool {
        dispatch_backend!(self, b => b.is_native_display_active())
    }

    pub fn open_window_count(&self) -> u32 {
        dispatch_backend!(self, b => b.open_window_count())
    }

    pub fn is_ffi_driver_loaded(&self) -> bool {
        dispatch_backend!(self, b => b.is_ffi_driver_loaded())
    }

    pub fn step_event_loop(&self, may_block: bool) -> Vec<NodeId> {
        self.step_native_events(may_block)
            .into_iter()
            .map(|e| e.node)
            .collect()
    }

    pub fn step_native_events(&self, may_block: bool) -> Vec<NativeEvent> {
        match self {
            Self::LinuxGtk(b) => PlatformBackend::step_native_events(&**b, may_block),
            Self::WindowsWinUi(b) => PlatformBackend::step_native_events(&**b, may_block),
            Self::MacOsAppKit(b) => PlatformBackend::step_native_events(&**b, may_block),
            Self::Headless(b) => PlatformBackend::step_native_events(b, may_block),
        }
    }
}

impl PlatformBackend for AnyPlatformBackend {
    fn backend_name(&self) -> &str {
        dispatch_backend!(self, b => b.backend_name())
    }

    fn map_native_widget(&self, tag: &str) -> &'static str {
        dispatch_backend!(self, b => b.map_native_widget(tag))
    }

    fn flush_commands(&mut self, commands: &[RenderCommand]) {
        dispatch_backend!(self, b => b.flush_commands(commands));
    }

    fn is_native_display_active(&self) -> bool {
        Self::is_native_display_active(self)
    }

    fn open_window_count(&self) -> u32 {
        Self::open_window_count(self)
    }

    fn is_ffi_driver_loaded(&self) -> bool {
        Self::is_ffi_driver_loaded(self)
    }

    fn step_native_events(&self, may_block: bool) -> Vec<NativeEvent> {
        Self::step_native_events(self, may_block)
    }

    fn gpu_telemetry_summary(&self) -> String {
        dispatch_backend!(self, b => b.gpu_telemetry_summary())
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
        assert_eq!(GpuRendererPreference::Vulkan.as_str(), "vulkan");
        assert_eq!(GpuRendererPreference::Dwm.as_str(), "dwm");
        assert_eq!(GpuRendererPreference::Software.as_str(), "software");
    }

    #[test]
    fn gpu_renderer_preference_honors_configured_fallback_and_telemetry() {
        GpuRendererPreference::set_configured(GpuRendererPreference::Metal);
        if std::env::var_os("NG_DESKTOP_GPU_BACKEND").is_none() {
            assert_eq!(
                GpuRendererPreference::from_env(),
                GpuRendererPreference::Metal
            );
        }
        GpuRendererPreference::reset_configured();
        if std::env::var_os("NG_DESKTOP_GPU_BACKEND").is_none() {
            assert_eq!(
                GpuRendererPreference::from_env(),
                GpuRendererPreference::Auto
            );
        }

        let headless = AnyPlatformBackend::headless();
        assert!(
            headless
                .gpu_telemetry_summary()
                .contains("compositor=headless")
        );
        let winui = WinUiBackend::new_headless();
        assert!(winui.gpu_telemetry_summary().contains("compositor="));
        let cocoa = MacOsAppKitBackend::new_headless();
        assert!(cocoa.gpu_telemetry_summary().contains("compositor="));
    }
}
