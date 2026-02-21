use crate::copilot::CopilotClient;
use crate::event::AppEvent;
use crate::session::store;
use crate::session::{Message, SessionMeta, SCHEMA_VERSION};
use crate::theme::Theme;
use crate::ui::catalog::{CatalogManager, TemplateDocument, UiIntent};
use crate::ui::runtime::UiRuntime;
use copilot_sdk::ConnectionState;
use eframe::egui::{self, Align, Frame, RichText, ScrollArea, Stroke};
use serde_json::Value;
use std::collections::BTreeSet;
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
    ui_runtime: UiRuntime,
    catalog_manager: CatalogManager,
    active_intent: Option<UiIntent>,
    selected_template: Option<TemplateSelectionContext>,
    no_matching_template: bool,
    pending_provisional_template: Option<TemplateDocument>,
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
            ui_runtime: UiRuntime::new(),
            catalog_manager,
            active_intent: None,
            selected_template: None,
            no_matching_template: false,
            pending_provisional_template: None,
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
        if let Some(intent) = Self::intent_from_prompt(&prompt) {
            self.resolve_canvas_for_intent(intent);
        } else {
            self.clear_canvas_intent();
            self.log_diagnostic("catalog resolve skipped: no intent detected");
        }

        let message = Message {
            role: "user".to_string(),
            content: prompt.clone(),
            timestamp: Self::timestamp(),
        };

        self.transcript.push(message.clone());
        if let Some(meta) = self.current_session.as_mut() {
            meta.messages.push(message);
            if let Err(err) = store::save(meta) {
                self.log_diagnostic(format!("failed to persist session: {err}"));
            }
        }

        self.copilot.send(prompt);
        self.input_buffer.clear();
        self.scroll_to_bottom = true;
        ctx.request_repaint();
    }

    fn intent_from_prompt(prompt: &str) -> Option<UiIntent> {
        let lowered = prompt.to_ascii_lowercase();
        let primary = if lowered.contains("list files")
            || lowered.contains("listing of files")
            || lowered.contains("file tree")
            || lowered.contains("directory tree")
            || lowered.contains("show files")
            || lowered.contains("show me files")
            || lowered.contains("all the files")
            || lowered.contains("all files")
            || lowered.contains("workspace files")
            || (lowered.contains("files") && lowered.contains("canvas"))
            || (lowered.contains("files") && lowered.contains("workspace"))
        {
            "file_listing".to_string()
        } else if lowered.contains("plan")
            || lowered.contains("roadmap")
            || lowered.contains("milestone")
        {
            "plan_review".to_string()
        } else if lowered.contains("ui") && lowered.contains("design") {
            "ui_design_review".to_string()
        } else if lowered.contains("review")
            || lowered.contains("approve")
            || lowered.contains("reject")
            || lowered.contains("decline")
            || lowered.contains("spec")
            || lowered.contains("diff")
            || lowered.contains("patch")
            || lowered.contains("security")
        {
            "code_review".to_string()
        } else {
            return None;
        };

        let mut operations = BTreeSet::new();
        if lowered.contains("approve") {
            operations.insert("approve".to_string());
        }
        if lowered.contains("reject") || lowered.contains("decline") {
            operations.insert("reject".to_string());
        }
        if lowered.contains("revise") || lowered.contains("change") {
            operations.insert("revise".to_string());
        }
        if operations.is_empty() {
            if primary == "file_listing" {
                operations.insert("list".to_string());
            } else if primary == "code_review" {
                operations.insert("review".to_string());
            }
        }

        let mut tags = BTreeSet::new();
        if lowered.contains("spec") {
            tags.insert("spec".to_string());
        }
        if lowered.contains("diff") || lowered.contains("patch") {
            tags.insert("diff".to_string());
        }
        if lowered.contains("security") {
            tags.insert("security".to_string());
        }
        if lowered.contains("plan") || lowered.contains("roadmap") {
            tags.insert("plan".to_string());
        }
        if primary == "file_listing" {
            tags.insert("files".to_string());
            tags.insert("workspace".to_string());
            if lowered.contains("tree") {
                tags.insert("tree".to_string());
            }
        }

        Some(UiIntent::new(
            primary,
            operations.into_iter().collect(),
            tags.into_iter().collect(),
        ))
    }

    fn clear_canvas_intent(&mut self) {
        self.active_intent = None;
        self.selected_template = None;
        self.no_matching_template = false;
        self.pending_provisional_template = None;
        self.ui_runtime.clear_schema();
    }

    fn resolve_canvas_for_intent(&mut self, intent: UiIntent) {
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
            );
            if let Err(err) = self.ui_runtime.load_schema_value(&schema) {
                self.log_diagnostic(format!("catalog runtime error: {err}"));
            }
        } else {
            self.selected_template = None;
            self.no_matching_template = true;
            self.ui_runtime.clear_schema();
        }
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
                self.resolve_canvas_for_intent(intent);
            }
            Err(err) => {
                self.log_diagnostic(format!("failed to save provisional template: {err}"));
            }
        }
    }

    fn materialize_template_schema(&self, template_id: &str, schema: &Value) -> Value {
        if template_id != "builtin.file_listing.default" {
            return schema.clone();
        }

        let mut materialized = schema.clone();
        let listing = self.workspace_root_listing();
        if let Some(components) = materialized
            .get_mut("components")
            .and_then(|value| value.as_array_mut())
        {
            for component in components {
                let is_workspace_tree = component
                    .get("id")
                    .and_then(|value| value.as_str())
                    .map(|id| id == "workspace_tree")
                    .unwrap_or(false);
                if is_workspace_tree {
                    if let Some(code) = component.get_mut("code") {
                        *code = Value::String(listing.clone());
                    }
                }
            }
        }

        materialized
    }

    fn workspace_root_listing(&self) -> String {
        let root_name = self
            .workspace
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("workspace");

        let mut entries = Vec::new();
        match fs::read_dir(&self.workspace) {
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
                return format!("{root_name}/\n└── <failed to read workspace: {err}>");
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
            self.current_session = Some(session);
            self.is_streaming = false;
            self.in_progress_assistant.clear();
            self.scroll_to_bottom = true;
            self.session_unavailable = false;
        } else {
            self.session_unavailable = true;
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
                        if let Err(err) = store::save(meta) {
                            self.log_diagnostic(format!("failed to persist session: {err}"));
                        }
                    }
                }

                self.is_streaming = false;
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
                    messages: Vec::new(),
                };

                self.current_session = Some(meta.clone());
                self.transcript.clear();
                self.in_progress_assistant.clear();
                self.is_streaming = false;
                self.session_unavailable = false;

                if let Err(err) = store::save(&meta) {
                    self.log_diagnostic(format!("failed to persist new session: {err}"));
                }

                self.refresh_sessions();
            }
            AppEvent::ToolCallSuppressed(tool_name) => {
                self.log_diagnostic(format!("tool call suppressed (passive mode): {tool_name}"));
            }
            AppEvent::CanvasToolRender {
                intent,
                template_id,
                title,
                provider_id,
                provider_kind,
                schema,
                provisional_template,
            } => {
                self.active_intent = Some(intent);
                self.no_matching_template = false;
                self.selected_template = Some(TemplateSelectionContext {
                    template_id: template_id.clone(),
                    title,
                    provider_id,
                    provider_kind,
                });
                self.pending_provisional_template = provisional_template;

                let schema = self.materialize_template_schema(&template_id, &schema);
                if let Err(err) = self.ui_runtime.load_schema_value(&schema) {
                    self.log_diagnostic(format!("canvas tool render failed: {err}"));
                }
                if let Some(ctx) = ctx {
                    ctx.request_repaint();
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
                self.theme.card_frame().show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(Theme::P8, Theme::P8);
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
                                egui::pos2(response.rect.min.x + 7.0, response.rect.max.y - 5.0),
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

                self.theme.card_frame().show(ui, |ui| {
                    ui.label(
                        RichText::new("Selection Context")
                            .strong()
                            .size(14.0)
                            .color(self.theme.text_primary),
                    );
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

                self.theme.card_frame().show(ui, |ui| {
                    ui.label(
                        RichText::new("Template Canvas")
                            .strong()
                            .size(14.0)
                            .color(self.theme.text_primary),
                    );
                    ui.add_space(Theme::P8);
                    if self.active_intent.is_none() {
                        ui.label(
                            RichText::new("No UI intent selected")
                                .size(13.0)
                                .color(self.theme.text_muted),
                        );
                    } else if self.no_matching_template {
                        ui.label(
                            RichText::new("No matching UI template found")
                                .size(13.0)
                                .color(self.theme.danger),
                        );
                    } else {
                        self.ui_runtime.render_canvas(ui, &self.theme);
                    }
                });

                let mut save_provisional = false;
                let mut dismiss_provisional = false;
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
                    self.ui_runtime.render_event_log(ui, &self.theme);
                });

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
