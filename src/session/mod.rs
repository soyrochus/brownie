use serde::{Deserialize, Serialize};

use crate::ui::workspace::CanvasWorkspaceState;

pub mod store;

pub const SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionMeta {
    pub schema_version: u32,
    pub session_id: String,
    pub workspace: String,
    pub title: Option<String>,
    pub created_at: String,
    #[serde(default)]
    pub canvas_workspace: CanvasWorkspaceState,
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub timestamp: String,
}
