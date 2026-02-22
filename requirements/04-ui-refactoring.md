# UI Refactoring Functional Description

## 1. Purpose

This document describes the required behavior for refactoring Brownie's Canvas/UI flow so that:

- Chat remains a conversational channel.
- Canvas represents persistent session UI state.
- UI blocks can be opened, reused, focused, and closed independently from chat turns.

This is a functional description with technical hints. It is intentionally **not** a full technical specification.

## 2. Product Direction

The system must move from:

- "chat prompt implies one immediate canvas replacement"

to:

- "session contains a living UI workspace with durable blocks that can evolve over time."

The user should experience Canvas as a first-class interaction surface, not as a visual side effect of chat responses.

## 3. UX Principles

### 3.1 Chat/UI separation

- Chat messages must not implicitly clear or replace Canvas content.
- Canvas state changes should only happen through explicit UI actions (by user or assistant).

### 3.2 Session-scoped persistence

- UI blocks persist for the current session.
- UI blocks are not required to persist across sessions.
- Opening an old session should restore that session's UI workspace state if available in memory or persisted session payload.

### 3.3 Reusability and continuity

- Assistant can refer to existing blocks ("update the existing file explorer", "focus the review panel") rather than re-creating UI each turn.
- User can manually keep a block open while discussing related items in chat.

### 3.4 Explicit lifecycle

- Every visible block has an explicit lifecycle: open, update, focus, minimize, close.
- Closing a block should be deliberate by user or assistant action, never a side effect of "no intent detected."

## 4. Functional Model

Canvas should be modeled as a **workspace of blocks**.

Each block should have at least:

- stable `block_id` within session
- `template_id` (or component family)
- title/label
- status (active, minimized, loading, error)
- internal view state (selection/filter/expanded nodes/etc.)
- provenance (opened by user or assistant)
- timestamps (created, last updated, last focused)

### 4.1 Block behaviors

- Multiple blocks can co-exist.
- One block can be focused without removing others.
- Blocks can be reordered.
- Blocks can be minimized/collapsed to reduce clutter.
- Blocks can be reopened from a recent/closed list (optional but recommended).

### 4.2 Assistant behaviors

Assistant should be able to request actions like:

- open a new block of a known type
- focus an existing block
- update block content/state
- close a block when explicitly instructed

Assistant responses should reflect action outcomes:

- "Opened block X"
- "Updated existing block Y"
- "Could not update block Z (reason)"

No message should claim rendering success without host-confirmed block action success.

## 5. Interaction Rules

### 5.1 User message handling

- Sending a chat message should never automatically wipe Canvas.
- Intent extraction can suggest UI actions, but those actions must be explicit and traceable.

### 5.2 No-intent messages

- If a message has no UI intent, keep current blocks unchanged.
- Chat continues normally.

### 5.3 Conflicting actions

- If assistant requests an update for a non-existent block, host should reject with a clear error and ask whether to create a new one.
- If multiple candidate blocks match, host should request disambiguation or apply deterministic selection rules.

### 5.4 Close semantics

- "Close block" means remove from active workspace.
- "Hide/minimize block" means keep state, reduce visual prominence.

## 6. Session Semantics

- UI workspace belongs to the active session object.
- Switching sessions should switch both transcript and UI workspace.
- New session starts with empty workspace unless explicitly seeded.
- Session save/load behavior should include enough data to restore open blocks and their local state for that session.

## 7. Diagnostics and Trust

The system should log structured UI action events, not just free-text status lines.

Recommended event families:

- `ui.block.open.requested|succeeded|failed`
- `ui.block.update.requested|succeeded|failed`
- `ui.block.focus.changed`
- `ui.block.minimize.toggled`
- `ui.block.closed`

Diagnostics should be understandable for debugging but not leak internal noise into normal UX.

## 8. Template and Rendering Direction

Existing template catalog can remain, but template selection should produce **block instances** rather than replacing a singleton canvas payload.

For templates like file listing:

- avoid static filler sections unrelated to current task
- prefer task-oriented structure (explorer + optional preview panel)
- preserve user navigation state inside the block between interactions

## 9. Technical Hints (Non-prescriptive)

These are implementation hints, not binding design constraints.

### 9.1 Introduce a dedicated Canvas workspace state

Maintain a state container separate from transcript state, e.g.:

- `CanvasWorkspace { blocks, active_block_id, layout }`

Do not derive current canvas content from "latest chat turn."

### 9.2 Use explicit UI action events

Replace implicit side effects with explicit host actions:

- `OpenBlock`
- `UpdateBlock`
- `FocusBlock`
- `CloseBlock`
- `MinimizeBlock`

Make all assistant-driven UI mutations go through this action layer.

### 9.3 Keep assistant tool contract action-oriented

Tool responses should return action results with:

- action type
- target `block_id` (if applicable)
- status
- optional message

This avoids ambiguous "rendered/not rendered" text interpretation.

### 9.4 Preserve per-block local state

Treat each block as stateful.

Examples:

- file explorer expansion and selected file
- form field values
- diff review decisions

### 9.5 Support deterministic block targeting

When updating/focusing blocks, prefer deterministic targeting:

- by explicit `block_id`
- fallback by template type + recency + focus state

### 9.6 Keep UI clear in crowded sessions

Introduce lightweight layout controls:

- tabs, stack, or split mode
- minimization and quick-switch list

This prevents multi-block sessions from becoming noisy.

## 10. Migration Guidance

Refactor in phases:

1. Stop clearing Canvas on non-UI chat.
2. Introduce workspace-with-blocks state model.
3. Route assistant UI actions through explicit block lifecycle events.
4. Update templates to reduce static filler and improve stateful reuse.
5. Add session-level restoration of UI workspace.

Each phase should keep chat functionality stable while increasing UI continuity.

## 11. Out of Scope for This Document

- Detailed Rust type definitions
- Final event schema/versioning contract
- Storage serialization format details
- Final widget/component catalog redesign
- Accessibility and visual design specifics

Those should be covered in a formal technical spec after this direction is accepted.

## 12. Acceptance Checklist

Use this as a go/no-go checklist for rollout validation.

### 12.1 Core behavior

- [ ] Sending a non-UI chat message does not clear or replace existing Canvas blocks.
- [ ] Canvas can hold multiple blocks at the same time.
- [ ] At least one block can be focused without removing other blocks.
- [ ] Blocks can be explicitly closed by user action.
- [ ] Blocks can be explicitly closed by assistant action only when intent is clear.
- [ ] "No intent detected" never triggers implicit block removal.

### 12.2 Session behavior

- [ ] Session switch updates both transcript and Canvas workspace state together.
- [ ] New session starts with an empty workspace unless intentionally seeded.
- [ ] Returning to a previously opened session restores its block set and per-block state.
- [ ] Cross-session leakage of Canvas state does not occur.

### 12.3 Assistant and tool behavior

- [ ] Assistant can open a new block and receives a host-confirmed success/failure result.
- [ ] Assistant can target and update an existing block by deterministic identity.
- [ ] Assistant can focus an existing block without recreating it.
- [ ] Assistant never claims "rendered" without host-confirmed action success.
- [ ] Failed UI actions return actionable error reasons (missing target, ambiguous target, invalid request).

### 12.4 Statefulness and reuse

- [ ] File explorer block preserves local interaction state (e.g., expanded nodes, current selection) during session.
- [ ] Form-based blocks preserve field values during session unless explicitly reset.
- [ ] Re-focusing or updating an existing block reuses the same block identity.
- [ ] Repeated related prompts reuse existing block when appropriate instead of spawning duplicates.

### 12.5 Diagnostics and observability

- [ ] Structured lifecycle events are logged for open/update/focus/minimize/close actions.
- [ ] Diagnostics clearly indicate success vs failure for each UI action.
- [ ] Diagnostics are attributable to actor/source (assistant or user).
- [ ] Diagnostics do not depend on natural-language assistant phrasing to infer state.

### 12.6 UX quality gates

- [ ] Canvas does not feel chat-driven; it feels like a persistent session workspace.
- [ ] Template content avoids irrelevant static filler in primary task blocks.
- [ ] Crowded Canvas scenarios remain usable (focus, minimize, reorder, or equivalent controls).
- [ ] User can continue chat while keeping important UI blocks visible and reusable.

### 12.7 Rollout readiness

- [ ] Legacy single-template replacement behavior is removed or fully gated behind a disabled fallback path.
- [ ] Migration does not regress existing chat transcript behavior.
- [ ] Known failure modes are documented with mitigation paths.
- [ ] Team agrees that this functional checklist is satisfied before declaring the refactor complete.
