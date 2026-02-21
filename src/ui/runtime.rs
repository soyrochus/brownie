use crate::theme::Theme;
use crate::ui::event::{UiEvent, UiEventLog, UiFieldValue};
use crate::ui::registry::ComponentRegistry;
use crate::ui::schema::{
    field_key, validate_schema, UiSchema, ValidatedComponent, ValidatedSchema,
};
use eframe::egui::{self, RichText, ScrollArea};
use std::collections::BTreeMap;
use std::fmt;

const EMBEDDED_SCHEMA: &str = include_str!("fixture.json");

#[derive(Debug, Clone)]
pub enum RuntimeError {
    Deserialize(String),
    Validation(String),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Deserialize(message) => write!(f, "schema deserialize error: {message}"),
            Self::Validation(message) => write!(f, "schema validation error: {message}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

pub struct UiRuntime {
    registry: ComponentRegistry,
    validated_schema: Option<ValidatedSchema>,
    runtime_error: Option<RuntimeError>,
    form_state: BTreeMap<String, UiFieldValue>,
    event_log: UiEventLog,
}

impl UiRuntime {
    pub fn new() -> Self {
        let mut runtime = Self {
            registry: ComponentRegistry::new(),
            validated_schema: None,
            runtime_error: None,
            form_state: BTreeMap::new(),
            event_log: UiEventLog::default(),
        };
        runtime.load_embedded_schema();
        runtime
    }

    pub fn load_embedded_schema(&mut self) {
        self.load_schema_json(EMBEDDED_SCHEMA);
    }

    pub fn load_schema_json(&mut self, raw_schema: &str) {
        self.validated_schema = None;
        self.runtime_error = None;
        self.form_state.clear();

        let parsed: UiSchema = match serde_json::from_str(raw_schema) {
            Ok(schema) => schema,
            Err(err) => {
                self.runtime_error = Some(RuntimeError::Deserialize(err.to_string()));
                return;
            }
        };

        let validated = match validate_schema(&parsed, &self.registry) {
            Ok(validated) => validated,
            Err(err) => {
                self.runtime_error = Some(RuntimeError::Validation(err.to_string()));
                return;
            }
        };

        self.seed_form_state(&validated.components);
        self.validated_schema = Some(validated);
    }

    pub fn event_log(&self) -> &[UiEvent] {
        self.event_log.entries()
    }

    pub fn render_canvas(&mut self, ui: &mut egui::Ui, theme: &Theme) {
        if let Some(error) = &self.runtime_error {
            let frame = theme.panel_frame(theme.surface_2, theme.spacing_16 as i8);
            frame.show(ui, |ui| {
                ui.label(
                    RichText::new("Canvas validation failed")
                        .color(theme.danger)
                        .size(13.0),
                );
                ui.add_space(theme.spacing_8);
                ui.label(
                    RichText::new(error.to_string())
                        .color(theme.text_muted)
                        .size(12.0),
                );
            });
            return;
        }

        let Some(schema) = self.validated_schema.clone() else {
            return;
        };
        let _schema_version = schema.schema_version;
        for component in &schema.components {
            self.registry.render_component(
                component,
                ui,
                theme,
                &mut self.form_state,
                &mut |event| self.event_log.push(event),
            );
            ui.add_space(theme.spacing_12);
        }
    }

    pub fn render_event_log(&self, ui: &mut egui::Ui, theme: &Theme) {
        ui.label(
            RichText::new("UI Event Log")
                .color(theme.text_primary)
                .size(13.0),
        );
        ui.add_space(theme.spacing_8);
        ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
            for event in self.event_log() {
                ui.label(
                    RichText::new(event.to_log_line())
                        .color(theme.text_muted)
                        .size(12.0),
                );
            }
        });
    }

    fn seed_form_state(&mut self, components: &[ValidatedComponent]) {
        for component in components {
            if let ValidatedComponent::Form(form) = component {
                for field in &form.fields {
                    self.form_state
                        .insert(field_key(&form.id, field.id()), field.default_value());
                }
            }
            self.seed_form_state(component.children());
        }
    }

    #[cfg(test)]
    pub fn simulate_button_click(&mut self, button_id: &str) {
        if let Some(button) = self.find_button(button_id) {
            self.event_log.push(UiEvent::ButtonClicked {
                component_id: button.id.clone(),
                output_event_id: button.output_event_id.clone(),
            });
        }
    }

    #[cfg(test)]
    pub fn simulate_form_commit(&mut self, form_id: &str, field_id: &str, value: UiFieldValue) {
        self.form_state
            .insert(field_key(form_id, field_id), value.clone());
        self.event_log.push(UiEvent::FormFieldCommitted {
            component_id: form_id.to_string(),
            form_id: form_id.to_string(),
            field_id: field_id.to_string(),
            value,
        });
    }

    #[cfg(test)]
    fn find_button(&self, button_id: &str) -> Option<crate::ui::schema::ButtonComponent> {
        let schema = self.validated_schema.as_ref()?;
        fn walk(
            components: &[ValidatedComponent],
            button_id: &str,
        ) -> Option<crate::ui::schema::ButtonComponent> {
            for component in components {
                match component {
                    ValidatedComponent::Button(button) if button.id == button_id => {
                        return Some(button.clone());
                    }
                    _ => {
                        if let Some(button) = walk(component.children(), button_id) {
                            return Some(button);
                        }
                    }
                }
            }
            None
        }
        walk(&schema.components, button_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_event_sequence_for_replayed_interactions() {
        let mut first = UiRuntime::new();
        first.simulate_button_click("approve_btn");
        first.simulate_form_commit(
            "review_form",
            "decision",
            UiFieldValue::Select {
                value: "needs-changes".to_string(),
            },
        );
        first.simulate_button_click("reject_btn");

        let mut second = UiRuntime::new();
        second.simulate_button_click("approve_btn");
        second.simulate_form_commit(
            "review_form",
            "decision",
            UiFieldValue::Select {
                value: "needs-changes".to_string(),
            },
        );
        second.simulate_button_click("reject_btn");

        assert_eq!(first.event_log(), second.event_log());
    }
}
