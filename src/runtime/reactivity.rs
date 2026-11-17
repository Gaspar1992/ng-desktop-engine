use super::{Instruction, JsRuntime};
use crate::engine::{EngineError, RenderEngine};
use crate::platforms::PlatformBackend;

/// Declarative `{{ signal() }}` text binding compiled from `bind_text`.
#[derive(Debug, Clone)]
pub(super) struct TextBinding {
    pub(super) node_var: String,
    pub(super) signal_name: String,
    pub(super) template: String,
}

/// Declarative `[attr]="signal()"` property binding compiled from `bind_attr`.
#[derive(Debug, Clone)]
pub(super) struct AttrBinding {
    pub(super) node_var: String,
    pub(super) attr_key: String,
    pub(super) signal_name: String,
    pub(super) template: String,
}

/// Declarative `@if` / `@switch` / `[visible]` binding compiled from `bind_visible`.
#[derive(Debug, Clone)]
pub(super) struct VisibilityBinding {
    pub(super) node_var: String,
    pub(super) signal_name: String,
    pub(super) op: String,
    pub(super) rhs: i64,
}

pub(super) fn evaluate_visibility_op(lhs: i64, op: &str, rhs: i64) -> bool {
    match op {
        "==" | "===" => lhs == rhs,
        "!=" | "!==" => lhs != rhs,
        ">" => lhs > rhs,
        ">=" => lhs >= rhs,
        "<" => lhs < rhs,
        "<=" => lhs <= rhs,
        _ => lhs != 0,
    }
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
    pub(super) fn execute_reactive_instruction<B: PlatformBackend>(
        &mut self,
        engine: &mut RenderEngine<B>,
        instruction: Instruction,
        is_hmr: bool,
    ) -> Result<(), EngineError> {
        match instruction {
            Instruction::DeclareSignal { name, initial } => {
                if is_hmr {
                    self.signals.entry(name).or_insert(initial);
                } else {
                    self.signals.insert(name, initial);
                }
                Ok(())
            }
            Instruction::BindText {
                var_name,
                signal_name,
                template,
            } => {
                let _ = self.resolve_var(&var_name)?;
                self.bindings.push(TextBinding {
                    node_var: var_name,
                    signal_name,
                    template,
                });
                Ok(())
            }
            Instruction::BindAttr {
                var_name,
                attr_key,
                signal_name,
                template,
            } => {
                let _ = self.resolve_var(&var_name)?;
                self.attr_bindings.push(AttrBinding {
                    node_var: var_name,
                    attr_key,
                    signal_name,
                    template,
                });
                Ok(())
            }
            Instruction::BindVisible {
                var_name,
                signal_name,
                op,
                rhs,
            } => {
                let _ = self.resolve_var(&var_name)?;
                self.visibility_bindings.push(VisibilityBinding {
                    node_var: var_name,
                    signal_name,
                    op,
                    rhs,
                });
                Ok(())
            }
            Instruction::OnAdd {
                var_name,
                event_name,
                signal_name,
                delta,
            } => self.attach_handler(
                engine,
                &var_name,
                &event_name,
                HandlerAction::AddSignal { signal_name, delta },
            ),
            Instruction::OnSet {
                var_name,
                event_name,
                signal_name,
                value,
            } => self.attach_handler(
                engine,
                &var_name,
                &event_name,
                HandlerAction::SetSignal { signal_name, value },
            ),
            Instruction::OnText {
                var_name,
                event_name,
                target_var,
                text,
            } => self.attach_handler(
                engine,
                &var_name,
                &event_name,
                HandlerAction::SetText {
                    node_var: target_var,
                    text,
                },
            ),
            _ => Ok(()),
        }
    }

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
        for binding in self
            .attr_bindings
            .iter()
            .filter(|b| b.signal_name == signal_name)
        {
            let target_id = self.resolve_var(&binding.node_var)?;
            let val = format_bound_template(&binding.template, &rendered_value);
            engine.set_attribute(target_id, &binding.attr_key, val)?;
        }
        for binding in self
            .visibility_bindings
            .iter()
            .filter(|b| b.signal_name == signal_name)
        {
            let target_id = self.resolve_var(&binding.node_var)?;
            let is_visible = evaluate_visibility_op(updated_val, &binding.op, binding.rhs);
            engine.set_attribute(target_id, "visible", is_visible.to_string())?;
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
    use super::{JsRuntime, evaluate_visibility_op, format_bound_template};
    use crate::engine::{RenderCommand, RenderEngine};
    use crate::platforms::HeadlessBackend;

    #[test]
    fn formats_pipe_placeholders_for_reactive_signal_updates() {
        assert_eq!(format_bound_template("ID: {}", "25"), "ID: 25");
        assert_eq!(
            format_bound_template("Name: {|uppercase}", "pikachu"),
            "Name: PIKACHU"
        );
        assert_eq!(format_bound_template("Rate: {|percent}", "2"), "Rate: 200%");
    }

    #[test]
    fn evaluates_visibility_operators_for_if_and_switch_bindings() {
        assert!(evaluate_visibility_op(25, "!=", 0));
        assert!(!evaluate_visibility_op(0, "!=", 0));
        assert!(evaluate_visibility_op(0, "===", 0));
        assert!(evaluate_visibility_op(10, ">", 5));
        assert!(evaluate_visibility_op(10, ">=", 10));
        assert!(evaluate_visibility_op(3, "<", 5));
        assert!(evaluate_visibility_op(5, "<=", 5));
        assert!(!evaluate_visibility_op(0, "unknown", 0));
    }

    #[test]
    fn propagates_bind_attr_and_bind_visible_when_signal_mutates() {
        let script = r"
            signal tab 0
            create win ng-window
            create list_box ng-box
            create detail_box ng-box
            create bar ng-progress-bar
            append win list_box
            append win detail_box
            append win bar
            bind_visible list_box tab == 0
            bind_visible detail_box tab != 0
            bind_attr bar value tab {}
        ";
        let mut engine = RenderEngine::new(HeadlessBackend::new());
        let mut runtime = JsRuntime::new();
        runtime.eval_tick(&mut engine, script).unwrap();

        let list_id = runtime.resolve_var("list_box").unwrap();
        let detail_id = runtime.resolve_var("detail_box").unwrap();
        let bar_id = runtime.resolve_var("bar").unwrap();

        runtime.set_signal(&mut engine, "tab", 25).unwrap();
        engine.flush_frame();
        let cmds = &engine.backend().frames[1];
        assert!(cmds.contains(&RenderCommand::SetAttribute {
            node: bar_id,
            key: String::from("value"),
            value: String::from("25"),
        }));
        assert!(cmds.contains(&RenderCommand::SetAttribute {
            node: list_id,
            key: String::from("visible"),
            value: String::from("false"),
        }));
        assert!(cmds.contains(&RenderCommand::SetAttribute {
            node: detail_id,
            key: String::from("visible"),
            value: String::from("true"),
        }));
    }
}
