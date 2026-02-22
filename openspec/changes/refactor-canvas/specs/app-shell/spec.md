# app-shell

## MODIFIED Requirements

### Requirement: Right column shows static Actions Panel placeholder

The right column SHALL render a persistent session-scoped Canvas workspace instead of a single replace-on-turn canvas payload. The panel SHALL support multiple coexisting blocks, visible focus context, and user controls to focus, minimize, and close blocks. A chat turn with no UI action SHALL NOT clear or replace existing blocks.

#### Scenario: Existing blocks persist across non-UI chat turn

- **WHEN** one or more blocks are open and the user sends a chat-only message
- **THEN** the existing blocks remain visible and unchanged

#### Scenario: New block opens without replacing existing blocks

- **WHEN** an explicit block open action is accepted
- **THEN** a new block appears in the workspace while previously open blocks remain available

#### Scenario: User closes a block from the canvas

- **WHEN** the user triggers close on a visible block
- **THEN** only that block is removed and remaining blocks keep their current state

---

### Requirement: Diagnostics area surfaces SDK and CLI errors

The application SHALL display a diagnostics area that includes SDK and CLI error events and SHALL additionally include structured UI action outcome events for Canvas block lifecycle operations. Diagnostics entries SHALL distinguish requested, succeeded, and failed block actions with actor/source attribution.

#### Scenario: Block action failure is logged as structured outcome

- **WHEN** a Canvas block action fails
- **THEN** diagnostics append an entry with action type, target reference, failure status, and reason

#### Scenario: Block action success is logged as structured outcome

- **WHEN** a Canvas block action succeeds
- **THEN** diagnostics append an entry with action type, target reference, and success status

#### Scenario: Existing SDK error logging remains intact

- **WHEN** `AppEvent::SdkError(message)` is received
- **THEN** a timestamped error entry appears in the diagnostics area
