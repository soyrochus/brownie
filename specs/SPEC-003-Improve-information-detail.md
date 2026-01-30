# SPEC-003 — Improve Information Detail and Evidence Depth

Status: Abandonded - Superseded by Spec-004  
Owner: Iwan van der Kleijn  
Last updated: 2026-01-30  

---

## 1. Purpose

This spec defines changes to Brownie’s analysis workflow to improve the **density, specificity, and evidence quality** of the generated documentation, **without relying on existing docs**. The goal is to shift the agent from “header skimming” to “behavioral understanding,” while still respecting bounded reads and no full‑repo ingestion.

---

## 2. Motivation

Current output is often shallow because the agent:
- Reads only the first ~20 lines of many files
- Records high‑level “file exists” facts rather than behavioral facts
- Lacks structured incentives to deepen exploration when evidence is thin
- Performs only minimal or generic search probes

Desired behavior:
- Identify and explore **core files** more deeply
- Extract **function‑level** facts with evidence
- Iterate when evidence is insufficient
- Produce richer, more explanatory docs without relying on existing project documentation

---

## 3. Requirements

### 3.1 Deep Sampling Policy

The agent MUST increase depth for files likely to contain core logic.  
Specifically:

- For “core” files (see 3.2), the agent MUST read at least **200–400 lines** total (in bounded slices) or **until the main functions/classes are observed**.
- For non‑core files, the agent MAY keep short reads, but MUST still extract at least one behavioral fact if possible.

### 3.2 Core File Identification (Heuristic)

The agent MUST classify files into:
- **Tier 1: Entry/Orchestration**
  - CLI entry points (`cli`, `__main__`, `main`)
  - Top‑level orchestration modules
  - Application bootstrap/startup files
- **Tier 2: Domain/Service Logic**
  - Service layers
  - Domain entities
  - Workflow orchestration
- **Tier 3: Utilities/Helpers**
  - Data helpers, internal utilities

The agent MUST deep‑read Tier 1 and Tier 2 files, and MAY skim Tier 3.

### 3.3 Structured Discovery Loop

The analysis phase MUST include a **depth check**:

1. Gather initial facts (Phase 1).
2. Assess evidence quality:
   - If more than 40% of facts are “file exists” or “module defines X” without behavior, the agent MUST rescan with deeper reads.
   - If any required doc section has fewer than **N evidence points** (see 3.7), the agent MUST rescan targeted areas.

### 3.4 Targeted Search Probes (Generalized)

The agent MUST run **generic, cross‑project search probes** to find behavioral signals:

Examples (non‑exhaustive, generic):
- Entry points: `main`, `__main__`, `cli`, `command`, `runner`
- Interfaces/boundaries: `api`, `route`, `endpoint`, `handler`, `controller`
- Domain/data: `model`, `schema`, `entity`, `repository`, `migration`
- Configuration: `config`, `settings`, `.env`, `toml`, `yaml`, `json`
- Infra: `db`, `cache`, `queue`, `worker`, `scheduler`

These probes are **language‑agnostic** and do not assume project‑specific conventions.

### 3.5 Adaptive Probes (Stack‑Aware, Optional)

If stack detection identifies a language/framework, the agent MAY use **stack‑aware probes** to improve precision.

Examples:
- Python: `argparse`, `click`, `fastapi`, `flask`, `django`, `pydantic`
- JS/TS: `express`, `fastify`, `next`, `router`, `koa`
- Go: `func main`, `http.Handle`, `gin`, `chi`
- Java: `@RestController`, `@Service`, `Spring`
- .NET: `Program.cs`, `Startup`, `Controller`, `Minimal API`

These probes are optional and only used when stack confidence is high.

### 3.6 Function‑Level Evidence Requirement

Each Tier 1 file MUST yield at least **2 function‑level facts**, such as:
- “Function X orchestrates Y”
- “Class Z validates inputs before calling W”

If not achieved, the agent MUST continue reading or searching.

### 3.7 Evidence Density Thresholds Per Doc

Each generated doc MUST include minimum evidence counts:

| Doc | Minimum evidence points |
|-----|--------------------------|
| Project Intent & Business Frame | 3 |
| Domain Landscape | 3 |
| Canonical Data Model | 4 |
| Service & Capability Map | 4 |
| Architectural Guardrails | 3 |
| API / Integration Contracts | 2 (or stub w/ explicit “insufficient evidence”) |
| User Journey & UI Intent | 2 (or stub w/ explicit “insufficient evidence”) |

If thresholds are not met, the agent MUST rescan targeted modules before writing.

### 3.8 Two‑Pass Scan Strategy

The agent MUST implement a two‑pass scan:

**Pass 1**:
- Identify structure (modules, entrypoints, dependencies)
- Collect initial facts

**Pass 2**:
- Deep‑read the top K “most referenced” files (entry points + files imported by them)
- Extract behavioral facts

### 3.9 Cross‑File Corroboration

The agent SHOULD correlate facts across files:
- Example: CLI → analyze module → runtime module

This should produce richer architectural narratives and more precise doc statements.

### 3.10 Open Questions Gating

If open questions remain above a threshold:
- Top 3 open questions MUST trigger a targeted rescan of relevant modules.
- If still unresolved, the agent should explicitly note “not found in bounded reads.”

---

## 4. CLI and Config Changes

### 4.1 Configurable Depth Parameters

Add configurable knobs to `.brownie/brownie.toml`:

```toml
[analysis]
deep_read_min_lines = 200
deep_read_max_lines = 400
core_file_min_facts = 2
min_evidence_per_doc = { intent = 3, domain = 3, data_model = 4, service = 4, guardrails = 3 }
```

Defaults should match values specified in this spec.

---

## 5. Behavioral Rules

1. **Depth before breadth** for Tier 1 and Tier 2 files.
2. **Evidence density** is required before document writing.
3. **No project docs** (README, /docs, /specs) are used as evidence.
4. **Iterative scanning** is mandatory when evidence is shallow.

---

## 6. Acceptance Criteria

1. Generated docs contain materially more behavioral facts (not just “file exists”).
2. Tier 1 files produce at least 2 function‑level facts each.
3. Each doc meets evidence thresholds or explicitly states insufficiency.
4. Agent re‑scans when evidence density is poor.
5. Works for repos with **no pre‑existing docs**.

---

## 7. Testing Requirements

### 7.1 Unit Tests
- Evidence density calculation
- Core file classification
- Search probe list generation
- Pass‑2 targeting logic

### 7.2 Integration Tests
- Analyze a small codebase and assert doc evidence counts
- Confirm re‑scan occurs when evidence is insufficient

---

## 8. Out of Scope

- Full static analysis or AST parsing
- Full‑repo ingestion
- Dependence on external documentation sources

---

## 9. References

- SPEC-001 — Initial Implementation  
- SPEC-002 — Improved Visual Feedback  
- Observed `log.txt` run (verbose analysis, shallow reads)

