use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UiFieldValue {
    Text { value: String },
    Number { value: f64 },
    Select { value: String },
    Checkbox { value: bool },
}

impl UiFieldValue {
    pub fn display_value(&self) -> String {
        match self {
            Self::Text { value } => value.clone(),
            Self::Number { value } => value.to_string(),
            Self::Select { value } => value.clone(),
            Self::Checkbox { value } => value.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum UiEvent {
    ButtonClicked {
        component_id: String,
        output_event_id: String,
    },
    FormFieldCommitted {
        component_id: String,
        form_id: String,
        field_id: String,
        value: UiFieldValue,
    },
}

impl UiEvent {
    pub fn to_log_line(&self) -> String {
        match self {
            Self::ButtonClicked {
                component_id,
                output_event_id,
            } => {
                format!("button_clicked component_id={component_id} output={output_event_id}")
            }
            Self::FormFieldCommitted {
                component_id,
                form_id,
                field_id,
                value,
            } => format!(
                "form_field_committed component_id={component_id} form_id={form_id} field_id={field_id} value={}",
                value.display_value()
            ),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct UiEventLog {
    entries: Vec<UiEvent>,
}

impl UiEventLog {
    pub fn entries(&self) -> &[UiEvent] {
        &self.entries
    }

    pub fn push(&mut self, event: UiEvent) {
        self.entries.push(event);
    }
}
