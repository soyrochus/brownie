use crate::event::AppEvent;
use crate::ui::catalog::{CatalogManager, TemplateDocument, TemplateMatch, TemplateMeta, UiIntent};
use crate::ui::intent::intent_from_text;
use copilot_sdk::{
    Client, ConnectionState, Session, SessionConfig, SessionEventData, SystemMessageConfig,
    SystemMessageMode, Tool, ToolHandler, ToolResultObject,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::runtime::Handle;
use tokio::sync::RwLock;
use tokio::time::{self, Duration};

#[derive(Clone)]
pub struct CopilotClient {
    workspace: PathBuf,
    tx: mpsc::Sender<AppEvent>,
    client: Arc<Client>,
    session: Arc<RwLock<Option<Arc<Session>>>>,
    runtime_handle: Handle,
    state_poller_started: Arc<AtomicBool>,
}

impl CopilotClient {
    fn brownie_system_message() -> &'static str {
        "You are the assistant inside the Brownie desktop app, not a standalone terminal-only chat.

UI model:
- Brownie has three panes: Workspace, Chat, and Canvas.
- Canvas is rendered by the host app from validated UiSchema templates selected by intent.
- You cannot directly draw arbitrary graphics, but users do have a Canvas surface you should refer to when asked about UI.

Current Canvas capabilities:
- code_review template: markdown, form fields, diff, action buttons
- plan_review template: markdown, form fields, action button
- file_listing template: workspace file listing rendered in canvas

Behavior requirements:
- Do not claim there is no canvas or that the UI is terminal-only.
- Use the `query_ui_catalog` tool for requests about showing UI in canvas.
- Never claim that something is rendered unless `query_ui_catalog` in the same turn returns `status=rendered_catalog` or `status=rendered_provisional`.
- If `query_ui_catalog` returns `status=text_only` or any error, explicitly say canvas was not rendered and provide a text fallback.
- If `query_ui_catalog` reports `rendered_catalog` or `rendered_provisional`, confirm what was rendered.
- If `query_ui_catalog` reports `needs_save_confirmation=true`, ask the user whether to save the provisional template to catalog.
- If a requested UI is not supported by current templates, say it is not currently available instead of inventing capabilities."
    }

    fn query_ui_catalog_tool() -> Tool {
        Tool::new("query_ui_catalog")
            .description("Resolve, render, and optionally provision a canvas UI template from the Brownie catalog")
            .schema(json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "User request to evaluate against the UI catalog"
                    },
                    "allow_provisional": {
                        "type": "boolean",
                        "description": "When no catalog template matches, create and render a provisional template",
                        "default": true
                    }
                },
                "required": ["query"]
            }))
    }

    fn query_ui_catalog_handler(workspace: PathBuf, tx: mpsc::Sender<AppEvent>) -> ToolHandler {
        Arc::new(move |_name, args| {
            let Some(query) = extract_tool_query(args) else {
                return ToolResultObject::error(
                    "query_ui_catalog requires a non-empty query string (supported keys: query, prompt, request, text, message)",
                );
            };

            let allow_provisional = args
                .get("allow_provisional")
                .and_then(|value| value.as_bool())
                .unwrap_or(true);

            let Some(intent) = intent_from_text(query.as_str()) else {
                return ToolResultObject::text(
                    json!({
                        "status": "text_only",
                        "message": "No UI intent detected for query. Reply in text.",
                        "query": query
                    })
                    .to_string(),
                );
            };

            let user_catalog_dir = workspace.join(".brownie").join("catalog");
            let catalog_manager = CatalogManager::with_default_providers(user_catalog_dir, false);
            let resolution = catalog_manager.resolve(&intent);

            if let Some(template) = resolution.selected {
                let event = AppEvent::CanvasToolRender {
                    intent: intent.clone(),
                    template_id: template.document.meta.id.clone(),
                    title: template.document.meta.title.clone(),
                    provider_id: template.source.provider_id.clone(),
                    provider_kind: template.source.kind.as_str().to_string(),
                    schema: template.schema_value().clone(),
                    provisional_template: None,
                };
                let _ = tx.send(event);

                return ToolResultObject::text(
                    json!({
                        "status": "rendered_catalog",
                        "intent": intent.summary(),
                        "template_id": template.document.meta.id,
                        "title": template.document.meta.title,
                        "provider": template.source.provider_id,
                        "needs_save_confirmation": false
                    })
                    .to_string(),
                );
            }

            if !allow_provisional {
                return ToolResultObject::text(
                    json!({
                        "status": "text_only",
                        "intent": intent.summary(),
                        "message": "No matching catalog template and provisional creation is disabled."
                    })
                    .to_string(),
                );
            }

            let provisional = build_provisional_template(query.as_str(), &intent);
            let event = AppEvent::CanvasToolRender {
                intent: intent.clone(),
                template_id: provisional.meta.id.clone(),
                title: provisional.meta.title.clone(),
                provider_id: "runtime-provisional".to_string(),
                provider_kind: "provisional".to_string(),
                schema: provisional.schema.clone(),
                provisional_template: Some(provisional.clone()),
            };
            let _ = tx.send(event);

            ToolResultObject::text(
                json!({
                    "status": "rendered_provisional",
                    "intent": intent.summary(),
                    "template_id": provisional.meta.id,
                    "title": provisional.meta.title,
                    "needs_save_confirmation": true
                })
                .to_string(),
            )
        })
    }

    pub fn new(workspace: PathBuf, tx: mpsc::Sender<AppEvent>) -> copilot_sdk::Result<Self> {
        let runtime_handle = Handle::try_current().map_err(|err| {
            copilot_sdk::CopilotError::InvalidConfig(format!("tokio runtime unavailable: {err}"))
        })?;

        let client = Client::builder()
            .use_stdio(true)
            .auto_restart(true)
            .cwd(workspace.clone())
            .build()?;

        Ok(Self {
            workspace,
            tx,
            client: Arc::new(client),
            session: Arc::new(RwLock::new(None)),
            runtime_handle,
            state_poller_started: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn start(&self) {
        let _ = self
            .tx
            .send(AppEvent::StatusChanged(ConnectionState::Connecting));
        self.spawn_state_poller();

        let client = Arc::clone(&self.client);
        let tx = self.tx.clone();
        let workspace = self.workspace.clone();
        let session_slot = Arc::clone(&self.session);
        let runtime_handle = self.runtime_handle.clone();

        self.runtime_handle.spawn(async move {
            if let Err(err) = client.start().await {
                let _ = tx.send(AppEvent::StatusChanged(ConnectionState::Error));
                let _ = tx.send(AppEvent::SdkError(format!(
                    "failed to start Copilot client: {err}"
                )));
                return;
            }

            match client.get_auth_status().await {
                Ok(auth) if auth.is_authenticated => {
                    let _ = tx.send(AppEvent::StatusChanged(ConnectionState::Connected));
                }
                Ok(auth) => {
                    let message = auth
                        .status_message
                        .unwrap_or_else(|| "copilot CLI is not authenticated".to_string());
                    let _ = tx.send(AppEvent::StatusChanged(ConnectionState::Error));
                    let _ = tx.send(AppEvent::SdkError(message));
                    return;
                }
                Err(err) => {
                    let _ = tx.send(AppEvent::StatusChanged(ConnectionState::Error));
                    let _ = tx.send(AppEvent::SdkError(format!(
                        "failed to query auth status: {err}"
                    )));
                    return;
                }
            }

            let query_ui_catalog_tool = Self::query_ui_catalog_tool();
            let mut session_config = SessionConfig {
                tools: vec![query_ui_catalog_tool.clone()],
                available_tools: Some(vec!["query_ui_catalog".to_string()]),
                excluded_tools: Some(vec![
                    "shell".to_string(),
                    "powershell".to_string(),
                    "write".to_string(),
                ]),
                request_permission: Some(false),
                system_message: Some(SystemMessageConfig {
                    mode: Some(SystemMessageMode::Append),
                    content: Some(Self::brownie_system_message().to_string()),
                }),
                ..Default::default()
            };
            session_config.working_directory = Some(workspace.to_string_lossy().to_string());

            match client.create_session(session_config).await {
                Ok(session) => {
                    let handler = Self::query_ui_catalog_handler(workspace.clone(), tx.clone());
                    session
                        .register_tool_with_handler(query_ui_catalog_tool, Some(handler))
                        .await;

                    let session_id = session.session_id().to_string();
                    {
                        let mut slot = session_slot.write().await;
                        *slot = Some(Arc::clone(&session));
                    }
                    let _ = tx.send(AppEvent::SessionCreated(session_id));
                    Self::spawn_event_listener(runtime_handle, session, tx);
                }
                Err(err) => {
                    let _ = tx.send(AppEvent::StatusChanged(ConnectionState::Error));
                    let _ = tx.send(AppEvent::SdkError(format!(
                        "failed to create session: {err}"
                    )));
                }
            }
        });
    }

    pub fn send(&self, prompt: String) {
        let tx = self.tx.clone();
        let session_slot = Arc::clone(&self.session);

        self.runtime_handle.spawn(async move {
            let session = {
                let guard = session_slot.read().await;
                guard.clone()
            };

            let Some(session) = session else {
                let _ = tx.send(AppEvent::SdkError("No active session".to_string()));
                return;
            };

            if let Err(err) = session.send(prompt).await {
                let _ = tx.send(AppEvent::SdkError(format!("failed to send prompt: {err}")));
            }
        });
    }

    fn spawn_state_poller(&self) {
        if self
            .state_poller_started
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }

        let tx = self.tx.clone();
        let client = Arc::clone(&self.client);
        self.runtime_handle.spawn(async move {
            let mut ticker = time::interval(Duration::from_millis(500));
            let mut last_state = client.state().await;

            loop {
                ticker.tick().await;
                let current_state = client.state().await;
                if current_state != last_state {
                    last_state = current_state;
                    let _ = tx.send(AppEvent::StatusChanged(current_state));
                }
            }
        });
    }

    fn spawn_event_listener(
        runtime_handle: Handle,
        session: Arc<Session>,
        tx: mpsc::Sender<AppEvent>,
    ) {
        runtime_handle.spawn(async move {
            let mut events = session.subscribe();
            let mut active_tool_calls: HashMap<String, String> = HashMap::new();
            loop {
                match events.recv().await {
                    Ok(event) => match event.data {
                        SessionEventData::AssistantMessageDelta(delta) => {
                            let _ = tx.send(AppEvent::StreamDelta(delta.delta_content));
                        }
                        SessionEventData::AssistantMessage(message) => {
                            let _ = tx.send(AppEvent::StreamDelta(message.content));
                            let _ = tx.send(AppEvent::StreamEnd);
                        }
                        SessionEventData::SessionIdle(_) => {
                            let _ = tx.send(AppEvent::StreamEnd);
                        }
                        SessionEventData::SessionError(err) => {
                            let _ = tx.send(AppEvent::SdkError(err.message));
                        }
                        SessionEventData::ToolUserRequested(data) => {
                            let tool_name = data.tool_name;
                            active_tool_calls.insert(data.tool_call_id, tool_name.clone());
                            if tool_name != "query_ui_catalog" {
                                let _ = tx.send(AppEvent::ToolCallSuppressed(tool_name));
                            }
                        }
                        SessionEventData::ToolExecutionStart(data) => {
                            let tool_name = data.tool_name;
                            active_tool_calls.insert(data.tool_call_id, tool_name.clone());
                            if tool_name != "query_ui_catalog" {
                                let _ = tx.send(AppEvent::ToolCallSuppressed(tool_name));
                            }
                        }
                        SessionEventData::ToolExecutionComplete(data) => {
                            let tool_name = active_tool_calls
                                .remove(&data.tool_call_id)
                                .unwrap_or_else(|| "unknown".to_string());
                            let (status, message) = summarize_tool_execution(
                                data.success,
                                data.result.as_ref().map(|result| result.content.as_str()),
                                data.error.as_ref().map(|err| err.message.as_str()),
                            );
                            let _ = tx.send(AppEvent::ToolExecutionOutcome {
                                tool_name,
                                status,
                                message,
                            });
                        }
                        _ => {}
                    },
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        let _ = tx.send(AppEvent::StatusChanged(ConnectionState::Disconnected));
                        break;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        continue;
                    }
                }
            }
        });
    }
}

fn extract_tool_query(args: &Value) -> Option<String> {
    for key in ["query", "prompt", "request", "text", "message"] {
        if let Some(query) = args.get(key).and_then(Value::as_str) {
            let query = query.trim();
            if !query.is_empty() {
                return Some(query.to_string());
            }
        }
    }

    if let Some(query) = args.as_str() {
        let query = query.trim();
        if !query.is_empty() {
            return Some(query.to_string());
        }
    }

    None
}

fn summarize_tool_execution(
    success: bool,
    result_content: Option<&str>,
    error_message: Option<&str>,
) -> (String, Option<String>) {
    if !success {
        return (
            "error".to_string(),
            error_message.map(|message| message.to_string()),
        );
    }

    if let Some(content) = result_content {
        if let Ok(payload) = serde_json::from_str::<Value>(content) {
            if let Some(status) = payload.get("status").and_then(Value::as_str) {
                let message = payload
                    .get("message")
                    .and_then(Value::as_str)
                    .map(|message| message.to_string());
                return (status.to_string(), message);
            }
        }
    }

    ("success".to_string(), None)
}

fn provisional_template_id(intent: &UiIntent) -> String {
    let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_millis(),
        Err(_) => 0,
    };
    format!(
        "provisional.{}.{}",
        sanitize_identifier(&intent.primary),
        now
    )
}

#[cfg(test)]
mod tests {
    use super::summarize_tool_execution;

    #[test]
    fn summarize_tool_execution_reads_status_from_json_payload() {
        let (status, message) = summarize_tool_execution(
            true,
            Some("{\"status\":\"text_only\",\"message\":\"No UI intent detected\"}"),
            None,
        );
        assert_eq!(status, "text_only");
        assert_eq!(message.as_deref(), Some("No UI intent detected"));
    }

    #[test]
    fn summarize_tool_execution_reports_error_when_execution_fails() {
        let (status, message) = summarize_tool_execution(false, None, Some("tool call failed"));
        assert_eq!(status, "error");
        assert_eq!(message.as_deref(), Some("tool call failed"));
    }
}

fn sanitize_identifier(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch.to_ascii_lowercase());
        }
        if out.len() >= 32 {
            break;
        }
    }
    if out.is_empty() {
        "ui".to_string()
    } else {
        out
    }
}

fn build_provisional_template(query: &str, intent: &UiIntent) -> TemplateDocument {
    let template_id = provisional_template_id(intent);
    let title = format!("Provisional {}", intent.primary.replace('_', " "));
    let mut components = vec![json!({
        "id": "provisional_intro",
        "kind": "markdown",
        "text": format!("### Provisional Canvas\\n{}", query.trim())
    })];

    if intent.primary == "file_listing" {
        components.push(json!({
            "id": "workspace_tree",
            "kind": "code",
            "language": "text",
            "code": "__WORKSPACE_TREE__"
        }));
    }

    TemplateDocument {
        meta: TemplateMeta {
            id: template_id,
            title,
            version: "0.1.0".to_string(),
            tags: intent.tags.clone(),
        },
        match_rules: TemplateMatch {
            primary: intent.primary.clone(),
            operations: intent.operations.clone(),
            tags: intent.tags.clone(),
        },
        schema: json!({
            "schema_version": 1,
            "outputs": [],
            "components": components,
        }),
    }
}
