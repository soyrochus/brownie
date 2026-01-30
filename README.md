# Brownie

[![Python 3.13+](https://img.shields.io/badge/python-3.13+-blue.svg)](https://www.python.org/downloads/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![GitHub Copilot SDK](https://img.shields.io/badge/powered%20by-GitHub%20Copilot%20SDK-8A2BE2)](https://github.com/github/copilot-sdk)
[![FOSS Pluralism](https://img.shields.io/badge/FOSS-Pluralism-green.svg)](FOSS_PLURALISM_MANIFESTO.md)

> Codebase to docs while you sleep

Brownie is an autonomous reverse engineering agent that analyzes codebases and produces comprehensive, standardized documentation. It uses a structured workflow powered by the GitHub Copilot SDK to systematically extract architecture, data models, and API surfaces from your code.

![Poch, the brownie who reads before he speaks.](/images/Poch-small.png)
> Poch, the brownie who reads before he speaks.

## What Brownie Does

- Runs an agentic CLI workflow over your repo using the GitHub Copilot SDK tools.
- Generates each document directly from bounded source reads (not from cached summaries).
- Produces a fixed, canonical documentation set in a docs directory.
- Writes explicit stubs for API/UI docs when those areas are not detected.

## Quickstart

```bash
brownie init
# edit .brownie/brownie.toml as needed
brownie analyze
# optional: merge + refine into a single doc
brownie analyze -r
```

Notes:
- `brownie init` creates `.brownie/`, `.brownie/cache/`, `.brownie/brownie.toml`, copies prompt templates to `.brownie/prompts/`, and adds `.brownie/` to `.gitignore`.
- `brownie analyze` replaces the docs directory (deletes and recreates it) before writing output.
- Use `--root` to point at another project root.

## Documentation Output (Fixed Set)

Brownie always generates these files (stubbing when not applicable):

1. `project-intent-business-frame.md`
2. `domain-landscape.md`
3. `canonical-data-model.md`
4. `service-capability-map.md`
5. `architectural-guardrails.md`
6. `api-integration-contracts.md`
7. `user-journey-ui-intent.md`

After the seven files are written, Brownie merges them into a single file in the same docs directory:
- `{derived-system-name}-documentation.md`

When `-r` / `--refining` is used, Brownie runs a final refinement pass and writes:
- `{derived-system-name}-documentation-FINAL.md`

`derived-system-name` is taken from `pyproject.toml` (`project.name`) when available, otherwise it falls back to the repo folder name.

## Configuration

Configuration lives in `.brownie/brownie.toml` and can be overridden via CLI flags.

```toml
model = "gpt-5"

[analysis]
include_dirs = ["src"]
exclude_dirs = ["node_modules", "dist", "build", ".git", ".brownie", "docs"]
docs_dir = "docs"
max_file_lines = 400
chunk_lines = 200
max_grep_hits = 200
```

Subscription example:

```toml
[provider]
mode = "subscription"
model = "gpt-5"
```

API-key example:

```toml
[provider]
mode = "api-key"
type = "openai"       # openai | azure | anthropic
api_key = "sk-..."
base_url = "https://api.openai.com/v1"
model = "gpt-4o"
# azure_api_version = "2024-10-21" # only for Azure
```

CLI overrides:
- `--include_dirs`, `--exclude_dirs`, `--docs_dir`
- `--write-config` to write effective config back to `.brownie/brownie.toml`
- `--reset-cache` to clear `.brownie/cache/` before analysis
- `-v` / `--verbose` for detailed output (see Output Modes below)
- `-r` / `--refining` to create a merged and refined single-document output

## Authentication and Provider Defaults

- **Subscription mode (default):** uses Copilot subscription auth. Model defaults to `gpt-5` unless `model` or `provider.model` overrides it.
- **API-key mode:** requires `type` and `api_key`; Azure also requires `base_url`.
- Default base URLs: OpenAI `https://api.openai.com/v1`, Anthropic `https://api.anthropic.com`.
- Default models: OpenAI `gpt-4o`, Anthropic `claude-sonnet-4-20250514`.

## Output Modes

**Default mode** displays phase progress messages showing scanning, processing, and documentation generation phases.

**Verbose mode** (`-v` / `--verbose`) adds streaming agent reasoning (prefixed with `[Agent]`) and tool invocations (`→` calls, `←` results) for debugging and transparency.

```bash
brownie analyze -v
```

## Prompt Templates and Stack Detection

`brownie init` copies prompt templates to `.brownie/prompts/`. Before analysis, Brownie selects a template based on lightweight stack detection and appends it to the system instructions:

Templates shipped: `generic`, `python`, `nodejs`, `react`, `go`, `dotnet`, `java`.

## Cache and Evidence Trail

During analysis, Brownie may store:
- `.brownie/cache/facts.jsonl` for evidence pointers (path + line ranges)
- `.brownie/cache/open-questions.md` for gaps and uncertainties
- `.brownie/cache/run-state.json` for run progress

Open questions are derived when evidence tags like `intent`, `data-model`, `service`, `api`, or `ui` are missing.

## Guardrails and Limitations

- Bounded file reads and search only; no ASTs, parsers, or full-repo ingestion.
- Only included directories are explored; excluded directories are never read.
- Documentation is written one file at a time, and output is regenerated each run.


## Principles of Participation

Everyone is invited and welcome to contribute: open issues, propose pull requests, share ideas, or help improve documentation.  
Participation is open to all, regardless of background or viewpoint.  

This project follows the [FOSS Pluralism Manifesto](./FOSS_PLURALISM_MANIFESTO.md),  
which affirms respect for people, freedom to critique ideas, and space for diverse perspectives.  


## License and Copyright

Copyright (c) 2026, Iwan van der Kleijn

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
