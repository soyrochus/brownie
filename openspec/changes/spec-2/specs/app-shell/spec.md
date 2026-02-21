# app-shell

## MODIFIED Requirements

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

## ADDED Requirements

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

