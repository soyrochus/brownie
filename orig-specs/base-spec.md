SPEC: Agentic UI Shell (Rust) – Copilot-backed, catalog-driven dynamic UI

0. Intent and scope

Build a distributable desktop client that embeds GitHub Copilot agentic workflows using the Rust community SDK, but where the primary UX is not “chat + fixed panels”. The core surface is a dynamic execution canvas rendered from a constrained UI DSL. Chat exists as a supporting channel, not the organizing principle.

Non-goals: re-implement the Copilot runtime protocol; bypass Copilot auth/licensing; arbitrary web rendering (no HTML/React injection).

1. Dependencies and prerequisites

1.1 Runtime prerequisites

* GitHub Copilot CLI installed and authenticated.
* `copilot` available on PATH, or path provided via environment variable supported by the SDK (e.g., `COPILOT_CLI_PATH`). ([GitHub][1])

1.2 SDK dependency

* Use `copilot-sdk-rust` as the sole integration layer for:

  * launching/connecting to Copilot CLI agent runtime
  * maintaining multi-turn sessions
  * receiving streaming events and tool invocations (as exposed by the SDK)
* Transport support: stdio and TCP are supported by the SDK; the app must allow configuration of transport mode. ([GitHub][1])

1.3 Language/toolchain

* Rust (edition aligned with SDK requirements; the repo indicates Rust 1.85+ / Edition 2024). ([GitHub][1])

2. System architecture

2.1 High-level components
A) Copilot Integration Layer (via SDK)

* `CopilotClient`: wrapper around `copilot-sdk-rust` to provide:

  * session lifecycle (create, resume, close)
  * message sending with streaming responses
  * tool invocation reception (if surfaced by SDK)
  * error mapping + reconnect strategy

B) Dynamic UI Runtime

* `UiRuntime`: validates and renders a `UiSchema` (UI DSL) into concrete UI components.
* `ComponentRegistry`: fixed allowlist of components and their rendering code.
* `EventBus`: collects UI events and forwards them to the Copilot session as structured messages.

C) UI Catalog

* `CatalogManager`: resolves “ui intent” → template selection, using precedence rules.
* `TemplateStore`: builtin (read-only) + local user catalog (read/write) + optional org catalog (read-only or controlled).
* `PromotionFlow`: after ephemeral UI use, offer to persist as a template.

D) Persistence

* store sessions (metadata), UI templates, and audit logs locally (SQLite or filesystem; see section 7).

2.2 Execution loop (agentic UI, catalog-driven)

1. Agent emits a `UiIntent` (structured).
2. Client resolves intent using UI Catalog:

   * if match: load template → render
   * if no match: request candidate UI schema from agent, with strict constraints → validate → render ephemeral
3. User interacts with rendered UI → client emits typed `UiEvent`s
4. Events forwarded to agent; agent updates plan and may emit next `UiIntent` or complete task
5. If UI was agent-generated and successfully used: ask user whether to promote into local catalog.

3) UI design (described to match your image)

The UI is a landscape, wide desktop window with a three-column structure and a minimal top bar.

Top bar: connection status in the center (e.g., “Copilot Connected”), and mode controls on the right (Passive/Active execution toggle), plus Settings.

Left column: Workspace + sessions/navigation.

* A workspace selector showing repo name and branch.
* A small section listing detected instruction sources (paths like `.github/copilot-instructions.md`, `AGENTS.md`, and any `.instructions.md` files).
* A “Recent Sessions” list with clickable session titles.

Center column: the primary interaction area.

* A conversational transcript area (this remains, but it is not the main “action UI”; it’s a narrative/log channel).
* Bottom input bar for sending messages and quick context buttons (e.g., attach files, include diff).

Right column: action-oriented surface.

* A panel area showing pending actions.
* A section showing a diff preview (in the static mock).
* Buttons at the bottom like “Apply Changes” and “Discard”.

In the dynamic-UI implementation, the right column is not a permanent diff viewer. It becomes a “Canvas” that can render whatever UI schema is selected from the catalog (forms, tables, approval dialogs, diffs, wizards). The diff view is one component among many, invoked only when the selected template uses it.

4. Dynamic UI DSL (UiSchema)

4.1 Requirements

* Declarative, typed, JSON-serializable schema.
* Strict allowlist of component types.
* Explicit event contract: every actionable component emits a typed event payload.
* Hard limits:

  * max components
  * max total schema bytes
  * max nesting depth

4.2 UiSchema structure (conceptual)

* `ui_version`
* `template_id` (optional when ephemeral)
* `title`
* `layout` (grid/rows/columns; simple to start)
* `components[]`
* `contracts`:

  * `outputs[]` (event types and payload schema)
  * `required_events[]` (e.g., must have confirm/cancel)

Component examples (initial allowlist):

* text, markdown
* form (group) with fields: text, number, select, checkbox
* table (selectable rows)
* code (readonly)
* diff (unified diff; later side-by-side)
* command_preview (readonly)
* log_view (append-only view for command output)
* stepper (wizard navigation)
* button (emits event)

4.3 Validation rules

* Reject unknown component types.
* Reject schemas exceeding limits.
* Reject schemas that don’t satisfy contract requirements (e.g., no completion/cancel path).
* Reject event payloads exceeding size limits.

5. UI Catalog

5.1 Catalog types and precedence

* Builtin catalog: shipped with app, read-only.
* User catalog: local folder, read/write.
* Optional org catalog: local path (synced folder/repo), usually read-only.

Precedence (first match wins): org → user → builtin.

5.2 Template definition
Each template consists of:

* `meta`: id, name, version, tags, description, author, created_at
* `match`: deterministic selectors (primary intent, operations, languages, risk)
* `schema`: UiSchema
* `contracts`: event contracts

5.3 Intent model (UiIntent)
Agent must emit `UiIntent` before requesting UI:

* `primary`: e.g., `review_changes`, `confirm_parameters`, `select_files`, `run_workflow`
* `operations[]`: e.g., `apply_patch`, `run_command`, `approve`
* `context`: language(s), artifact types, risk level

The client selects the template. The agent does not pick the template by name.

5.4 Fallback: agent-generated ephemeral UI
If no template matches:

* client sends a constrained “UI generation request” to agent:

  * allowed components
  * contract requirements
  * schema limits
* agent returns `UiSchema` + suggested metadata
* client validates and renders as ephemeral

5.5 Promotion flow
After successful use of an ephemeral UI:

* prompt user: “Save this UI to your catalog?”
* allow quick edit of: template name, tags, match rules
* write into user catalog; update index

Promotion conditions (to avoid nagging):

* UI used to completion AND no validation errors
* intent occurs more than once in session OR agent marks as reusable
* schema hash is not near-duplicate of existing template

6. Copilot integration using `copilot-sdk-rust`

6.1 Client lifecycle

* At startup, verify Copilot CLI availability and auth by invoking SDK initialization.
* Support two connection modes:

  * stdio: spawn CLI server as child process
  * TCP: connect to an existing server or spawn and connect (as SDK supports)

6.2 Session handling

* One Copilot session per workspace by default.
* Persist session metadata locally so you can resume or at least restore transcript.

6.3 Streaming output

* SDK should provide streaming events; UI must render assistant output incrementally.

6.4 Tooling and approvals (policy layer)
Even if SDK exposes tool calls, the UI client is responsible for gating:

* Passive mode (default): never executes commands, never writes files; only previews and emits “approval events” back to agent.
* Active mode (explicit opt-in): allows execution/writes after user approval.

7. Persistence

Start filesystem-first for templates; SQLite optional for sessions/audit.

7.1 Templates

* `~/.agentic-ui/catalog/user/templates/*.json`
* Builtin templates packaged into the binary or as read-only assets.

7.2 Session logs and audit

* Store:

  * session transcript (messages)
  * UI schemas rendered (template id + version or ephemeral hash)
  * approvals/denials
  * command outputs (if active mode)

8. MVP deliverables and acceptance criteria

MVP must demonstrate:

* Copilot integration via `copilot-sdk-rust` (connect, create session, stream response). ([GitHub][1])
* Render at least 6 builtin UI templates via UiSchema.
* UI Catalog:

  * load builtin + user templates
  * resolve intents deterministically
  * promote an ephemeral UI into user catalog
* Passive/Active mode toggle with explicit approvals.
* The right-hand side “canvas” can render multiple UI types (form + table + diff), proving the diff viewer is not fixed.

9. Initial builtin UI template set (seed catalog)

Ship templates that cover interaction primitives, not domains:

* Confirm/deny action bundle
* Patch review (diff component)
* Parameter confirmation (form)
* Command review + run (command_preview + log_view)
* Select files (table)
* Multi-step wizard (stepper + embedded components)

That gives you enough to avoid constant UI generation.
