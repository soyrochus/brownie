use crate::copilot::CopilotClient;
use crate::event::AppEvent;
use crate::session::store;
use crate::session::{Message, SessionMeta, SCHEMA_VERSION};
use crate::theme::Theme;
use crate::ui::catalog::{CatalogManager, TemplateDocument, UiIntent};
use crate::ui::event::{UiEvent, UiEventLog};
use crate::ui::runtime::UiRuntime;
use crate::ui::workspace::{
    CanvasBlockActionStatus, CanvasBlockActionType, CanvasBlockActor, CanvasBlockState,
    CanvasWorkspaceState,
};
use copilot_sdk::ConnectionState;
use eframe::egui::{self, Align, Frame, RichText, ScrollArea, Stroke};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
struct TemplateSelectionContext {
    template_id: String,
    title: String,
    provider_id: String,
    provider_kind: String,
}

struct CanvasBlock {
    state: CanvasBlockState,
    ui_runtime: UiRuntime,
    synced_event_count: usize,
    last_touched_at: u128,
}

struct CanvasRenderRequest {
    intent: UiIntent,
    template_id: String,
    title: String,
    provider_id: String,
    provider_kind: String,
    target_block_id: Option<String>,
    root_path: Option<String>,
    schema: Value,
    provisional_template: Option<TemplateDocument>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BlockTargetResolution {
    Existing(usize),
    NotFound,
    Ambiguous(Vec<String>),
}

fn resolve_block_target_for_template(
    blocks: &[CanvasBlock],
    active_block_id: Option<&str>,
    template_id: &str,
) -> BlockTargetResolution {
    // Explicit active block id has priority when it matches the requested template.
    if let Some(active_block_id) = active_block_id {
        if let Some(index) = blocks.iter().position(|block| {
            block.state.block_id == active_block_id && block.state.template_id == template_id
        }) {
            return BlockTargetResolution::Existing(index);
        }
    }

    let mut matches = blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| block.state.template_id == template_id)
        .collect::<Vec<_>>();

    if matches.is_empty() {
        return BlockTargetResolution::NotFound;
    }

    let newest_touch = matches
        .iter()
        .map(|(_, block)| block.last_touched_at)
        .max()
        .unwrap_or(0);
    matches.retain(|(_, block)| block.last_touched_at == newest_touch);

    if matches.len() == 1 {
        return BlockTargetResolution::Existing(matches[0].0);
    }

    let mut block_ids = matches
        .into_iter()
        .map(|(_, block)| block.state.block_id.clone())
        .collect::<Vec<_>>();
    block_ids.sort();
    BlockTargetResolution::Ambiguous(block_ids)
}

fn apply_focus_transition(
    blocks: &mut [CanvasBlock],
    active_block_id: &mut Option<String>,
    block_id: &str,
    touched_at: u128,
) -> bool {
    let Some(index) = blocks
        .iter()
        .position(|block| block.state.block_id == block_id)
    else {
        return false;
    };
    *active_block_id = Some(block_id.to_string());
    blocks[index].last_touched_at = touched_at;
    true
}

fn apply_toggle_minimize_transition(
    blocks: &mut [CanvasBlock],
    block_id: &str,
    touched_at: u128,
) -> Option<bool> {
    let index = blocks
        .iter()
        .position(|block| block.state.block_id == block_id)?;
    let block = &mut blocks[index];
    block.state.minimized = !block.state.minimized;
    block.last_touched_at = touched_at;
    Some(block.state.minimized)
}

fn apply_close_transition(
    blocks: &mut Vec<CanvasBlock>,
    active_block_id: &mut Option<String>,
    block_id: &str,
) -> bool {
    let before = blocks.len();
    blocks.retain(|block| block.state.block_id != block_id);
    if blocks.len() == before {
        return false;
    }

    if active_block_id.as_deref() == Some(block_id) {
        *active_block_id = blocks.last().map(|block| block.state.block_id.clone());
    }
    true
}

pub struct BrownieApp {
    rx: Receiver<AppEvent>,
    copilot: CopilotClient,
    connection_state: ConnectionState,
    transcript: Vec<Message>,
    sessions: Vec<SessionMeta>,
    current_session: Option<SessionMeta>,
    input_buffer: String,
    in_progress_assistant: String,
    is_streaming: bool,
    diagnostics_log: Vec<String>,
    workspace: PathBuf,
    instruction_files: Vec<String>,
    scroll_to_bottom: bool,
    session_unavailable: bool,
    theme: Theme,
    catalog_manager: CatalogManager,
    active_intent: Option<UiIntent>,
    selected_template: Option<TemplateSelectionContext>,
    no_matching_template: bool,
    pending_provisional_template: Option<TemplateDocument>,
    canvas_blocks: Vec<CanvasBlock>,
    active_block_id: Option<String>,
    canvas_event_log: UiEventLog,
    block_nonce: u64,
    awaiting_assistant_turn: bool,
    pending_canvas_renders: Vec<CanvasRenderRequest>,
}

impl BrownieApp {
    pub fn new(
        rx: Receiver<AppEvent>,
        copilot: CopilotClient,
        workspace: PathBuf,
        instruction_files: Vec<String>,
    ) -> Self {
        let user_catalog_dir = workspace.join(".brownie").join("catalog");
        let catalog_manager = CatalogManager::with_default_providers(user_catalog_dir, false);
        let (sessions, warnings) = store::load_all();
        let mut app = Self {
            rx,
            copilot,
            connection_state: ConnectionState::Disconnected,
            transcript: Vec::new(),
            sessions,
            current_session: None,
            input_buffer: String::new(),
            in_progress_assistant: String::new(),
            is_streaming: false,
            diagnostics_log: Vec::new(),
            workspace,
            instruction_files,
            scroll_to_bottom: false,
            session_unavailable: false,
            theme: Theme::default(),
            catalog_manager,
            active_intent: None,
            selected_template: None,
            no_matching_template: false,
            pending_provisional_template: None,
            canvas_blocks: Vec::new(),
            active_block_id: None,
            canvas_event_log: UiEventLog::default(),
            block_nonce: 0,
            awaiting_assistant_turn: false,
            pending_canvas_renders: Vec::new(),
        };

        let catalog_diagnostics = app
            .catalog_manager
            .load_diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.to_log_line())
            .collect::<Vec<_>>();
        for diagnostic in catalog_diagnostics {
            app.log_diagnostic(diagnostic);
        }

        for warning in warnings {
            app.apply_event(AppEvent::SdkError(warning), None);
        }

        app
    }

    fn timestamp() -> String {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => duration.as_secs().to_string(),
            Err(_) => "0".to_string(),
        }
    }

    fn log_diagnostic(&mut self, message: impl Into<String>) {
        self.diagnostics_log
            .push(format!("[{}] {}", Self::timestamp(), message.into()));
    }

    fn connection_label(&self) -> (&'static str, egui::Color32) {
        match self.connection_state {
            ConnectionState::Connected => ("Copilot Connected", self.theme.success),
            ConnectionState::Connecting => ("Connecting...", self.theme.warning),
            ConnectionState::Disconnected => ("Disconnected", self.theme.text_muted),
            ConnectionState::Error => ("Copilot Error", self.theme.danger),
        }
    }

    fn connection_state_name(state: ConnectionState) -> &'static str {
        match state {
            ConnectionState::Connected => "connected",
            ConnectionState::Connecting => "connecting",
            ConnectionState::Disconnected => "disconnected",
            ConnectionState::Error => "error",
        }
    }

    fn primary_button(&self, label: &str) -> egui::Button<'static> {
        egui::Button::new(
            RichText::new(label.to_string())
                .size(13.0)
                .color(self.theme.text_on_accent),
        )
        .fill(self.theme.accent_primary)
        .stroke(self.theme.primary_button_stroke())
        .corner_radius(egui::CornerRadius::same(self.theme.radius_8))
    }

    fn secondary_button(&self, label: &str) -> egui::Button<'static> {
        egui::Button::new(
            RichText::new(label.to_string())
                .size(13.0)
                .color(self.theme.text_primary),
        )
        .fill(self.theme.surface_2)
        .stroke(self.theme.subtle_button_stroke())
        .corner_radius(egui::CornerRadius::same(self.theme.radius_8))
    }

    fn refresh_sessions(&mut self) {
        let (sessions, warnings) = store::load_all();
        self.sessions = sessions;
        for warning in warnings {
            self.log_diagnostic(format!("session load warning: {warning}"));
        }
    }

    fn submit_prompt(&mut self, ctx: &egui::Context) {
        let prompt = self.input_buffer.trim().to_string();
        if prompt.is_empty() {
            return;
        }

        let message = Message {
            role: "user".to_string(),
            content: prompt.clone(),
            timestamp: Self::timestamp(),
        };

        self.transcript.push(message.clone());
        if let Some(meta) = self.current_session.as_mut() {
            meta.messages.push(message);
        }
        self.persist_current_session();

        self.copilot.send(prompt);
        self.awaiting_assistant_turn = true;
        self.input_buffer.clear();
        self.scroll_to_bottom = true;
        ctx.request_repaint();
    }

    fn clear_canvas_intent(&mut self) {
        self.active_intent = None;
        self.selected_template = None;
        self.no_matching_template = false;
        self.pending_provisional_template = None;
        self.canvas_blocks.clear();
        self.active_block_id = None;
    }

    fn now_millis() -> u128 {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => duration.as_millis(),
            Err(_) => 0,
        }
    }

    fn next_block_id(&mut self) -> String {
        self.block_nonce = self.block_nonce.saturating_add(1);
        format!("block-{}", self.block_nonce)
    }

    fn active_block_index(&self) -> Option<usize> {
        let active_id = self.active_block_id.as_ref()?;
        self.canvas_blocks
            .iter()
            .position(|block| &block.state.block_id == active_id)
    }

    fn sync_active_selection_context(&mut self) {
        let Some(index) = self.active_block_index() else {
            self.selected_template = None;
            return;
        };

        let block = &self.canvas_blocks[index];
        self.selected_template = Some(TemplateSelectionContext {
            template_id: block.state.template_id.clone(),
            title: block.state.title.clone(),
            provider_id: block.state.provider_id.clone(),
            provider_kind: block.state.provider_kind.clone(),
        });
        self.active_intent = Some(block.state.intent.clone());
    }

    fn snapshot_canvas_workspace(&self) -> CanvasWorkspaceState {
        let mut blocks = Vec::with_capacity(self.canvas_blocks.len());
        for block in &self.canvas_blocks {
            let mut state = block.state.clone();
            state.form_state = block.ui_runtime.form_state_snapshot();
            blocks.push(state);
        }
        CanvasWorkspaceState {
            blocks,
            active_block_id: self.active_block_id.clone(),
        }
    }

    fn persist_current_session(&mut self) {
        let snapshot = self.snapshot_canvas_workspace();
        if let Some(meta) = self.current_session.as_mut() {
            meta.canvas_workspace = snapshot;
            if let Err(err) = store::save(meta) {
                self.log_diagnostic(format!("failed to persist session: {err}"));
            }
        }
    }

    fn restore_canvas_workspace(&mut self, workspace: &CanvasWorkspaceState) {
        self.canvas_blocks.clear();
        self.canvas_event_log = UiEventLog::default();
        self.active_block_id = workspace.active_block_id.clone();

        for state in &workspace.blocks {
            let mut runtime = UiRuntime::new();
            let mut synced_event_count = 0usize;
            if let Err(err) = runtime.load_schema_value(&state.schema) {
                self.log_diagnostic(format!(
                    "failed to restore canvas block {}: {err}",
                    state.block_id
                ));
            } else {
                runtime.restore_form_state(state.form_state.clone());
                synced_event_count = runtime.event_log().len();
            }

            let touched = Self::now_millis();
            self.canvas_blocks.push(CanvasBlock {
                state: state.clone(),
                ui_runtime: runtime,
                synced_event_count,
                last_touched_at: touched,
            });
        }

        if self.active_block_index().is_none() {
            self.active_block_id = self
                .canvas_blocks
                .first()
                .map(|block| block.state.block_id.clone());
        }

        let highest_nonce = self
            .canvas_blocks
            .iter()
            .filter_map(|block| block.state.block_id.strip_prefix("block-"))
            .filter_map(|suffix| suffix.parse::<u64>().ok())
            .max()
            .unwrap_or(0);
        self.block_nonce = highest_nonce;

        self.sync_active_selection_context();
    }

    fn emit_canvas_lifecycle(
        &mut self,
        action: CanvasBlockActionType,
        actor: CanvasBlockActor,
        status: CanvasBlockActionStatus,
        block_id: Option<String>,
        message: Option<String>,
    ) {
        self.canvas_event_log.push(UiEvent::CanvasBlockLifecycle {
            action,
            actor,
            status,
            block_id: block_id.clone(),
            message: message.clone(),
        });

        let mut line = format!(
            "canvas lifecycle action={:?} actor={:?} status={:?} block_id={}",
            action,
            actor,
            status,
            block_id.as_deref().unwrap_or("-")
        );
        if let Some(message) = message {
            line.push_str(&format!(" message={}", message.replace('\n', " ")));
        }
        self.log_diagnostic(line);
    }

    fn resolve_canvas_for_intent(
        &mut self,
        intent: UiIntent,
        actor: CanvasBlockActor,
        target_block_id: Option<String>,
    ) {
        self.active_intent = Some(intent.clone());
        let resolution = self.catalog_manager.resolve(&intent);
        for line in resolution.trace.diagnostic_lines() {
            self.log_diagnostic(line);
        }

        if let Some(template) = resolution.selected {
            self.no_matching_template = false;
            self.pending_provisional_template = None;
            self.selected_template = Some(TemplateSelectionContext {
                template_id: template.document.meta.id.clone(),
                title: template.document.meta.title.clone(),
                provider_id: template.source.provider_id.clone(),
                provider_kind: template.source.kind.as_str().to_string(),
            });

            let schema = self.materialize_template_schema(
                template.document.meta.id.as_str(),
                template.schema_value(),
                None,
            );
            self.apply_canvas_block_from_schema(
                intent,
                template.document.meta.id,
                template.document.meta.title,
                template.source.provider_id,
                template.source.kind.as_str().to_string(),
                schema,
                actor,
                target_block_id,
            );
        } else {
            self.selected_template = None;
            self.no_matching_template = true;
        }
    }

    fn resolve_target_block(&self, template_id: &str) -> BlockTargetResolution {
        resolve_block_target_for_template(
            &self.canvas_blocks,
            self.active_block_id.as_deref(),
            template_id,
        )
    }

    fn apply_canvas_block_from_schema(
        &mut self,
        intent: UiIntent,
        template_id: String,
        title: String,
        provider_id: String,
        provider_kind: String,
        schema: Value,
        actor: CanvasBlockActor,
        target_block_id: Option<String>,
    ) {
        enum UpdateTarget {
            Existing(usize),
            OpenNew,
        }

        let target = if let Some(target_block_id) = target_block_id {
            match self
                .canvas_blocks
                .iter()
                .position(|block| block.state.block_id == target_block_id)
            {
                Some(index) => UpdateTarget::Existing(index),
                None => {
                    self.emit_canvas_lifecycle(
                        CanvasBlockActionType::Update,
                        actor,
                        CanvasBlockActionStatus::Failed,
                        Some(target_block_id),
                        Some("explicit target block_id not found".to_string()),
                    );
                    return;
                }
            }
        } else {
            match self.resolve_target_block(&template_id) {
                BlockTargetResolution::Existing(index) => UpdateTarget::Existing(index),
                BlockTargetResolution::NotFound => UpdateTarget::OpenNew,
                BlockTargetResolution::Ambiguous(block_ids) => {
                    self.emit_canvas_lifecycle(
                        CanvasBlockActionType::Update,
                        actor,
                        CanvasBlockActionStatus::Failed,
                        None,
                        Some(format!(
                            "ambiguous target; specify block_id (candidates: {})",
                            block_ids.join(", ")
                        )),
                    );
                    return;
                }
            }
        };

        if let UpdateTarget::Existing(index) = target {
            let block_id = self.canvas_blocks[index].state.block_id.clone();
            self.emit_canvas_lifecycle(
                CanvasBlockActionType::Update,
                actor,
                CanvasBlockActionStatus::Requested,
                Some(block_id.clone()),
                Some(format!("template_id={template_id}")),
            );

            if let Err(err) = self.canvas_blocks[index]
                .ui_runtime
                .load_schema_value(&schema)
            {
                self.emit_canvas_lifecycle(
                    CanvasBlockActionType::Update,
                    actor,
                    CanvasBlockActionStatus::Failed,
                    Some(block_id),
                    Some(err.to_string()),
                );
                return;
            }

            self.canvas_blocks[index].state.schema = schema;
            self.canvas_blocks[index].state.title = title;
            self.canvas_blocks[index].state.provider_id = provider_id;
            self.canvas_blocks[index].state.provider_kind = provider_kind;
            self.canvas_blocks[index].state.intent = intent;
            self.canvas_blocks[index].state.minimized = false;
            self.canvas_blocks[index].last_touched_at = Self::now_millis();
            self.canvas_blocks[index].synced_event_count = 0;
            self.active_block_id = Some(self.canvas_blocks[index].state.block_id.clone());
            self.sync_active_selection_context();
            self.persist_current_session();
            self.emit_canvas_lifecycle(
                CanvasBlockActionType::Update,
                actor,
                CanvasBlockActionStatus::Succeeded,
                self.active_block_id.clone(),
                None,
            );
            return;
        }

        self.emit_canvas_lifecycle(
            CanvasBlockActionType::Open,
            actor,
            CanvasBlockActionStatus::Requested,
            None,
            Some(format!("template_id={template_id}")),
        );

        let mut runtime = UiRuntime::new();
        if let Err(err) = runtime.load_schema_value(&schema) {
            self.emit_canvas_lifecycle(
                CanvasBlockActionType::Open,
                actor,
                CanvasBlockActionStatus::Failed,
                None,
                Some(err.to_string()),
            );
            return;
        }

        let block_id = self.next_block_id();
        let block = CanvasBlock {
            state: CanvasBlockState {
                block_id: block_id.clone(),
                template_id: template_id.clone(),
                title,
                provider_id,
                provider_kind,
                schema,
                intent,
                minimized: false,
                form_state: runtime.form_state_snapshot(),
            },
            ui_runtime: runtime,
            synced_event_count: 0,
            last_touched_at: Self::now_millis(),
        };
        self.canvas_blocks.push(block);
        self.active_block_id = Some(block_id.clone());
        self.sync_active_selection_context();
        self.persist_current_session();
        self.emit_canvas_lifecycle(
            CanvasBlockActionType::Open,
            actor,
            CanvasBlockActionStatus::Succeeded,
            Some(block_id),
            Some(format!("template_id={template_id}")),
        );
    }

    fn focus_block(&mut self, block_id: &str, actor: CanvasBlockActor) {
        self.emit_canvas_lifecycle(
            CanvasBlockActionType::Focus,
            actor,
            CanvasBlockActionStatus::Requested,
            Some(block_id.to_string()),
            None,
        );

        if !apply_focus_transition(
            &mut self.canvas_blocks,
            &mut self.active_block_id,
            block_id,
            Self::now_millis(),
        ) {
            self.emit_canvas_lifecycle(
                CanvasBlockActionType::Focus,
                actor,
                CanvasBlockActionStatus::Failed,
                Some(block_id.to_string()),
                Some("block not found".to_string()),
            );
            return;
        }

        self.sync_active_selection_context();
        self.persist_current_session();
        self.emit_canvas_lifecycle(
            CanvasBlockActionType::Focus,
            actor,
            CanvasBlockActionStatus::Succeeded,
            Some(block_id.to_string()),
            None,
        );
    }

    fn toggle_minimize_block(&mut self, block_id: &str, actor: CanvasBlockActor) {
        self.emit_canvas_lifecycle(
            CanvasBlockActionType::Minimize,
            actor,
            CanvasBlockActionStatus::Requested,
            Some(block_id.to_string()),
            None,
        );

        let Some(minimized) =
            apply_toggle_minimize_transition(&mut self.canvas_blocks, block_id, Self::now_millis())
        else {
            self.emit_canvas_lifecycle(
                CanvasBlockActionType::Minimize,
                actor,
                CanvasBlockActionStatus::Failed,
                Some(block_id.to_string()),
                Some("block not found".to_string()),
            );
            return;
        };

        self.persist_current_session();
        self.emit_canvas_lifecycle(
            CanvasBlockActionType::Minimize,
            actor,
            CanvasBlockActionStatus::Succeeded,
            Some(block_id.to_string()),
            Some(if minimized {
                "minimized".to_string()
            } else {
                "expanded".to_string()
            }),
        );
    }

    fn close_block(&mut self, block_id: &str, actor: CanvasBlockActor) {
        self.emit_canvas_lifecycle(
            CanvasBlockActionType::Close,
            actor,
            CanvasBlockActionStatus::Requested,
            Some(block_id.to_string()),
            None,
        );

        if !apply_close_transition(&mut self.canvas_blocks, &mut self.active_block_id, block_id) {
            self.emit_canvas_lifecycle(
                CanvasBlockActionType::Close,
                actor,
                CanvasBlockActionStatus::Failed,
                Some(block_id.to_string()),
                Some("block not found".to_string()),
            );
            return;
        }

        self.sync_active_selection_context();
        self.persist_current_session();
        self.emit_canvas_lifecycle(
            CanvasBlockActionType::Close,
            actor,
            CanvasBlockActionStatus::Succeeded,
            Some(block_id.to_string()),
            None,
        );
    }

    fn save_pending_provisional_template(&mut self) {
        let Some(template) = self.pending_provisional_template.clone() else {
            return;
        };

        match self.catalog_manager.upsert_user_template(&template) {
            Ok(()) => {
                self.log_diagnostic(format!(
                    "saved provisional template to user catalog: {}",
                    template.meta.id
                ));
                self.pending_provisional_template = None;
                let intent = UiIntent::new(
                    template.match_rules.primary,
                    template.match_rules.operations,
                    template.match_rules.tags,
                );
                self.resolve_canvas_for_intent(intent, CanvasBlockActor::System, None);
            }
            Err(err) => {
                self.log_diagnostic(format!("failed to save provisional template: {err}"));
            }
        }
    }

    fn materialize_template_schema(
        &self,
        template_id: &str,
        schema: &Value,
        root_path: Option<&str>,
    ) -> Value {
        if template_id != "builtin.file_listing.default" {
            return schema.clone();
        }

        let mut materialized = schema.clone();
        let listing = self.file_explorer_listing(root_path);
        let root_label = self.file_explorer_root_label(root_path);
        if let Some(components) = materialized
            .get_mut("components")
            .and_then(|value| value.as_array_mut())
        {
            components.retain(|component| {
                matches!(
                    component.get("id").and_then(|value| value.as_str()),
                    Some("explorer_intro") | Some("workspace_tree")
                )
            });
            for component in components {
                let is_workspace_tree = component
                    .get("id")
                    .and_then(|value| value.as_str())
                    .map(|id| id == "workspace_tree")
                    .unwrap_or(false);
                let is_intro = component
                    .get("id")
                    .and_then(|value| value.as_str())
                    .map(|id| id == "explorer_intro")
                    .unwrap_or(false);
                if is_workspace_tree {
                    if let Some(code) = component.get_mut("code") {
                        *code = Value::String(listing.clone());
                    }
                }
                if is_intro {
                    if let Some(text) = component.get_mut("text") {
                        *text = Value::String(
                            format!(
                                "### File Explorer\nRoot: `{root_label}`\nPersistent session block. Use focus/minimize/close controls."
                            ),
                        );
                    }
                }
            }
        }

        materialized
    }

    fn file_explorer_root_path(&self, root_path: Option<&str>) -> PathBuf {
        let Some(root_path) = root_path.map(str::trim).filter(|value| !value.is_empty()) else {
            return self.workspace.clone();
        };

        let candidate = PathBuf::from(root_path);
        if candidate.is_absolute() {
            candidate
        } else {
            self.workspace.join(candidate)
        }
    }

    fn file_explorer_root_label(&self, root_path: Option<&str>) -> String {
        self.file_explorer_root_path(root_path)
            .display()
            .to_string()
    }

    fn file_explorer_listing(&self, root_path: Option<&str>) -> String {
        let root = self.file_explorer_root_path(root_path);
        let root_name = root
            .file_name()
            .and_then(|value| value.to_str())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| root.display().to_string());

        let mut entries = Vec::new();
        match fs::read_dir(&root) {
            Ok(read_dir) => {
                for entry in read_dir.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let is_dir = entry
                        .file_type()
                        .map(|value| value.is_dir())
                        .unwrap_or(false);
                    entries.push((name, is_dir));
                }
            }
            Err(err) => {
                return format!("{root_name}/\n└── <failed to read root: {err}>");
            }
        }

        entries.sort_by(|left, right| left.0.cmp(&right.0));
        let mut lines = vec![format!("{root_name}/")];
        for (index, (name, is_dir)) in entries.iter().enumerate() {
            let branch = if index + 1 == entries.len() {
                "└──"
            } else {
                "├──"
            };
            let suffix = if *is_dir { "/" } else { "" };
            lines.push(format!("{branch} {name}{suffix}"));
        }

        lines.join("\n")
    }

    fn open_session(&mut self, session_id: &str) {
        let (session, warning) = store::load_one(session_id);
        if let Some(warning) = warning {
            self.apply_event(AppEvent::SdkError(warning), None);
        }

        if let Some(session) = session {
            self.transcript = session.messages.clone();
            self.restore_canvas_workspace(&session.canvas_workspace);
            self.current_session = Some(session);
            self.is_streaming = false;
            self.in_progress_assistant.clear();
            self.scroll_to_bottom = true;
            self.session_unavailable = false;
            self.awaiting_assistant_turn = false;
            self.pending_canvas_renders.clear();
        } else {
            self.session_unavailable = true;
            self.clear_canvas_intent();
            self.canvas_event_log = UiEventLog::default();
            self.awaiting_assistant_turn = false;
            self.pending_canvas_renders.clear();
        }
    }

    fn apply_canvas_render_request(
        &mut self,
        request: CanvasRenderRequest,
        ctx: Option<&egui::Context>,
    ) {
        self.active_intent = Some(request.intent.clone());
        self.no_matching_template = false;
        self.pending_provisional_template = request.provisional_template;

        let schema = self.materialize_template_schema(
            &request.template_id,
            &request.schema,
            request.root_path.as_deref(),
        );
        self.apply_canvas_block_from_schema(
            request.intent,
            request.template_id,
            request.title,
            request.provider_id,
            request.provider_kind,
            schema,
            CanvasBlockActor::Assistant,
            request.target_block_id,
        );
        if let Some(ctx) = ctx {
            ctx.request_repaint();
        }
    }

    fn flush_pending_canvas_renders(&mut self, ctx: Option<&egui::Context>) {
        let pending = std::mem::take(&mut self.pending_canvas_renders);
        for render in pending {
            self.apply_canvas_render_request(render, ctx);
        }
    }

    fn drain_events(&mut self, ctx: &egui::Context) {
        loop {
            match self.rx.try_recv() {
                Ok(event) => self.apply_event(event, Some(ctx)),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.log_diagnostic("event channel disconnected");
                    break;
                }
            }
        }
    }

    fn apply_event(&mut self, event: AppEvent, ctx: Option<&egui::Context>) {
        match event {
            AppEvent::StreamDelta(text) => {
                self.in_progress_assistant.push_str(&text);
                self.is_streaming = true;
                self.scroll_to_bottom = true;
                if let Some(ctx) = ctx {
                    ctx.request_repaint();
                }
            }
            AppEvent::StreamEnd => {
                if !self.in_progress_assistant.is_empty() {
                    let message = Message {
                        role: "assistant".to_string(),
                        content: std::mem::take(&mut self.in_progress_assistant),
                        timestamp: Self::timestamp(),
                    };
                    self.transcript.push(message.clone());
                    if let Some(meta) = self.current_session.as_mut() {
                        meta.messages.push(message);
                    }
                    self.persist_current_session();
                }

                self.is_streaming = false;
                self.awaiting_assistant_turn = false;
                self.flush_pending_canvas_renders(ctx);
                self.scroll_to_bottom = true;
                if let Some(ctx) = ctx {
                    ctx.request_repaint();
                }
            }
            AppEvent::StatusChanged(state) => {
                self.connection_state = state;
                self.log_diagnostic(format!(
                    "connection state changed: {}",
                    Self::connection_state_name(state)
                ));
            }
            AppEvent::SdkError(message) => {
                self.log_diagnostic(format!("sdk error: {message}"));
                self.is_streaming = false;
                self.awaiting_assistant_turn = false;
                self.flush_pending_canvas_renders(ctx);
            }
            AppEvent::SessionCreated(session_id) => {
                let meta = SessionMeta {
                    schema_version: SCHEMA_VERSION,
                    session_id: session_id.clone(),
                    workspace: self.workspace.to_string_lossy().to_string(),
                    title: Some(format!(
                        "Session {}",
                        session_id.chars().take(8).collect::<String>()
                    )),
                    created_at: Self::timestamp(),
                    canvas_workspace: CanvasWorkspaceState::default(),
                    messages: Vec::new(),
                };

                self.current_session = Some(meta.clone());
                self.transcript.clear();
                self.in_progress_assistant.clear();
                self.is_streaming = false;
                self.session_unavailable = false;
                self.awaiting_assistant_turn = false;
                self.pending_canvas_renders.clear();
                self.clear_canvas_intent();
                self.canvas_event_log = UiEventLog::default();

                if let Err(err) = store::save(&meta) {
                    self.log_diagnostic(format!("failed to persist new session: {err}"));
                }

                self.refresh_sessions();
            }
            AppEvent::ToolCallSuppressed(tool_name) => {
                self.log_diagnostic(format!("tool call suppressed (passive mode): {tool_name}"));
            }
            AppEvent::ToolExecutionOutcome {
                tool_name,
                status,
                message,
            } => {
                let mut diagnostic = format!("tool outcome tool={} status={}", tool_name, status);
                if tool_name == "query_ui_catalog" && (status == "text_only" || status == "error") {
                    diagnostic.push_str(" canvas_not_rendered=true");
                }
                if let Some(message) = message {
                    let compact = message.replace('\n', " ");
                    diagnostic.push_str(&format!(" message={compact}"));
                }
                self.log_diagnostic(diagnostic);
            }
            AppEvent::CanvasToolRender {
                intent,
                template_id,
                title,
                provider_id,
                provider_kind,
                target_block_id,
                root_path,
                schema,
                provisional_template,
            } => {
                let request = CanvasRenderRequest {
                    intent,
                    template_id,
                    title,
                    provider_id,
                    provider_kind,
                    target_block_id,
                    root_path,
                    schema,
                    provisional_template,
                };
                if self.awaiting_assistant_turn || self.is_streaming {
                    self.log_diagnostic("deferred canvas render until assistant turn completed");
                    self.pending_canvas_renders.push(request);
                } else {
                    self.apply_canvas_render_request(request, ctx);
                }
            }
        }
    }

    fn render_top_bar(&mut self, ctx: &egui::Context) {
        let (status_label, status_color) = self.connection_label();
        let top_frame = Frame::new()
            .inner_margin(egui::Margin::symmetric(
                self.theme.spacing_16 as i8,
                self.theme.spacing_8 as i8,
            ))
            .fill(self.theme.surface_1);

        egui::TopBottomPanel::top("top_bar")
            .exact_height(44.0)
            .frame(top_frame)
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let split_y = rect.center().y;
                let top_rect = egui::Rect::from_min_max(rect.min, egui::pos2(rect.max.x, split_y));
                let bottom_rect =
                    egui::Rect::from_min_max(egui::pos2(rect.min.x, split_y), rect.max);
                ui.painter()
                    .rect_filled(top_rect, egui::CornerRadius::ZERO, self.theme.surface_1);
                ui.painter().rect_filled(
                    bottom_rect,
                    egui::CornerRadius::ZERO,
                    self.theme.top_bar_gradient_end,
                );

                ui.columns(3, |columns| {
                    columns[0].with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                        ui.label(
                            RichText::new("Brownie")
                                .size(14.0)
                                .color(self.theme.text_primary),
                        );
                    });

                    columns[1].with_layout(
                        egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                        |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("●").color(status_color).size(10.0));
                                ui.label(
                                    RichText::new(status_label)
                                        .size(13.0)
                                        .color(self.theme.text_primary),
                                );
                            });
                        },
                    );

                    columns[2].with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        ui.add_enabled(false, self.secondary_button("Active Mode"));
                        ui.label(
                            RichText::new("Passive Mode")
                                .size(12.0)
                                .color(self.theme.success),
                        );
                    });
                });
            });
    }

    fn render_left_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("workspace_panel")
            .resizable(true)
            .frame(
                self.theme
                    .panel_frame(self.theme.surface_1, self.theme.spacing_16 as i8),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(Theme::P8, Theme::P8);
                ui.label(
                    RichText::new("Workspace")
                        .strong()
                        .size(16.0)
                        .color(self.theme.text_primary),
                );

                self.theme.card_frame().show(ui, |ui| {
                    ui.label(
                        RichText::new(self.workspace.display().to_string())
                            .size(12.0)
                            .color(self.theme.text_muted),
                    );
                });

                self.theme.card_frame().show(ui, |ui| {
                    ui.label(
                        RichText::new("Copilot Instructions")
                            .strong()
                            .size(14.0)
                            .color(self.theme.text_primary),
                    );
                    ui.add_space(Theme::P8);
                    if self.instruction_files.is_empty() {
                        ui.label(
                            RichText::new("No instruction files detected")
                                .size(12.0)
                                .color(self.theme.text_muted),
                        );
                    } else {
                        for path in &self.instruction_files {
                            ui.label(RichText::new(path).size(12.0).color(self.theme.text_muted));
                        }
                    }
                });

                ui.add_space(Theme::P8);
                ui.label(
                    RichText::new("Recent Sessions")
                        .strong()
                        .size(14.0)
                        .color(self.theme.text_primary),
                );
                let mut clicked_session: Option<String> = None;
                let active_session_id = self
                    .current_session
                    .as_ref()
                    .map(|session| &session.session_id);
                let sessions_height = (ui.available_height() - Theme::P8).max(120.0);
                self.theme.card_frame().show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(Theme::P8, Theme::P8);
                    ScrollArea::vertical()
                        .id_salt("recent_sessions_scroll")
                        .max_height(sessions_height)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for session in &self.sessions {
                                let label = session
                                    .title
                                    .clone()
                                    .unwrap_or_else(|| session.session_id.clone());
                                let is_active = active_session_id
                                    .map(|current| current == &session.session_id)
                                    .unwrap_or(false);

                                let base_fill = if is_active {
                                    self.theme.surface_3
                                } else {
                                    self.theme.surface_2
                                };
                                let button = egui::Button::new(
                                    RichText::new(label)
                                        .size(13.0)
                                        .color(self.theme.text_primary),
                                )
                                .fill(base_fill)
                                .stroke(Stroke::NONE)
                                .corner_radius(egui::CornerRadius::same(self.theme.radius_10))
                                .min_size(egui::vec2(ui.available_width(), 34.0));
                                let response = ui.add(button);

                                if !is_active && response.hovered() {
                                    ui.painter().rect_filled(
                                        response.rect,
                                        egui::CornerRadius::same(self.theme.radius_10),
                                        self.theme.hover_overlay,
                                    );
                                }
                                if is_active {
                                    let accent_rect = egui::Rect::from_min_max(
                                        response.rect.min + egui::vec2(4.0, 5.0),
                                        egui::pos2(
                                            response.rect.min.x + 7.0,
                                            response.rect.max.y - 5.0,
                                        ),
                                    );
                                    ui.painter().rect_filled(
                                        accent_rect,
                                        egui::CornerRadius::same(2),
                                        self.theme.accent_primary,
                                    );
                                }

                                if response.clicked() {
                                    clicked_session = Some(session.session_id.clone());
                                }
                            }
                        });
                });

                if let Some(session_id) = clicked_session {
                    self.open_session(&session_id);
                }
            });
    }

    fn render_right_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("actions_panel")
            .resizable(true)
            .frame(
                self.theme
                    .panel_frame(self.theme.surface_1, self.theme.spacing_24 as i8),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(Theme::P12, Theme::P12);
                ui.label(
                    RichText::new("Canvas")
                        .strong()
                        .size(16.0)
                        .color(self.theme.text_primary),
                );

                let mut focus_block: Option<String> = None;
                let mut toggle_block: Option<String> = None;
                let mut close_block: Option<String> = None;
                let mut new_events: Vec<UiEvent> = Vec::new();
                let mut save_provisional = false;
                let mut dismiss_provisional = false;

                ScrollArea::vertical()
                    .id_salt("canvas_panel_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.theme.card_frame().show(ui, |ui| {
                            egui::CollapsingHeader::new(
                                RichText::new("Selection Context")
                                    .strong()
                                    .size(14.0)
                                    .color(self.theme.text_primary),
                            )
                            .id_salt("selection_context")
                            .default_open(false)
                            .show(ui, |ui| {
                                ui.add_space(Theme::P8);
                                ui.label(
                                    RichText::new(match &self.active_intent {
                                        Some(intent) => format!("Intent: {}", intent.summary()),
                                        None => "Intent: none".to_string(),
                                    })
                                    .size(12.0)
                                    .color(self.theme.text_muted),
                                );
                                if let Some(selection) = &self.selected_template {
                                    ui.label(
                                        RichText::new(format!(
                                            "Template: {} ({})",
                                            selection.title, selection.template_id
                                        ))
                                        .size(13.0)
                                        .color(self.theme.text_primary),
                                    );
                                    ui.label(
                                        RichText::new(format!(
                                            "Source: {} [{}]",
                                            selection.provider_id, selection.provider_kind
                                        ))
                                        .size(12.0)
                                        .color(self.theme.text_muted),
                                    );
                                }
                            });
                        });

                        self.theme.card_frame().show(ui, |ui| {
                            ui.label(
                                RichText::new("Workspace Blocks")
                                    .strong()
                                    .size(14.0)
                                    .color(self.theme.text_primary),
                            );
                            ui.add_space(Theme::P8);
                            if self.canvas_blocks.is_empty() {
                                if self.no_matching_template {
                                    ui.label(
                                        RichText::new("No matching UI template found")
                                            .size(13.0)
                                            .color(self.theme.danger),
                                    );
                                } else {
                                    ui.label(
                                        RichText::new("No open Canvas blocks")
                                            .size(13.0)
                                            .color(self.theme.text_muted),
                                    );
                                }
                            } else {
                                for index in 0..self.canvas_blocks.len() {
                                    let block_id = self.canvas_blocks[index].state.block_id.clone();
                                    let block_title = self.canvas_blocks[index].state.title.clone();
                                    let provider_id =
                                        self.canvas_blocks[index].state.provider_id.clone();
                                    let provider_kind =
                                        self.canvas_blocks[index].state.provider_kind.clone();
                                    let is_minimized = self.canvas_blocks[index].state.minimized;
                                    let is_active =
                                        self.active_block_id.as_deref() == Some(block_id.as_str());
                                    let border_color = if is_active {
                                        self.theme.accent_primary
                                    } else {
                                        self.theme.border_subtle
                                    };
                                    Frame::new()
                                        .fill(self.theme.surface_2)
                                        .stroke(Stroke::new(1.0, border_color))
                                        .corner_radius(egui::CornerRadius::same(
                                            self.theme.radius_10,
                                        ))
                                        .inner_margin(egui::Margin::same(
                                            self.theme.spacing_12 as i8,
                                        ))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{} ({})",
                                                        block_title, block_id
                                                    ))
                                                    .size(13.0)
                                                    .color(self.theme.text_primary),
                                                );
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(Align::Center),
                                                    |ui| {
                                                        if ui
                                                            .small_button("x")
                                                            .on_hover_text("Close block")
                                                            .clicked()
                                                        {
                                                            close_block = Some(block_id.clone());
                                                        }
                                                        if ui
                                                            .small_button(if is_minimized {
                                                                "+"
                                                            } else {
                                                                "-"
                                                            })
                                                            .on_hover_text(if is_minimized {
                                                                "Expand block"
                                                            } else {
                                                                "Minimize block"
                                                            })
                                                            .clicked()
                                                        {
                                                            toggle_block = Some(block_id.clone());
                                                        }
                                                        if !is_active
                                                            && ui
                                                                .small_button("o")
                                                                .on_hover_text("Focus block")
                                                                .clicked()
                                                        {
                                                            focus_block = Some(block_id.clone());
                                                        }
                                                    },
                                                );
                                            });
                                            ui.label(
                                                RichText::new(format!(
                                                    "Source: {} [{}]",
                                                    provider_id, provider_kind
                                                ))
                                                .size(12.0)
                                                .color(self.theme.text_muted),
                                            );
                                            if is_minimized {
                                                ui.label(
                                                    RichText::new("Block is minimized")
                                                        .size(12.0)
                                                        .color(self.theme.text_muted),
                                                );
                                            } else {
                                                ui.add_space(Theme::P8);
                                                let block = &mut self.canvas_blocks[index];
                                                block.ui_runtime.render_canvas(ui, &self.theme);
                                                let events = block.ui_runtime.event_log();
                                                if block.synced_event_count < events.len() {
                                                    new_events.extend_from_slice(
                                                        &events[block.synced_event_count..],
                                                    );
                                                    block.synced_event_count = events.len();
                                                }
                                            }
                                        });
                                    ui.add_space(Theme::P8);
                                }
                            }
                        });

                        if let Some(template) = &self.pending_provisional_template {
                            self.theme.card_frame().show(ui, |ui| {
                                ui.label(
                                    RichText::new("Provisional Template")
                                        .strong()
                                        .size(14.0)
                                        .color(self.theme.text_primary),
                                );
                                ui.add_space(Theme::P8);
                                ui.label(
                                    RichText::new(format!(
                                        "Save '{}' to your user UI catalog?",
                                        template.meta.title
                                    ))
                                    .size(12.0)
                                    .color(self.theme.text_muted),
                                );
                                ui.add_space(Theme::P8);
                                ui.horizontal(|ui| {
                                    if ui.add(self.primary_button("Save to Catalog")).clicked() {
                                        save_provisional = true;
                                    }
                                    if ui.add(self.secondary_button("Not Now")).clicked() {
                                        dismiss_provisional = true;
                                    }
                                });
                            });
                        }

                        self.theme.card_frame().show(ui, |ui| {
                            egui::CollapsingHeader::new(
                                RichText::new("UI Event Log")
                                    .color(self.theme.text_primary)
                                    .size(13.0),
                            )
                            .id_salt("ui_event_log")
                            .default_open(false)
                            .show(ui, |ui| {
                                ui.add_space(Theme::P8);
                                for event in self.canvas_event_log.entries() {
                                    ui.label(
                                        RichText::new(event.to_log_line())
                                            .color(self.theme.text_muted)
                                            .size(12.0),
                                    );
                                }
                            });
                        });
                    });

                let had_new_events = !new_events.is_empty();
                for event in new_events {
                    self.canvas_event_log.push(event);
                }
                if had_new_events {
                    self.persist_current_session();
                }

                if let Some(block_id) = focus_block {
                    self.focus_block(&block_id, CanvasBlockActor::User);
                }
                if let Some(block_id) = toggle_block {
                    self.toggle_minimize_block(&block_id, CanvasBlockActor::User);
                }
                if let Some(block_id) = close_block {
                    self.close_block(&block_id, CanvasBlockActor::User);
                }

                if save_provisional {
                    self.save_pending_provisional_template();
                } else if dismiss_provisional {
                    self.pending_provisional_template = None;
                }
            });
    }

    fn render_center_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                self.theme
                    .panel_frame(self.theme.surface_1, self.theme.spacing_16 as i8),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(Theme::P12, Theme::P12);
                ui.label(
                    RichText::new("Chat")
                        .strong()
                        .size(16.0)
                        .color(self.theme.text_primary),
                );

                let transcript_height = (ui.available_height() - 260.0).max(140.0);
                ScrollArea::vertical()
                    .id_salt("chat_transcript")
                    .max_height(transcript_height)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if self.session_unavailable {
                            ui.label(
                                RichText::new("Session unavailable")
                                    .size(12.0)
                                    .color(self.theme.danger),
                            );
                        }

                        ui.spacing_mut().item_spacing.y = Theme::P12;
                        for message in &self.transcript {
                            let is_user = message.role == "user";
                            let bubble = Frame::new()
                                .fill(if is_user {
                                    self.theme.surface_2
                                } else {
                                    self.theme.surface_3
                                })
                                .corner_radius(egui::CornerRadius::same(self.theme.radius_12))
                                .stroke(Stroke::NONE)
                                .inner_margin(egui::Margin::same(self.theme.spacing_12 as i8));

                            if is_user {
                                ui.horizontal(|ui| {
                                    ui.add_space(self.theme.spacing_24);
                                    bubble.show(ui, |ui| {
                                        ui.label(
                                            RichText::new(format!("[You] {}", message.content))
                                                .size(14.0)
                                                .color(self.theme.text_primary),
                                        );
                                    });
                                });
                            } else {
                                bubble.show(ui, |ui| {
                                    ui.label(
                                        RichText::new(format!("[Copilot] {}", message.content))
                                            .size(14.0)
                                            .color(self.theme.text_primary),
                                    );
                                });
                            }
                        }

                        if self.is_streaming && !self.in_progress_assistant.is_empty() {
                            Frame::new()
                                .fill(self.theme.surface_3)
                                .corner_radius(egui::CornerRadius::same(self.theme.radius_12))
                                .stroke(Stroke::NONE)
                                .inner_margin(egui::Margin::same(self.theme.spacing_12 as i8))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "[Copilot] {}",
                                            self.in_progress_assistant
                                        ))
                                        .size(14.0)
                                        .color(self.theme.text_primary),
                                    );
                                });
                        }

                        if self.scroll_to_bottom {
                            ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                        }
                    });
                self.scroll_to_bottom = false;

                self.theme.card_frame().show(ui, |ui| {
                    egui::CollapsingHeader::new(
                        RichText::new("Diagnostics")
                            .size(14.0)
                            .strong()
                            .color(self.theme.text_primary),
                    )
                    .default_open(false)
                    .show(ui, |ui| {
                        ScrollArea::vertical()
                            .id_salt("diagnostics_log")
                            .max_height(100.0)
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                for entry in &self.diagnostics_log {
                                    ui.label(
                                        RichText::new(entry)
                                            .size(12.0)
                                            .color(self.theme.text_muted),
                                    );
                                }
                            });
                    });
                });

                let connected = self.connection_state == ConnectionState::Connected;
                let input_enabled = connected && !self.is_streaming;
                let hint = if !connected {
                    "Not connected"
                } else if self.is_streaming {
                    "Waiting for response..."
                } else {
                    "Type a message..."
                };

                let mut send_now = false;
                self.theme.composer_frame().show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(Theme::P8, Theme::P8);
                    let response = ui
                        .add_enabled_ui(input_enabled, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.input_buffer)
                                    .hint_text(hint)
                                    .desired_rows(4)
                                    .desired_width(f32::INFINITY)
                                    .lock_focus(true),
                            )
                        })
                        .inner;

                    if response.has_focus() {
                        let glow_rect = response.rect.expand(2.0);
                        ui.painter().rect_stroke(
                            glow_rect,
                            egui::CornerRadius::same(self.theme.radius_10),
                            Stroke::new(1.0, self.theme.input_focus_glow),
                            egui::StrokeKind::Outside,
                        );
                        ui.input(|input| {
                            if input.key_pressed(egui::Key::Enter)
                                && (input.modifiers.ctrl || input.modifiers.command)
                            {
                                send_now = true;
                            }
                        });
                    }

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Ctrl+Enter to send")
                                .size(12.0)
                                .color(self.theme.text_muted),
                        );
                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                            let clicked = ui
                                .add_enabled_ui(
                                    input_enabled && !self.input_buffer.trim().is_empty(),
                                    |ui| {
                                        ui.add_sized(
                                            [96.0, self.theme.button_height],
                                            self.primary_button("Send"),
                                        )
                                    },
                                )
                                .inner
                                .clicked();
                            send_now |= clicked;
                        });
                    });
                });

                if send_now && input_enabled {
                    self.submit_prompt(ctx);
                }
            });
    }
}

impl eframe::App for BrownieApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.theme.apply_visuals(ctx);
        let bg_painter = ctx.layer_painter(egui::LayerId::background());
        bg_painter.rect_filled(
            ctx.screen_rect(),
            egui::CornerRadius::ZERO,
            self.theme.surface_0,
        );
        self.drain_events(ctx);
        self.render_top_bar(ctx);
        self.render_left_panel(ctx);
        self.render_right_panel(ctx);
        self.render_center_panel(ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_close_transition, apply_focus_transition, apply_toggle_minimize_transition,
        resolve_block_target_for_template, BlockTargetResolution, CanvasBlock,
    };
    use crate::ui::catalog::UiIntent;
    use crate::ui::runtime::UiRuntime;
    use crate::ui::workspace::CanvasBlockState;
    use serde_json::json;
    use std::collections::BTreeMap;

    fn block(block_id: &str, template_id: &str, touched: u128) -> CanvasBlock {
        CanvasBlock {
            state: CanvasBlockState {
                block_id: block_id.to_string(),
                template_id: template_id.to_string(),
                title: block_id.to_string(),
                provider_id: "builtin-default".to_string(),
                provider_kind: "builtin".to_string(),
                schema: json!({
                    "schema_version": 1,
                    "outputs": [],
                    "components": [
                        {
                            "id": "intro",
                            "kind": "markdown",
                            "text": "hello"
                        }
                    ]
                }),
                intent: UiIntent::new("file_listing", vec!["list".to_string()], vec![]),
                minimized: false,
                form_state: BTreeMap::new(),
            },
            ui_runtime: UiRuntime::new(),
            synced_event_count: 0,
            last_touched_at: touched,
        }
    }

    #[test]
    fn target_selection_prefers_active_matching_block() {
        let blocks = vec![
            block("block-1", "builtin.file_listing.default", 1),
            block("block-2", "builtin.file_listing.default", 999),
        ];
        let selected = resolve_block_target_for_template(
            &blocks,
            Some("block-1"),
            "builtin.file_listing.default",
        );
        assert_eq!(selected, BlockTargetResolution::Existing(0));
    }

    #[test]
    fn target_selection_falls_back_to_unique_most_recent_matching_block() {
        let blocks = vec![
            block("block-1", "builtin.file_listing.default", 1),
            block("block-2", "builtin.file_listing.default", 999),
            block("block-3", "builtin.plan_review.default", 2000),
        ];
        let selected = resolve_block_target_for_template(
            &blocks,
            Some("block-3"),
            "builtin.file_listing.default",
        );
        assert_eq!(selected, BlockTargetResolution::Existing(1));
    }

    #[test]
    fn target_selection_fails_when_recent_candidates_are_ambiguous() {
        let blocks = vec![
            block("block-1", "builtin.file_listing.default", 777),
            block("block-2", "builtin.file_listing.default", 777),
        ];
        let selected =
            resolve_block_target_for_template(&blocks, None, "builtin.file_listing.default");
        assert_eq!(
            selected,
            BlockTargetResolution::Ambiguous(vec!["block-1".to_string(), "block-2".to_string()])
        );
    }

    #[test]
    fn focus_transition_sets_active_without_removing_blocks() {
        let mut blocks = vec![
            block("block-1", "builtin.file_listing.default", 1),
            block("block-2", "builtin.plan_review.default", 2),
        ];
        let mut active = Some("block-1".to_string());

        assert!(apply_focus_transition(
            &mut blocks,
            &mut active,
            "block-2",
            5000,
        ));
        assert_eq!(active.as_deref(), Some("block-2"));
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[1].last_touched_at, 5000);
    }

    #[test]
    fn minimize_transition_toggles_without_removing_block() {
        let mut blocks = vec![block("block-1", "builtin.file_listing.default", 1)];
        let minimized = apply_toggle_minimize_transition(&mut blocks, "block-1", 100);
        assert_eq!(minimized, Some(true));
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].state.minimized);
    }

    #[test]
    fn close_transition_removes_only_target_and_updates_active_fallback() {
        let mut blocks = vec![
            block("block-1", "builtin.file_listing.default", 1),
            block("block-2", "builtin.plan_review.default", 2),
            block("block-3", "builtin.status.default", 3),
        ];
        let mut active = Some("block-2".to_string());

        assert!(apply_close_transition(&mut blocks, &mut active, "block-2"));
        assert_eq!(blocks.len(), 2);
        assert!(blocks.iter().all(|block| block.state.block_id != "block-2"));
        assert_eq!(active.as_deref(), Some("block-3"));
    }
}
