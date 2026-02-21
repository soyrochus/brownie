## 1. Project Scaffold

- [ ] 1.1 Create `Cargo.toml` with binary target `brownie`; add dependencies: `copilot-sdk = { path = "vendor/copilot-sdk-rust" }`, `eframe`, `egui`, `tokio` (full features), `serde`, `serde_json`
- [ ] 1.2 Create `src/` module files: `main.rs`, `app.rs`, `event.rs`, `copilot/mod.rs`, `session/mod.rs`, `session/store.rs`
- [ ] 1.3 Define `AppEvent` enum in `event.rs`: `StreamDelta(String)`, `StreamEnd`, `StatusChanged(ConnectionState)`, `SdkError(String)`, `SessionCreated(String)`, `ToolCallSuppressed(String)`
- [ ] 1.4 Verify `cargo check` passes with empty stubs for all modules

## 2. Session Persistence

- [ ] 2.1 Define `SessionMeta` struct in `session/mod.rs`: `schema_version: u32`, `session_id: String`, `workspace: String`, `title: Option<String>`, `created_at: String`, `messages: Vec<Message>`
- [ ] 2.2 Define `Message` struct: `role: String` (`"user"` | `"assistant"`), `content: String`, `timestamp: String`
- [ ] 2.3 Implement `store::save(meta: &SessionMeta)` — atomic write to `~/.brownie/sessions/<id>.json` (write `.tmp`, rename)
- [ ] 2.4 Implement `store::load_all()` — scan `~/.brownie/sessions/`, deserialize each file; skip and warn on corrupt files or unknown `schema_version`
- [ ] 2.5 Implement `store::load_one(session_id: &str)` — load a single session file; return `None` with a warning if missing or corrupt
- [ ] 2.6 Ensure `~/.brownie/sessions/` is created on first use if absent

## 3. CopilotClient

- [ ] 3.1 Implement `CopilotClient::new(workspace: PathBuf, tx: mpsc::Sender<AppEvent>)` — constructs `Client::builder().use_stdio(true).auto_restart(true).build()`
- [ ] 3.2 Implement `CopilotClient::start(&self)` — calls `client.start().await`; on error sends `AppEvent::StatusChanged(Error)` and returns; on success calls `get_auth_status().await` and sends `StatusChanged(Connected)` or `StatusChanged(Error)`
- [ ] 3.3 Implement session creation: calls `client.create_session(SessionConfig::default()).await`; on success extracts `session.session_id()`, sends `AppEvent::SessionCreated(id)`, stores session
- [ ] 3.4 Implement event listener task: calls `session.subscribe()`, spawns tokio task that loops on `events.recv().await` and maps SDK events to `AppEvent` values via `tx.send()`
- [ ] 3.5 Map `SessionEventData::AssistantMessageDelta` → `AppEvent::StreamDelta(delta.delta_content)`
- [ ] 3.6 Map `SessionEventData::AssistantMessage` → `AppEvent::StreamDelta(msg.content)` + `AppEvent::StreamEnd`
- [ ] 3.7 Map `SessionEventData::SessionIdle` → `AppEvent::StreamEnd`
- [ ] 3.8 Map `SessionEventData::SessionError` → `AppEvent::SdkError(err.message)`
- [ ] 3.9 Map `SessionEventData::ToolUserRequested` / `ToolExecutionStart` → `AppEvent::ToolCallSuppressed(tool_name)`
- [ ] 3.10 Map `broadcast::RecvError::Closed` → `AppEvent::StatusChanged(Disconnected)`
- [ ] 3.11 Implement `CopilotClient::send(prompt: String)` — spawns tokio task calling `session.send(prompt).await`; sends `AppEvent::SdkError` on failure
- [ ] 3.12 Poll `client.state().await` periodically (e.g., every 500 ms) and emit `AppEvent::StatusChanged` on state transitions

## 4. Application Shell — Layout

- [ ] 4.1 Implement `BrownieApp` struct in `app.rs`: holds `rx: mpsc::Receiver<AppEvent>`, connection state, transcript, session list, session metadata, input buffer, is-streaming flag, diagnostics log
- [ ] 4.2 Implement `eframe::App::update` entry point: drain `rx.try_recv()` loop each frame before rendering
- [ ] 4.3 Implement top bar with egui `TopBottomPanel`: left — app title; center — connection status label (color-coded); right — non-interactive "Passive Mode" active label + disabled "Active Mode" label + Settings placeholder
- [ ] 4.4 Implement left `SidePanel` ("Workspace"): workspace path label at top; "Copilot Instructions" section listing detected instruction files; "Recent Sessions" list of clickable session titles
- [ ] 4.5 Implement right `SidePanel` ("Actions Panel"): static placeholder label "Dynamic UI will render here"
- [ ] 4.6 Implement center `CentralPanel` ("Chat"): vertically split into transcript scroll area (top, grows) + diagnostics collapsible area + input bar (bottom, fixed height)

## 5. Application Shell — Transcript and Input

- [ ] 5.1 Render transcript messages in `ScrollArea`; user messages right-aligned or labeled `[You]`; assistant messages left-aligned or labeled `[Copilot]`
- [ ] 5.2 On `AppEvent::StreamDelta(text)`: append text to the current in-progress assistant message string; set is-streaming flag; call `ctx.request_repaint()`
- [ ] 5.3 On `AppEvent::StreamEnd`: finalize current assistant message; push to transcript; clear in-progress buffer; clear is-streaming flag; scroll transcript to bottom
- [ ] 5.4 Render the in-progress streaming message as a live-updating transcript entry while is-streaming is true
- [ ] 5.5 Implement input bar: single-line text field + Send button; disabled when is-streaming or not connected; Enter key submits
- [ ] 5.6 On submit: add user message to transcript immediately; call `CopilotClient::send()`; save message to session store; clear input buffer

## 6. Application Shell — Status and Diagnostics

- [ ] 6.1 On `AppEvent::StatusChanged`: update connection state label in top bar; append timestamped entry to diagnostics log
- [ ] 6.2 On `AppEvent::SdkError`: append timestamped error entry to diagnostics log; re-enable input bar if was streaming
- [ ] 6.3 On `AppEvent::ToolCallSuppressed`: append "tool call suppressed (passive mode): `<name>`" entry to diagnostics log
- [ ] 6.4 Render diagnostics area as a collapsible `CollapsingHeader` with a scrollable append-only list of entries

## 7. Session Lifecycle Integration

- [ ] 7.1 On `AppEvent::SessionCreated(id)`: create a new `SessionMeta` record; call `store::save()`; refresh session list in left panel
- [ ] 7.2 On left panel session click: call `store::load_one(id)`; if `Some`, render transcript; if `None`, append error to diagnostics
- [ ] 7.3 On `AppEvent::StreamEnd` (after assistant message finalized): call `store::save()` to persist updated transcript
- [ ] 7.4 At startup: call `store::load_all()` to populate the left-panel session list; emit any load warnings as `AppEvent::SdkError`

## 8. Startup and Wiring

- [ ] 8.1 In `main.rs`: create `mpsc::channel`; construct `BrownieApp` with receiver; spawn tokio runtime on a background thread; start `CopilotClient` on the tokio runtime passing the sender
- [ ] 8.2 Configure `eframe::NativeOptions` for landscape window (e.g., 1280×800 minimum size), dark theme
- [ ] 8.3 Detect instruction files at startup (`.github/copilot-instructions.md`, `AGENTS.md`, `*.instructions.md`) relative to CWD; store list in app state for left panel rendering
- [ ] 8.4 Verify end-to-end: launch app, connect to Copilot CLI, send a message, observe streaming response in transcript, confirm session file written to `~/.brownie/sessions/`
- [ ] 8.5 Verify passive mode: confirm tool-call events appear in diagnostics and no approval dialog is rendered
- [ ] 8.6 Verify restart recovery: kill the CLI process mid-session, confirm app reconnects and transcript is preserved
