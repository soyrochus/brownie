# Brownie

[![Rust 1.85+](https://img.shields.io/badge/rust-1.85%2B-CE422B.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![GitHub Copilot SDK](https://img.shields.io/badge/powered%20by-GitHub%20Copilot%20SDK-8A2BE2)](https://github.com/github/copilot-sdk)
[![FOSS Pluralism](https://img.shields.io/badge/FOSS-Pluralism-green.svg)](FOSS_PLURALISM_MANIFESTO.md)

![Brownie](/images/Brownie-small.png)
> Brownie, Your helpfull AI assistant. Treat it well.

Brownie is a native desktop application that embeds GitHub Copilot agentic workflows in a structured execution shell. It is not a sidebar. It is a surface where your AI assistant works alongside you — sends and receives messages in real time, runs in passive mode by default, and persists your sessions locally.

This release (SPEC-2) builds on that execution backbone and adds a validated runtime-driven Canvas: typed `UiSchema` deserialization, strict validation before render, enum-based component rendering, and deterministic typed UI event logging.

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
| Right | **Canvas** | Runtime-rendered validated UI schema · append-only typed UI event log |

**Top bar:** centered connection status with semantic marker · Passive Mode indicator · disabled Active Mode toggle.

### Passive Mode

All tool calls are blocked at three independent layers: the CLI is started with `--deny-tool *`, the SDK permission handler returns `denied` by default, and the application has no code path to approve tool calls. Any tool-call events that arrive are logged to the diagnostics panel.

### Session Persistence

Sessions are stored as JSON files at `~/.brownie/sessions/<session-id>.json`. Writes are atomic (write to `.tmp`, then rename). Sessions reload on restart and appear in the left panel in reverse chronological order.

### Canvas Runtime (SPEC-2)

- Embedded fixture schema loaded from `src/ui/fixture.json`
- Schema deserialized into strongly typed Rust models
- Validation gate before rendering:
  - allowlisted component kinds only
  - supported form field kinds only (`text`, `number`, `select`, `checkbox`)
  - max component count and nesting depth
  - button output-contract mapping required
- Rendering uses typed enum dispatch (no string-fallback renderer path)
- Interactions emit typed `UiEvent` values shown in an append-only event log

## Project Structure

```text
src/
  main.rs          — entry point; instruction file detection; eframe wiring
  app.rs           — egui App shell; chat + runtime canvas integration
  event.rs         — AppEvent enum bridging async SDK events to the UI thread
  copilot/mod.rs   — CopilotClient; SDK lifecycle; streaming event mapping
  theme.rs         — centralized visual tokens (surfaces, accents, spacing, radii)
  session/
    mod.rs         — SessionMeta and Message types
    store.rs       — atomic filesystem persistence (~/.brownie/sessions/)
  ui/
    schema.rs      — typed UiSchema + validation rules + validation tests
    registry.rs    — typed component allowlist + enum-based render dispatch
    runtime.rs     — runtime loader/validator/renderer orchestration + event-order test
    event.rs       — typed UiEvent models and event log helpers
    fixture.json   — deterministic embedded schema fixture for SPEC-2
vendor/
  copilot-sdk-rust/  — community Rust SDK (git submodule)
```

## SPEC-2 Scope

What works:

- Connect to Copilot CLI via the Rust SDK (stdio transport, auto-restart on crash)
- Create a session bound to the current workspace
- Send messages and receive streaming responses in the transcript
- Passive mode enforced unconditionally
- Connection status visible in the top bar; errors and suppressed tool calls in the diagnostics panel
- Session transcript persisted locally and reloadable from the session list
- Runtime-driven right panel Canvas rendered from validated typed schema
- Typed UI event emission and append-only debug/event log
- Centralized tokenized visual styling across shell panels and controls

What is explicitly **not** in this release:

- Active mode and tool approval
- Copilot-driven UI intent resolution, catalog/template lookup, or schema generation
- Workspace selector (uses CWD; manual override planned for a later spec)

## Configuration

No config file yet — planned for `~/.brownie/config.toml` in a later spec.

`copilot` must be discoverable on PATH for SDK startup.

## Principles of Participation

Everyone is invited and welcome to contribute: open issues, propose pull requests, share ideas, or help improve documentation. Participation is open to all, regardless of background or viewpoint. 

This project follows the [FOSS Pluralism Manifesto](./FOSS_PLURALISM_MANIFESTO.md), which affirms respect for people, freedom to critique ideas, and space for diverse perspectives. 

## License and Copyright

Copyright (c) 2026, Iwan van der Kleijn

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
