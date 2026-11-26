#![allow(unsafe_code)]

use super::PlatformBackend;
use super::layout_solver::LayoutRect;
#[cfg(target_os = "windows")]
use super::layout_solver::compute_parent_relative_bounds;
use super::retained_backend::{RetainedNativeDriver, RetainedSceneBackend};
use super::scene_state::{NativeSceneNode, NativeSceneState};
#[cfg(target_os = "windows")]
use super::win32_sys::{
    Win32Handle, Win32Sys, is_headless_env, resolve_dwm_backdrop_type,
    resolve_dwm_corner_preference,
};
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
    fn apply_live_layout(
        &self,
        state: &NativeSceneState,
        layout_rects: &HashMap<NodeId, LayoutRect>,
    ) {
        let Some(win32) = &self.win32 else {
            return;
        };
        let rel_rects = compute_parent_relative_bounds(state, layout_rects, false);
        for (&node_id, rect) in &rel_rects {
            if self.live_toplevels.contains(&node_id) {
                continue;
            }
            if let Some(&handle) = self.live_handles.get(&node_id) {
                win32.set_bounds(handle, rect.x, rect.y, rect.width, rect.height);
            }
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
                        "style" if self.live_toplevels.contains(node) => {
                            if value.to_ascii_lowercase().contains("border-radius")
                                && let Some(win32) = &self.win32
                            {
                                win32.apply_dwm_composition_attrs(
                                    h,
                                    Some(resolve_dwm_corner_preference(value)),
                                    None,
                                );
                            }
                        }
                        "backdrop" | "window-backdrop" if self.live_toplevels.contains(node) => {
                            if let Some(win32) = &self.win32 {
                                win32.apply_dwm_composition_attrs(
                                    h,
                                    None,
                                    Some(resolve_dwm_backdrop_type(value)),
                                );
                            }
                        }
                        "theme" | "dark-mode" if self.live_toplevels.contains(node) => {
                            if let Some(win32) = &self.win32 {
                                let dark = value.eq_ignore_ascii_case("dark")
                                    || NativeSceneState::parse_truthy_attr(value);
                                win32.apply_dwm_theme(h, dark);
                            }
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
            RenderCommand::RemoveChild { child, .. } => {
                if !self.live_toplevels.contains(child)
                    && let (Some(win32), Some(&h)) = (&self.win32, self.live_handles.get(child))
                {
                    win32.set_visible(h, false);
                }
            }
            RenderCommand::SetTheme { theme } => {
                if let Some(win32) = &self.win32 {
                    let dark = matches!(theme, crate::desktop::DesktopTheme::Dark);
                    for id in &self.live_toplevels {
                        if let Some(&h) = self.live_handles.get(id) {
                            win32.apply_dwm_theme(h, dark);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

impl RetainedNativeDriver for WinUiFfiDriver {
    fn is_native_display_active(&self) -> bool {
        #[cfg(target_os = "windows")]
        {
            self.win32.is_some()
        }
        #[cfg(not(target_os = "windows"))]
        {
            false
        }
    }

    fn is_ffi_loaded(&self) -> bool {
        self.ffi_loaded
    }

    fn open_window_count(&self) -> u32 {
        Self::open_window_count(self)
    }

    fn live_handle_count(&self) -> usize {
        #[cfg(target_os = "windows")]
        {
            self.live_handles.len()
        }
        #[cfg(not(target_os = "windows"))]
        {
            0
        }
    }

    fn root_default_sizes(&self) -> Vec<(NodeId, i32, i32)> {
        self.bindings
            .iter()
            .filter_map(|(&id, binding)| match binding.win32_class {
                "Window" => Some((id, 1020, 720)),
                "Dialog" => Some((id, 460, 280)),
                _ => None,
            })
            .collect()
    }

    fn register_node(&mut self, id: NodeId, mapped_tag: &'static str) {
        Self::register_node(self, id, mapped_tag);
    }

    fn update_event_mask(&mut self, id: NodeId, mask: EventMask) {
        Self::update_event_mask(self, id, mask);
    }

    fn unregister_node(&mut self, id: NodeId) {
        Self::unregister_node(self, id);
    }

    fn pump_live_events(&self) -> Vec<NativeEvent> {
        #[cfg(target_os = "windows")]
        if let Some(win32) = &self.win32 {
            return win32.pump_messages();
        }
        Vec::new()
    }

    fn apply_live_command(&mut self, cmd: &RenderCommand, state: &NativeSceneState) {
        #[cfg(target_os = "windows")]
        Self::apply_live_command(self, cmd, state);
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (cmd, state);
        }
    }

    fn apply_live_layout(
        &self,
        state: &NativeSceneState,
        layout_rects: &HashMap<NodeId, LayoutRect>,
    ) {
        #[cfg(target_os = "windows")]
        Self::apply_live_layout(self, state, layout_rects);
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (state, layout_rects);
        }
    }

    fn shutdown(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let toplevels: Vec<NodeId> = self.live_toplevels.drain().collect();
            for id in toplevels {
                if let Some(handle) = self.live_handles.remove(&id)
                    && let Some(win32) = &self.win32
                {
                    win32.destroy_window(handle);
                }
            }
            self.live_handles.clear();
        }
    }
}

/// Concrete Windows WinUI 3 (`Microsoft.UI.Xaml`) + Win32 `user32.dll` platform adapter.
pub struct WinUiBackend {
    pub flushed_frames: u64,
    inner: RetainedSceneBackend<WinUiFfiDriver>,
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
            inner: RetainedSceneBackend::new(WinUiFfiDriver::new(false)),
        }
    }

    pub fn new_headless() -> Self {
        Self {
            flushed_frames: 0,
            inner: RetainedSceneBackend::new(WinUiFfiDriver::new(true)),
        }
    }

    #[must_use]
    pub fn is_native_display_active(&self) -> bool {
        self.inner.driver.is_native_display_active()
    }

    #[must_use]
    pub fn live_handle_count(&self) -> usize {
        self.inner.driver.live_handle_count()
    }

    pub const fn is_ffi_loaded(&self) -> bool {
        self.inner.driver.ffi_loaded
    }

    pub fn resolved_symbol_count(&self) -> usize {
        self.inner.driver.resolved_symbols.len()
    }

    pub fn open_window_count(&self) -> u32 {
        self.inner.driver.open_window_count()
    }

    #[must_use]
    pub fn toplevel_count(&self) -> usize {
        self.inner.state.toplevel_count()
    }

    pub fn get_control_binding(&self, id: NodeId) -> Option<&WinUiControlBinding> {
        self.inner.driver.bindings.get(&id)
    }

    pub const fn scene_state(&self) -> &NativeSceneState {
        &self.inner.state
    }

    pub fn control_count(&self) -> usize {
        self.inner.state.node_count()
    }

    pub fn get_control(&self, id: NodeId) -> Option<&NativeSceneNode> {
        self.inner.state.get_node(id)
    }

    #[must_use]
    pub fn get_layout_rect(&self, id: NodeId) -> Option<LayoutRect> {
        self.inner.layout_rects.get(&id).copied()
    }

    pub fn queue_native_event(&self, event: NativeEvent) {
        self.inner.state.push_event(event);
    }

    pub fn step_native_events(&self) -> Vec<NativeEvent> {
        self.inner.step_native_events()
    }

    fn map_winui_tag(tag: &str) -> &'static str {
        match super::WidgetTag::from_tag(tag) {
            super::WidgetTag::Window => "Microsoft.UI.Xaml.Window",
            super::WidgetTag::Dialog => "Microsoft.UI.Xaml.Controls.ContentDialog",
            super::WidgetTag::HeaderBar => "Microsoft.UI.Xaml.Controls.TitleBar",
            super::WidgetTag::Scroll => "Microsoft.UI.Xaml.Controls.ScrollViewer",
            super::WidgetTag::Box => "Microsoft.UI.Xaml.Controls.StackPanel",
            super::WidgetTag::Grid | super::WidgetTag::Clamp => "Microsoft.UI.Xaml.Controls.Grid",
            super::WidgetTag::Separator => "Microsoft.UI.Xaml.Controls.AppBarSeparator",
            super::WidgetTag::Label => "Microsoft.UI.Xaml.Controls.TextBlock",
            super::WidgetTag::Button => "Microsoft.UI.Xaml.Controls.Button",
            super::WidgetTag::Entry => "Microsoft.UI.Xaml.Controls.TextBox",
            super::WidgetTag::Switch => "Microsoft.UI.Xaml.Controls.ToggleSwitch",
            super::WidgetTag::CheckButton => "Microsoft.UI.Xaml.Controls.CheckBox",
            super::WidgetTag::Slider => "Microsoft.UI.Xaml.Controls.Slider",
            super::WidgetTag::DropDown => "Microsoft.UI.Xaml.Controls.ComboBox",
            super::WidgetTag::Image => "Microsoft.UI.Xaml.Controls.Image",
            // WinUI has no read-only meter control: both map to ProgressBar.
            super::WidgetTag::LevelBar | super::WidgetTag::ProgressBar => {
                "Microsoft.UI.Xaml.Controls.ProgressBar"
            }
            super::WidgetTag::Spinner => "Microsoft.UI.Xaml.Controls.ProgressRing",
            super::WidgetTag::FlowBox => "Microsoft.UI.Xaml.Controls.ItemsRepeater",
            super::WidgetTag::Paned => "Microsoft.UI.Xaml.Controls.SplitView",
            super::WidgetTag::Stack => "Microsoft.UI.Xaml.Controls.NavigationView",
            super::WidgetTag::Expander => "Microsoft.UI.Xaml.Controls.Expander",
            super::WidgetTag::TextView => "Microsoft.UI.Xaml.Controls.RichEditBox",
            super::WidgetTag::PasswordEntry => "Microsoft.UI.Xaml.Controls.PasswordBox",
            super::WidgetTag::SearchEntry => "Microsoft.UI.Xaml.Controls.AutoSuggestBox",
            super::WidgetTag::SpinButton => "Microsoft.UI.Xaml.Controls.NumberBox",
            super::WidgetTag::ListView => "Microsoft.UI.Xaml.Controls.ListView",
            super::WidgetTag::Popover => "Microsoft.UI.Xaml.Controls.Flyout",
            super::WidgetTag::Anchor => "Microsoft.UI.Xaml.AnchorComment",
            super::WidgetTag::Unknown => "Microsoft.UI.Xaml.UIElement",
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
        self.inner.flush_commands(
            "Windows WinUI 3",
            commands,
            "Microsoft.UI.Xaml.AnchorComment",
            Self::map_winui_tag,
        );
        self.flushed_frames = self.inner.flushed_frames;
    }

    fn is_native_display_active(&self) -> bool {
        Self::is_native_display_active(self)
    }

    fn open_window_count(&self) -> u32 {
        self.inner.open_window_count()
    }

    fn is_ffi_driver_loaded(&self) -> bool {
        self.inner.is_ffi_loaded()
    }

    fn step_native_events(&self, _may_block: bool) -> Vec<NativeEvent> {
        Self::step_native_events(self)
    }

    fn gpu_telemetry_summary(&self) -> String {
        let pref = super::GpuRendererPreference::from_env();
        let accel = pref.prefers_hardware_acceleration();
        let compositor = if accel {
            "DWM/DirectComposition"
        } else {
            "Software"
        };
        format!(
            "preference={} compositor={} hardware_accel={}",
            pref.as_str(),
            compositor,
            accel
        )
    }
}
