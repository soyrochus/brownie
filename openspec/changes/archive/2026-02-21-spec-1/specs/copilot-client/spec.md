# copilot-client

## ADDED Requirements

### Requirement: CopilotClient initializes using the SDK Client builder

At startup the `CopilotClient` SHALL construct a `copilot_sdk::Client` via `Client::builder().use_stdio(true).auto_restart(true).build()`, call `client.start().await`, and then call `client.get_auth_status().await` to verify authentication. If either call returns an error, the client SHALL emit a `StatusChanged(Error(...))` `AppEvent` and SHALL NOT proceed to session creation.

#### Scenario: SDK start and auth check succeed

- **WHEN** `client.start().await` and `client.get_auth_status().await` both return `Ok`
- **THEN** the client emits `StatusChanged(ConnectionState::Connected)` and proceeds to create a session

#### Scenario: CLI binary not found

- **WHEN** `client.start().await` returns `Err(CopilotError::InvalidConfig(...))` indicating the CLI could not be located
- **THEN** the client emits `StatusChanged(ConnectionState::Error)` with the error message, the input bar is disabled, and the application does not crash

#### Scenario: CLI found but not authenticated

- **WHEN** `client.get_auth_status().await` returns an auth failure error
- **THEN** the client emits `StatusChanged(ConnectionState::Error)` with a human-readable reason and the input bar is disabled

---

### Requirement: CopilotClient creates a session bound to a workspace path

After successful initialization the `CopilotClient` SHALL call `client.create_session(SessionConfig::default()).await`. On success it SHALL store the `Arc<Session>` and emit a `SessionCreated(session_id)` `AppEvent` where `session_id` comes from `session.session_id()`.

#### Scenario: Session created successfully

- **WHEN** `client.create_session(SessionConfig::default()).await` returns `Ok(session)`
- **THEN** `session.session_id()` is captured, a `SessionCreated(session_id)` event is emitted, and the session is held for message sending and event subscription

#### Scenario: Session creation fails

- **WHEN** `client.create_session(...)` returns an `Err`
- **THEN** a `SdkError(message)` `AppEvent` is emitted, the input bar remains disabled, and no session subscription is started

---

### Requirement: CopilotClient streams assistant output as AppEvent::StreamDelta

After session creation the `CopilotClient` SHALL call `session.subscribe()` to obtain an `EventSubscription`, then spawn a tokio task that calls `events.recv().await` in a loop. Each `SessionEventData::AssistantMessageDelta` SHALL be forwarded as `AppEvent::StreamDelta(delta.delta_content)`. `SessionEventData::SessionIdle` SHALL be forwarded as `AppEvent::StreamEnd`. Streaming MUST NOT block the UI thread.

#### Scenario: AssistantMessageDelta received

- **WHEN** `events.recv().await` returns `Ok(event)` with `SessionEventData::AssistantMessageDelta(delta)`
- **THEN** `AppEvent::StreamDelta(delta.delta_content)` is sent on the mpsc channel to the UI

#### Scenario: SessionIdle received

- **WHEN** `events.recv().await` returns `Ok(event)` with `SessionEventData::SessionIdle(_)`
- **THEN** `AppEvent::StreamEnd` is sent on the mpsc channel

#### Scenario: Full AssistantMessage received (non-streaming fallback)

- **WHEN** `events.recv().await` returns `Ok(event)` with `SessionEventData::AssistantMessage(msg)`
- **THEN** `AppEvent::StreamDelta(msg.content)` followed immediately by `AppEvent::StreamEnd` is sent on the mpsc channel

#### Scenario: Streaming runs concurrently with UI

- **WHEN** streaming is in progress
- **THEN** the egui render loop is not blocked; the UI drains `AppEvent` values via `mpsc::Receiver::try_recv` each frame

---

### Requirement: CopilotClient sends user messages via session.send

The `CopilotClient` SHALL expose a `send(prompt: String)` method that calls `session.send(prompt).await`. The call is made from a tokio task so it does not block the UI thread.

#### Scenario: Message sent successfully

- **WHEN** `session.send(prompt).await` returns `Ok(message_id)`
- **THEN** the message is in flight and subsequent streaming events will arrive on the subscription channel

#### Scenario: Send called with no active session

- **WHEN** `send()` is called before a session has been established
- **THEN** an `SdkError("No active session")` `AppEvent` is emitted and the call returns without panicking

---

### Requirement: CopilotClient enforces passive mode via SDK configuration

Passive mode SHALL be enforced at three independent layers: (1) `deny_tools` arguments passed at the CLI builder level prevent all tool execution; (2) no `session.register_tool` calls are made, so no tools are exposed to the agent; (3) the SDK default `PermissionRequestResult::denied()` handles any `permission.request` callbacks. The `CopilotClient` struct SHALL have no method to approve tool calls.

#### Scenario: Tool-call suppression logged

- **WHEN** `events.recv().await` returns an event with `SessionEventData::ToolUserRequested` or `SessionEventData::ToolExecutionStart`
- **THEN** `AppEvent::ToolCallSuppressed(tool_name)` is sent on the mpsc channel (rendered as a diagnostics entry) and no approval action is taken

---

### Requirement: CopilotClient surfaces session errors as AppEvent::SdkError

Any `SessionEventData::SessionError` received on the subscription SHALL be forwarded as `AppEvent::SdkError(err.message)`. Any `broadcast::error::RecvError::Closed` SHALL emit `AppEvent::StatusChanged(ConnectionState::Disconnected)`.

#### Scenario: SessionError event received

- **WHEN** `events.recv().await` returns `Ok(event)` with `SessionEventData::SessionError(err)`
- **THEN** `AppEvent::SdkError(err.message)` is sent; the transcript finalizes any in-progress message and the input bar is re-enabled

#### Scenario: Broadcast channel closed

- **WHEN** `events.recv().await` returns `Err(RecvError::Closed)`
- **THEN** `AppEvent::StatusChanged(ConnectionState::Disconnected)` is sent and the listener task exits cleanly

---

### Requirement: CopilotClient recovers from CLI restarts via SDK auto_restart

The `CopilotClient` SHALL set `auto_restart(true)` on the SDK `ClientBuilder`. The SDK's built-in restart logic handles CLI process exits and reconnection transparently. The `CopilotClient` SHALL monitor `client.state().await` and emit `StatusChanged` events when the state transitions.

#### Scenario: CLI crashes and SDK auto-restarts

- **WHEN** the Copilot CLI child process exits unexpectedly and `auto_restart` is true
- **THEN** the SDK restarts the CLI; the `CopilotClient` emits `StatusChanged(Connecting)` then `StatusChanged(Connected)` and the transcript is preserved

#### Scenario: Auto-restart transitions to Error state

- **WHEN** the SDK transitions to `ConnectionState::Error` after restart attempts fail
- **THEN** `AppEvent::StatusChanged(ConnectionState::Error)` is emitted and the input bar is disabled
