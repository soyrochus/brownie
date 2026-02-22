# ui-event-log

## MODIFIED Requirements

### Requirement: UI interactions emit typed UiEvent values

Interactive UI behavior SHALL emit strongly typed events for both component interactions and Canvas block lifecycle actions. Lifecycle events SHALL include action type, target block identity, actor/source, and outcome status.

#### Scenario: User closes a block

- **WHEN** a user closes a Canvas block
- **THEN** a typed lifecycle event is emitted with `action=close`, target block id, actor=`user`, and success status

#### Scenario: Assistant update action fails

- **WHEN** an assistant-triggered block update action fails validation or targeting
- **THEN** a typed lifecycle event is emitted with actor=`assistant`, failed status, and reason payload

---

### Requirement: Debug panel displays append-only UiEvent log

The app SHALL display append-only typed UI event records for component interactions and lifecycle action outcomes in chronological order.

#### Scenario: Lifecycle outcomes appear in debug panel

- **WHEN** lifecycle actions are requested and resolved
- **THEN** requested and resolved events are appended to the debug panel in order

#### Scenario: Prior log entries are retained

- **WHEN** additional UI interactions occur
- **THEN** previous event entries remain visible and are not overwritten
