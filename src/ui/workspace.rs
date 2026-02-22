use crate::ui::catalog::UiIntent;
use crate::ui::event::UiFieldValue;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CanvasWorkspaceState {
    #[serde(default)]
    pub blocks: Vec<CanvasBlockState>,
    #[serde(default)]
    pub active_block_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasBlockState {
    pub block_id: String,
    pub template_id: String,
    pub title: String,
    pub provider_id: String,
    pub provider_kind: String,
    pub schema: Value,
    pub intent: UiIntent,
    #[serde(default)]
    pub minimized: bool,
    #[serde(default)]
    pub form_state: BTreeMap<String, UiFieldValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanvasBlockActionType {
    Open,
    Update,
    Focus,
    Minimize,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanvasBlockActor {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanvasBlockActionStatus {
    Requested,
    Succeeded,
    Failed,
}
