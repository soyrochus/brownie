# ui-event-log

## Requirements

### Requirement: UI interactions emit typed UiEvent values

Interactive components SHALL emit strongly typed `UiEvent` values with structured payloads. Each event SHALL include the originating component identifier and event-specific typed data.

#### Scenario: Button click emits typed event

- **WHEN** a user clicks a rendered button bound to an output contract
- **THEN** a typed `UiEvent` button variant is emitted with the button id and mapped output event id

#### Scenario: Form field update emits typed event

- **WHEN** a user edits a form field and commits the change
- **THEN** a typed `UiEvent` form-field variant is emitted with form id, field id, and typed value

---

### Requirement: Event emission order is deterministic

The event pipeline SHALL preserve interaction order. For the same user interaction sequence and initial state, emitted `UiEvent` variants and payload ordering SHALL be identical across runs.

#### Scenario: Repeated interaction sequence

- **WHEN** the same ordered sequence of button and form interactions is replayed
- **THEN** emitted `UiEvent` entries appear in the same order with equivalent typed payloads

---

### Requirement: Debug panel displays append-only UiEvent log

The app SHALL display emitted `UiEvent` values in a dedicated debug or log panel as append-only records so event propagation can be inspected during SPEC-2.

#### Scenario: Event appears in debug panel

- **WHEN** a `UiEvent` is emitted by an interaction
- **THEN** a new log entry is appended to the debug panel with structured event data

#### Scenario: Existing event log entries are retained

- **WHEN** additional interactions occur
- **THEN** previously logged events remain visible in chronological order and are not overwritten

---

### Requirement: Event capture is local and independent of agent connectivity

`UiEvent` emission and logging SHALL function even when agent intent resolution and schema generation are disabled for SPEC-2.

#### Scenario: Local runtime mode

- **WHEN** the canvas is driven only by the embedded schema fixture without agent involvement
- **THEN** interactive components still emit and log typed `UiEvent` values

