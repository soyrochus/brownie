## Why

SPEC-2 proved deterministic schema validation and rendering, but it still depends on hardcoded schemas. SPEC-3 is needed to introduce deterministic intent-to-template resolution so the UI runtime can select and render cataloged templates predictably without enabling agent-generated UI.

## What Changes

- Replace hardcoded SPEC-2 schema selection with catalog-driven template resolution based on `UiIntent`.
- Add a `CatalogProvider` abstraction with at least two providers:
  - Builtin catalog provider (embedded assets, read-only)
  - User catalog provider (filesystem-backed, writable)
- Add deterministic `CatalogManager` resolution logic with precedence `org -> user -> builtin` when org is enabled (otherwise `user -> builtin`).
- Define deterministic matching rules where `primary` intent must match exactly and secondary fields (`operations`, contextual tags) refine ranking.
- Validate template definitions (`meta`, `match`, `schema`) at load time and reject invalid templates.
- Add transparent selection diagnostics that record which template was selected and why.
- If no template matches, surface explicit "No matching UI template found" state (no silent fallback).
- Continue rendering through SPEC-2 runtime once a template is selected; no schema generation path is introduced in SPEC-3.

## Capabilities

### New Capabilities

- `ui-catalog`: Catalog loading, indexing, validation, and deterministic template lookup from multiple providers.
- `catalog-providers`: Provider abstraction and implementations for embedded builtin templates and writable user filesystem templates.
- `intent-template-resolution`: Deterministic `UiIntent` matching and ranking logic with explicit selection trace logging.

### Modified Capabilities

- `ui-runtime`: Replace hardcoded schema source with selected catalog template schema as runtime input while preserving validation gate behavior.
- `app-shell`: Surface explicit no-match UI state ("No matching UI template found") and display template selection diagnostics for inspectability.

## Impact

- Adds catalog domain modules and resolver pipeline under `src/` (provider interfaces, template models, matcher, precedence handling, diagnostics).
- Requires embedded catalog assets in the binary for builtin templates and a local on-disk directory contract for user templates.
- Changes initialization flow from "load fixed schema" to "resolve from `UiIntent` then render selected `UiSchema`".
- Expands tests to cover provider loading, template validation, deterministic ranking, precedence overrides, no-match behavior, and repeatable outcomes for identical intents.
