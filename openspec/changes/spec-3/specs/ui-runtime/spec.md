# ui-runtime

## MODIFIED Requirements

### Requirement: UiRuntime loads an embedded schema fixture for deterministic rendering

The runtime SHALL load a `UiSchema` from the template selected by deterministic catalog resolution for SPEC-3 instead of loading a hardcoded embedded fixture directly. The selected template schema SHALL still be deserialized into strongly typed Rust structures and SHALL remain subject to runtime validation before render.

#### Scenario: Catalog-selected template schema loads successfully

- **WHEN** a `UiIntent` resolves to a matching catalog template
- **THEN** the template `schema` is deserialized into typed `UiSchema` values and runtime initialization for that render cycle succeeds

#### Scenario: Selected template schema is malformed

- **WHEN** a resolved template contains an invalid schema payload
- **THEN** runtime initialization for that selection returns an error state and the canvas does not render interactive components

#### Scenario: No template matches the intent

- **WHEN** catalog resolution returns no-match for the incoming `UiIntent`
- **THEN** runtime does not attempt schema rendering and returns a no-template outcome for the shell to display

