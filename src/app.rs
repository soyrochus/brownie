use crate::copilot::CopilotClient;
use crate::event::AppEvent;
use crate::session::store;
use crate::session::{Message, SessionMeta, SCHEMA_VERSION};
use copilot_sdk::ConnectionState;
use eframe::egui::{self, Color32, RichText, ScrollArea};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{SystemTime, UNIX_EPOCH};

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
}

impl BrownieApp {
    pub fn new(
        rx: Receiver<AppEvent>,
        copilot: CopilotClient,
        workspace: PathBuf,
        instruction_files: Vec<String>,
    ) -> Self {
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
        };

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

    fn connection_label(&self) -> (&'static str, Color32) {
        match self.connection_state {
            ConnectionState::Connected => ("Copilot Connected", Color32::LIGHT_GREEN),
            ConnectionState::Connecting => ("Connecting...", Color32::YELLOW),
            ConnectionState::Disconnected => ("Disconnected", Color32::GRAY),
            ConnectionState::Error => ("Copilot Error", Color32::RED),
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
            if let Err(err) = store::save(meta) {
                self.log_diagnostic(format!("failed to persist session: {err}"));
            }
        }

        self.copilot.send(prompt);
        self.input_buffer.clear();
        self.scroll_to_bottom = true;
        ctx.request_repaint();
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
        }
    }

    fn render_top_bar(&mut self, ctx: &egui::Context) {
        let (status_label, status_color) = self.connection_label();
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.strong("Brownie");
                ui.separator();
                ui.label(RichText::new(status_label).color(status_color));
                ui.separator();
                ui.label(RichText::new("Passive Mode").color(Color32::LIGHT_GREEN));
                ui.add_enabled(false, egui::Button::new("Active Mode"));
                ui.add_enabled(false, egui::Button::new("Settings"));
            });
        });
    }

    fn render_left_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("workspace_panel")
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Workspace");
                ui.label(self.workspace.display().to_string());
                ui.separator();

                ui.strong("Copilot Instructions");
                if self.instruction_files.is_empty() {
                    ui.label("No instruction files detected");
                } else {
                    for path in &self.instruction_files {
                        ui.label(path);
                    }
                }

                ui.separator();
                ui.strong("Recent Sessions");
                let mut clicked_session: Option<String> = None;
                for session in &self.sessions {
                    let label = session
                        .title
                        .clone()
                        .unwrap_or_else(|| session.session_id.clone());
                    if ui.button(label).clicked() {
                        clicked_session = Some(session.session_id.clone());
                    }
                }

                if let Some(session_id) = clicked_session {
                    self.open_session(&session_id);
                }
            });
    }

    fn render_right_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("actions_panel")
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Actions Panel");
                ui.separator();
                ui.label("Dynamic UI will render here");
            });
    }

    fn render_center_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Chat");
            ui.separator();

            let transcript_height = (ui.available_height() - 170.0).max(120.0);
            ScrollArea::vertical()
                .id_salt("chat_transcript")
                .max_height(transcript_height)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    if self.session_unavailable {
                        ui.label(RichText::new("Session unavailable").color(Color32::RED));
                    }

                    for message in &self.transcript {
                        let label = if message.role == "user" {
                            format!("[You] {}", message.content)
                        } else {
                            format!("[Copilot] {}", message.content)
                        };
                        ui.label(label);
                    }

                    if self.is_streaming && !self.in_progress_assistant.is_empty() {
                        ui.label(format!("[Copilot] {}", self.in_progress_assistant));
                    }

                    if self.scroll_to_bottom {
                        ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                    }
                });
            self.scroll_to_bottom = false;

            ui.separator();
            egui::CollapsingHeader::new("Diagnostics")
                .default_open(false)
                .show(ui, |ui| {
                    ScrollArea::vertical()
                        .id_salt("diagnostics_log")
                        .max_height(90.0)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for entry in &self.diagnostics_log {
                                ui.label(entry);
                            }
                        });
                });

            ui.separator();
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
            ui.horizontal(|ui| {
                let response = ui.add_enabled(
                    input_enabled,
                    egui::TextEdit::singleline(&mut self.input_buffer)
                        .desired_width(f32::INFINITY)
                        .hint_text(hint),
                );
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    send_now = true;
                }

                let clicked = ui
                    .add_enabled(
                        input_enabled && !self.input_buffer.trim().is_empty(),
                        egui::Button::new("Send"),
                    )
                    .clicked();
                send_now |= clicked;
            });

            if send_now && input_enabled {
                self.submit_prompt(ctx);
            }
        });
    }
}

impl eframe::App for BrownieApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_events(ctx);
        self.render_top_bar(ctx);
        self.render_left_panel(ctx);
        self.render_right_panel(ctx);
        self.render_center_panel(ctx);
    }
}
