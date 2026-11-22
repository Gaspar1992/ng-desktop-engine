#![allow(unsafe_code)]

use super::PlatformBackend;
use super::scene_state::{NativeSceneNode, NativeSceneState};
use crate::engine::{EventMask, NativeEvent, NodeId, RenderCommand};
#[cfg(target_os = "windows")]
use crate::ffi_dl::DynLib;
use std::collections::{HashMap, HashSet};
#[cfg(target_os = "windows")]
use std::ffi::c_void;

/// Win32 / WinUI 3 runtime binding metadata recorded per live control node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WinUiControlBinding {
    pub xaml_class: &'static str,
    pub win32_class: &'static str,
    pub symbol_bound: bool,
    pub event_mask: EventMask,
    pub win32_event_routes: Vec<&'static str>,
}

struct WinUiFfiDriver {
    ffi_loaded: bool,
    resolved_symbols: HashSet<&'static str>,
    bindings: HashMap<NodeId, WinUiControlBinding>,
}

impl WinUiFfiDriver {
    fn new() -> Self {
        #[cfg(target_os = "windows")]
        {
            Self::init_windows_driver()
        }
        #[cfg(not(target_os = "windows"))]
        {
            Self {
                ffi_loaded: false,
                resolved_symbols: HashSet::new(),
                bindings: HashMap::new(),
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn init_windows_driver() -> Self {
        let Some(user32) = (unsafe { DynLib::open(c"user32.dll") }) else {
            return Self {
                ffi_loaded: false,
                resolved_symbols: HashSet::new(),
                bindings: HashMap::new(),
            };
        };

        let mut resolved_symbols = HashSet::new();
        for (name, c_name) in [
            ("CreateWindowExW", c"CreateWindowExW"),
            ("DestroyWindow", c"DestroyWindow"),
            ("ShowWindow", c"ShowWindow"),
            ("SetWindowTextW", c"SetWindowTextW"),
            ("MoveWindow", c"MoveWindow"),
            ("IsWindow", c"IsWindow"),
            ("GetMessageW", c"GetMessageW"),
            ("TranslateMessage", c"TranslateMessage"),
            ("DispatchMessageW", c"DispatchMessageW"),
            ("TrackMouseEvent", c"TrackMouseEvent"),
            ("GetKeyState", c"GetKeyState"),
        ] {
            if unsafe { user32.sym::<*mut c_void>(c_name) }.is_some() {
                resolved_symbols.insert(name);
            }
        }

        let ffi_loaded = resolved_symbols.contains("CreateWindowExW")
            && resolved_symbols.contains("DestroyWindow")
            && resolved_symbols.contains("SetWindowTextW");

        Self {
            ffi_loaded,
            resolved_symbols,
            bindings: HashMap::new(),
        }
    }

    fn win32_class_for(xaml_class: &str) -> &'static str {
        match xaml_class {
            "Microsoft.UI.Xaml.Window" => "Window",
            "Microsoft.UI.Xaml.Controls.ContentDialog" => "Dialog",
            "Microsoft.UI.Xaml.Controls.Button"
            | "Microsoft.UI.Xaml.Controls.CheckBox"
            | "Microsoft.UI.Xaml.Controls.ToggleSwitch" => "BUTTON",
            "Microsoft.UI.Xaml.Controls.TextBox"
            | "Microsoft.UI.Xaml.Controls.RichEditBox"
            | "Microsoft.UI.Xaml.Controls.PasswordBox"
            | "Microsoft.UI.Xaml.Controls.AutoSuggestBox" => "EDIT",
            "Microsoft.UI.Xaml.Controls.ComboBox" => "COMBOBOX",
            "Microsoft.UI.Xaml.Controls.ScrollViewer" => "SCROLLBAR",
            _ => "STATIC",
        }
    }

    fn win32_routes_for_mask(mask: EventMask) -> Vec<&'static str> {
        const TABLE: &[(EventMask, &str)] = &[
            (EventMask::CLICK, "WM_LBUTTONUP/Tapped"),
            (EventMask::CONTEXT_MENU, "WM_CONTEXTMENU/RightTapped"),
            (EventMask::POINTER_MOTION, "WM_MOUSEMOVE/PointerMoved"),
            (EventMask::FOCUS, "WM_SETFOCUS/GotFocus"),
            (EventMask::KEYBOARD, "WM_KEYDOWN/KeyDown"),
            (EventMask::ACTIVATE, "VK_RETURN/QuerySubmitted"),
            (EventMask::VALUE_CHANGE, "EN_CHANGE/ValueChanged"),
            (EventMask::SCROLL, "WM_VSCROLL/ViewChanged"),
            (EventMask::WHEEL, "WM_MOUSEWHEEL/PointerWheelChanged"),
            (EventMask::WINDOW_STATE, "WM_SIZE/SizeChanged"),
            (EventMask::CLOSE_REQUEST, "WM_CLOSE/Closing"),
            (EventMask::DRAG_DROP, "WM_DROPFILES/Drop"),
        ];
        TABLE
            .iter()
            .filter_map(|&(bit, route)| mask.contains(bit).then_some(route))
            .collect()
    }

    fn register_node(&mut self, id: NodeId, xaml_class: &'static str) {
        let win32_class = Self::win32_class_for(xaml_class);
        let symbol_bound = if xaml_class == "Microsoft.UI.Xaml.Window" {
            self.resolved_symbols.contains("CreateWindowExW")
                && self.resolved_symbols.contains("ShowWindow")
        } else {
            self.resolved_symbols.contains("CreateWindowExW")
                && self.resolved_symbols.contains("SetWindowTextW")
        };
        self.bindings.insert(
            id,
            WinUiControlBinding {
                xaml_class,
                win32_class,
                symbol_bound,
                event_mask: EventMask::NONE,
                win32_event_routes: Vec::new(),
            },
        );
    }

    fn update_event_mask(&mut self, id: NodeId, mask: EventMask) {
        if let Some(binding) = self.bindings.get_mut(&id) {
            binding.event_mask = binding.event_mask.union(mask);
            binding.win32_event_routes = Self::win32_routes_for_mask(binding.event_mask);
        }
    }

    fn unregister_node(&mut self, id: NodeId) {
        self.bindings.remove(&id);
    }

    fn open_window_count(&self) -> u32 {
        let count = self
            .bindings
            .values()
            .filter(|b| b.xaml_class == "Microsoft.UI.Xaml.Window")
            .count();
        u32::try_from(count).unwrap_or(u32::MAX)
    }
}

/// Concrete Windows WinUI 3 (`Microsoft.UI.Xaml`) platform adapter.
pub struct WinUiBackend {
    pub flushed_frames: u64,
    state: NativeSceneState,
    ffi_driver: WinUiFfiDriver,
}

impl Default for WinUiBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl WinUiBackend {
    pub fn new() -> Self {
        Self {
            flushed_frames: 0,
            state: NativeSceneState::default(),
            ffi_driver: WinUiFfiDriver::new(),
        }
    }

    pub const fn is_ffi_loaded(&self) -> bool {
        self.ffi_driver.ffi_loaded
    }

    pub fn resolved_symbol_count(&self) -> usize {
        self.ffi_driver.resolved_symbols.len()
    }

    pub fn open_window_count(&self) -> u32 {
        self.ffi_driver.open_window_count()
    }

    #[must_use]
    pub fn toplevel_count(&self) -> usize {
        self.state.toplevel_count()
    }

    pub fn get_control_binding(&self, id: NodeId) -> Option<&WinUiControlBinding> {
        self.ffi_driver.bindings.get(&id)
    }

    pub const fn scene_state(&self) -> &NativeSceneState {
        &self.state
    }

    pub fn control_count(&self) -> usize {
        self.state.node_count()
    }

    pub fn get_control(&self, id: NodeId) -> Option<&NativeSceneNode> {
        self.state.get_node(id)
    }

    pub fn queue_native_event(&self, event: NativeEvent) {
        self.state.push_event(event);
    }

    pub fn step_native_events(&self) -> Vec<NativeEvent> {
        self.state.drain_events()
    }

    fn map_winui_tag(tag: &str) -> &'static str {
        match tag {
            "ng-window" | "adw-window" => "Microsoft.UI.Xaml.Window",
            "ng-dialog" | "gtk-dialog" | "adw-dialog" => "Microsoft.UI.Xaml.Controls.ContentDialog",
            "ng-header-bar" | "adw-header-bar" | "gtk-header-bar" => {
                "Microsoft.UI.Xaml.Controls.TitleBar"
            }
            "ng-scroll" | "gtk-scrolled-window" => "Microsoft.UI.Xaml.Controls.ScrollViewer",
            "ng-box" | "gtk-box" => "Microsoft.UI.Xaml.Controls.StackPanel",
            "ng-grid" | "gtk-grid" | "ng-clamp" | "adw-clamp" => "Microsoft.UI.Xaml.Controls.Grid",
            "ng-separator" | "gtk-separator" => "Microsoft.UI.Xaml.Controls.AppBarSeparator",
            "ng-label" | "gtk-label" => "Microsoft.UI.Xaml.Controls.TextBlock",
            "ng-button" | "gtk-button" => "Microsoft.UI.Xaml.Controls.Button",
            "ng-entry" | "gtk-entry" => "Microsoft.UI.Xaml.Controls.TextBox",
            "ng-switch" | "gtk-switch" => "Microsoft.UI.Xaml.Controls.ToggleSwitch",
            "ng-checkbox" | "ng-check" | "gtk-check-button" => {
                "Microsoft.UI.Xaml.Controls.CheckBox"
            }
            "ng-slider" | "ng-scale" | "gtk-scale" => "Microsoft.UI.Xaml.Controls.Slider",
            "ng-select" | "ng-dropdown" | "gtk-drop-down" => "Microsoft.UI.Xaml.Controls.ComboBox",
            "ng-image" | "gtk-picture" | "ng-picture" => "Microsoft.UI.Xaml.Controls.Image",
            // WinUI has no read-only meter control: both map to ProgressBar.
            "ng-level-bar" | "gtk-level-bar" | "ng-progress-bar" | "gtk-progress-bar" => {
                "Microsoft.UI.Xaml.Controls.ProgressBar"
            }
            "ng-spinner" | "gtk-spinner" => "Microsoft.UI.Xaml.Controls.ProgressRing",
            "ng-flow" | "ng-flow-box" | "gtk-flow-box" => {
                "Microsoft.UI.Xaml.Controls.ItemsRepeater"
            }
            "ng-paned" | "gtk-paned" => "Microsoft.UI.Xaml.Controls.SplitView",
            "ng-stack" | "gtk-stack" | "adw-navigation-view" => {
                "Microsoft.UI.Xaml.Controls.NavigationView"
            }
            "ng-expander" | "gtk-expander" => "Microsoft.UI.Xaml.Controls.Expander",
            "ng-text-view" | "gtk-text-view" => "Microsoft.UI.Xaml.Controls.RichEditBox",
            "ng-password-entry" | "gtk-password-entry" => "Microsoft.UI.Xaml.Controls.PasswordBox",
            "ng-search-entry" | "gtk-search-entry" => "Microsoft.UI.Xaml.Controls.AutoSuggestBox",
            "ng-spin-button" | "gtk-spin-button" => "Microsoft.UI.Xaml.Controls.NumberBox",
            _ => "Microsoft.UI.Xaml.UIElement",
        }
    }
}

impl PlatformBackend for WinUiBackend {
    fn backend_name(&self) -> &str {
        "Windows WinUI 3"
    }

    fn map_native_widget(&self, tag: &str) -> &'static str {
        Self::map_winui_tag(tag)
    }

    fn flush_commands(&mut self, commands: &[RenderCommand]) {
        if commands.is_empty() {
            return;
        }
        self.flushed_frames += 1;
        super::trace_flush_commands(
            self.backend_name(),
            self.flushed_frames,
            commands,
            Self::map_winui_tag,
        );
        for cmd in commands {
            match cmd {
                RenderCommand::CreateWidget { id, tag } => {
                    self.ffi_driver.register_node(*id, Self::map_winui_tag(tag));
                }
                RenderCommand::SetAttribute { node, key, value } if key == "event-mask" => {
                    self.ffi_driver
                        .update_event_mask(*node, EventMask::parse_attr(value));
                }
                RenderCommand::SetAttribute { node, key, value }
                    if key == "prevent-close" && NativeSceneState::parse_truthy_attr(value) =>
                {
                    self.ffi_driver
                        .update_event_mask(*node, EventMask::CLOSE_REQUEST);
                }
                RenderCommand::DestroyWidget { id } => {
                    self.ffi_driver.unregister_node(*id);
                }
                _ => {}
            }
            self.state
                .apply_command(cmd, "Microsoft.UI.Xaml.AnchorComment", Self::map_winui_tag);
        }
    }
}
