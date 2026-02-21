# app-shell

## MODIFIED Requirements

### Requirement: Right column shows static Actions Panel placeholder

The right column SHALL render a runtime-driven Canvas from the schema of the template selected by catalog resolution in SPEC-3. The panel SHALL no longer depend on hardcoded fixture schemas and SHALL include a visible selection context showing which template was chosen. If no template matches the current intent, the panel SHALL explicitly display `No matching UI template found` and SHALL NOT silently fallback to any other UI source.

#### Scenario: Matching template renders in canvas

- **WHEN** an incoming `UiIntent` resolves to a catalog template
- **THEN** the right column renders that template's validated `UiSchema` in the canvas

#### Scenario: No matching template shows explicit message

- **WHEN** catalog resolution returns no-match for an incoming `UiIntent`
- **THEN** the right column displays `No matching UI template found` and renders no fallback interactive schema

#### Scenario: Selected template context is visible

- **WHEN** a template is selected for rendering
- **THEN** the right column includes visible selection context identifying the selected template id or display name

---

### Requirement: Diagnostics area surfaces SDK and CLI errors

The application SHALL display a diagnostics area that includes SDK and CLI error events and SHALL additionally include catalog-resolution diagnostics in SPEC-3. Resolution diagnostics SHALL record selection outcomes, selected template source, and no-match outcomes.

#### Scenario: Resolution success is logged

- **WHEN** a template is selected through catalog resolution
- **THEN** a diagnostics entry records the selected template id and provider source

#### Scenario: Resolution no-match is logged

- **WHEN** no template matches an incoming intent
- **THEN** a diagnostics entry records explicit no-match outcome for that intent

#### Scenario: Existing SDK error logging remains intact

- **WHEN** `AppEvent::SdkError(message)` is received
- **THEN** a timestamped error entry appears in the diagnostics area

