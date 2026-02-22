# canvas-block-lifecycle

## ADDED Requirements

### Requirement: Canvas mutations use explicit block lifecycle actions

All Canvas state changes SHALL be applied through explicit lifecycle actions: `open`, `update`, `focus`, `minimize`, and `close`.

#### Scenario: Block open is explicit

- **WHEN** an assistant or user requests a new Canvas block
- **THEN** the host applies an explicit `open` action and creates one new block instance

#### Scenario: Block update is explicit

- **WHEN** an assistant or user changes an existing block
- **THEN** the host applies an explicit `update` action to the targeted block

---

### Requirement: Block action targeting is deterministic

The lifecycle action pipeline SHALL resolve target blocks deterministically, preferring explicit block identity and otherwise applying stable fallback rules.

#### Scenario: Explicit block id target

- **WHEN** an action specifies a valid `block_id`
- **THEN** the action applies only to that block

#### Scenario: Ambiguous inferred target

- **WHEN** no explicit `block_id` is provided and multiple blocks match fallback criteria
- **THEN** the host returns a failed outcome requiring disambiguation and no block mutation occurs

---

### Requirement: Lifecycle action outcomes are host-confirmed and machine-readable

Each lifecycle action SHALL emit a structured success or failure outcome with actionable reason fields. Assistant text SHALL NOT be treated as authoritative action state.

#### Scenario: Unknown block update fails cleanly

- **WHEN** an `update` action targets a non-existent block
- **THEN** the host emits a failed outcome with a machine-readable reason and leaves workspace state unchanged

#### Scenario: Successful close emits completion outcome

- **WHEN** a valid `close` action is applied
- **THEN** the host emits a success outcome and the targeted block is removed from active workspace
