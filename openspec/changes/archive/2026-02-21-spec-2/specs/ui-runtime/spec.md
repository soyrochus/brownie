# ui-runtime

## ADDED Requirements

### Requirement: UiRuntime loads an embedded schema fixture for deterministic rendering

The runtime SHALL load a deterministic `UiSchema` fixture from an embedded source (hardcoded Rust data or `include_str!` JSON) during app startup for SPEC-2. The fixture SHALL be deserialized into strongly typed Rust structures before any canvas rendering path executes.

#### Scenario: Embedded fixture loads successfully

- **WHEN** the application initializes the runtime with a valid embedded schema fixture
- **THEN** the fixture deserializes into typed `UiSchema` values and runtime initialization succeeds

#### Scenario: Embedded fixture is malformed

- **WHEN** the embedded fixture cannot be deserialized into typed `UiSchema` values
- **THEN** runtime initialization returns an error state and the canvas does not render interactive components

---

### Requirement: UiRuntime validates schema constraints before rendering

Before rendering any component, the runtime SHALL validate the schema against structural and contract constraints including: component allowlist membership, maximum component count, maximum nesting depth, and output-contract consistency for actionable components.

#### Scenario: Valid schema passes validation

- **WHEN** a schema respects component allowlist, nesting, count, and contract constraints
- **THEN** the runtime returns a validated schema object eligible for rendering

#### Scenario: Schema exceeds maximum component count

- **WHEN** a schema contains more components than the configured maximum
- **THEN** validation fails and the schema is rejected

#### Scenario: Schema exceeds maximum nesting depth

- **WHEN** a schema contains nested components deeper than the configured maximum
- **THEN** validation fails and the schema is rejected

---

### Requirement: UiRuntime must never render unvalidated schema data

The render path SHALL only accept validated schema objects. Raw or unvalidated schema inputs MUST NOT be passed directly to component renderers.

#### Scenario: Validation fails

- **WHEN** schema validation returns an error
- **THEN** the canvas renders a non-interactive validation error state and no unvalidated component content is rendered

#### Scenario: Validation succeeds

- **WHEN** schema validation succeeds
- **THEN** rendering proceeds from the validated schema object only

---

### Requirement: Canvas rendering is driven by typed component enums

The runtime SHALL render the canvas by matching typed component enum variants rather than string-dispatched component names. Rendering order SHALL follow the component order declared in the validated schema.

#### Scenario: Fixture includes multiple component variants

- **WHEN** the validated fixture contains markdown, form, code or diff, and button components
- **THEN** each component is rendered by its typed enum variant renderer in declared order

#### Scenario: Unsupported component variant is encountered

- **WHEN** a component variant is not supported by the runtime allowlist
- **THEN** validation rejects the schema before rendering begins

