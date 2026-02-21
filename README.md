# Brownie

[![Rust 1.85+](https://img.shields.io/badge/rust-1.85%2B-CE422B.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![GitHub Copilot SDK](https://img.shields.io/badge/powered%20by-GitHub%20Copilot%20SDK-8A2BE2)](https://github.com/github/copilot-sdk)
[![FOSS Pluralism](https://img.shields.io/badge/FOSS-Pluralism-green.svg)](FOSS_PLURALISM_MANIFESTO.md)

![Brownie](/images/Brownie-small.png)
> Brownie, Your helpfull AI assistant. Treat it well.

Brownie is a native desktop application that embeds GitHub Copilot agentic workflows in a structured execution shell. It is not a sidebar. It is a surface where your AI assistant works alongside you — sends and receives messages in real time, runs in passive mode by default, and persists your sessions locally.

The latest release adds a tool-driven catalog Canvas flow: a single `query_ui_catalog` interface for UI discovery/render decisions, deterministic `UiIntent`-to-template resolution, embedded builtin templates, writable user templates, strict schema validation before rendering, and typed UI event logging.

## Prerequisites

- **Rust 1.85+** (edition 2021) — `rustup update stable`
- **GitHub Copilot CLI** installed and authenticated on your PATH
  - Install: follow the [Copilot CLI installation guide](https://docs.github.com/en/copilot/how-tos/set-up/install-copilot-cli)
  - Verify: `copilot --version`
- A valid GitHub Copilot subscription (free tier works)

## Build

```bash
git clone --recurse-submodules https://github.com/your-org/brownie
cd brownie
cargo build --release
```

The binary lands at `target/release/brownie`. No installer, no runtime dependencies beyond the Copilot CLI.

## Run

```bash
# Run from your project directory — Brownie uses the CWD as the workspace
cd /your/project
brownie
```

## What You Get

![Brownie UI draft](images/brownie-ui.png)
> Note that this is still a 'work-in-progress'. The real beauty of your brownie will be coming apparent very soon.

The window is a three-column layout with a dark theme:

| Column | Label | Contents |
| --- | --- | --- |
| Left | **Workspace** | Active workspace path · detected instruction files (`.github/copilot-instructions.md`, `AGENTS.md`, `*.instructions.md`) · recent session list |
| Center | **Chat** | Streaming conversation transcript · collapsible diagnostics log · input bar |
| Right | **Canvas** | Intent-gated validated template rendering · selection context · provisional template save prompt · append-only typed UI event log |

**Top bar:** centered connection status with semantic marker · Passive Mode indicator · disabled Active Mode toggle.

### Passive Mode

Execution tools (shell/write/powershell) are blocked for the model. The session exposes one host-controlled tool, `query_ui_catalog`, for Canvas decisions only. Permission prompts are disabled (`request_permission=false`), and non-allowed tool requests are logged to diagnostics.

### Session Persistence

Sessions are stored as JSON files at `~/.brownie/sessions/<session-id>.json`. Writes are atomic (write to `.tmp`, then rename). Sessions reload on restart and appear in the left panel in reverse chronological order.

### UI Catalog and Canvas Runtime

- Builtin templates are embedded in the binary (`src/ui/catalog_builtin/*.json`) and loaded through a read-only provider
- User templates are loaded from a writable local catalog directory at `<workspace>/.brownie/catalog/*.json`
- Canvas components are not rendered by default; rendering is intent-gated
- A single tool interface (`query_ui_catalog`) is used by the assistant to query catalog/UI capabilities
- Template resolution is deterministic:
  - exact match on `UiIntent.primary`
  - secondary ranking via `operations` and `tags`
  - stable precedence order (`user` over `builtin`; `org` slot reserved when enabled)
- Template documents must validate (`meta`, `match`, `schema`) before they become selectable
- Selected template schema is deserialized into typed Rust models and validated before render
- Canvas render path uses typed enum dispatch (no string-fallback renderer path)
- No silent fallback: if no template matches, the UI explicitly shows `No matching UI template found`
- On no-match, a provisional template may be rendered; the user can save it to catalog from the Canvas prompt
- Resolution and selection are logged in diagnostics (selected template/source/score or no-match reason)
- Interactions emit typed `UiEvent` values shown in an append-only event log

## Project Structure

```text
src/
  main.rs          — entry point; instruction file detection; eframe wiring
  app.rs           — egui App shell; chat + runtime canvas integration
  event.rs         — AppEvent enum bridging async SDK events + tool-driven canvas renders to the UI thread
  copilot/mod.rs   — CopilotClient; SDK lifecycle; `query_ui_catalog` tool registration + handler
  theme.rs         — centralized visual tokens (surfaces, accents, spacing, radii)
  session/
    mod.rs         — SessionMeta and Message types
    store.rs       — atomic filesystem persistence (~/.brownie/sessions/)
  ui/
    catalog.rs     — catalog providers, deterministic template resolver, resolution traces, and user-template upsert
    catalog_builtin/
      *.json       — embedded builtin template documents
    schema.rs      — typed UiSchema + validation rules + validation tests
    registry.rs    — typed component allowlist + enum-based render dispatch
    runtime.rs     — runtime loader/validator/renderer orchestration + event-order test
    event.rs       — typed UiEvent models and event log helpers
    fixture.json   — development/test schema fixture
vendor/
  copilot-sdk-rust/  — community Rust SDK (git submodule)
```

## Current Release Scope

What works:

- Connect to Copilot CLI via the Rust SDK (stdio transport, auto-restart on crash)
- Create a session bound to the current workspace
- Send messages and receive streaming responses in the transcript
- Passive mode enforced unconditionally
- Connection status visible in the top bar; errors and suppressed tool calls in the diagnostics panel
- Session transcript persisted locally and reloadable from the session list
- Catalog-driven right panel Canvas rendered from validated typed template schema
- Deterministic intent-to-template resolution with transparent diagnostics
- Single assistant tool interface (`query_ui_catalog`) for UI catalog lookup/render decisions
- Embedded builtin catalog templates plus writable user catalog templates
- Builtin file-listing template rendered in Canvas from workspace state
- Provisional template render path with explicit "Save to Catalog" user confirmation
- Explicit no-match handling (`No matching UI template found`)
- Typed UI event emission and append-only debug/event log
- Centralized tokenized visual styling across shell panels and controls

What is explicitly **not** in this release:

- Active mode and tool approval
- Broad arbitrary tool execution from the model (only `query_ui_catalog` is exposed)
- Workspace selector (uses CWD; manual override planned for a later spec)
- Full org catalog provider implementation (provider slot is reserved)

## Configuration

No config file yet — planned for `~/.brownie/config.toml` in a later release.

Catalog paths:
- Builtin catalog: embedded assets under `src/ui/catalog_builtin/`
- User catalog: `<workspace>/.brownie/catalog/`

`copilot` must be discoverable on PATH for SDK startup.

## Principles of Participation

Everyone is invited and welcome to contribute: open issues, propose pull requests, share ideas, or help improve documentation. Participation is open to all, regardless of background or viewpoint. 

This project follows the [FOSS Pluralism Manifesto](./FOSS_PLURALISM_MANIFESTO.md), which affirms respect for people, freedom to critique ideas, and space for diverse perspectives. 

## License and Copyright

Copyright (c) 2026, Iwan van der Kleijn

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
