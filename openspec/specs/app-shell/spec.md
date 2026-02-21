# app-shell

## Requirements

### Requirement: Application launches as a wide landscape desktop window

The application SHALL launch as a native desktop window with a landscape aspect ratio, a three-column layout (left, center, right), and a top bar. The layout SHALL be established at startup and SHALL NOT change in SPEC-1.

#### Scenario: Window opens on launch

- **WHEN** the binary is executed
- **THEN** a native desktop window opens with a visible three-column layout and a top bar

#### Scenario: Window is resizable

- **WHEN** the user resizes the window
- **THEN** the three columns and top bar reflow to fill the new dimensions without layout breakage

---

### Requirement: Top bar displays connection status and passive mode indicator

The top bar SHALL display the current Copilot connection status (e.g., "Copilot Connected", "Connecting…", "Disconnected", or an error label) derived from `AppEvent::StatusChanged`. It SHALL show a "Passive Mode" label and a non-interactive "Active Mode" label. The mode toggle is visible but the Active Mode option SHALL be disabled (non-clickable) in SPEC-1.

#### Scenario: Connected state displayed

- **WHEN** `AppEvent::StatusChanged(ConnectionState::Connected)` is received
- **THEN** the top bar shows "Copilot Connected"

#### Scenario: Error state displayed

- **WHEN** `AppEvent::StatusChanged(ConnectionState::Error)` is received
- **THEN** the top bar shows the error reason in a visually distinct style (e.g., red label)

#### Scenario: Passive mode indicator always visible

- **WHEN** the application is running
- **THEN** the top bar shows a "Passive Mode" active indicator; "Active Mode" is visible but disabled with no functional toggle

---

### Requirement: Left column shows workspace name and session list

The left column (labeled "Workspace") SHALL display the current workspace path defaulting to the working directory at launch, a static list of detected instruction source paths (`.github/copilot-instructions.md`, `AGENTS.md`, and any `.instructions.md` files found under the workspace), and a "Recent Sessions" list loaded from local storage. Clicking a session entry SHALL load its transcript into the center column.

#### Scenario: Workspace path shown on startup

- **WHEN** the application starts
- **THEN** the left column displays the current working directory as the active workspace label

#### Scenario: Instruction sources listed

- **WHEN** the application starts and scans the workspace directory
- **THEN** known instruction file paths that exist are shown under a "Copilot Instructions" section; non-existent paths are omitted

#### Scenario: Recent sessions listed

- **WHEN** session metadata files exist in the local storage directory
- **THEN** the left column lists session titles (or session IDs if no title) in reverse chronological order under "Recent Sessions"

#### Scenario: Session selected from list

- **WHEN** the user clicks a session entry in the left column
- **THEN** the stored transcript for that session is loaded into the center transcript area

#### Scenario: No sessions exist

- **WHEN** no session metadata files are present
- **THEN** the left column shows an empty "Recent Sessions" section with no placeholder error

---

### Requirement: Center column shows streaming transcript and input bar

The center column (labeled "Chat") SHALL display the full conversation transcript in a scrollable area, render incoming `AppEvent::StreamDelta` events as incrementally appended text on the current assistant message, and provide an input bar at the bottom with a text field and a send button.

#### Scenario: User message displayed immediately

- **WHEN** the user submits a message via the input bar
- **THEN** the user's message appears at the bottom of the transcript before the assistant responds

#### Scenario: Streaming assistant output rendered incrementally

- **WHEN** `AppEvent::StreamDelta(text)` events arrive
- **THEN** each chunk is appended to the current assistant message in the transcript in order

#### Scenario: Stream completion finalizes message

- **WHEN** `AppEvent::StreamEnd` is received
- **THEN** the assistant message is finalized and the transcript scrolls to the bottom

#### Scenario: Input bar disabled while streaming

- **WHEN** a streaming response is in progress
- **THEN** the input bar is disabled until `AppEvent::StreamEnd` is received

#### Scenario: Input bar disabled when not connected

- **WHEN** connection status is not `Connected`
- **THEN** the input bar is disabled and shows a hint (e.g., "Not connected")

#### Scenario: Enter key submits message

- **WHEN** the input bar has focus and the user presses Enter with non-empty text
- **THEN** the message is submitted and the input bar is cleared

---

### Requirement: Right column shows static Actions Panel placeholder

The right column (labeled "Actions Panel") SHALL display a static placeholder panel with the message "Dynamic UI will render here". No interaction, tool output, diff rendering, or canvas logic SHALL appear in this panel in SPEC-1.

#### Scenario: Placeholder visible on launch

- **WHEN** the application starts
- **THEN** the right column displays the static placeholder message

#### Scenario: Placeholder unaffected by session activity

- **WHEN** messages are sent and responses received
- **THEN** the right column content does not change

---

### Requirement: Diagnostics area surfaces SDK and CLI errors

The application SHALL display a collapsible diagnostics area (within the center column below the transcript) that shows `AppEvent::SdkError`, `AppEvent::ToolCallSuppressed`, and `AppEvent::StatusChanged` events with timestamps. The area SHALL be scrollable and append-only.

#### Scenario: SDK error surfaced in diagnostics

- **WHEN** `AppEvent::SdkError(message)` is received
- **THEN** a timestamped error entry appears in the diagnostics area

#### Scenario: Suppressed tool call logged

- **WHEN** `AppEvent::ToolCallSuppressed(tool_name)` is received
- **THEN** an entry "tool call suppressed (passive mode): <tool_name>" appears in the diagnostics area

#### Scenario: Connection event logged

- **WHEN** `AppEvent::StatusChanged` is received with any state value
- **THEN** a timestamped connection state entry appears in the diagnostics area
