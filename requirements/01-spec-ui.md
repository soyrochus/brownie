# 1. UiIntent (Agent → Client)

Purpose:
The agent must declare what kind of UI it needs before any UI is rendered.

This prevents the agent from directly pushing arbitrary UI.

### JSON Schema (conceptual)

```json
{
  "type": "object",
  "required": ["primary"],
  "properties": {
    "primary": {
      "type": "string",
      "description": "Primary intent key, e.g. review_changes, confirm_parameters"
    },
    "operations": {
      "type": "array",
      "items": { "type": "string" }
    },
    "context": {
      "type": "object",
      "properties": {
        "languages": {
          "type": "array",
          "items": { "type": "string" }
        },
        "artifacts": {
          "type": "array",
          "items": { "type": "string" }
        },
        "risk": {
          "type": "string",
          "enum": ["low", "medium", "high"]
        }
      }
    }
  }
}
```

Example:

```json
{
  "primary": "review_changes",
  "operations": ["apply_patch", "confirm"],
  "context": {
    "languages": ["dockerfile"],
    "risk": "medium"
  }
}
```

Client resolves this through the UI Catalog.

Agent never references template IDs directly.

---

# 2. UiSchema (Rendered UI DSL)

This is the runtime surface definition.

Strictly declarative.
Strictly validated.
No scripting.

---

## 2.1 Top-Level Structure

```json
{
  "ui_version": "1.0",
  "template_id": "review_patch_v1",
  "title": "Review Dockerfile Update",
  "layout": {
    "type": "vertical"
  },
  "components": [],
  "contracts": {
    "outputs": []
  }
}
```

Validation rules:

* `ui_version` must match client-supported version.
* `components.length <= max_components`
* `serialized_size <= max_schema_bytes`
* no unknown component types

---

## 2.2 Component Types (Allowlist)

Initial allowlist:

* `markdown`
* `text`
* `form`
* `table`
* `code`
* `diff`
* `button`
* `command_preview`
* `log_view`
* `stepper`

Anything else → rejected.

---

## 2.3 Component Definitions

### 2.3.1 Markdown

```json
{
  "type": "markdown",
  "content": "### Review required"
}
```

---

### 2.3.2 Form

```json
{
  "type": "form",
  "id": "env_config",
  "fields": [
    {
      "id": "port",
      "label": "Port",
      "field_type": "number",
      "default": 8080
    },
    {
      "id": "debug",
      "label": "Enable Debug",
      "field_type": "checkbox",
      "default": false
    }
  ]
}
```

Allowed field types:

* text
* number
* select
* checkbox

---

### 2.3.3 Diff

```json
{
  "type": "diff",
  "file": "Dockerfile",
  "format": "unified",
  "content": "--- old\n+++ new\n..."
}
```

This replaces your fixed diff panel.

---

### 2.3.4 Command Preview

```json
{
  "type": "command_preview",
  "command": "pip install -r requirements.txt",
  "cwd": "/app"
}
```

Readonly until approved.

---

### 2.3.5 Button

```json
{
  "type": "button",
  "id": "confirm",
  "label": "Apply Changes",
  "style": "primary"
}
```

---

### 2.3.6 Stepper (Wizard)

```json
{
  "type": "stepper",
  "steps": [
    {
      "title": "Review",
      "components": [ ... ]
    },
    {
      "title": "Confirm",
      "components": [ ... ]
    }
  ]
}
```

Max nesting depth enforced (e.g., 3).

---

# 3. Contracts (Critical)

Every UI must define output events.

Example:

```json
{
  "contracts": {
    "outputs": [
      {
        "event": "confirm",
        "payload": {
          "apply": "boolean"
        }
      },
      {
        "event": "cancel",
        "payload": {}
      }
    ]
  }
}
```

Rules:

* At least one completion path required.
* Every `button.id` must correspond to a declared output event.
* Payload schema must be statically typed.

---

# 4. UiEvent (Client → Agent)

When user interacts, client emits structured event.

```json
{
  "event": "confirm",
  "payload": {
    "apply": true
  },
  "origin": {
    "template_id": "review_patch_v1",
    "component_id": "confirm"
  }
}
```

Agent receives this as a structured message via Copilot session.

No free text.

---

# 5. UI Catalog Structure

Filesystem-based.

```
~/.agentic-ui/
  config.toml
  catalog/
    builtin/
      templates/
    user/
      templates/
    org/
      templates/
```

Each template file:

```json
{
  "meta": {
    "id": "review_patch_v1",
    "version": 1,
    "name": "Review Patch",
    "tags": ["review", "patch"],
    "created_at": "2026-02-20T10:00:00Z"
  },
  "match": {
    "primary": "review_changes",
    "operations": ["apply_patch"]
  },
  "schema": { ...UiSchema... }
}
```

Resolution order:
org → user → builtin.

---

# 6. Agent-Generated UI (Ephemeral Mode)

When no match:

Client sends constrained instruction:

* allowed component types
* max components
* required contract structure
* must emit confirm/cancel

Agent returns candidate UiSchema.

Client validates:

* size
* component allowlist
* contract completeness
* no recursive stepper explosion

If valid → render ephemeral.

After successful completion:
Prompt:
“Save this UI to your catalog?”

If yes:

* request minimal metadata
* write to `user/templates/`
* update index

---

# 7. Rust Type Model (Strongly Typed)

Core enums:

```rust
enum Component {
    Markdown(Markdown),
    Text(Text),
    Form(Form),
    Table(Table),
    Code(Code),
    Diff(Diff),
    Button(Button),
    CommandPreview(CommandPreview),
    LogView(LogView),
    Stepper(Stepper),
}
```

Schema:

```rust
struct UiSchema {
    ui_version: String,
    template_id: Option<String>,
    title: String,
    layout: Layout,
    components: Vec<Component>,
    contracts: Contracts,
}
```

Intent:

```rust
struct UiIntent {
    primary: String,
    operations: Vec<String>,
    context: Option<Context>,
}
```

Event:

```rust
struct UiEvent {
    event: String,
    payload: serde_json::Value,
    origin: EventOrigin,
}
```

All JSON validated against Rust types + schema rules.

---

# 8. Governance Controls

Global config options:

* disable agent UI generation entirely
* disable promotion
* max schema size
* max component count
* max nesting depth
* allowlist enforcement strict/lenient

---

# 9. Why This Architecture Is Coherent

This design:

* Uses Copilot SDK for execution layer.
* Makes UI deterministic via DSL.
* Enables dynamic UI without chaos.
* Enables learning via catalog promotion.
* Keeps governance centralized.

This is not “AI in a sidebar”.

It is an execution surface where UI becomes a compiled artifact of intent.

