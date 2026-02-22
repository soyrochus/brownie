# ui-runtime

## MODIFIED Requirements

### Requirement: UiRuntime loads an embedded schema fixture for deterministic rendering

The runtime SHALL load validated schema inputs per Canvas block instance selected by workspace actions, rather than treating one global fixture as the entire canvas state. Each block schema SHALL still be deserialized into strongly typed structures and validated before rendering.

#### Scenario: Block schema loads on open

- **WHEN** a new block is opened with a valid schema payload
- **THEN** the runtime deserializes and validates that block schema and renders the block

#### Scenario: One block schema fails validation

- **WHEN** a block schema fails validation
- **THEN** that block shows a non-interactive error state while other valid blocks continue rendering

---

### Requirement: Canvas rendering is driven by typed component enums

The runtime SHALL render each block's content by matching typed component enum variants and SHALL preserve per-block local interaction state across focus and layout changes.

#### Scenario: Block state is retained after focus change

- **WHEN** a user changes focus away from a block and later returns to it
- **THEN** the block's local state (such as form values or explorer expansion state) remains intact

#### Scenario: Multiple block instances render deterministically

- **WHEN** the workspace contains multiple blocks
- **THEN** each block renders by typed component dispatch with deterministic ordering per workspace layout state
