use super::JsRuntime;
use crate::engine::{EngineError, RenderEngine};
use crate::platforms::PlatformBackend;

/// Declarative `{{ signal() }}` text binding compiled from `bind_text`.
#[derive(Debug, Clone)]
pub(super) struct TextBinding {
    pub(super) node_var: String,
    pub(super) signal_name: String,
    pub(super) template: String,
}

/// Declarative event handler compiled from `on_add` / `on_set` / `on_text`.
#[derive(Debug, Clone)]
pub(super) enum HandlerAction {
    AddSignal { signal_name: String, delta: i64 },
    SetSignal { signal_name: String, value: i64 },
    SetText { node_var: String, text: String },
}

fn format_bound_template(template: &str, rendered_value: &str) -> String {
    if template.contains("{|uppercase}") {
        template.replace("{|uppercase}", &rendered_value.to_uppercase())
    } else if template.contains("{|lowercase}") {
        template.replace("{|lowercase}", &rendered_value.to_lowercase())
    } else if template.contains("{|titlecase}") {
        template.replace("{|titlecase}", &rendered_value.to_uppercase())
    } else if template.contains("{|percent}") {
        let pct = rendered_value.parse::<f64>().map_or_else(
            |_| rendered_value.to_string(),
            |v| format!("{}%", v * 100.0),
        );
        template.replace("{|percent}", &pct)
    } else {
        template.replace("{}", rendered_value)
    }
}

/// Reactivity (SRP): Signal state mutation, binding propagation, and handler registration.
impl JsRuntime {
    pub(super) fn execute_handler_action<B: PlatformBackend>(
        &mut self,
        engine: &mut RenderEngine<B>,
        action: HandlerAction,
    ) -> Result<(), EngineError> {
        match action {
            HandlerAction::AddSignal { signal_name, delta } => {
                let current = self.signals.entry(signal_name.clone()).or_insert(0);
                *current += delta;
                let updated_val = *current;
                self.propagate_signal(engine, &signal_name, updated_val)
            }
            HandlerAction::SetSignal { signal_name, value } => {
                self.signals.insert(signal_name.clone(), value);
                self.propagate_signal(engine, &signal_name, value)
            }
            HandlerAction::SetText { node_var, text } => {
                let target_id = self.resolve_var(&node_var)?;
                engine.set_text(target_id, text)
            }
        }
    }

    /// Host-side signal write (e.g. async results): updates the value and re-renders bindings.
    pub fn set_signal<B: PlatformBackend>(
        &mut self,
        engine: &mut RenderEngine<B>,
        name: &str,
        value: i64,
    ) -> Result<(), EngineError> {
        self.execute_handler_action(
            engine,
            HandlerAction::SetSignal {
                signal_name: name.to_string(),
                value,
            },
        )
    }

    pub(super) fn propagate_signal<B: PlatformBackend>(
        &self,
        engine: &mut RenderEngine<B>,
        signal_name: &str,
        updated_val: i64,
    ) -> Result<(), EngineError> {
        let rendered_value = updated_val.to_string();
        for binding in self
            .bindings
            .iter()
            .filter(|b| b.signal_name == signal_name)
        {
            let target_id = self.resolve_var(&binding.node_var)?;
            let text = format_bound_template(&binding.template, &rendered_value);
            engine.set_text(target_id, text)?;
        }
        Ok(())
    }

    pub(super) fn attach_handler<B: PlatformBackend>(
        &mut self,
        engine: &mut RenderEngine<B>,
        var_name: &str,
        event_name: &str,
        action: HandlerAction,
    ) -> Result<(), EngineError> {
        let node_id = self.resolve_var(var_name)?;
        let listener_id = self.next_listener_id;
        self.next_listener_id += 1;
        engine.listen(node_id, event_name, listener_id)?;
        self.handlers.insert(listener_id, action);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::format_bound_template;

    #[test]
    fn formats_pipe_placeholders_for_reactive_signal_updates() {
        assert_eq!(format_bound_template("ID: {}", "25"), "ID: 25");
        assert_eq!(
            format_bound_template("Name: {|uppercase}", "pikachu"),
            "Name: PIKACHU"
        );
        assert_eq!(format_bound_template("Rate: {|percent}", "2"), "Rate: 200%");
    }
}
