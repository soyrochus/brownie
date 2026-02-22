## Why

Brownie currently treats Canvas as a chat-driven single render target, which causes state loss, contradictory chat/UI outcomes, and weak reuse across turns. This change is needed now to make Canvas a persistent session workspace where UI blocks are explicitly managed and remain useful across interactions.

## What Changes

- Refactor Canvas from "single selected template" to a session-scoped workspace of persistent UI blocks.
- Separate chat flow from UI state mutations so ordinary chat messages do not implicitly clear or replace Canvas.
- Introduce explicit block lifecycle semantics: open, update, focus, minimize, and close.
- Add deterministic block targeting and reuse so assistant updates/focuses existing blocks instead of recreating them per turn.
- Add user-facing close/minimize controls and keep block state reusable until explicitly closed.
- Persist per-session Canvas workspace state (open blocks + local block state) with transcript/session context.
- Evolve diagnostics from text-only render assertions to structured UI action outcomes for trust and debuggability.
- Update builtin file-listing behavior to favor task-oriented block content and reduce irrelevant static filler.
- **BREAKING**: Replace current implicit "intent resolves to one canvas replacement" behavior with explicit block action behavior and lifecycle-driven state changes.

## Capabilities

### New Capabilities

- `canvas-workspace`: Session-scoped multi-block Canvas model with explicit block identity, lifecycle, focus, and layout semantics.
- `canvas-block-lifecycle`: Host-enforced action model for `open/update/focus/minimize/close` with deterministic success/failure outcomes.

### Modified Capabilities

- `app-shell`: Change right panel behavior from singleton template replacement to persistent workspace rendering and user block controls.
- `ui-runtime`: Support rendering and maintaining multiple block instances with per-block local state continuity.
- `session-persistence`: Persist and restore Canvas workspace state as part of the active session lifecycle.
- `copilot-client`: Shift assistant-to-Canvas contract from render-claim text coupling to explicit, host-confirmed UI action outcomes.
- `ui-event-log`: Add structured lifecycle event logging for block actions and action outcomes (requested/succeeded/failed).

## Impact

- Affects core UI state and rendering flow in `src/app.rs`, `src/ui/runtime.rs`, and event contracts in `src/event.rs`.
- Requires session model/storage updates under `src/session/` to persist and restore Canvas workspace state per session.
- Changes assistant integration behavior in `src/copilot/mod.rs` to use action-oriented UI outcomes rather than chat-coupled replacement semantics.
- Requires updated builtin template expectations/assets for file listing and other task blocks to reduce static filler and improve continuity.
- Requires expanded tests for block lifecycle rules, deterministic block targeting, session restore behavior, and chat/UI separation regressions.
