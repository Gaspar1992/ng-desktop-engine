use super::{JsEngineBackend, JsRuntime};
use crate::engine::{EngineError, EventPayload, NodeId, RenderEngine};
use crate::platforms::PlatformBackend;

/// JS Bridge (SRP): evaluates code in the embedded `JsEngineBackend`, drains queued
/// `__NG_NATIVE__` mutations into the scene, and routes native OS events to both
/// JS listeners and declarative `.ngb` handlers.
impl JsRuntime {
    /// Evaluates JavaScript code inside the active `JsEngineBackend` (`JscEngine` or `QuickJsEngine`)
    /// and flushes any `globalThis.__NG_NATIVE__` scene mutations to `RenderEngine`.
    pub fn eval_js<B: PlatformBackend>(
        &mut self,
        engine: &mut RenderEngine<B>,
        js_code: &str,
    ) -> Result<String, EngineError> {
        let eval_result = self.js_engine.eval_script(js_code)?;
        let _ = self.flush_js_mutations(engine)?;
        Ok(eval_result)
    }

    /// Dispatches a native OS event on a resolved element variable and runs registered Angular handlers.
    pub fn trigger_event<B: PlatformBackend>(
        &mut self,
        engine: &mut RenderEngine<B>,
        var_name: &str,
        event_name: &str,
    ) -> Result<(), EngineError> {
        self.trigger_event_with_payload(engine, var_name, event_name, &EventPayload::None)
    }

    /// Dispatches a native OS event carrying a typed `EventPayload` (`Text`, `Bool`, `Int`).
    pub fn trigger_event_with_payload<B: PlatformBackend>(
        &mut self,
        engine: &mut RenderEngine<B>,
        var_name: &str,
        event_name: &str,
        payload: &EventPayload,
    ) -> Result<(), EngineError> {
        let node_id = self.resolve_var(var_name)?;
        self.dispatch_to_js(engine, var_name, event_name, payload)?;
        self.trigger_node_event(engine, node_id, event_name)
    }

    /// Dispatches a native OS event by `NodeId` with a typed `EventPayload`, notifying both
    /// the embedded JS engine (`__dispatchEvent`) and declarative scene handlers.
    pub fn trigger_node_event_with_payload<B: PlatformBackend>(
        &mut self,
        engine: &mut RenderEngine<B>,
        node_id: NodeId,
        event_name: &str,
        payload: &EventPayload,
    ) -> Result<(), EngineError> {
        if let Some(var) = self.node_vars.get(&node_id).cloned() {
            self.dispatch_to_js(engine, &var, event_name, payload)?;
        }
        match self.trigger_node_event(engine, node_id, event_name) {
            Ok(()) | Err(EngineError::StaleGeneration { .. } | EngineError::InvalidNode(_)) => {
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    /// Dispatches a native OS event by `NodeId` and runs registered Angular handlers.
    pub fn trigger_node_event<B: PlatformBackend>(
        &mut self,
        engine: &mut RenderEngine<B>,
        node_id: NodeId,
        event_name: &str,
    ) -> Result<(), EngineError> {
        let mut runtime_err: Option<EngineError> = None;

        engine.emit_event(node_id, event_name, |eng, listener_id| {
            if let Some(action) = self.handlers.get(&listener_id).cloned()
                && let Err(e) = self.execute_handler_action(eng, action)
            {
                runtime_err = Some(e);
            }
        })?;

        runtime_err.map_or(Ok(()), Err)
    }

    fn dispatch_to_js<B: PlatformBackend>(
        &mut self,
        engine: &mut RenderEngine<B>,
        var_name: &str,
        event_name: &str,
        payload: &EventPayload,
    ) -> Result<(), EngineError> {
        let drained = self
            .js_engine
            .dispatch_and_drain(var_name, event_name, payload)?;
        if !drained.trim().is_empty() {
            self.eval_tick(engine, &drained)?;
        }
        Ok(())
    }

    /// Pumps completed asynchronous host calls and elapsed JS timers/animation frames,
    /// then flushes any resulting `__NG_NATIVE__` scene mutations into `engine`.
    pub fn tick<B: PlatformBackend>(
        &mut self,
        engine: &mut RenderEngine<B>,
    ) -> Result<bool, EngineError> {
        let completions = self.host_bridge.drain_async_completions();
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0_u64, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
        let drained = self.js_engine.tick_and_drain(now_ms, &completions)?;
        if !drained.trim().is_empty() {
            self.eval_tick(engine, &drained)?;
            return Ok(true);
        }
        Ok(false)
    }

    fn flush_js_mutations<B: PlatformBackend>(
        &mut self,
        engine: &mut RenderEngine<B>,
    ) -> Result<bool, EngineError> {
        let drained = self.js_engine.drain_native_commands()?;
        if !drained.trim().is_empty() {
            self.eval_tick(engine, &drained)?;
            return Ok(true);
        }
        Ok(false)
    }
}
