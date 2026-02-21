# Design: spec-1

## Context

Brownie is a greenfield Rust desktop application. At this point `src/` contains only a `.gitkeep`. The community Rust SDK (`copilot-sdk`) exists at `vendor/copilot-sdk-rust` (git submodule, `https://github.com/copilot-community-sdk/copilot-sdk-rust`). It wraps the Copilot CLI JSON-RPC protocol completely — the app does not speak JSON-RPC directly. The SDK's top-level types (`Client`, `Session`, `SessionConfig`, `SessionEventData`, `ConnectionState`) are the sole integration surface.

UI layout is defined by the UI draft (`specs/images/ui-draft.png`): three-column landscape window, dark theme, top bar with connection status and Passive/Active mode toggle, left "Workspace" panel with workspace name, instruction sources, and recent sessions, center "Chat" panel, right "Actions Panel" placeholder (canvas in later specs).

## Goals / Non-Goals

**Goals:**

- Establish a runnable Rust binary with the static three-column desktop shell from the UI draft
- Integrate `copilot-sdk` (path dep) as the sole Copilot integration layer; wrap it in a thin `CopilotClient` facade
- Create a session via `Client::create_session`, subscribe to events via `session.subscribe()`, and forward `AssistantMessageDelta` events to the UI in real time
- Enforce passive mode unconditionally: no tool registration, default deny-all permission handler, `deny_tools` at the CLI builder level
- Display connection status (`ConnectionState`) in the top bar; surface errors in a diagnostics area
- Persist minimal session metadata to the local filesystem so sessions can be resumed or reconstructed on restart

**Non-Goals:**

- Dynamic UI rendering (UiSchema / DSL) — deferred
- Active mode (tool approval, file writes, command execution) — deferred
- Right-side canvas beyond a static placeholder ("Dynamic UI will render here") — deferred
- UI Catalog, intent resolution, template promotion — deferred
- MCP tool integration — deferred

## Decisions

### 1. Copilot integration: use `copilot-sdk` as a path dependency

**Decision:** Reference the community SDK via a Cargo path dependency:

```toml
[dependencies]
copilot-sdk = { path = "vendor/copilot-sdk-rust" }
```

Use `Client::builder().use_stdio(true).auto_restart(true).build()` for the connection. The SDK spawns the CLI process, manages the JSON-RPC transport, and provides `session.subscribe()` which returns a `broadcast::Receiver<SessionEvent>`. The app does not write any JSON-RPC code.

**Rationale:** The SDK is vendored as a git submodule, has a clean API (`Client`, `Session`, `SessionConfig`, `SessionEventData`), and handles all transport and protocol concerns. Writing our own JSON-RPC layer on top of this would be redundant and would introduce a maintenance burden with no benefit. The `CopilotClient` struct in the app becomes a thin facade that translates SDK types to app-level `AppEvent` values and forwards them over an `mpsc` channel.

**Alternative considered:** Writing JSON-RPC directly over stdio. Rejected: the SDK exists, is vendored, and covers the full protocol surface.

### 2. UI framework: egui (via eframe)

**Decision:** Use `egui`/`eframe` as the UI framework.

**Rationale:** Immediate-mode, pure Rust, cross-platform, minimal scaffolding for a static layout. The three-column layout and appending transcript are trivially implemented with `egui::SidePanel`, `egui::CentralPanel`, and `egui::ScrollArea`. The dark theme matches the UI draft out of the box.

**Alternative considered:** `iced` (Elm-style). More complex for a validation slice; revisit when dynamic canvas rendering is introduced.

### 3. Async runtime: tokio + mpsc bridge to egui

**Decision:** Run `tokio` (already a dependency of `copilot-sdk`) for all async work. Bridge events to the egui render loop via `std::sync::mpsc::channel`. A background task calls `events.recv().await` on the SDK broadcast receiver and forwards `AppEvent` values to the channel. The egui `App::update` method drains the receiver each frame.

```text
[tokio task]  session.subscribe() → recv() → tx.send(AppEvent)
[egui frame]  rx.try_recv() loop → update UI state
```

**Rationale:** egui's update loop is synchronous. The mpsc receiver is non-blocking (`try_recv`), so draining it in the render loop is safe and cheap. This avoids shared mutable state between async tasks and the UI thread.

### 4. AppEvent enum

Internal bridge type; SDK event variants map to:

```rust
enum AppEvent {
    StreamDelta(String),              // AssistantMessageDelta → delta.delta_content
    StreamEnd,                        // SessionIdle
    StatusChanged(ConnectionState),   // polled or derived from SDK errors
    SdkError(String),                 // SessionError or CopilotError
    SessionCreated(String),           // session_id from session.session_id()
    ToolCallSuppressed(String),       // logged diagnostic entry (tool name)
}
```

### 5. Passive mode enforcement

**Decision:** Enforce passive mode at three layers, each independent:

1. **CLI level** — `Client::builder().deny_tools(["*"])`: denies all tool execution in the CLI itself
2. **Session level** — no `register_tool` calls; default `PermissionRequestResult::denied()` (SDK default) handles any `permission.request` callbacks
3. **Application level** — `CopilotClient` has no method to approve tool calls; `ToolUserRequested` and `ToolExecutionStart` events are captured, logged to diagnostics, and not forwarded to the UI as actionable events

**Rationale:** Defense in depth. The CLI flag is the strongest guarantee; the SDK default deny-all is the second layer; the app-level omission of any approval affordance is the third.

### 6. Module structure

```text
src/
  main.rs            — eframe entry point; constructs App and launches tokio runtime
  app.rs             — egui App impl; three-column layout, transcript state, event drain
  copilot/
    mod.rs           — CopilotClient: creates SDK Client, starts session, forwards AppEvents
  session/
    mod.rs           — SessionMeta struct (id, workspace, title, created_at)
    store.rs         — ~/.brownie/sessions/ filesystem read/write (atomic JSON)
  event.rs           — AppEvent enum
Cargo.toml           — copilot-sdk path dep, egui/eframe, tokio, serde/serde_json
```

No `copilot/protocol.rs` or `copilot/process.rs` — the SDK owns all of that.

### 7. Persistence: filesystem JSON (no SQLite in SPEC-1)

**Decision:** Write one JSON file per session at `~/.brownie/sessions/<session-id>.json`. Atomic write (`.tmp` + rename). Schema version field `"schema_version": 1` in every file.

**Rationale:** Simple, auditable, no migration logic. SQLite deferred until query complexity justifies it.

## Risks / Trade-offs

- **SDK crate name vs directory name**: the submodule is at `vendor/copilot-sdk-rust` but the Cargo package name is `copilot-sdk`. The path dep references the directory; the `use` statements use the crate name `copilot_sdk`. Verify this compiles without renaming.
- **broadcast::RecvError::Lagged**: if the egui render loop falls behind the broadcast channel (capacity 1024), events may be dropped. For SPEC-1 this is acceptable; implement batching or a bounded mpsc if needed.
- **egui limitations for future dynamic canvas**: the right column is a bounded `Frame`; future DSL rendering replaces the interior without touching the layout scaffold.
- **CLI not found / not authenticated**: `client.get_auth_status()` will fail. `Client::start()` returns `Err(CopilotError::InvalidConfig(...))` or equivalent. Surface this in the diagnostics panel; disable the input bar. Do not crash.
- **Atomic write on Windows**: `std::fs::rename` is not guaranteed atomic on Windows if source and destination are on different volumes. Acceptable for SPEC-1 (both paths are under `~/.brownie`).

## Migration Plan

Greenfield binary. No migration. Deployment: `cargo build --release` produces a single native binary. Copilot CLI must be installed and authenticated separately.

## Open Questions

- Should the workspace selector in SPEC-1 allow manual path entry, or only show the CWD? (Recommend: default to CWD; allow override in `~/.brownie/config.toml` in a later spec.)
- What is the correct `deny_tools` wildcard syntax accepted by the CLI for "deny all tools"? Needs empirical test against the CLI — may need to enumerate known tool names instead of using a glob.
