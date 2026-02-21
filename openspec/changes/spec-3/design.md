# Design: spec-3

## Context

SPEC-2 established a deterministic `UiSchema` validation and rendering runtime but still uses a hardcoded schema source. SPEC-3 replaces that hardcoded source with a deterministic catalog resolution layer so runtime rendering is selected from validated templates based on agent-declared `UiIntent`.

Current constraints:
- UI rendering must remain fully deterministic and gated by schema validation.
- Agent-driven schema generation is still out of scope.
- Existing runtime (`ui-runtime`) and event instrumentation (`ui-event-log`) should remain reusable.
- Template selection must be transparent (inspectable decision trace) and must not silently fallback on no-match.

## Goals / Non-Goals

**Goals:**
- Add a `CatalogProvider` abstraction with builtin and user implementations.
- Add `CatalogManager` intent-to-template resolution with deterministic precedence and ranking.
- Load and validate template definitions (`meta`, `match`, `schema`) before they become selectable.
- Replace hardcoded runtime schema loading with catalog-selected template schemas.
- Surface selection diagnostics and explicit no-match state in the UI shell.
- Prove deterministic behavior under repeated identical intents.

**Non-Goals:**
- No agent-generated schema path.
- No fuzzy/LLM ranking for template selection.
- No mutation of builtin catalog at runtime.
- No change to SPEC-2 schema validation gate semantics.
- No mandatory org catalog implementation in SPEC-3 (org slot remains optional/feature-gated).

## Decisions

### 1. Provider model: `CatalogProvider` + `CatalogTemplate` normalization

**Decision:** Introduce a provider interface that loads template definitions into a normalized in-memory representation:
- `CatalogProvider::list_templates() -> Result<Vec<CatalogTemplate>, CatalogError>`
- optional provider metadata (provider kind, source path, read-only flag)

Two concrete providers for SPEC-3:
- `BuiltinCatalogProvider`: embedded assets compiled into binary; read-only
- `UserCatalogProvider`: filesystem directory under user config space; writable/persisted

Each loaded template is normalized into:
- `meta` (id, title, version, tags, source provider)
- `match` (primary, operations, contextual tags/constraints)
- `schema` (`UiSchema`)

**Rationale:** Normalization decouples resolution logic from storage backends and keeps matching deterministic.

### 2. Template validation at load-time, not selection-time

**Decision:** Providers validate template documents when loading:
- structural template validation (`meta`, `match`, `schema` presence and shape)
- schema validation via existing SPEC-2 `UiSchema` validation pipeline

Invalid templates are excluded from candidate sets and logged with explicit validation reasons.

**Rationale:** Avoids runtime surprises and guarantees all selectable templates are render-safe.

### 3. Resolution precedence is explicit and stable

**Decision:** `CatalogManager` resolves by provider precedence:
- with org enabled: `org -> user -> builtin`
- without org: `user -> builtin`

Within each provider tier, candidates are ranked deterministically by scoring rules and tie-breakers.

Tie-breakers (in order):
1. higher match score
2. lexicographically smaller template id
3. lexicographically smaller provider id (for deterministic final ordering)

**Rationale:** Determinism requires stable ordering even when scores collide.

### 4. Matching algorithm is rule-based and deterministic

**Decision:** Candidate matching rules:
- hard requirement: `UiIntent.primary` must equal template `match.primary`
- optional refiners contribute deterministic score:
  - overlap count for `operations`
  - overlap count for contextual tags
  - exact equality bonuses for constrained fields if present

No probabilistic ranking, no heuristics that depend on mutable runtime state.

**Rationale:** Predictable and repeatable selection behavior is the core acceptance criterion.

### 5. Resolution results always include an inspectable decision trace

**Decision:** `CatalogManager::resolve(intent)` returns a structured result:
- selected template (if found)
- ranked candidates with scores and exclusion reasons
- provider precedence path used

UI logs a concise trace entry (selected template id/provider/score) and optionally exposes extended details in diagnostics.

**Rationale:** Transparent resolution is required for debugging and trust.

### 6. No-match is explicit and blocks render

**Decision:** If no template matches:
- runtime does not attempt fallback schema generation
- app-shell displays explicit message: `No matching UI template found`
- diagnostics include intent summary + no-match reasons

**Rationale:** Silent fallback would violate deterministic behavior guarantees and hide configuration issues.

### 7. Runtime integration: selected catalog schema becomes sole input

**Decision:** Replace SPEC-2 hardcoded schema source with:
1. receive/construct `UiIntent`
2. resolve template through `CatalogManager`
3. pass selected template `schema` into existing `UiRuntime` validation + render flow

`UiRuntime` remains authoritative for schema safety and rendering semantics.

**Rationale:** Reuses validated runtime core while changing only schema source/orchestration.

### 8. User catalog persistence is filesystem-first

**Decision:** User catalog templates live in a dedicated local directory as JSON files; reads occur on startup (and optionally manual reload trigger in SPEC-3). Builtin catalog remains embedded and immutable.

**Rationale:** Keeps persistence simple, auditable, and consistent with current app storage patterns.

### 9. Testing strategy focuses on determinism and precedence

**Decision:** Add tests for:
- provider load/validation (valid templates loaded, invalid rejected)
- primary hard-match enforcement
- secondary score ranking behavior
- precedence override (`user` beats `builtin` on equal fit)
- no-match explicit result
- repeated identical intents producing identical resolution output

**Rationale:** These directly map to SPEC-3 acceptance criteria.

## Risks / Trade-offs

- [Risk] Template drift between builtin and user catalogs causes ambiguous matches. -> Mitigation: strict deterministic tie-breakers + diagnostics showing ranked alternatives.
- [Risk] Invalid user templates degrade UX. -> Mitigation: fail-fast validation with explicit per-file errors while keeping valid templates available.
- [Risk] Scoring rules become too rigid for future use cases. -> Mitigation: isolate scoring in a dedicated resolver module with versioned match semantics.
- [Risk] No-match frequency increases during rollout. -> Mitigation: ship multiple builtin templates covering known intents and provide clear no-match messaging.

## Migration Plan

1. Introduce catalog domain types (`CatalogTemplate`, `TemplateMatch`, `CatalogSource`).
2. Implement `CatalogProvider` trait with builtin and user providers.
3. Implement `CatalogManager` resolution engine (precedence, scoring, tie-breakers, trace output).
4. Wire intent resolution into app flow before runtime rendering.
5. Replace hardcoded SPEC-2 schema path with selected template schema input.
6. Add app-shell display updates for selected-template diagnostics and explicit no-match state.
7. Add tests for providers, resolver determinism, precedence, and no-match behavior.

Rollback: disable catalog resolution path and restore SPEC-2 hardcoded schema source while retaining non-destructive catalog code behind a feature flag.

## Open Questions

- Should user catalog loading be startup-only in SPEC-3, or include a manual reload action in settings?
- For diagnostics, should full ranked candidate lists be shown by default or only on expandable detail view?
- Should org-provider support in SPEC-3 be scaffolded as interface-only, or include a concrete local-path implementation behind config?
