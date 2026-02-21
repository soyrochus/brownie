# catalog-providers

## ADDED Requirements

### Requirement: BuiltinCatalogProvider serves embedded read-only templates

The builtin provider SHALL load templates from embedded binary assets and SHALL expose them as read-only catalog entries.

#### Scenario: Builtin templates load from embedded assets

- **WHEN** `BuiltinCatalogProvider` is initialized
- **THEN** templates are loaded from embedded assets without filesystem dependency

#### Scenario: Builtin template mutation is rejected

- **WHEN** a write or delete operation is attempted against a builtin template
- **THEN** the provider rejects the operation as read-only

---

### Requirement: UserCatalogProvider serves writable filesystem templates

The user provider SHALL load templates from a local filesystem directory and SHALL support create, update, and delete persistence operations for user-owned templates.

#### Scenario: User templates load from local directory

- **WHEN** the configured user catalog directory contains template files
- **THEN** `UserCatalogProvider` loads them as resolution candidates

#### Scenario: User template is persisted

- **WHEN** a valid template is created or updated through user-catalog persistence
- **THEN** the template is written to disk and available on the next catalog load

---

### Requirement: Provider outputs include source identity for diagnostics

Each template loaded from any provider SHALL include provider source metadata so selection logs can indicate where the template originated.

#### Scenario: Selected template source is inspectable

- **WHEN** a template is selected for an intent
- **THEN** diagnostics include both template id and provider source identity

