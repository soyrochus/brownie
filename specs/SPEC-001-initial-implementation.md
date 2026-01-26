```md
# Feature Spec: Brownie Docs Generator (Staged, No-Parser; Copilot SDK only for Codegen Phase)

Feature ID: BROWNIE-DOCS

Owner: <name/role>

Service owner(s): brownie-cli, brownie-runner

Status: Draft

Last updated: 2026-01-29

## 0. Purpose of this Feature Spec

Implement Brownie as a CLI app (`init`, `analyze`) that generates a `/docs` directory containing a fixed documentation set via staged, bounded repository reading (directory-scoped), persisting intermediate facts in `.brownie/cache`, and writing each output doc as a separate file to avoid context growth.

Important boundary: GitHub Copilot SDK is NOT a dependency of the Brownie application runtime/architecture; it is ONLY a dependency for the *code generation phase* performed by the code-generation agent that implements this feature. :contentReference[oaicite:0]{index=0} :contentReference[oaicite:1]{index=1}

## 1. Intent and Outcome

Provide a deterministic(ish) workflow that generates `docs/` with:
- Project Intent & Business Frame
- Domain Landscape
- Canonical Data Model
- Service & Capability Map
- Architectural Guardrails
- API / Integration Contracts (if applicable)
- User Journey & UI Intent (if applicable)

The analysis must be staged and repeatable; cross-document consistency is achieved via a persistent “facts” artifact stored in `.brownie/cache`.

## 2. Scope (In / Out)

In:
- The Brownie app must be buold on the Copilot SDK. 
- `brownie init` creates `.brownie/` structure and `.brownie/brownie.toml` with commented example parameters.
- `brownie analyze` generates `/docs` with the required doc set.
- Configuration-first design: all behavior configurable via `.brownie/brownie.toml`; CLI flags can override; optional write-back to config.
- Directory-level include/exclude controls (directories only in this version).
- `.brownie/cache` stores temporary state (facts, open questions, run-state).

Out:
- Mandatory parsing (Icode/tree-sitter/AST) as a required step.
- Default scanning of all folders in the project root.
- Glob-based include/exclude (possible v3).


## 3. Primary Constraints and Guardrails

Must:
- Keep reads bounded: file reads are chunked and capped; grep/search bounded by hits.
- Avoid “single-context” repo ingestion.
- Write each document as its own file, generated from cached facts.
- Continue under partial evidence; mark gaps explicitly rather than failing.

Must not:
- Fail because a parser fails.
- Read excluded directories.
- Use Copilot SDK in the Brownie application itself.

## 4. Functional Behaviour

### 4.1 Init

`brownie init` creates:
- `.brownie/`
- `.brownie/brownie.toml` (authoritative configuration; includes commented examples)
- `.brownie/cache/` (scratch/temp state)

### 4.2 Analyze

`brownie analyze` creates or replaces `<root>/docs/` and writes (exact names):
- `docs/project-intent-business-frame.md`
- `docs/domain-landscape.md`
- `docs/canonical-data-model.md`
- `docs/service-capability-map.md`
- `docs/architectural-guardrails.md`
- `docs/api-integration-contracts.md` (if not applicable: write stub with evidence note)
- `docs/user-journey-ui-intent.md` (if not applicable: write stub with evidence note)

### 4.3 Include / Exclude directories (directories-only in first version)

- `include_dirs`: default `["src"]` (overrideable)
- `exclude_dirs`: default excludes typical dirs: `["node_modules","dist","build",".git",".brownie","docs"]` (overrideable)
- Only files under included dirs are considered. Excluded dirs are never read.

### 4.4 Config-first with override parity

- Precedence: CLI overrides > config > defaults.
- Optional `--write-config` updates `.brownie/brownie.toml` to match effective parameters.

## 5. Local Data and Outputs

### 5.1 Structure

`.brownie/`
- `brownie.toml`
- `cache/`

`docs/`
- canonical markdown files listed above

### 5.2 Cache artifacts (minimum)

`.brownie/cache/`
- `facts.md` (or `facts.jsonl`): atomic claims with evidence pointers (file path + line ranges)
- `open-questions.md`: unresolved items discovered
- `run-state.json`: progress markers to resume

## 6. Implementation Boundary: Copilot SDK (Code Generation Agent Only)

This feature will be implemented by a code-generation agent. That agent MUST use GitHub Copilot SDK *only during code generation*, not as a runtime dependency of the Brownie application.

For the code-generation phase, the agent’s primary reference is:
- `Copilot-SDK-Tutorial.md` (primary) :contentReference[oaicite:2]{index=2}

Secondary reference (when needed) for API details/edge behavior:
- `(workspace)/vendor/copilot-sdk/` source tree (secondary)

Non-goal: the Brownie app must not require Copilot SDK to run once generated; Copilot SDK is a *build-time / generation-time* dependency for the agent, not a product dependency.

## 7. Interfaces

### 7.1 CLI

Commands:
- `brownie init [--root <path>]`
- `brownie analyze [--root <path>] [--include_dirs <csv>] [--exclude_dirs <csv>] [--docs_dir <path>] [--write-config] [--reset-cache]`

### 7.2 Internal tool contract (conceptual)

The agent may implement/assume these primitives:
- list directories/files (bounded)
- read file slice (start line + max lines)
- grep/search (bounded hits)

No parser tool is required!

## 8. Test Expectations

Unit tests:
- config merge precedence
- include/exclude directory filtering
- docs file set creation (including “not applicable” stubs)

Integration tests:
- `init` creates correct structure
- `analyze` generates `/docs` for a fixture repo
- resume behavior with existing run-state

## 9. Open Questions / Decisions

- Applicability detection rules (minimum heuristic):
  - API/Integration: OpenAPI/Swagger files, router/controller conventions, API gateway configs
  - UI Intent: presence of UI frameworks, `/ui` or `frontend` directories, route/view components
- Replace vs update docs directory (default: replace)

## 10. References

- Feature spec template: `feature-spec.md` :contentReference[oaicite:3]{index=3}
- Copilot SDK reference for code-generation agent: `Copilot-SDK-Tutorial.md` :contentReference[oaicite:4]{index=4}
- Copilot SDK source (secondary): `vendor/copilot-sdk/` (workspace)

## 11. Change Log

- 2026-01-29: Added explicit Copilot SDK boundary (code-generation phase only).
```
