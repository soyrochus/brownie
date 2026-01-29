# SPEC-001 — Brownie Agentic Documentation Generator (Initial Implementation)

Status: Draft  
Owner: <TBD>  
Last updated: 2026-01-29  

---

## 1. Purpose

Brownie is an **agentic CLI application** that generates a structured documentation set for a software project by running an AI agent powered by the **GitHub Copilot SDK**.

The agent explores the project workspace using Copilot SDK tools (directory listing, file reading, search) and writes documentation files directly into a `/docs` directory.

Brownie does **not** statically analyze, parse, or model code. It behaves as a disciplined technical writer operating over the repository through bounded observation and iterative writing.

---

## 2. High-Level Vision

Brownie:

- Runs as a CLI (`init`, `analyze`)
- Creates a Copilot SDK session at runtime
- Lets the model:
  - inspect the workspace using tools
  - form evidence-backed claims
  - write documentation files incrementally
- Persists minimal state on disk to avoid context growth
- Produces a fixed, canonical set of documentation files

Brownie is **not** a compiler, indexer, or semantic analyzer.

---

## 3. Non-Goals (Explicit)

Brownie does **not**:

- Build ASTs or semantic models
- Use parsers (tree-sitter, Icode, etc.)
- Ingest the entire repository into a single context
- Guarantee correctness beyond observed evidence
- Perform runtime provider switching or multi-provider fallbacks
- Maintain long-lived conversational memory

If a behavior cannot be implemented via:
- directory listing
- bounded file reads
- text search (grep-like)

…it is out of scope for this version.

---

## 4. Runtime Architecture Overview

### 4.1 Agentic Runtime Model

At runtime, Brownie:

1. Creates a Copilot SDK session
2. Provides the agent with:
   - workspace access tools
   - system instructions
   - bounded execution rules
3. Lets the agent:
   - explore included directories
   - collect evidence
   - write documentation files
4. Terminates the session

The Copilot SDK is a **first-class runtime dependency**.

---

## 5. Documentation Output (Fixed Set)

Brownie generates a `/docs` directory containing exactly the following files:

1. `project-intent-business-frame.md`
2. `domain-landscape.md`
3. `canonical-data-model.md`
4. `service-capability-map.md`
5. `architectural-guardrails.md`
6. `api-integration-contracts.md` (if applicable; otherwise stub)
7. `user-journey-ui-intent.md` (if applicable; otherwise stub)

Each document is written independently to avoid context accumulation.

---

## 6. Commands

### 6.1 `brownie init`

Initializes Brownie project state.

Creates:

```

.brownie/
brownie.toml
cache/

```

Responsibilities:

- Create `.brownie/brownie.toml` with commented example configuration
- Create `.brownie/cache/`
- Do **not** generate `/docs`
- Create `.brownie/prompts/` and copy default prompt templates for customization

### 6.2 `brownie analyze`

Runs the agentic documentation process.

Responsibilities:

- Load configuration
- Create Copilot SDK session
- Run staged agent workflow
- Create or replace `/docs`
- Write documentation files

---

## 7. Configuration

### 7.1 Configuration File

Authoritative configuration file:

```

.brownie/brownie.toml

````

This file contains **both**:
- analysis configuration
- provider / authentication configuration

CLI flags may override config values; optional write-back is supported.

---

## 8. Authentication (Copilot SDK Session)

### 8.1 Overview

Brownie supports:

- GitHub Copilot **subscription authentication** (default)
- **Bring-Your-Own-Key (BYOK)** authentication

Authentication applies to **runtime Copilot SDK session creation**.

---

### 8.2 Provider Configuration Schema

```toml
[provider]
mode = "subscription" # or "api-key"

# Required when mode = "api-key"
type = "openai"       # openai | azure | anthropic
api_key = "sk-..."

# Optional
base_url = "https://api.openai.com/v1"
model = "gpt-4o"

# Azure only
azure_api_version = "2024-12-01-preview"
````

---

### 8.3 Defaults

#### Base URLs

| Provider  | Default Base URL                                       |
| --------- | ------------------------------------------------------ |
| openai    | [https://api.openai.com/v1](https://api.openai.com/v1) |
| anthropic | [https://api.anthropic.com](https://api.anthropic.com) |
| azure     | none (required)                                        |

#### Models

| Provider     | Default Model                      |
| ------------ | ---------------------------------- |
| openai       | gpt-4o                             |
| anthropic    | claude-sonnet-4-20250514           |
| azure        | none                               |
| subscription | from config / CLI (default: gpt-5) |

Model precedence:

```
brownie.toml > provider default > CLI / config
```

---

### 8.4 Validation Rules

Errors MUST be raised before session creation if:

* `mode = api-key` without `type`
* `mode = api-key` without `api_key`
* `type = azure` without `base_url`
* Invalid `mode` or `type`

---

### 8.5 Security

* `.brownie/` SHOULD be gitignored
* Warn if API keys are present and directory is tracked
* Environment variable interpolation is explicitly deferred

---

## 9. Analysis Scope Control

### 9.1 Include / Exclude Directories

Configuration keys:

```toml
include_dirs = ["src"]
exclude_dirs = ["node_modules", "dist", "build", ".git", ".brownie", "docs"]
```

Rules:

* Only included directories are explored
* Excluded directories are never read
* Directories only (no globs in v1)

---

## 9.2 Prompt Templates and Tech Stack Detection

Brownie supports stack-specific prompt templates to improve depth and relevance.

### Prompt Directory

`.brownie/prompts/` contains editable Markdown prompts. `brownie init` must populate this directory with defaults.

### Detection Phase

Before analysis, Brownie performs a lightweight tech stack detection based on:
- File extensions in included directories
- Common build/config markers (e.g., `pyproject.toml`, `package.json`, `go.mod`, `.csproj`, `pom.xml`)

The detected stack selects the prompt file:

```
.brownie/prompts/<stack>.md
```

If no match is found, `generic.md` is used.

### Required Default Prompts

At minimum, the following templates are shipped and copied to `.brownie/prompts/`:

- `generic.md`
- `python.md`
- `nodejs.md`
- `react.md`
- `go.md`
- `dotnet.md`
- `java.md`

### Prompt Usage

The selected prompt is appended to the system instructions for the agent and can define:
- file discovery priorities
- evidence collection requirements
- doc depth expectations
- framework-specific conventions

---

## 10. Agent Behavior Contract

The agent:

* Uses Copilot SDK tools to explore the workspace
* Reads files in bounded slices
* Searches text where useful
* Writes documentation files directly

The agent MUST:

* Base claims on observed evidence
* Mark uncertainty explicitly
* Avoid speculative completion
* Write one document at a time

The agent MUST NOT:

* Assume completeness
* Invent structures not observed
* Attempt full-repo ingestion

---

## 11. Persistent Scratch State

`.brownie/cache/` contains **temporary run state only**:

* `facts.md` or `facts.jsonl`
  Atomic, evidence-linked claims
* `open-questions.md`
  Explicit unknowns
* `run-state.json`
  Progress markers (optional resume)

This state exists to replace chat memory, not to build models.

---

## 12. Implementation Notes (Normative)

* Brownie runtime **must** use `github-copilot-sdk` as a library
* Copilot SDK session is created inside Brownie
* Agent uses SDK tool interface for workspace interaction
* Agent workflow is staged:
  1. Inspect and collect facts/open questions (no docs)
  2. Write each required document one at a time, using exact filenames
  3. If a required file is missing, the agent must retry writing it

### Important Clarification

The following are **implementation references only**:

* `Copilot-SDK-Tutorial.md`
* `vendor/copilot-sdk/` source tree

They are used by:

* human developers
* code-generation agents

They MUST NOT become runtime dependencies or be read by Brownie at execution time.

---

## 13. Acceptance Criteria

1. Brownie runs without parsers
2. Copilot SDK session is created at runtime
3. Subscription auth works unchanged by default
4. BYOK works for OpenAI, Anthropic, Azure
5. Docs directory is generated deterministically
6. Missing applicability produces explicit stubs
7. Configuration errors fail fast
8. No full-repo ingestion occurs

---

## 14. Mental Model (Authoritative)

Brownie is:

> a disciplined technical writer with a notebook, limited eyesight, and access to a filing cabinet — not a compiler.

Any implementation violating this model is incorrect.
