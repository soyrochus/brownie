## 1. Workspace State Model

- [x] 1.1 Add `CanvasWorkspace` and `CanvasBlock` domain models with stable `block_id`, status, focus metadata, and per-block local state containers.
- [x] 1.2 Extend `AppEvent` and internal action types to represent explicit block lifecycle operations (`open`, `update`, `focus`, `minimize`, `close`).
- [x] 1.3 Implement a deterministic workspace reducer/path so all Canvas mutations flow through lifecycle actions instead of ad-hoc state changes.

## 2. App Shell Refactor

- [x] 2.1 Refactor `BrownieApp` canvas state from singleton intent/template fields to a session-scoped workspace state object.
- [x] 2.2 Remove implicit canvas clearing/replacement on non-UI chat turns from the submit path.
- [x] 2.3 Update right-panel rendering to support multiple coexisting blocks with visible focus state.
- [x] 2.4 Add user controls for block focus, minimize, and close in the Canvas UI.

## 3. Runtime and Block Rendering

- [x] 3.1 Update `UiRuntime` orchestration to load/validate/render schemas per block instance rather than one global active schema.
- [x] 3.2 Preserve per-block local state across focus/minimize/layout changes.
- [x] 3.3 Ensure validation failures isolate to the affected block and do not block rendering of other valid blocks.

## 4. Copilot and Action Outcomes

- [x] 4.1 Replace render-claim-centric assistant/UI coupling with host-confirmed lifecycle action outcomes.
- [x] 4.2 Implement deterministic block targeting (explicit `block_id` first, stable fallback resolver second, fail on ambiguity).
- [x] 4.3 Ensure failed lifecycle actions do not mutate workspace state and return actionable failure reasons.
- [x] 4.4 Keep passive mode constraints intact while allowing only host-governed Canvas lifecycle action interfaces.

## 5. Session Persistence Integration

- [x] 5.1 Extend session schema/version to include Canvas workspace snapshot and block-local persisted state.
- [x] 5.2 Persist workspace updates atomically together with transcript/session metadata.
- [x] 5.3 Restore transcript + workspace together when selecting or loading a session.
- [x] 5.4 Add deterministic handling for supported legacy session files that do not include workspace state.

## 6. Structured UI Event Logging

- [x] 6.1 Add typed lifecycle event variants and payloads for requested/succeeded/failed block actions with actor/source attribution.
- [x] 6.2 Render lifecycle outcome events in append-only diagnostics and UI event log surfaces.
- [x] 6.3 Ensure diagnostics are generated from host action results, not inferred from assistant prose.

## 7. Template and UX Refinement

- [x] 7.1 Update file-listing template behavior/content to reduce irrelevant static filler and align with persistent block usage.
- [x] 7.2 Keep block content reusable across related prompts without spawning unnecessary duplicates.
- [x] 7.3 Add minimal workspace management affordances for crowded sessions (focus + minimize + deterministic order).

## 8. Verification and Regression Coverage

- [x] 8.1 Add unit tests for workspace reducer invariants and lifecycle transitions.
- [x] 8.2 Add tests for deterministic block targeting and ambiguity failure behavior.
- [x] 8.3 Add tests for session persistence/restore including workspace-aware and legacy schema files.
- [x] 8.4 Add tests proving non-UI chat turns do not clear Canvas blocks.
- [x] 8.5 Add tests for structured lifecycle outcome logging and append-only event visibility.
