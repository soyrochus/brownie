# Design: refactor-canvas

## Context

Brownie currently couples Canvas behavior to chat-turn intent extraction and a singleton runtime schema. In practice, this causes three systemic issues:

- Canvas state is treated as ephemeral and may be cleared/replaced by unrelated chat turns.
- Chat narration can diverge from host-rendered UI outcomes.
- UI reuse across a session is weak because there is no durable block identity/lifecycle.

The proposal for `refactor-canvas` requires a session-scoped persistent UI workspace where the assistant and user operate on explicit blocks (`open/update/focus/minimize/close`) rather than replacing one global canvas payload.

Current constraints:

- Runtime validation safety guarantees from `ui-runtime` must remain intact.
- Session behavior must remain deterministic and inspectable through diagnostics.
- Refactor should preserve existing chat transcript behavior while decoupling UI state transitions.
- This change is architectural and cross-cutting (`app-shell`, `ui-runtime`, `copilot-client`, `session-persistence`, `ui-event-log`).

## Goals / Non-Goals

**Goals:**

- Introduce a dedicated session-scoped Canvas workspace state model containing multiple UI blocks.
- Define explicit block lifecycle actions and deterministic action outcomes.
- Ensure non-UI chat turns do not mutate Canvas state.
- Preserve per-block local state for continuity (selection, form values, expand/collapse state, etc.).
- Persist and restore Canvas workspace state as part of session state.
- Provide structured diagnostics for block action outcomes that are host-truth, not chat-phrasing inference.

**Non-Goals:**

- Rewriting all UI templates/components in one pass.
- Cross-session global UI memory or long-term personalization behavior.
- Designing final visual layout system (tabs vs stacked vs split) beyond minimum workable controls.
- Introducing agent-generated arbitrary schema behavior.

## Decisions

### 1. Introduce a first-class `CanvasWorkspace` state owned by session context

**Decision:** Replace the singleton canvas selection model with a workspace model:

- `CanvasWorkspace`
  - `blocks: Vec<CanvasBlock>`
  - `active_block_id: Option<String>`
  - optional layout metadata (order/size/minimized state)

Each `CanvasBlock` includes:

- stable `block_id` (session-unique)
- `template_id` and display metadata
- lifecycle status (`active`, `minimized`, `loading`, `error`)
- block-local state payload (typed where feasible)
- provenance and timestamps

**Rationale:** This is the minimum structural shift needed for persistence and reuse.

**Alternatives considered:**

- Keep singleton canvas and add history stack. Rejected: does not support concurrent visible/reusable UI blocks.
- Derive UI solely from transcript replay. Rejected: brittle and too implicit for deterministic lifecycle control.

### 2. Make UI state mutation event-driven with explicit block lifecycle actions

**Decision:** All Canvas mutations must flow through explicit host actions:

- `OpenBlock`
- `UpdateBlock`
- `FocusBlock`
- `MinimizeBlock`
- `CloseBlock`

Each action returns deterministic outcome (`succeeded`/`failed`) with machine-readable reason.

**Rationale:** Prevents accidental mutations and gives a stable contract between assistant intent and host state.

**Alternatives considered:**

- Continue intent-to-render implicit updates from chat parsing. Rejected: repeats current coupling problems.

### 3. Enforce chat/UI separation at submission path

**Decision:** `submit_prompt` path must not directly clear/replace Canvas. UI changes are applied only when an explicit lifecycle action is emitted and accepted.

**Rationale:** Guarantees that ordinary conversation cannot wipe persistent workspace state.

**Alternatives considered:**

- Keep current pre-send intent resolution as an optimization. Rejected for default path; it may only exist if transformed into explicit action generation with strict safeguards.

### 4. Keep template catalog/resolution, but instantiate block instances instead of replacing canvas

**Decision:** Existing catalog/template selection remains useful for "what to open," but selection now creates/updates a block instance in workspace.

`template_id` is no longer equivalent to "the entire current Canvas"; it becomes an instance type for one block among many.

**Rationale:** Preserves prior investment while aligning behavior to persistent workspace semantics.

**Alternatives considered:**

- Remove catalog entirely and hardcode blocks. Rejected: loses deterministic template governance.

### 5. Add deterministic block targeting and reuse rules

**Decision:** Assistant updates/focuses target blocks by:

1. explicit `block_id` when provided
2. otherwise deterministic resolver (template type + recency + active state)
3. ambiguity returns failure requiring disambiguation

**Rationale:** Enables reliable reuse and prevents duplicate block sprawl.

**Alternatives considered:**

- Best-effort fuzzy targeting. Rejected: non-deterministic and hard to debug.

### 6. Persist Canvas workspace inside session storage

**Decision:** Extend session persistence to include Canvas workspace snapshot and block-local state.

- Session switch restores transcript + workspace together.
- New sessions begin with empty workspace unless explicitly seeded.

**Rationale:** Session continuity is a core product requirement of this refactor.

**Alternatives considered:**

- Memory-only workspace state. Rejected: session switch/reload loses continuity and violates stated UX direction.

### 7. Move diagnostics to structured UI action outcomes

**Decision:** Add structured diagnostics/event log entries for lifecycle actions and outcomes:

- `ui.block.open.requested|succeeded|failed`
- `ui.block.update.requested|succeeded|failed`
- `ui.block.focus.changed`
- `ui.block.minimize.toggled`
- `ui.block.closed`

**Rationale:** Host-truth observability prevents chat/UI contradiction and improves debugging.

**Alternatives considered:**

- Keep free-text diagnostics. Rejected: fragile, hard to machine-check, and too dependent on model phrasing.

### 8. Ensure user close control is a hard requirement

**Decision:** Each block must expose user-invoked close behavior; assistant close is allowed only through explicit action path.

**Rationale:** Prevents lock-in clutter and aligns with explicit lifecycle requirement.

**Alternatives considered:**

- Assistant-only close. Rejected: violates user agency and acceptance criteria.

## Risks / Trade-offs

- [Risk] Added state complexity increases implementation and bug surface. -> Mitigation: keep workspace core small; use strict action reducers and tests around invariants.
- [Risk] Backward compatibility with existing singleton assumptions may break. -> Mitigation: phased migration with compatibility adapter and feature-flagged fallback path during rollout.
- [Risk] Persisting block-local state can bloat session files. -> Mitigation: store minimal state; cap large payloads; avoid persisting derived render caches.
- [Risk] Ambiguous block targeting degrades assistant UX. -> Mitigation: deterministic resolver + explicit disambiguation prompt path.
- [Risk] Multi-block UI can become visually noisy. -> Mitigation: minimum controls for focus/minimize/reorder and sensible defaults.

## Migration Plan

1. Introduce workspace domain types and action reducer without removing existing singleton path.
2. Route UI mutations through lifecycle actions and log structured action outcomes.
3. Decouple `submit_prompt` from implicit canvas replacement; disable `clear on no-intent`.
4. Adapt catalog resolution to block instantiation (`open/update`) instead of full canvas replacement.
5. Add per-session workspace persistence/load; wire session switching to restore both transcript and workspace.
6. Add user close/minimize/focus controls in app-shell and deterministic assistant targeting.
7. Refine builtin block template content (especially file listing) to reduce static filler and preserve local state.
8. Remove legacy singleton replacement path after parity checks pass.

Rollback strategy:

- Keep a temporary compatibility switch to restore singleton canvas behavior during early rollout.
- If critical regressions occur, disable workspace action path and revert to prior render orchestration while retaining non-destructive data model additions.

## Open Questions

- Should block-local state be fully typed per template family now, or use a typed-envelope hybrid first for faster migration?
- What is the preferred default layout model for initial rollout (stacked list, tabbed groups, split panes)?
- Should closed blocks be recoverable in-session via a "recently closed" list in MVP?
- How strict should persistence be for volatile data (e.g., large previews, transient errors)?
- Should assistant responses include canonical block references (`block_id` + title) by default for user transparency?
