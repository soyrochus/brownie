# canvas-workspace

## ADDED Requirements

### Requirement: Canvas workspace maintains persistent block state within a session

The system SHALL maintain a session-scoped `CanvasWorkspace` containing multiple UI blocks with stable identities. Canvas blocks SHALL remain available across chat turns until explicitly closed.

#### Scenario: Non-UI chat does not clear workspace

- **WHEN** a user sends a chat message that does not trigger a UI action
- **THEN** all existing Canvas blocks remain present with unchanged state

#### Scenario: Multiple blocks coexist in one session

- **WHEN** the user or assistant opens additional Canvas blocks
- **THEN** the workspace contains all opened blocks without replacing earlier blocks

---

### Requirement: Canvas workspace is isolated per session

The system SHALL bind `CanvasWorkspace` to the active session and SHALL switch workspace state together with transcript state when sessions change.

#### Scenario: Session switch restores corresponding workspace

- **WHEN** the user opens a different saved session
- **THEN** the transcript and that session's Canvas workspace are restored together

#### Scenario: New session starts with empty workspace

- **WHEN** a new session is created
- **THEN** the session starts with an empty Canvas workspace unless explicitly seeded

---

### Requirement: Users can directly manage visible workspace blocks

Each visible Canvas block SHALL provide user controls for focus, minimize, and close.

#### Scenario: User closes a block

- **WHEN** the user activates a block close control
- **THEN** only the targeted block is removed from the active workspace

#### Scenario: User minimizes a block

- **WHEN** the user minimizes a block
- **THEN** the block remains in workspace state and can be restored without losing its local state
