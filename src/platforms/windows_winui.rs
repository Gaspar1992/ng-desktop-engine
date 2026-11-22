#![allow(unsafe_code)]

use super::PlatformBackend;
use super::scene_state::{NativeSceneNode, NativeSceneState};
#[cfg(target_os = "windows")]
use super::win32_sys::{Win32Handle, Win32Sys, is_headless_env};
#[cfg(target_os = "windows")]
use crate::engine::coalesce_frame_events;
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
    #[cfg(target_os = "windows")]
    win32: Option<Win32Sys>,
    #[cfg(target_os = "windows")]
    live_handles: HashMap<NodeId, Win32Handle>,
    #[cfg(target_os = "windows")]
    live_toplevels: HashSet<NodeId>,
}

impl WinUiFfiDriver {
    fn new(headless: bool) -> Self {
        #[cfg(target_os = "windows")]
        {
            Self::init_windows_driver(headless || is_headless_env())
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = headless;
            Self {
                ffi_loaded: false,
                resolved_symbols: HashSet::new(),
                bindings: HashMap::new(),
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn init_windows_driver(headless: bool) -> Self {
        let Some(user32) = (unsafe { DynLib::open(c"user32.dll") }) else {
            return Self {
                ffi_loaded: false,
                resolved_symbols: HashSet::new(),
                bindings: HashMap::new(),
                win32: None,
                live_handles: HashMap::new(),
                live_toplevels: HashSet::new(),
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
            ("PeekMessageW", c"PeekMessageW"),
            ("GetMessageW", c"GetMessageW"),
            ("TranslateMessage", c"TranslateMessage"),
            ("DispatchMessageW", c"DispatchMessageW"),
            ("DefWindowProcW", c"DefWindowProcW"),
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
        let win32 = if headless { None } else { Win32Sys::try_load() };

        Self {
            ffi_loaded,
            resolved_symbols,
            bindings: HashMap::new(),
            win32,
            live_handles: HashMap::new(),
            live_toplevels: HashSet::new(),
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
        #[cfg(target_os = "windows")]
        if matches!(win32_class, "Window" | "Dialog")
            && let Some(win32) = &self.win32
            && let Some(handle) = win32.create_toplevel(
                id,
                if win32_class == "Dialog" {
                    "Dialog"
                } else {
                    "Untitled"
                },
                win32_class == "Dialog",
                EventMask::NONE,
                false,
            )
        {
            self.live_handles.insert(id, handle);
            self.live_toplevels.insert(id);
        }
    }

    fn update_event_mask(&mut self, id: NodeId, mask: EventMask) {
        if let Some(binding) = self.bindings.get_mut(&id) {
            binding.event_mask = binding.event_mask.union(mask);
            binding.win32_event_routes = Self::win32_routes_for_mask(binding.event_mask);
        }
        #[cfg(target_os = "windows")]
        if let Some(&handle) = self.live_handles.get(&id) {
            Win32Sys::set_event_mask(handle, mask);
        }
    }

    fn unregister_node(&mut self, id: NodeId) {
        self.bindings.remove(&id);
        #[cfg(target_os = "windows")]
        {
            self.live_toplevels.remove(&id);
            if let Some(handle) = self.live_handles.remove(&id)
                && let Some(win32) = &self.win32
            {
                win32.destroy_window(handle);
                self.live_handles.retain(|_, h| win32.is_window_alive(*h));
                self.live_toplevels
                    .retain(|nid| self.live_handles.contains_key(nid));
            }
        }
    }

    fn open_window_count(&self) -> u32 {
        #[cfg(target_os = "windows")]
        if let Some(win32) = &self.win32 {
            let live_count = self
                .live_toplevels
                .iter()
                .filter_map(|id| self.live_handles.get(id))
                .filter(|&&h| win32.is_window_alive(h))
                .count();
            return u32::try_from(live_count).unwrap_or(u32::MAX);
        }
        let count = self
            .bindings
            .values()
            .filter(|b| b.xaml_class == "Microsoft.UI.Xaml.Window")
            .count();
        u32::try_from(count).unwrap_or(u32::MAX)
    }

    #[cfg(target_os = "windows")]
    fn realize_child_subtree(
        &mut self,
        parent_id: NodeId,
        child_id: NodeId,
        slot_index: usize,
        state: &NativeSceneState,
    ) {
        let (Some(win32), Some(&parent_handle)) = (&self.win32, self.live_handles.get(&parent_id))
        else {
            return;
        };
        if self.live_toplevels.contains(&child_id) {
            return;
        }
        let Some(binding) = self.bindings.get(&child_id).cloned() else {
            return;
        };
        let Some(scene_node) = state.get_node(child_id) else {
            return;
        };
        let handle = if let Some(&existing) = self.live_handles.get(&child_id) {
            existing
        } else if let Some(created) =
            win32.create_child_control(parent_handle, child_id, &binding, &scene_node.text)
        {
            self.live_handles.insert(child_id, created);
            created
        } else {
            return;
        };
        let y = 12 + i32::try_from(slot_index).unwrap_or(0) * 36;
        let width = if binding.win32_class == "STATIC" {
            420
        } else {
            340
        };
        let height = if binding.win32_class == "STATIC" {
            320
        } else {
            30
        };
        win32.set_bounds(handle, 12, y, width, height);
        if binding.xaml_class == "Microsoft.UI.Xaml.Controls.Flyout" {
            let is_open = scene_node
                .attributes
                .get("open")
                .is_some_and(|v| NativeSceneState::parse_truthy_attr(v));
            win32.set_visible(handle, is_open);
        } else if let Some(vis) = scene_node.attributes.get("visible") {
            win32.set_visible(handle, NativeSceneState::parse_truthy_attr(vis));
        }
        if let Some(sens) = scene_node.attributes.get("sensitive") {
            win32.set_enabled(handle, NativeSceneState::parse_truthy_attr(sens));
        }
        for (idx, &grandchild) in scene_node.children.iter().enumerate() {
            self.realize_child_subtree(child_id, grandchild, idx, state);
        }
    }

    #[cfg(target_os = "windows")]
    fn apply_live_command(&mut self, cmd: &RenderCommand, state: &NativeSceneState) {
        match cmd {
            RenderCommand::SetText { id, text } => {
                if let (Some(win32), Some(&h)) = (&self.win32, self.live_handles.get(id)) {
                    win32.set_text(h, text);
                }
            }
            RenderCommand::SetAttribute { node, key, value } => {
                if let Some(&h) = self.live_handles.get(node) {
                    match key.as_str() {
                        "title" | "value" | "text" | "label" => {
                            if let Some(win32) = &self.win32 {
                                win32.set_text(h, value);
                            }
                        }
                        "visible" => {
                            if let Some(win32) = &self.win32 {
                                win32.set_visible(h, NativeSceneState::parse_truthy_attr(value));
                            }
                        }
                        "open" => {
                            if self.bindings.get(node).is_some_and(|b| {
                                b.xaml_class == "Microsoft.UI.Xaml.Controls.Flyout"
                            }) && let Some(win32) = &self.win32
                            {
                                win32.set_visible(h, NativeSceneState::parse_truthy_attr(value));
                            }
                        }
                        "sensitive" => {
                            if let Some(win32) = &self.win32 {
                                win32.set_enabled(h, NativeSceneState::parse_truthy_attr(value));
                            }
                        }
                        "prevent-close" => {
                            Win32Sys::set_prevent_close(
                                h,
                                NativeSceneState::parse_truthy_attr(value),
                            );
                        }
                        "close-window" if NativeSceneState::parse_truthy_attr(value) => {
                            self.unregister_node(*node);
                        }
                        _ => {}
                    }
                }
            }
            RenderCommand::AppendChild { parent, child }
            | RenderCommand::InsertBefore { parent, child, .. } => {
                let slot = state
                    .get_node(*parent)
                    .and_then(|p| p.children.iter().position(|c| c == child))
                    .unwrap_or(0);
                self.realize_child_subtree(*parent, *child, slot, state);
                if self.live_toplevels.contains(parent)
                    && let (Some(win32), Some(&parent_h)) =
                        (&self.win32, self.live_handles.get(parent))
                {
                    let explicit_hidden = state
                        .get_node(*parent)
                        .and_then(|n| n.attributes.get("visible"))
                        .is_some_and(|v| !NativeSceneState::parse_truthy_attr(v));
                    if !explicit_hidden {
                        win32.set_visible(parent_h, true);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Concrete Windows WinUI 3 (`Microsoft.UI.Xaml`) + Win32 `user32.dll` platform adapter.
pub struct WinUiBackend {
    pub flushed_frames: u64,
    state: NativeSceneState,
    ffi_driver: WinUiFfiDriver,
}

impl Drop for WinUiBackend {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let toplevels: Vec<NodeId> = self.ffi_driver.live_toplevels.drain().collect();
            for id in toplevels {
                if let Some(handle) = self.ffi_driver.live_handles.remove(&id)
                    && let Some(win32) = &self.ffi_driver.win32
                {
                    win32.destroy_window(handle);
                }
            }
            self.ffi_driver.live_handles.clear();
        }
    }
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
            ffi_driver: WinUiFfiDriver::new(false),
        }
    }

    pub fn new_headless() -> Self {
        Self {
            flushed_frames: 0,
            state: NativeSceneState::default(),
            ffi_driver: WinUiFfiDriver::new(true),
        }
    }

    #[must_use]
    pub const fn is_native_display_active(&self) -> bool {
        #[cfg(target_os = "windows")]
        {
            self.ffi_driver.win32.is_some()
        }
        #[cfg(not(target_os = "windows"))]
        {
            false
        }
    }

    #[must_use]
    pub fn live_handle_count(&self) -> usize {
        #[cfg(target_os = "windows")]
        {
            self.ffi_driver.live_handles.len()
        }
        #[cfg(not(target_os = "windows"))]
        {
            0
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
        #[cfg(target_os = "windows")]
        if let Some(win32) = &self.ffi_driver.win32 {
            let mut merged = win32.pump_messages();
            merged.retain(|ev| self.state.get_node(ev.node).is_some());
            merged.extend(self.state.drain_events());
            return coalesce_frame_events(merged);
        }
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
            "ng-list" | "ng-table" | "gtk-list-box" | "gtk-list-view" => {
                "Microsoft.UI.Xaml.Controls.ListView"
            }
            "ng-popover" | "gtk-popover" | "ng-menu" => "Microsoft.UI.Xaml.Controls.Flyout",
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
            self.state
                .apply_command(cmd, "Microsoft.UI.Xaml.AnchorComment", Self::map_winui_tag);
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
            #[cfg(target_os = "windows")]
            self.ffi_driver.apply_live_command(cmd, &self.state);
        }
    }
}
