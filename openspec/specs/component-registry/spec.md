# component-registry

## Requirements

### Requirement: ComponentRegistry defines an explicit render allowlist

The registry SHALL define an explicit set of supported component kinds for SPEC-2. At minimum the allowlist SHALL include `markdown`, `form`, `code` or `diff`, and `button`, and MAY include additional declared variants. Any component not present in the registry allowlist MUST be rejected before rendering.

#### Scenario: Allowed component is resolved

- **WHEN** validation or runtime requests a renderer for an allowlisted component kind
- **THEN** the registry returns the matching renderer contract

#### Scenario: Unknown component is requested

- **WHEN** validation encounters a component kind not present in the allowlist
- **THEN** validation fails with an unknown-component error and rendering is blocked

---

### Requirement: Registry-driven field typing for form components

Form fields SHALL be represented by typed field variants and validated against the supported field-type set for SPEC-2 (`text`, `number`, `select`, `checkbox`).

#### Scenario: Supported form field types are accepted

- **WHEN** a form contains only supported field types
- **THEN** the form is accepted and rendered with typed field handlers

#### Scenario: Unsupported form field type is provided

- **WHEN** a form includes a field type outside the supported set
- **THEN** schema validation fails and the form is not rendered

---

### Requirement: Registry enforces button output-contract bindings

Every button renderer contract SHALL require that the button `id` resolves to a declared output event in schema contracts. A button without a declared output mapping MUST cause schema rejection.

#### Scenario: Button id maps to declared output

- **WHEN** a button id is present in the schema output contract declarations
- **THEN** validation passes for that button and interaction wiring is enabled

#### Scenario: Button id is missing from outputs

- **WHEN** a button id is not declared in output contracts
- **THEN** validation fails with a contract-mismatch error and rendering is blocked

---

### Requirement: Render dispatch is registry and enum based

Component rendering SHALL be dispatched through the registry using typed component enum variants; string-based fallback dispatch MUST NOT be used.

#### Scenario: Typed dispatch path is used

- **WHEN** runtime renders a validated component list
- **THEN** each component is dispatched through the registry using its enum variant and no generic string dispatch path is executed

