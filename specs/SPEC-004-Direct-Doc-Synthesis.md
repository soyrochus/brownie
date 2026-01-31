# SPEC-004 - Direct Doc Synthesis for High-Fidelity Output

Status: Implemented  
Owner: Iwan van der Kleijn  
Last updated: 2026-01-31  
Version: 1.1

---

## 1. Purpose

This spec defines Brownie's **direct doc synthesis** approach: documentation is produced directly from bounded source reads during the analysis run, without relying on intermediate fact caches. The goal is to achieve output quality comparable to `REFERENCE-BROWNIE-DOCSET.md` while preserving concrete evidence.

---

## 2. Motivation

Cache-driven workflows degraded detail and introduced abstraction:
- Facts were too generic to support detailed tables and precise documentation.
- Doc writing became detached from source context, leading to vague summaries.
- Output diverged from the manual reference docset.

Direct doc synthesis preserves context and yields cohesive, high-detail documentation.

---

## 3. Requirements

### 3.1 Unified Source Enumeration and Reads

The host runtime MUST:

1. Enumerate source files using `include_dirs`/`exclude_dirs` plus stack-based filtering.
2. Pass the full file list to the agent.
3. Require bounded reads (200-400 lines per file chunk).

The agent MUST read every listed source file before writing documentation. Doc writing MUST NOT rely on `facts.jsonl` or other cached summaries.

### 3.2 Document-Specific Focus and Depth

Each document is governed by the per-doc configuration in `DOC_PROMPTS` (source focus, required sections, depth requirements). At minimum, every document MUST:

- Follow the required section structure for that doc.
- Meet the minimum tables and evidence-backed bullet counts.
- Include required elements (tables with Evidence columns, diagrams, etc.).

### 3.3 Evidence Requirements

- Every major claim MUST cite file:line evidence.
- Tables MUST include an Evidence column with direct citations.
- If evidence is missing, the doc must explicitly state: "Not found in bounded reads."

### 3.4 Stub Behavior

- For API/UI docs, the agent MUST write explicit stubs when not applicable (per `DOC_PROMPTS`).
- If any required doc file is missing after the agent run, the host MUST create a stub with:
  "Not applicable or insufficient evidence found during bounded analysis."

### 3.5 Cache Policy

- No fact or open-question caches are used in the current pipeline.
- The only cache artifact is `.brownie/cache/run-state.json` for run bookkeeping.

---

## 4. Workflow (Current Implementation)

### 4.1 Pipeline

1. Detect stack and load the matching prompt template.
2. Enumerate all source files using include/exclude rules and stack filtering.
3. Start a single unified analysis run where the agent reads all files and writes the full docset.
4. Ensure all required docs exist (stub if missing).
5. Merge the seven docs into `{derived-system-name}-documentation.md`.

### 4.2 Prompting Contract

The system and unified prompts enforce:

- Bounded reads (200-400 lines per file chunk).
- Evidence format (`file:line` citations).
- Minimum tables/bullets per doc.
- Explicit section structures per document.

---

## 5. Quality Targets

Output should approximate the level of detail in `REFERENCE-BROWNIE-DOCSET.md`:

- Structured tables for classes/config/CLI
- Explicit enumeration of functions and responsibilities
- Concrete defaults with evidence
- Cohesive narrative flow across modules

---

## 6. Acceptance Criteria

1. Docs contain evidence-anchored tables and concrete enumerations.
2. Summaries are grounded in direct source reads, not cached summaries.
3. Required docs are always present (agent or stub fallback).
4. A merged `{derived-system-name}-documentation.md` file is produced.

---

## 7. Testing Requirements

- No dedicated integration test is implemented yet.
- Current coverage is limited to helper/unit tests; manual verification is required.

---

## 8. Out of Scope

- Full AST parsing
- Full-repo ingestion
- Schema inference beyond observed evidence

---

## 9. References

- SPEC-001 - Initial Implementation  
- SPEC-002 - Improved Visual Feedback  
- SPEC-003 - Improve Information Detail and Evidence Depth  
- `REFERENCE-BROWNIE-DOCSET.md`
