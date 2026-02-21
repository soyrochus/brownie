use crate::copilot::CopilotClient;
use crate::event::AppEvent;
use crate::session::store;
use crate::session::{Message, SessionMeta, SCHEMA_VERSION};
use crate::theme::Theme;
use crate::ui::runtime::UiRuntime;
use copilot_sdk::ConnectionState;
use eframe::egui::{self, Align, Frame, RichText, ScrollArea, Stroke};
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
    theme: Theme,
    ui_runtime: UiRuntime,
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
            theme: Theme::default(),
            ui_runtime: UiRuntime::new(),
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
                        ui.add_enabled(
                            false,
                            egui::Button::new(
                                RichText::new("Active Mode")
                                    .size(12.0)
                                    .color(self.theme.text_muted),
                            ),
                        );
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
                ui.label(
                    RichText::new("Workspace")
                        .size(14.0)
                        .color(self.theme.text_primary),
                );
                ui.add_space(self.theme.spacing_8);
                ui.label(
                    RichText::new(self.workspace.display().to_string())
                        .size(12.0)
                        .color(self.theme.text_muted),
                );
                ui.add_space(self.theme.spacing_16);

                ui.label(
                    RichText::new("Copilot Instructions")
                        .size(13.0)
                        .color(self.theme.text_primary),
                );
                ui.add_space(self.theme.spacing_8);
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

                ui.add_space(self.theme.spacing_32);
                ui.label(
                    RichText::new("Recent Sessions")
                        .size(13.0)
                        .color(self.theme.text_primary),
                );
                ui.add_space(self.theme.spacing_8);
                let mut clicked_session: Option<String> = None;
                let active_session_id = self
                    .current_session
                    .as_ref()
                    .map(|session| &session.session_id);
                for session in &self.sessions {
                    let label = session
                        .title
                        .clone()
                        .unwrap_or_else(|| session.session_id.clone());
                    let is_active = active_session_id
                        .map(|current| current == &session.session_id)
                        .unwrap_or(false);

                    ui.horizontal(|ui| {
                        if is_active {
                            ui.label(RichText::new("▌").color(self.theme.accent_primary));
                        } else {
                            ui.label(RichText::new(" ").color(self.theme.surface_1));
                        }

                        let button = egui::Button::new(
                            RichText::new(label)
                                .size(12.0)
                                .color(self.theme.text_primary),
                        )
                        .fill(if is_active {
                            self.theme.surface_3
                        } else {
                            self.theme.surface_2
                        })
                        .stroke(Stroke::new(
                            1.0,
                            if is_active {
                                self.theme.accent_primary
                            } else {
                                self.theme.border_subtle
                            },
                        ))
                        .corner_radius(egui::CornerRadius::same(self.theme.radius_8))
                        .min_size(egui::vec2(ui.available_width(), 28.0));
                        if ui.add(button).clicked() {
                            clicked_session = Some(session.session_id.clone());
                        }
                    });
                    ui.add_space(6.0);
                }

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
                ui.label(
                    RichText::new("Canvas")
                        .size(14.0)
                        .color(self.theme.text_primary),
                );
                ui.add_space(self.theme.spacing_12);
                self.ui_runtime.render_canvas(ui, &self.theme);
                ui.add_space(self.theme.spacing_12);
                ui.separator();
                ui.add_space(self.theme.spacing_12);

                let debug_frame = self
                    .theme
                    .panel_frame(self.theme.surface_2, self.theme.spacing_12 as i8);
                debug_frame.show(ui, |ui| {
                    self.ui_runtime.render_event_log(ui, &self.theme);
                });
            });
    }

    fn render_center_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                self.theme
                    .panel_frame(self.theme.surface_1, self.theme.spacing_16 as i8),
            )
            .show(ctx, |ui| {
                ui.label(
                    RichText::new("Chat")
                        .size(13.0)
                        .color(self.theme.text_primary),
                );
                ui.add_space(self.theme.spacing_12);

                let transcript_height = (ui.available_height() - 210.0).max(120.0);
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

                        ui.spacing_mut().item_spacing.y = self.theme.spacing_16;
                        for message in &self.transcript {
                            let is_user = message.role == "user";
                            let bubble = Frame::new()
                                .fill(if is_user {
                                    self.theme.surface_2
                                } else {
                                    self.theme.surface_3
                                })
                                .corner_radius(egui::CornerRadius::same(self.theme.radius_12))
                                .stroke(Stroke::new(1.0, self.theme.border_subtle))
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
                                .stroke(Stroke::new(1.0, self.theme.border_subtle))
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

                ui.add_space(self.theme.spacing_12);
                egui::CollapsingHeader::new(
                    RichText::new("Diagnostics")
                        .size(13.0)
                        .color(self.theme.text_primary),
                )
                .default_open(false)
                .show(ui, |ui| {
                    ScrollArea::vertical()
                        .id_salt("diagnostics_log")
                        .max_height(90.0)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for entry in &self.diagnostics_log {
                                ui.label(
                                    RichText::new(entry).size(12.0).color(self.theme.text_muted),
                                );
                            }
                        });
                });

                ui.add_space(self.theme.spacing_12);
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
                let input_frame = Frame::new()
                    .fill(self.theme.surface_2)
                    .stroke(Stroke::new(1.0, self.theme.border_subtle))
                    .corner_radius(egui::CornerRadius::same(self.theme.radius_12))
                    .inner_margin(egui::Margin::symmetric(
                        self.theme.spacing_12 as i8,
                        self.theme.spacing_12 as i8,
                    ));

                input_frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let button_width = 96.0;
                        let row_spacing = self.theme.spacing_12;
                        let text_width =
                            (ui.available_width() - button_width - row_spacing).max(140.0);

                        let response = ui
                            .add_enabled_ui(input_enabled, |ui| {
                                ui.add_sized(
                                    [text_width, 0.0],
                                    egui::TextEdit::singleline(&mut self.input_buffer)
                                        .hint_text(hint),
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
                        }
                        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            send_now = true;
                        }

                        let send_button = egui::Button::new(
                            RichText::new("Send")
                                .size(13.0)
                                .color(self.theme.text_primary),
                        )
                        .fill(self.theme.accent_primary)
                        .stroke(Stroke::NONE)
                        .corner_radius(egui::CornerRadius::same(self.theme.radius_8));
                        let clicked = ui
                            .add_enabled_ui(
                                input_enabled && !self.input_buffer.trim().is_empty(),
                                |ui| {
                                    ui.add_sized(
                                        [button_width, self.theme.button_height],
                                        send_button,
                                    )
                                },
                            )
                            .inner
                            .clicked();
                        send_now |= clicked;
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
