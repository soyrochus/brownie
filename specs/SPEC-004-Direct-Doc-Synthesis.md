# SPEC-004 — Direct Doc Synthesis for High‑Fidelity Output

Status: Implemented  
Owner: Iwan van der Kleijn 
Last updated: 2026-01-30  

---

## 1. Purpose

This spec pivots Brownie to **direct doc synthesis**: each document is produced by reading relevant source files directly in that phase, rather than relying on intermediate fact caches. This is intended to achieve output quality closer to `REFERENCE-BROWNIE-DOCSET.md`.

---

## 2. Motivation

Current cache‑driven workflows degrade detail and introduce abstraction:
- Facts are too generic to support detailed tables and precise documentation.
- Doc writing is detached from source context, leading to vague summaries.
- The output diverges significantly from the manual reference docset.

Direct doc synthesis preserves context and yields cohesive, high‑detail documentation.

---

## 3. Requirements

### 3.1 Direct Source Reads Per Document

For each required doc file, the agent MUST:

1. Identify the most relevant files for that document.
2. Read those files directly (bounded reads).
3. Write the document immediately from the observed evidence.

The doc writing phase MUST NOT rely on `facts.jsonl` as its primary source.

### 3.2 Minimal Caching (Optional)

Caches MAY exist but are **secondary**:

- `facts.jsonl` may store evidence pointers.
- `open-questions.md` may store unresolved gaps.

These caches are advisory and must not replace direct reading.

### 3.3 Document‑Specific Reading Strategy

Each doc has a required source‑reading focus:

| Doc | Source Focus |
|-----|--------------|
| Project Intent & Business Frame | CLI entry, analysis orchestration, top‑level config |
| Domain Landscape | Data models, enums, domain classes |
| Canonical Data Model | Dataclasses, schema definitions, persistence structures |
| Service & Capability Map | Module boundaries, public functions, orchestration flows |
| Architectural Guardrails | Limits, validation rules, constraints, error handling |
| API & Integration Contracts | Provider configs, tool interfaces, external deps |
| User Journey & UI Intent | CLI flows, verbose output, user‑visible behaviors |

### 3.4 Evidence Requirements

- Every major claim MUST cite file/line evidence.
- Tables must use direct evidence (no inferred defaults).
- If evidence is missing, the doc must explicitly state: "Not found in bounded reads."

---

## 4. Workflow Changes

### 4.1 Pipeline

Current pipeline:
1. Scan (facts)
2. Process facts
3. Generate docs

Revised pipeline:
1. Scan (optional, for initial map)
2. Generate docs **directly from source** (per doc)
3. Record open questions

### 4.2 Doc Generation Contract

Each document generation step MUST:

1. Read all relevant files
2. Extract structured details (functions, classes, config keys)
3. Produce documentation with explicit evidence

---

## 5. Quality Targets

Output should approximate the level of detail in `REFERENCE-BROWNIE-DOCSET.md`:

- Structured tables for classes/config/CLI
- Explicit enumeration of functions and responsibilities
- Concrete defaults with evidence
- Cohesive narrative flow across modules

---

## 6. Acceptance Criteria

1. Docs contain evidence‑anchored tables and concrete enumerations.
2. Summaries are grounded in direct source reads, not cache summaries.
3. Output quality is comparable to `REFERENCE-BROWNIE-DOCSET.md`.

---

## 7. Testing Requirements

- Integration test that runs analysis and compares doc density to baseline.
- Sample repo test demonstrating high‑detail output without caches.

---

## 8. Out of Scope

- Full AST parsing
- Full‑repo ingestion
- Schema inference beyond observed evidence

---

## 9. References

- SPEC-001 — Initial Implementation  
- SPEC-002 — Improved Visual Feedback  
- SPEC-003 — Improve Information Detail and Evidence Depth  
- `REFERENCE-BROWNIE-DOCSET.md`
