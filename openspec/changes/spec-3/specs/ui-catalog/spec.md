# ui-catalog

## ADDED Requirements

### Requirement: CatalogManager loads templates from configured providers

The system SHALL load UI templates through a `CatalogManager` that aggregates templates from enabled providers and builds a deterministic in-memory catalog index before intent resolution begins.

#### Scenario: Builtin and user templates are loaded at startup

- **WHEN** the app starts with builtin and user providers enabled
- **THEN** `CatalogManager` loads templates from both providers and exposes them as resolution candidates

#### Scenario: Multiple builtin templates are available

- **WHEN** the builtin provider initializes successfully
- **THEN** at least two builtin templates are present in the catalog index

---

### Requirement: Template documents must pass load-time validation

Each template SHALL include valid `meta`, `match`, and `schema` sections and SHALL pass `UiSchema` validation at catalog load time. Templates that fail validation MUST be excluded from resolution candidates.

#### Scenario: Valid template is accepted

- **WHEN** a template file has valid `meta`, `match`, and `schema` content
- **THEN** the template is added to the catalog index and can be selected during intent resolution

#### Scenario: Invalid template is rejected

- **WHEN** a template has invalid structure or fails schema validation
- **THEN** the template is excluded from the catalog index and a validation error is logged

---

### Requirement: Catalog selection uses deterministic provider precedence

`CatalogManager` SHALL resolve templates using precedence order `org -> user -> builtin` when org is enabled, and `user -> builtin` when org is disabled.

#### Scenario: User template overrides builtin template

- **WHEN** user and builtin catalogs both contain matching templates for the same intent
- **THEN** the user template is selected when org provider is not enabled

#### Scenario: Org template overrides user and builtin templates

- **WHEN** org, user, and builtin catalogs all contain matching templates for the same intent and org is enabled
- **THEN** the org template is selected

