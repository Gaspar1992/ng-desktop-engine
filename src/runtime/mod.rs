//! Embedded JavaScript runtime engines (`JavaScriptCore` JIT and `QuickJS-NG`), `.ngb`
//! bytecode parser, reactive signal/attribute bindings, `HostBridge`, and `NativeServerWorker`.

pub mod bootstrap_js;
pub mod host_bridge;
mod js_bridge;
pub mod js_engine;
pub mod jsc_sys;
pub mod parser;
pub mod quickjs_engine;
mod reactivity;
mod scene_exec;
pub mod server_worker;

use crate::config::AngularNativeConfig;
use crate::desktop::{DesktopServices, DesktopTheme};
use crate::engine::{EngineError, NodeId, RenderEngine};
use crate::platforms::PlatformBackend;
pub use host_bridge::HostBridge;
pub use js_engine::{AnyJsEngine, JsEngineBackend, JscEngine, QuickJsEngine};
pub use parser::{Instruction, parse_instruction, parse_script};
use reactivity::{AttrBinding, HandlerAction, TextBinding, VisibilityBinding};
pub use server_worker::{NativeServerWorker, is_local_api_url};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Embedded JS Bridge Runtime that executes compiled Angular instructions (`.ngb`),
/// hosts a pluggable `JsEngineBackend` (`JavaScriptCore` JIT or `QuickJS`),
/// maintains reactive Signal state, supports state-preserving Hot Reload (HMR),
/// and dispatches native OS events back into Angular handlers.
///
/// Responsibilities are split across submodules:
/// - `js_bridge`: JS evaluation and native event routing.
/// - `reactivity`: Signals, text bindings, and declarative handlers.
/// - `scene_exec`: structural scene graph instructions and handle tables.
pub struct JsRuntime {
    js_engine: AnyJsEngine,
    desktop: DesktopServices,
    host_bridge: Arc<HostBridge>,
    variables: HashMap<String, NodeId>,
    node_vars: HashMap<NodeId, String>,
    primary_window: Option<NodeId>,
    signals: HashMap<String, i64>,
    bindings: Vec<TextBinding>,
    attr_bindings: Vec<AttrBinding>,
    visibility_bindings: Vec<VisibilityBinding>,
    handlers: HashMap<u64, HandlerAction>,
    next_listener_id: u64,
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl JsRuntime {
    pub fn new() -> Self {
        Self::with_engine(AnyJsEngine::default())
    }

    pub fn with_engine(js_engine: AnyJsEngine) -> Self {
        Self::with_engine_and_desktop(js_engine, DesktopServices::new())
    }

    /// Creates a `JsRuntime` binding a specific `AnyJsEngine` and configured `DesktopServices`.
    pub fn with_engine_and_desktop(mut js_engine: AnyJsEngine, desktop: DesktopServices) -> Self {
        js_engine.attach_http_engine(desktop.shared_http_engine());
        let host_bridge = Arc::new(HostBridge::new(&desktop));
        js_engine.attach_host_bridge(Arc::clone(&host_bridge));
        let escaped_app_id = desktop.app_id().replace('\\', "\\\\").replace('\'', "\\'");
        let _ = js_engine.eval_script(&format!(
            "globalThis.__NG_NATIVE_APP_ID__ = '{escaped_app_id}';"
        ));
        Self {
            js_engine,
            desktop,
            host_bridge,
            variables: HashMap::new(),
            node_vars: HashMap::new(),
            primary_window: None,
            signals: HashMap::new(),
            bindings: Vec::new(),
            attr_bindings: Vec::new(),
            visibility_bindings: Vec::new(),
            handlers: HashMap::new(),
            next_listener_id: 1,
        }
    }

    pub fn from_engine_config(engine_kind: &str) -> Result<Self, EngineError> {
        let js_engine = AnyJsEngine::from_config(engine_kind)?;
        Ok(Self::with_engine(js_engine))
    }

    /// Builds a `JsRuntime` directly from workspace/runtime configuration, wiring the app id,
    /// JS engine choice (`jsc` / `quickjs`), and build-time capability manifest into `DesktopServices`.
    pub fn from_config(cfg: &AngularNativeConfig) -> Result<Self, EngineError> {
        let js_engine = AnyJsEngine::from_config(&cfg.js_engine)?;
        let desktop = DesktopServices::with_app_config(&cfg.project_name, cfg.permissions.clone());
        Ok(Self::with_engine_and_desktop(js_engine, desktop))
    }

    pub const fn js_engine(&self) -> &AnyJsEngine {
        &self.js_engine
    }

    pub const fn desktop(&self) -> &DesktopServices {
        &self.desktop
    }

    pub const fn desktop_mut(&mut self) -> &mut DesktopServices {
        &mut self.desktop
    }

    pub const fn host_bridge(&self) -> &Arc<HostBridge> {
        &self.host_bridge
    }

    /// Starts an isolated `JavaScriptCore` backend worker from `bundle_js` and attaches it
    /// to this runtime's `HostBridge` so local `/api/*` HTTP calls execute in-memory.
    pub fn attach_server_bundle(
        &self,
        bundle_js: &str,
        persistent_sqlite: bool,
    ) -> Result<Arc<NativeServerWorker>, String> {
        let worker = Arc::new(NativeServerWorker::start(
            &self.desktop,
            bundle_js,
            persistent_sqlite,
        )?);
        self.host_bridge.attach_server_worker(Arc::clone(&worker));
        Ok(worker)
    }

    /// Hot-reloads the attached isolated `NativeServerWorker` in-place (preserving SQLite state),
    /// or starts a new worker if none was attached yet.
    pub fn reload_server_bundle(
        &self,
        bundle_js: &str,
        persistent_sqlite: bool,
    ) -> Result<(), String> {
        if self.host_bridge.reload_server_worker(bundle_js)? {
            return Ok(());
        }
        self.attach_server_bundle(bundle_js, persistent_sqlite)
            .map(|_| ())
    }

    pub fn signal_value(&self, name: &str) -> Option<i64> {
        self.signals.get(name).copied()
    }

    pub fn eval_tick<B: PlatformBackend>(
        &mut self,
        engine: &mut RenderEngine<B>,
        script: &str,
    ) -> Result<(), EngineError> {
        let instructions = parse_script(script)?;
        let mut newly_created = HashSet::new();
        for inst in instructions {
            self.execute_instruction(engine, inst, false, &mut newly_created)?;
        }

        engine.flush_frame();
        Ok(())
    }

    /// Hot-reloads an updated `.ngb` bundle in-place against the live scene graph,
    /// preserving active Signal values while updating styles, text, and new widgets.
    pub fn hot_reload<B: PlatformBackend>(
        &mut self,
        engine: &mut RenderEngine<B>,
        script: &str,
    ) -> Result<(), EngineError> {
        let instructions = parse_script(script)?;
        self.bindings.clear();
        self.attr_bindings.clear();
        self.visibility_bindings.clear();
        self.handlers.clear();
        engine.clear_listeners();

        let mut newly_created = HashSet::new();
        for inst in instructions {
            self.execute_instruction(engine, inst, true, &mut newly_created)?;
        }

        let active_signals: Vec<(String, i64)> =
            self.signals.iter().map(|(k, v)| (k.clone(), *v)).collect();
        for (sig_name, val) in active_signals {
            self.propagate_signal(engine, &sig_name, val)?;
        }

        engine.flush_frame();
        Ok(())
    }

    fn execute_instruction<B: PlatformBackend>(
        &mut self,
        engine: &mut RenderEngine<B>,
        instruction: Instruction,
        is_hmr: bool,
        newly_created: &mut HashSet<String>,
    ) -> Result<(), EngineError> {
        match instruction {
            Instruction::RegisterStyle(rule) => {
                engine.register_style(rule);
                Ok(())
            }
            Instruction::SetTheme { mode } => {
                let theme = DesktopTheme::from_str_lossy(&mode);
                self.desktop.set_theme(theme);
                engine.set_theme(theme);
                Ok(())
            }
            Instruction::SetClipboard { text } => {
                self.desktop.set_clipboard(&text);
                Ok(())
            }
            Instruction::DeclareSignal { .. }
            | Instruction::BindText { .. }
            | Instruction::BindAttr { .. }
            | Instruction::BindVisible { .. }
            | Instruction::OnAdd { .. }
            | Instruction::OnSet { .. }
            | Instruction::OnText { .. } => {
                self.execute_reactive_instruction(engine, instruction, is_hmr)
            }
            other => self.execute_scene_instruction(engine, other, is_hmr, newly_created),
        }
    }

    pub fn resolve_var(&self, name: &str) -> Result<NodeId, EngineError> {
        self.variables
            .get(name)
            .copied()
            .or_else(|| {
                if name == "win" {
                    self.primary_window
                } else {
                    None
                }
            })
            .ok_or_else(|| EngineError::ScriptSyntaxError(format!("Unknown JS handle: {name}")))
    }

    pub fn resolve_primary_window(&self) -> Result<NodeId, EngineError> {
        self.primary_window
            .or_else(|| self.variables.get("win").copied())
            .ok_or_else(|| {
                EngineError::ScriptSyntaxError(String::from(
                    "No root <ng-window> found in compiled bundle",
                ))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppPermissions;
    use crate::platforms::HeadlessBackend;

    #[test]
    fn configures_runtime_app_id_and_permissions_from_workspace_config() {
        let mut cfg = AngularNativeConfig::default_standalone("invoice-studio");
        cfg.permissions = AppPermissions {
            sqlite: true,
            ..AppPermissions::minimal()
        };

        let mut rt = JsRuntime::from_config(&cfg).unwrap();
        let mut engine = RenderEngine::new(HeadlessBackend::new());
        assert_eq!(rt.desktop().app_id(), "invoice-studio");
        assert!(rt.desktop().check_sqlite_allowed());
        assert!(!rt.desktop().check_network_allowed());
        assert!(!rt.desktop().check_filesystem_xdg_allowed());
        let denied = rt.eval_js(
            &mut engine,
            "__NG_NATIVE__.fsResolvePath('config', 'prefs.json')",
        );
        let err_msg = format!("{:?}", denied.unwrap_err());
        assert!(
            err_msg.contains("Permission denied: filesystem_xdg"),
            "unexpected error: {err_msg}"
        );

        cfg.permissions.filesystem_xdg = true;
        let mut allowed_rt = JsRuntime::from_config(&cfg).unwrap();
        let resolved_path = allowed_rt
            .eval_js(
                &mut engine,
                "__NG_NATIVE__.fsResolvePath('config', 'prefs.json')",
            )
            .unwrap();
        assert!(
            resolved_path
                .replace('\\', "/")
                .ends_with("/invoice-studio/prefs.json"),
            "unexpected resolved XDG path: {resolved_path}"
        );

        cfg.js_engine = String::from("quickjs");
        let mut quickjs_rt = JsRuntime::from_config(&cfg).unwrap();
        let quickjs_path = quickjs_rt
            .eval_js(
                &mut engine,
                "__NG_NATIVE__.fsResolvePath('config', 'prefs.json')",
            )
            .unwrap();
        assert_eq!(quickjs_path, resolved_path);
    }

    #[test]
    fn resolves_primary_window_without_hardcoded_win_variable_name() {
        let mut rt = JsRuntime::new();
        let mut engine = RenderEngine::new(HeadlessBackend::new());
        rt.eval_tick(
            &mut engine,
            "create main_window ng-window\ntext main_window Custom Root",
        )
        .unwrap();

        let primary = rt.resolve_primary_window().unwrap();
        assert_eq!(primary, rt.resolve_var("main_window").unwrap());
        assert!(
            engine
                .render_window_preview(primary)
                .unwrap()
                .contains("Custom Root")
        );
    }
}
