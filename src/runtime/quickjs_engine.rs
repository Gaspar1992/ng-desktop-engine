use super::bootstrap_js::NG_NATIVE_BOOTSTRAP_JS;
use super::host_bridge::{HostAsyncCompletion, HostBridge};
use super::js_engine::{JsEngineBackend, format_completions_js, format_event_payload_js};
use super::jsc_sys::{HostBridgeSlot, HttpEngineSlot, JscCallbackKind, dispatch_host_bridge_call};
use crate::desktop::NativeHttpEngine;
use crate::engine::{EngineError, EventPayload};
use rquickjs::{CatchResultExt, Coerced, Context, FromJs, Function, Runtime, Value};
use std::sync::{Arc, Mutex};

/// Embedded `QuickJS-NG` ES2024+ JavaScript VM for Windows, portable builds,
/// and `jsEngine: "quickjs"` / `JscEngine` fallback when `JavaScriptCore` is absent.
/// Executes `NG_NATIVE_BOOTSTRAP_JS` and compiled Angular 22+ Ivy bundles natively.
pub struct QuickJsEngine {
    runtime: Runtime,
    context: Context,
    http_slot: HttpEngineSlot,
    host_slot: HostBridgeSlot,
}

impl Default for QuickJsEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl QuickJsEngine {
    pub fn new() -> Self {
        let runtime = Runtime::new().expect("Failed to initialize QuickJS-NG runtime");
        runtime.set_max_stack_size(8 * 1024 * 1024);
        let context = Context::full(&runtime).expect("Failed to initialize QuickJS-NG context");
        let http_slot: HttpEngineSlot =
            Arc::new(Mutex::new(Arc::new(Mutex::new(NativeHttpEngine::new()))));
        let host_slot: HostBridgeSlot = Arc::new(Mutex::new(None));

        Self::register_host_callbacks(&context, &http_slot, &host_slot);
        let _ = Self::eval_with_context(&runtime, &context, NG_NATIVE_BOOTSTRAP_JS);

        Self {
            runtime,
            context,
            http_slot,
            host_slot,
        }
    }

    fn register_host_callbacks(
        context: &Context,
        http_slot: &HttpEngineSlot,
        host_slot: &HostBridgeSlot,
    ) {
        let callbacks: [(&str, JscCallbackKind); 3] = [
            ("__NG_NATIVE_HOST_HTTP__", JscCallbackKind::Http),
            (
                "__NG_NATIVE_HOST_HTTP_CONFIG__",
                JscCallbackKind::HttpConfig,
            ),
            ("__NG_NATIVE_HOST_SERVICE__", JscCallbackKind::HostService),
        ];

        context.with(|ctx| {
            let globals = ctx.globals();
            for (name, kind) in callbacks {
                let http = Arc::clone(http_slot);
                let host = Arc::clone(host_slot);
                if let Ok(func) = Function::new(ctx.clone(), move |payload: String| -> String {
                    dispatch_host_bridge_call(kind, &payload, &http, &host)
                }) {
                    let _ = globals.set(name, func);
                }
            }
        });
    }

    fn eval_with_context(
        runtime: &Runtime,
        context: &Context,
        js_code: &str,
    ) -> Result<String, EngineError> {
        let output = context.with(|ctx| {
            let val: Value<'_> = ctx
                .eval(js_code)
                .catch(&ctx)
                .map_err(|caught| EngineError::ScriptSyntaxError(format!("{caught:?}")))?;
            if val.is_undefined() {
                Ok(String::from("undefined"))
            } else {
                Coerced::<String>::from_js(&ctx, val)
                    .map(|c| c.0)
                    .map_err(|err| EngineError::ScriptSyntaxError(err.to_string()))
            }
        })?;

        while runtime.is_job_pending() {
            let _ = runtime.execute_pending_job();
        }
        Ok(output)
    }

    fn eval_raw(&self, js_code: &str) -> Result<String, EngineError> {
        Self::eval_with_context(&self.runtime, &self.context, js_code)
    }

    fn eval_and_drain_with_microtasks(&self, call: &str) -> Result<String, EngineError> {
        let primary = self.eval_raw(call)?;
        let followup = self.eval_raw("globalThis.__NG_NATIVE__.__drainQueue()")?;
        let mut combined = String::new();
        for chunk in [primary.trim(), followup.trim()] {
            if !chunk.is_empty() && chunk != "undefined" {
                if !combined.is_empty() {
                    combined.push('\n');
                }
                combined.push_str(chunk);
            }
        }
        Ok(combined)
    }
}

impl JsEngineBackend for QuickJsEngine {
    fn engine_name(&self) -> &str {
        "QuickJS-NG Embedded (Bytecode Interpreter)"
    }

    fn is_jit_enabled(&self) -> bool {
        false
    }

    fn attach_http_engine(&mut self, engine: Arc<Mutex<NativeHttpEngine>>) {
        if let Ok(mut guard) = self.http_slot.lock() {
            *guard = engine;
        }
    }

    fn attach_host_bridge(&mut self, bridge: Arc<HostBridge>) {
        if let Ok(mut guard) = self.host_slot.lock() {
            *guard = Some(bridge);
        }
    }

    fn eval_script(&mut self, js_code: &str) -> Result<String, EngineError> {
        self.eval_raw(js_code)
    }

    fn dispatch_js_event_with_payload(
        &mut self,
        node_var: &str,
        event_name: &str,
        payload: &EventPayload,
    ) -> Result<(), EngineError> {
        let payload_js = format_event_payload_js(payload);
        let call = format!(
            "globalThis.__NG_NATIVE__.__dispatchEvent({node_var:?}, {event_name:?}, {payload_js}); globalThis.__NG_NATIVE__.__flushTimers();"
        );
        let _ = self.eval_raw(&call)?;
        Ok(())
    }

    fn dispatch_and_drain(
        &mut self,
        node_var: &str,
        event_name: &str,
        payload: &EventPayload,
    ) -> Result<String, EngineError> {
        let payload_js = format_event_payload_js(payload);
        let call = format!(
            "globalThis.__NG_NATIVE__.__dispatchAndDrain({node_var:?}, {event_name:?}, {payload_js})"
        );
        self.eval_and_drain_with_microtasks(&call)
    }

    fn tick_and_drain(
        &mut self,
        now_ms: u64,
        completions: &[HostAsyncCompletion],
    ) -> Result<String, EngineError> {
        let call = if completions.is_empty() {
            format!("globalThis.__NG_NATIVE__.__tickAndDrain({now_ms})")
        } else {
            let completions_js = format_completions_js(completions);
            format!("globalThis.__NG_NATIVE__.__tickAndDrain({now_ms}, {completions_js})")
        };
        self.eval_and_drain_with_microtasks(&call)
    }

    fn drain_native_commands(&mut self) -> Result<String, EngineError> {
        self.eval_raw("globalThis.__NG_NATIVE__.__drainQueue()")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quickjs_translates_bind_attr_bind_visible_and_skips_closure_bodies() {
        let mut qjs = QuickJsEngine::new();
        let script = r#"
            __NG_NATIVE__.createElement("box", "ng-box");
            __NG_NATIVE__.bindAttr("box", "value", "pokeId", "{}");
            __NG_NATIVE__.bindVisible("box", "pokeId", "!=", "0");
            __NG_NATIVE__.listen("box", "clicked", function(evt) {
                __NG_NATIVE__.setText("box", evt.value);
            });
            __NG_NATIVE__.sqliteExecute("CREATE TABLE IF NOT EXISTS t (id TEXT)");
        "#;
        qjs.eval_script(script).unwrap();
        let drained = qjs.drain_native_commands().unwrap();
        assert!(drained.contains("create box ng-box"));
        assert!(drained.contains("bind_attr box value pokeId {}"));
        assert!(drained.contains("bind_visible box pokeId != 0"));
        assert!(!drained.contains("evt.value"));
    }

    #[test]
    fn quickjs_executes_es2024_closures_and_event_dispatch() {
        let mut qjs = QuickJsEngine::new();
        let script = r#"
            let count = 10;
            __NG_NATIVE__.createElement("lbl", "ng-label");
            __NG_NATIVE__.createElement("btn", "ng-button");
            __NG_NATIVE__.listen("btn", "clicked", () => {
                count += 5;
                __NG_NATIVE__.setText("lbl", `Count: ${count}`);
            });
        "#;
        qjs.eval_script(script).unwrap();
        let _ = qjs.drain_native_commands().unwrap();

        let after_click = qjs
            .dispatch_and_drain("btn", "clicked", &EventPayload::None)
            .unwrap();
        assert_eq!(after_click.trim(), "text lbl Count: 15");
    }

    #[test]
    fn quickjs_drains_promise_microtasks_within_the_same_dispatch_tick() {
        let mut qjs = QuickJsEngine::new();
        let script = r#"
            __NG_NATIVE__.createElement("lbl", "ng-label");
            __NG_NATIVE__.createElement("btn", "ng-button");
            __NG_NATIVE__.listen("btn", "clicked", () => {
                Promise.resolve("microtask-ready").then((msg) => {
                    const bytes = new TextEncoder().encode("Pokémon 🚀");
                    const roundtrip = new TextDecoder().decode(bytes);
                    __NG_NATIVE__.setText("lbl", `${msg}:${roundtrip}`);
                });
            });
        "#;
        qjs.eval_script(script).unwrap();
        let _ = qjs.drain_native_commands().unwrap();

        let after_click = qjs
            .dispatch_and_drain("btn", "clicked", &EventPayload::None)
            .unwrap();
        assert_eq!(after_click.trim(), "text lbl microtask-ready:Pokémon 🚀");
    }
}
