# session-persistence

## MODIFIED Requirements

### Requirement: Session metadata is persisted locally on creation and update

When a new session is created, a message is added, or Canvas workspace state changes, the application SHALL persist session metadata to `~/.brownie/sessions/<session-id>.json`. The persisted session payload SHALL include transcript data and Canvas workspace state (open blocks, focus/layout state, and block-local persisted state). Writes SHALL remain atomic.

#### Scenario: Canvas block mutation persists session file

- **WHEN** a block is opened, updated, minimized, focused, or closed
- **THEN** the session JSON file is updated atomically with the new workspace state

#### Scenario: Transcript and workspace are persisted together

- **WHEN** either transcript or workspace state changes
- **THEN** the canonical session file reflects both transcript and workspace fields in one consistent snapshot

---

### Requirement: Selecting a session loads its transcript into the transcript area

When a session is selected from the left-column list, the application SHALL load both the full message transcript and the persisted Canvas workspace for that session.

#### Scenario: Session selection restores transcript and workspace

- **WHEN** the user clicks a session entry
- **THEN** transcript messages and that session's Canvas workspace are restored together

#### Scenario: Session file missing at load time

- **WHEN** a session entry is clicked but its JSON file no longer exists on disk
- **THEN** an `AppEvent::SdkError` is emitted and the transcript area shows a "Session unavailable" placeholder while no stale workspace state is applied

---

### Requirement: Session metadata schema is versioned

The session JSON file SHALL include a `schema_version` field. Workspace-aware session snapshots SHALL use a newer schema version than transcript-only snapshots, and loading logic SHALL handle supported legacy versions deterministically.

#### Scenario: Workspace-aware schema version loads

- **WHEN** a session file uses the current workspace-aware schema version
- **THEN** transcript and workspace state load normally

#### Scenario: Legacy schema version loads without workspace

- **WHEN** a session file uses a supported legacy schema version without workspace fields
- **THEN** transcript data loads and workspace defaults to empty state
