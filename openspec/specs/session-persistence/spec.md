# session-persistence

## Requirements

### Requirement: Session metadata is persisted locally on creation and update

When a new session is created or a message is added, the application SHALL write the session metadata to a local JSON file at `~/.brownie/sessions/<session-id>.json`. Writes SHALL be atomic (write to a temporary `.tmp` file, then rename) to prevent corruption on unclean shutdown.

#### Scenario: Session file created on session start

- **WHEN** `AppEvent::SessionCreated(session_id)` is received
- **THEN** a JSON file is written at `~/.brownie/sessions/<session-id>.json` containing at minimum: `schema_version`, session ID, workspace path, creation timestamp, and an empty transcript array

#### Scenario: Message appended to session file

- **WHEN** a user message is submitted or `AppEvent::StreamEnd` is received finalizing an assistant response
- **THEN** the session JSON file is updated to include the new message with role, content, and timestamp

#### Scenario: Write is atomic

- **WHEN** a session file is being written
- **THEN** the write targets a `.tmp` file first; only on successful write is it renamed to the canonical path; a crash during write leaves no partial file at the canonical path

---

### Requirement: Session metadata is loaded on startup to populate the session list

At startup the application SHALL scan `~/.brownie/sessions/` for valid session JSON files and load their metadata to populate the left-column session list. Corrupt or unreadable files SHALL be skipped with a warning emitted as `AppEvent::SdkError`.

#### Scenario: Valid session files loaded

- **WHEN** the application starts and `~/.brownie/sessions/` contains valid JSON files
- **THEN** each session appears in the left-column list sorted by creation timestamp descending

#### Scenario: Corrupt session file skipped

- **WHEN** a file in `~/.brownie/sessions/` is not valid JSON or is missing required fields
- **THEN** that file is skipped, an `AppEvent::SdkError` is emitted naming the skipped file, and all other sessions load normally

#### Scenario: Sessions directory does not exist

- **WHEN** `~/.brownie/sessions/` does not exist at startup
- **THEN** the directory is created, no sessions are listed, and no error is shown

---

### Requirement: Selecting a session loads its transcript into the transcript area

When a session is selected from the left-column list, the application SHALL load the full message transcript from the session JSON file and render it in the center transcript area.

#### Scenario: Transcript rendered from file

- **WHEN** the user clicks a session entry in the left column
- **THEN** all messages stored in that session's JSON file are rendered in the transcript area in chronological order

#### Scenario: Session file missing at load time

- **WHEN** a session entry is clicked but its JSON file no longer exists on disk
- **THEN** an `AppEvent::SdkError` is emitted and the transcript area shows a "Session unavailable" placeholder

---

### Requirement: Session metadata schema is versioned

The session JSON file SHALL include a `schema_version` field set to `1`. On load, if `schema_version` is absent or holds an unrecognized value, the file SHALL be treated as corrupt and skipped with a diagnostic warning.

#### Scenario: File with correct schema version loaded

- **WHEN** a session file contains `"schema_version": 1`
- **THEN** it is loaded normally

#### Scenario: File with unknown schema version skipped

- **WHEN** a session file contains an unrecognized `schema_version` value or no `schema_version` field
- **THEN** it is skipped with a diagnostic warning naming the file and the unrecognized version
