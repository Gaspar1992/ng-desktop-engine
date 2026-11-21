use super::PlatformBackend;
use super::scene_state::{NativeSceneNode, NativeSceneState};
use crate::engine::{NativeEvent, NodeId, RenderCommand};

/// Concrete macOS AppKit (Cocoa) platform adapter.
#[derive(Default)]
pub struct MacOsAppKitBackend {
    pub flushed_frames: u64,
    state: NativeSceneState,
}

impl MacOsAppKitBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub const fn scene_state(&self) -> &NativeSceneState {
        &self.state
    }

    pub fn view_count(&self) -> usize {
        self.state.node_count()
    }

    pub fn get_view(&self, id: NodeId) -> Option<&NativeSceneNode> {
        self.state.get_node(id)
    }

    pub fn queue_native_event(&self, event: NativeEvent) {
        self.state.push_event(event);
    }

    pub fn step_native_events(&self) -> Vec<NativeEvent> {
        self.state.drain_events()
    }

    fn map_appkit_tag(tag: &str) -> &'static str {
        match tag {
            "ng-window" | "adw-window" => "NSWindow",
            "ng-header-bar" | "adw-header-bar" | "gtk-header-bar" => "NSToolbar",
            "ng-scroll" | "gtk-scrolled-window" => "NSScrollView",
            "ng-box" | "gtk-box" => "NSStackView",
            "ng-grid" | "gtk-grid" => "NSGridView",
            "ng-separator" | "gtk-separator" => "NSBoxSeparator",
            "ng-label" | "gtk-label" => "NSTextField",
            "ng-button" | "gtk-button" => "NSButton",
            "ng-entry" | "gtk-entry" => "NSTextFieldEditable",
            "ng-switch" | "gtk-switch" => "NSSwitch",
            "ng-checkbox" | "ng-check" | "gtk-check-button" => "NSButtonCheckbox",
            "ng-slider" | "ng-scale" | "gtk-scale" => "NSSlider",
            "ng-select" | "ng-dropdown" | "gtk-drop-down" => "NSPopUpButton",
            "ng-image" | "gtk-picture" | "ng-picture" => "NSImageView",
            "ng-level-bar" | "gtk-level-bar" => "NSLevelIndicator",
            "ng-spinner" | "gtk-spinner" | "ng-progress-bar" | "gtk-progress-bar" => {
                "NSProgressIndicator"
            }
            "ng-flow" | "ng-flow-box" | "gtk-flow-box" => "NSCollectionView",
            "ng-paned" | "gtk-paned" => "NSSplitView",
            "ng-stack" | "gtk-stack" | "adw-navigation-view" => "NSTabView",
            "ng-expander" | "gtk-expander" => "NSDisclosureButton",
            "ng-text-view" | "gtk-text-view" => "NSTextView",
            "ng-password-entry" | "gtk-password-entry" => "NSSecureTextField",
            "ng-search-entry" | "gtk-search-entry" => "NSSearchField",
            "ng-spin-button" | "gtk-spin-button" => "NSStepper",
            "ng-clamp" | "adw-clamp" => "NSBox",
            _ => "NSView",
        }
    }
}

impl PlatformBackend for MacOsAppKitBackend {
    fn backend_name(&self) -> &str {
        "macOS AppKit (Cocoa)"
    }

    fn map_native_widget(&self, tag: &str) -> &'static str {
        Self::map_appkit_tag(tag)
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
            Self::map_appkit_tag,
        );
        for cmd in commands {
            self.state
                .apply_command(cmd, "NSAnchorComment", Self::map_appkit_tag);
        }
    }
}
