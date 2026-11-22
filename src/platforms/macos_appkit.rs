#![allow(unsafe_code)]

use super::PlatformBackend;
use super::scene_state::{NativeSceneNode, NativeSceneState};
use crate::engine::{EventMask, NativeEvent, NodeId, RenderCommand};
#[cfg(target_os = "macos")]
use crate::ffi_dl::DynLib;
use std::collections::{HashMap, HashSet};
#[cfg(target_os = "macos")]
use std::ffi::{CString, c_char, c_void};

/// Objective-C runtime binding metadata recorded per live `NSView` / `NSWindow` node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppKitWidgetBinding {
    pub objc_class: &'static str,
    pub class_resolved: bool,
    pub primary_selector: &'static str,
    pub selector_resolved: bool,
    pub event_mask: EventMask,
    pub event_selectors: Vec<&'static str>,
}

#[cfg(target_os = "macos")]
type ObjcGetClassFn = unsafe extern "C" fn(*const c_char) -> *mut c_void;
#[cfg(target_os = "macos")]
type SelRegisterNameFn = unsafe extern "C" fn(*const c_char) -> *mut c_void;

struct AppKitFfiDriver {
    ffi_loaded: bool,
    resolved_classes: HashSet<&'static str>,
    resolved_selectors: HashSet<&'static str>,
    bindings: HashMap<NodeId, AppKitWidgetBinding>,
    #[cfg(target_os = "macos")]
    objc_get_class: Option<ObjcGetClassFn>,
    #[cfg(target_os = "macos")]
    sel_register_name: Option<SelRegisterNameFn>,
}

impl AppKitFfiDriver {
    fn new() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self::init_macos_driver()
        }
        #[cfg(not(target_os = "macos"))]
        {
            Self {
                ffi_loaded: false,
                resolved_classes: HashSet::new(),
                resolved_selectors: HashSet::new(),
                bindings: HashMap::new(),
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn init_macos_driver() -> Self {
        let objc_lib = unsafe {
            DynLib::open(c"/usr/lib/libobjc.A.dylib").or_else(|| DynLib::open(c"libobjc.A.dylib"))
        };
        let appkit_loaded = unsafe {
            DynLib::open(c"/System/Library/Frameworks/AppKit.framework/AppKit").is_some()
        };

        let Some(objc) = objc_lib else {
            return Self {
                ffi_loaded: false,
                resolved_classes: HashSet::new(),
                resolved_selectors: HashSet::new(),
                bindings: HashMap::new(),
                objc_get_class: None,
                sel_register_name: None,
            };
        };

        let objc_get_class: Option<ObjcGetClassFn> = unsafe { objc.sym(c"objc_getClass") };
        let sel_register_name: Option<SelRegisterNameFn> = unsafe { objc.sym(c"sel_registerName") };
        let msg_send_ok = unsafe { objc.sym::<*mut c_void>(c"objc_msgSend").is_some() };
        let ffi_loaded =
            appkit_loaded && objc_get_class.is_some() && sel_register_name.is_some() && msg_send_ok;

        let mut driver = Self {
            ffi_loaded,
            resolved_classes: HashSet::new(),
            resolved_selectors: HashSet::new(),
            bindings: HashMap::new(),
            objc_get_class,
            sel_register_name,
        };
        if ffi_loaded {
            for class_name in [
                "NSWindow",
                "NSToolbar",
                "NSScrollView",
                "NSStackView",
                "NSGridView",
                "NSTextField",
                "NSButton",
                "NSSwitch",
                "NSSlider",
                "NSPopUpButton",
                "NSImageView",
                "NSProgressIndicator",
                "NSCollectionView",
                "NSSplitView",
                "NSTabView",
                "NSTextView",
                "NSSecureTextField",
                "NSSearchField",
                "NSStepper",
                "NSBox",
                "NSView",
            ] {
                driver.probe_class(class_name);
                driver.probe_selector(Self::primary_selector_for(class_name));
            }
        }
        driver
    }

    const fn runtime_objc_class(mapped_class: &'static str) -> &'static str {
        match mapped_class.as_bytes() {
            b"NSTextFieldEditable" => "NSTextField",
            b"NSButtonCheckbox" | b"NSDisclosureButton" => "NSButton",
            b"NSBoxSeparator" => "NSBox",
            _ => mapped_class,
        }
    }

    const fn primary_selector_for(objc_class: &'static str) -> &'static str {
        match objc_class.as_bytes() {
            b"NSWindow" => "initWithContentRect:styleMask:backing:defer:",
            b"NSButton" => "setTitle:",
            b"NSTextField" | b"NSSecureTextField" | b"NSSearchField" => "setStringValue:",
            b"NSStackView" => "addArrangedSubview:",
            b"NSScrollView" => "setDocumentView:",
            _ => "init",
        }
    }

    #[cfg_attr(
        not(target_os = "macos"),
        allow(clippy::unused_self, clippy::needless_pass_by_ref_mut)
    )]
    fn probe_class(&mut self, objc_class: &'static str) -> bool {
        #[cfg(target_os = "macos")]
        {
            if self.resolved_classes.contains(objc_class) {
                return true;
            }
            let Some(get_class) = self.objc_get_class else {
                return false;
            };
            let Ok(c_name) = CString::new(objc_class) else {
                return false;
            };
            let ptr = unsafe { get_class(c_name.as_ptr()) };
            if !ptr.is_null() {
                self.resolved_classes.insert(objc_class);
                return true;
            }
            false
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = objc_class;
            false
        }
    }

    #[cfg_attr(
        not(target_os = "macos"),
        allow(clippy::unused_self, clippy::needless_pass_by_ref_mut)
    )]
    fn probe_selector(&mut self, selector: &'static str) -> bool {
        #[cfg(target_os = "macos")]
        {
            if self.resolved_selectors.contains(selector) {
                return true;
            }
            let Some(register_sel) = self.sel_register_name else {
                return false;
            };
            let Ok(c_name) = CString::new(selector) else {
                return false;
            };
            let ptr = unsafe { register_sel(c_name.as_ptr()) };
            if !ptr.is_null() {
                self.resolved_selectors.insert(selector);
                return true;
            }
            false
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = selector;
            false
        }
    }

    fn selectors_for_mask(mask: EventMask) -> Vec<&'static str> {
        const TABLE: &[(EventMask, &str)] = &[
            (EventMask::CLICK, "mouseUp:"),
            (EventMask::CONTEXT_MENU, "rightMouseDown:"),
            (EventMask::POINTER_MOTION, "mouseMoved:"),
            (EventMask::FOCUS, "becomeFirstResponder"),
            (EventMask::KEYBOARD, "keyDown:"),
            (EventMask::ACTIVATE, "insertNewline:"),
            (EventMask::VALUE_CHANGE, "controlTextDidChange:"),
            (EventMask::SCROLL, "reflectScrolledClipView:"),
            (EventMask::WHEEL, "scrollWheel:"),
            (EventMask::WINDOW_STATE, "windowDidResize:"),
            (EventMask::CLOSE_REQUEST, "windowShouldClose:"),
            (EventMask::DRAG_DROP, "performDragOperation:"),
        ];
        TABLE
            .iter()
            .filter_map(|&(bit, sel)| mask.contains(bit).then_some(sel))
            .collect()
    }

    fn register_node(&mut self, id: NodeId, mapped_tag: &'static str) {
        let objc_class = Self::runtime_objc_class(mapped_tag);
        let primary_selector = Self::primary_selector_for(objc_class);
        let class_resolved = self.probe_class(objc_class);
        let selector_resolved = self.probe_selector(primary_selector);
        self.bindings.insert(
            id,
            AppKitWidgetBinding {
                objc_class,
                class_resolved,
                primary_selector,
                selector_resolved,
                event_mask: EventMask::NONE,
                event_selectors: Vec::new(),
            },
        );
    }

    fn update_event_mask(&mut self, id: NodeId, mask: EventMask) {
        let Some(next_mask) = self
            .bindings
            .get(&id)
            .map(|binding| binding.event_mask.union(mask))
        else {
            return;
        };
        let selectors = Self::selectors_for_mask(next_mask);
        for sel in &selectors {
            let _ = self.probe_selector(sel);
        }
        if let Some(binding) = self.bindings.get_mut(&id) {
            binding.event_mask = next_mask;
            binding.event_selectors = selectors;
        }
    }

    fn unregister_node(&mut self, id: NodeId) {
        self.bindings.remove(&id);
    }

    fn open_window_count(&self) -> u32 {
        let count = self
            .bindings
            .values()
            .filter(|b| b.objc_class == "NSWindow")
            .count();
        u32::try_from(count).unwrap_or(u32::MAX)
    }
}

/// Concrete macOS AppKit (Cocoa) platform adapter.
pub struct MacOsAppKitBackend {
    pub flushed_frames: u64,
    state: NativeSceneState,
    ffi_driver: AppKitFfiDriver,
}

impl Default for MacOsAppKitBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MacOsAppKitBackend {
    pub fn new() -> Self {
        Self {
            flushed_frames: 0,
            state: NativeSceneState::default(),
            ffi_driver: AppKitFfiDriver::new(),
        }
    }

    pub const fn is_ffi_loaded(&self) -> bool {
        self.ffi_driver.ffi_loaded
    }

    pub fn resolved_class_count(&self) -> usize {
        self.ffi_driver.resolved_classes.len()
    }

    pub fn resolved_selector_count(&self) -> usize {
        self.ffi_driver.resolved_selectors.len()
    }

    pub fn open_window_count(&self) -> u32 {
        self.ffi_driver.open_window_count()
    }

    pub fn get_view_binding(&self, id: NodeId) -> Option<&AppKitWidgetBinding> {
        self.ffi_driver.bindings.get(&id)
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
            match cmd {
                RenderCommand::CreateWidget { id, tag } => {
                    self.ffi_driver
                        .register_node(*id, Self::map_appkit_tag(tag));
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
                .apply_command(cmd, "NSAnchorComment", Self::map_appkit_tag);
        }
    }
}
