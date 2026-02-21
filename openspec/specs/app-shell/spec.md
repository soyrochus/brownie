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

The top bar SHALL display current Copilot connection status derived from `AppEvent::StatusChanged` and SHALL keep a visible passive-mode indicator with a non-interactive active-mode label. In SPEC-2, the status indicator SHALL be visually centered in the top bar and SHALL include a small semantic state marker (success, warning, or error). The top bar SHALL use a subtle layered style treatment (including a low-contrast gradient) without changing application layout structure.

#### Scenario: Connected state displayed in centered status area

- **WHEN** `AppEvent::StatusChanged(ConnectionState::Connected)` is received
- **THEN** the top bar shows centered "Copilot Connected" status with a success-colored indicator

#### Scenario: Error state displayed in centered status area

- **WHEN** `AppEvent::StatusChanged(ConnectionState::Error)` is received
- **THEN** the top bar shows centered error status with a danger-colored indicator and readable error label

#### Scenario: Passive mode indicator remains visible

- **WHEN** the application is running
- **THEN** the top bar shows an active passive-mode label and a visible but non-clickable active-mode control

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

The right column SHALL render a runtime-driven Canvas from a validated `UiSchema` in SPEC-2 instead of a static placeholder. The panel SHALL support rendering multiple component types from the validated schema and SHALL include an append-only UI event debug section. If schema validation fails, the right column SHALL show a non-interactive validation error state and MUST NOT render unvalidated UI content.

#### Scenario: Valid schema renders canvas components

- **WHEN** the app starts with a valid embedded schema fixture
- **THEN** the right column renders the schema-defined canvas content rather than the static placeholder text

#### Scenario: Validation failure blocks canvas rendering

- **WHEN** the embedded schema fixture fails validation
- **THEN** the right column shows a validation error state and no interactive canvas components are rendered

#### Scenario: Canvas interaction appends typed event log entries

- **WHEN** a user interacts with rendered canvas components
- **THEN** typed `UiEvent` entries are appended to the right-column debug log section

---

### Requirement: App shell visual styling follows tokenized SPEC-2 style constraints

The app shell SHALL apply the SPEC-2 style token system while preserving layout and hierarchy. This includes layered dark surfaces, softened text contrast, controlled accent usage, and consistent spacing tokens. Required surface tokens are: Surface 0 `#0F1115`, Surface 1 `#161A20`, Surface 2 `#1C222B`, Surface 3 `#222A35`; required text tokens are primary `#E6EDF3` and muted `#8B949E`; required accent is `#3B82F6` with defined hover and muted variants.

#### Scenario: Layout remains unchanged while style tokens are applied

- **WHEN** SPEC-2 styling is enabled
- **THEN** panel positions, component hierarchy, and existing component IDs remain unchanged while tokenized colors are applied

#### Scenario: Surface layering is visible across shell panels

- **WHEN** the shell renders top bar, sidebar, center panel, and canvas panel
- **THEN** each region uses the defined layered surface tokens instead of flat uniform background values

---

### Requirement: App shell spacing and control styling follow the constrained design scale

The shell SHALL use a constrained spacing scale of `4, 8, 12, 16, 24, 32` pixels and SHALL apply SPEC-2 control styling constraints: rounded panel containers, subtle borders, low-elevation shadows, rounded buttons, and a rounded input container with focus glow. Sidebar session items SHALL support hover highlighting and active-state accent border, and diff/code display styling SHALL use subtle semantic backgrounds for additions and removals.

#### Scenario: Required spacing updates are applied

- **WHEN** the workspace sidebar, chat transcript, canvas panel, action groups, and input bar are rendered
- **THEN** each area reflects SPEC-2 spacing targets (sidebar `16px` internal padding, chat `16px` vertical rhythm, canvas `24px` padding, action-item `12px` spacing, input vertical `12px`)

#### Scenario: Buttons and input bar match constrained styling

- **WHEN** primary and secondary buttons plus the chat input container are displayed
- **THEN** buttons and input follow rounded, low-contrast styles with accent-driven focus and hover behavior defined by SPEC-2 tokens

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
