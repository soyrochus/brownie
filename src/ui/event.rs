use serde::{Deserialize, Serialize};

use crate::ui::workspace::{CanvasBlockActionStatus, CanvasBlockActionType, CanvasBlockActor};

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
    CanvasBlockLifecycle {
        action: CanvasBlockActionType,
        actor: CanvasBlockActor,
        status: CanvasBlockActionStatus,
        block_id: Option<String>,
        #[serde(default)]
        message: Option<String>,
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
            Self::CanvasBlockLifecycle {
                action,
                actor,
                status,
                block_id,
                message,
            } => format!(
                "canvas_block_lifecycle action={:?} actor={:?} status={:?} block_id={}{}",
                action,
                actor,
                status,
                block_id.as_deref().unwrap_or("-"),
                message
                    .as_deref()
                    .map(|value| format!(" message={value}"))
                    .unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use super::{UiEvent, UiEventLog};
    use crate::ui::workspace::{CanvasBlockActionStatus, CanvasBlockActionType, CanvasBlockActor};

    #[test]
    fn lifecycle_events_render_machine_readable_log_line() {
        let event = UiEvent::CanvasBlockLifecycle {
            action: CanvasBlockActionType::Close,
            actor: CanvasBlockActor::User,
            status: CanvasBlockActionStatus::Succeeded,
            block_id: Some("block-7".to_string()),
            message: Some("ok".to_string()),
        };
        let line = event.to_log_line();
        assert!(line.contains("canvas_block_lifecycle"));
        assert!(line.contains("action=Close"));
        assert!(line.contains("actor=User"));
        assert!(line.contains("status=Succeeded"));
        assert!(line.contains("block_id=block-7"));
        assert!(line.contains("message=ok"));
    }

    #[test]
    fn ui_event_log_is_append_only_and_ordered() {
        let mut log = UiEventLog::default();
        let first = UiEvent::CanvasBlockLifecycle {
            action: CanvasBlockActionType::Open,
            actor: CanvasBlockActor::Assistant,
            status: CanvasBlockActionStatus::Requested,
            block_id: None,
            message: Some("template_id=builtin.file_listing.default".to_string()),
        };
        let second = UiEvent::CanvasBlockLifecycle {
            action: CanvasBlockActionType::Open,
            actor: CanvasBlockActor::Assistant,
            status: CanvasBlockActionStatus::Succeeded,
            block_id: Some("block-1".to_string()),
            message: None,
        };
        log.push(first.clone());
        log.push(second.clone());

        assert_eq!(log.entries().len(), 2);
        assert_eq!(log.entries()[0], first);
        assert_eq!(log.entries()[1], second);
    }
}
