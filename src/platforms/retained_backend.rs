use super::layout_solver::{LayoutRect, solve_scene_layout};
use super::scene_state::NativeSceneState;
use crate::engine::{EventMask, NativeEvent, NodeId, RenderCommand, coalesce_frame_events};
use std::cell::Cell;
use std::collections::HashMap;

/// Platform-specific native driver plugged into [`RetainedSceneBackend`].
///
/// Separates OS handle realization, live event pumping, and layout/attribute application
/// (`WinUiFfiDriver`, `AppKitFfiDriver`) from the shared retained scene-graph lifecycle.
pub(super) trait RetainedNativeDriver {
    fn is_native_display_active(&self) -> bool;
    fn is_ffi_loaded(&self) -> bool;
    fn open_window_count(&self) -> u32;
    fn live_handle_count(&self) -> usize;
    fn root_default_sizes(&self) -> Vec<(NodeId, i32, i32)>;
    fn register_node(&mut self, id: NodeId, mapped_tag: &'static str);
    fn update_event_mask(&mut self, id: NodeId, mask: EventMask);
    fn unregister_node(&mut self, id: NodeId);
    fn pump_live_events(&self) -> Vec<NativeEvent>;
    fn apply_live_command(&mut self, cmd: &RenderCommand, state: &NativeSceneState);
    fn apply_live_layout(
        &self,
        state: &NativeSceneState,
        layout_rects: &HashMap<NodeId, LayoutRect>,
    );
    fn shutdown(&mut self);
}

/// Shared Bridge/Template-Method engine for retained native OS backends (`Windows WinUI`, `macOS AppKit`).
pub(super) struct RetainedSceneBackend<D: RetainedNativeDriver> {
    pub flushed_frames: u64,
    pub(super) state: NativeSceneState,
    pub(super) layout_rects: HashMap<NodeId, LayoutRect>,
    pub(super) layout_dirty: Cell<bool>,
    pub(super) driver: D,
}

impl<D: RetainedNativeDriver> RetainedSceneBackend<D> {
    pub(super) fn new(driver: D) -> Self {
        Self {
            flushed_frames: 0,
            state: NativeSceneState::default(),
            layout_rects: HashMap::new(),
            layout_dirty: Cell::new(false),
            driver,
        }
    }

    pub(super) fn is_ffi_loaded(&self) -> bool {
        self.driver.is_ffi_loaded()
    }

    pub(super) fn open_window_count(&self) -> u32 {
        self.driver.open_window_count()
    }

    pub(super) fn recompute_layout(&mut self) {
        self.layout_rects.clear();
        for (root_id, default_w, default_h) in self.driver.root_default_sizes() {
            let (win_w, win_h) = self.state.window_size_or(root_id, default_w, default_h);
            let solved = solve_scene_layout(&self.state, root_id, win_w, win_h);
            self.layout_rects.extend(solved);
        }
    }

    pub(super) fn step_native_events(&self) -> Vec<NativeEvent> {
        let mut merged = self.driver.pump_live_events();
        merged.retain(|ev| self.state.get_node(ev.node).is_some());
        merged.extend(self.state.drain_events());
        let coalesced = coalesce_frame_events(merged);
        self.state.record_window_resize_events(&coalesced);
        if coalesced.iter().any(|ev| ev.event_name == "resize") {
            self.layout_dirty.set(true);
        }
        coalesced
    }

    pub(super) fn flush_commands(
        &mut self,
        backend_name: &str,
        commands: &[RenderCommand],
        anchor_tag: &'static str,
        map_tag: fn(&str) -> &'static str,
    ) {
        let layout_dirty = self.layout_dirty.replace(false);
        if commands.is_empty() && !layout_dirty {
            return;
        }
        if !commands.is_empty() {
            self.flushed_frames += 1;
            super::trace_flush_commands(backend_name, self.flushed_frames, commands, map_tag);
            for cmd in commands {
                self.state.apply_command(cmd, anchor_tag, map_tag);
                match cmd {
                    RenderCommand::CreateWidget { id, tag } => {
                        self.driver.register_node(*id, map_tag(tag));
                    }
                    RenderCommand::SetAttribute { node, key, value } if key == "event-mask" => {
                        self.driver
                            .update_event_mask(*node, EventMask::parse_attr(value));
                    }
                    RenderCommand::SetAttribute { node, key, value }
                        if key == "prevent-close" && NativeSceneState::parse_truthy_attr(value) =>
                    {
                        self.driver
                            .update_event_mask(*node, EventMask::CLOSE_REQUEST);
                    }
                    RenderCommand::DestroyWidget { id } => {
                        self.driver.unregister_node(*id);
                    }
                    _ => {}
                }
                self.driver.apply_live_command(cmd, &self.state);
            }
        }
        self.recompute_layout();
        self.driver
            .apply_live_layout(&self.state, &self.layout_rects);
    }
}

impl<D: RetainedNativeDriver> Drop for RetainedSceneBackend<D> {
    fn drop(&mut self) {
        self.driver.shutdown();
    }
}
