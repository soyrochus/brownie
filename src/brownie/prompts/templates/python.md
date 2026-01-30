# Python Project Instructions

You must produce a high-detail technical docset from source code only. Avoid abstract summaries. Prefer concrete inventories, tables, and evidence-anchored facts.

Focus on:
- Entry points: `__main__.py`, `main.py`, CLI definitions, `setup.cfg`/`pyproject.toml` scripts.
- Frameworks: FastAPI, Flask, Django, Typer, Click.
- Structure: packages in `src/`, `app/`, or top-level modules.

Mandatory workflow (do not skip):
1) Inventory extraction (before any docs):
   - CLI commands, flags, defaults, exit codes (tag: api or intent).
   - Public classes/dataclasses/enums with fields and defaults (tag: data-model).
   - Public functions/methods with signatures and responsibilities (tag: function).
   - Configuration keys and defaults (tag: guardrail or intent).
   - Tool interfaces (inputs/outputs/constraints) (tag: api).
   - Error handling paths and warnings (tag: guardrail).
   - External dependencies and usage sites (tag: api or service).
2) Use evidence for every inventory item (file:line-range).
3) Do NOT write docs until inventories are collected.

Probe usage rules:
- Use probe terms only to locate files for deep reads.
- Do NOT create open questions from probe hits or misses.
- Only create open questions after deep reads fail to find evidence.

Output format (strict, required):
1) Use tables for inventories:
   - Classes/Models (Name, Fields, Defaults, Evidence)
   - Config keys (Key, Type, Default, Evidence)
   - CLI commands (Command, Flags, Defaults, Evidence)
   - Tools/Interfaces (Name, Parameters, Result, Evidence)
2) Use bullet lists for behavioral facts (each bullet must include file:line-range).
3) Include at least one code snippet per document for critical structures.
4) Every section must contain at least 3 evidence-anchored bullets.
5) Avoid vague language ("appears to", "seems"). State facts with evidence or mark unknowns.

Document structure requirements:
- Project Intent & Business Frame: Purpose + Goals table + Value Proposition bullets + Target Users list.
- Domain Landscape: Concept table (Concept, Description, Location) + boundary diagram (ASCII ok).
- Canonical Data Model: Enumerate all dataclasses/enums and fields with defaults/types.
- Service & Capability Map: Module table (Module, Responsibility, Key Functions).
- Architectural Guardrails: Constraint table with enforcement evidence.
- API & Integration Contracts: External packages + usage sites + function signatures.
- User Journey & UI Intent: CLI user flows as step sequences with evidence.

Doc depth enforcement:
- Each section must cite file/line evidence.
- Prefer concrete symbol names (functions, classes, enums).
- Map at least 3 key flows (startup, request handling, data persistence) when applicable.
- If evidence is missing after deep reads, explicitly mark it as "not found in bounded reads."

## Probe Terms
- argparse
- click
- typer
- fastapi
- flask
- django
- pydantic
