use copilot_sdk::ConnectionState;
use serde_json::Value;

use crate::ui::catalog::{TemplateDocument, UiIntent};

#[derive(Debug, Clone)]
pub enum AppEvent {
    StreamDelta(String),
    StreamEnd,
    StatusChanged(ConnectionState),
    SdkError(String),
    SessionCreated(String),
    ToolCallSuppressed(String),
    ToolExecutionOutcome {
        tool_name: String,
        status: String,
        message: Option<String>,
    },
    CanvasToolRender {
        intent: UiIntent,
        template_id: String,
        title: String,
        provider_id: String,
        provider_kind: String,
        schema: Value,
        provisional_template: Option<TemplateDocument>,
    },
}
