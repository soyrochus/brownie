# Design: spec-2

## Context

SPEC-1 delivered a stable app shell, chat transcript, Copilot connectivity, and session persistence, but the right panel is still a static placeholder. The next risk to retire is whether Brownie can safely and deterministically render a dynamic UI from a declarative schema without relying on agent intent resolution or catalog lookup.

Current constraints:
- The application is an egui desktop app with a fixed three-column shell.
- `AppEvent` currently covers chat/connection diagnostics only; no typed UI interaction channel exists yet.
- UI rendering must not accept arbitrary or unvalidated schema data.
- SPEC-2 must prove runtime behavior in isolation using a hardcoded or embedded schema fixture.
- Visual refactor must move the current UI toward `specs/images/ui-draft.png` while keeping layout grid, component hierarchy, interaction logic, panel positioning, and component IDs unchanged.

## Goals / Non-Goals

**Goals:**
- Introduce a deterministic `UiRuntime` that deserializes and validates `UiSchema` before rendering.
- Introduce a typed `ComponentRegistry` and typed component enum to render the Canvas without string-based runtime branching.
- Render a representative hardcoded schema including markdown, mixed-type form fields, code or diff content, and two buttons.
- Emit typed `UiEvent` values from component interactions and show them in a debug panel to prove deterministic propagation.
- Enforce schema constraints locally: allowlist, max component count, max nesting depth, and event-contract checks.
- Keep chat functional while making Canvas the primary interactive surface in SPEC-2.
- Apply the style requirements from `specs/spec-ui-style.md` as first-class implementation constraints.

**Non-Goals:**
- No Copilot-driven `UiIntent` resolution.
- No catalog or template matching or promotion flow.
- No runtime schema generation from the agent.
- No Active Mode or tool approval workflow changes.
- No layout grid changes.
- No component tree or hierarchy changes.
- No panel positioning changes.
- No component ID changes.

## Decisions

### 1. Schema and event domain models are strongly typed Rust enums and structs

**Decision:** Define `UiSchema`, component variants, form field variants, contracts, and `UiEvent` as strongly typed Rust types (`serde::{Serialize, Deserialize}`), using internally tagged enums for component and field types.

**Rationale:** This removes ambiguous string interpretation at render time and forces invalid or unknown variants to fail parse or validation.

**Alternative considered:** `serde_json::Value` plus manual string matching. Rejected because it weakens type guarantees and promotes ad-hoc interpretation.

### 2. Validation is a required gate before any render call

**Decision:** Add `validate_schema(&UiSchema) -> Result<ValidatedSchema, ValidationError>` inside `UiRuntime::load_schema`. Rendering only accepts `ValidatedSchema`; raw schema values are never rendered.

Validation rules in SPEC-2:
- component type must be in allowlist (enum parse + registry check)
- total component count must be `<= MAX_COMPONENTS`
- nesting depth must be `<= MAX_DEPTH`
- actionable component IDs must be unique
- each button ID must map to a declared contract output event

**Rationale:** This creates a hard trust boundary and enforces the acceptance criterion that unvalidated UI definitions never render.

### 3. `ComponentRegistry` owns renderer lookup and component contracts

**Decision:** Introduce a registry keyed by component enum discriminant, where each entry provides:
- a render function implementing a shared `RenderableComponent` trait
- declared interaction contract metadata (event-emitting behavior, payload shape)

**Rationale:** Centralized registry keeps allowlist policy and rendering behavior aligned and prevents unsupported component rendering.

### 4. Deterministic fixture schema is embedded locally for this phase

**Decision:** Use an embedded schema fixture (hardcoded constant or `include_str!` JSON) loaded at app startup for Canvas rendering.

Fixture must include:
- markdown text
- form with mixed field types
- code block or diff component
- two buttons bound to declared output events

**Rationale:** This isolates runtime correctness from agent and catalog concerns and keeps behavior reproducible.

### 5. Canvas and UI event debug panel are promoted in the shell

**Decision:** Replace the right-panel placeholder with runtime-driven Canvas rendering plus an append-only UI event log section. Chat remains functional in center panel but is not the primary focus.

Integration shape:
- Canvas render path consumes `ValidatedSchema`
- component interactions push typed `UiEvent` values into app state
- event log renders typed events in stable append order

**Rationale:** Satisfies SPEC-2 acceptance criteria while preserving SPEC-1 chat and diagnostics behavior.

### 6. Testing strategy is validator-first plus deterministic interaction tests

**Decision:** Add tests in three layers:
- validation unit tests (valid schema accepted; malformed or contract-violating schemas rejected)
- runtime render tests (validated schema produces expected component render state)
- interaction tests (button clicks and form edits produce deterministic typed `UiEvent` sequences)

**Rationale:** SPEC-2 requires proof of both correctness and safety boundaries.

### 7. Visual style tokens are centralized and mandatory

**Decision:** Define shared UI style tokens and use them consistently rather than inline per-widget colors and spacing.

Required color system:
- Surface 0 (app background): `#0F1115`
- Surface 1 (panels): `#161A20`
- Surface 2 (elevated blocks): `#1C222B`
- Surface 3 (active focus): `#222A35`
- Primary accent: `#3B82F6`
- Hover accent: `#4C8EF7`
- Muted accent: `#2F6ED8`
- Success: `#22C55E`
- Warning: `#F59E0B`
- Danger: `#EF4444`
- Primary text: `#E6EDF3`
- Muted text: `#8B949E`

**Rationale:** Tokenization enables coherent surface layering and controlled contrast across all panels and components.

### 8. Spacing uses a strict 8px-derived scale

**Decision:** Use only `{4, 8, 12, 16, 24, 32}` for padding and spacing.

Required adjustments:
- left panel internal padding: `16px`
- chat message vertical spacing: `16px`
- Canvas padding: `24px`
- action item spacing: `12px`
- bottom input vertical padding: `12px`

**Rationale:** Removes cramped spacing and makes the UI rhythm predictable.

### 9. Panel styling and top bar refinement follow low-contrast layering

**Decision:** Apply panel container styling:
- corner radius `10-12px`
- border `1px solid rgba(255,255,255,0.05)`
- shadow:
  - `0px 1px 2px rgba(0,0,0,0.4)`
  - `0px 8px 24px rgba(0,0,0,0.25)`

Top bar styling:
- subtle gradient `#161A20 -> #14181E`
- reduced vertical height
- centered connection status with a small success dot
- softer separators and reduced contrast

**Rationale:** Matches the intended layered depth without heavy borders.

### 10. Typography hierarchy is tuned without broad font-family change

**Decision:** Keep current font family unless blocked by framework constraints; apply explicit scale and weight hierarchy:
- workspace title: `14px` semibold
- section headers: `13px` medium
- chat body: `14px` regular
- code: `13px` monospace
- metadata: `12px` muted
- line height target: `1.4-1.6` (body), `1.3` (code)

**Rationale:** Improves readability and hierarchy while minimizing implementation risk.

### 11. Component-level visual rules are explicit and constrained

**Decision:** Apply these visual changes without changing component structure:
- Chat bubbles:
  - user bubble: Surface 2 background, `12px` radius, `12px` padding, slight right offset
  - assistant bubble: slightly lighter surface, left-aligned, soft internal shadow
  - bubble vertical gap: `8px`
- Buttons:
  - radius `8px`, height `34-36px`, horizontal padding `14px`
  - primary: accent fill, white text, lighter on hover, darker when pressed
  - secondary: Surface 2 fill with border `rgba(255,255,255,0.08)`
- Sidebar session list items:
  - rounded shape, hover on Surface 3
  - active item has `3px` accent left border
  - item spacing `6px`
- Diff component:
  - added rows `rgba(34,197,94,0.15)` with green left indicator
  - removed rows `rgba(239,68,68,0.15)` with red left indicator
  - code font `13px` monospace, line vertical padding `4px`
- Input bar:
  - full-width rounded container
  - Surface 2 background and subtle 1px border
  - internal padding `12px`
  - focus glow from accent color at ~20% opacity

**Rationale:** These are the highest-impact visual deltas needed to align with the style target.

## Risks / Trade-offs

- [Risk] Validation rules drift from registry capabilities. -> Mitigation: derive allowlist checks from shared registry metadata.
- [Risk] egui widget state can emit noisy intermediate events. -> Mitigation: emit only on explicit transitions, not every frame repaint.
- [Risk] Embedded fixture diverges from future external schema format. -> Mitigation: run the fixture through the same serde and validation pipeline as external input.
- [Risk] Visual tuning accidentally changes layout behavior. -> Mitigation: enforce no-layout-change checklist (grid, hierarchy, positions, IDs) during review.
- [Risk] Lower contrast reduces readability. -> Mitigation: verify contrast for primary and muted text against each surface token and tune if needed.

## Migration Plan

1. Add runtime modules (`ui_schema`, `ui_validate`, `ui_runtime`, `component_registry`, `ui_event`).
2. Add embedded fixture schema and runtime initialization path.
3. Replace right-panel placeholder with Canvas + UI event log.
4. Add centralized style token definitions and panel-level styling (surfaces, borders, shadows, top bar gradient).
5. Apply component-level styling (chat bubbles, buttons, sidebar rows, diff, input bar) without structural changes.
6. Run runtime correctness tests plus style-conformance checks from `specs/spec-ui-style.md`.
7. Verify chat, diagnostics, and session persistence regressions are absent.
8. On runtime initialization or validation failure, render explicit Canvas error state and never render unvalidated schema.

Rollback: restore SPEC-1 right-panel placeholder and disable runtime initialization path.

## Open Questions

- Should the debug panel default to compact typed event rows or raw JSON payload display?
- For SPEC-2, should `diff` remain a separate component variant or be represented as `code` with mode metadata?
- Should output contracts be closed enum variants now, or typed string IDs validated against declared outputs to ease future catalog evolution?
