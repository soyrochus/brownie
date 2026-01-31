# Node.js Project Instructions

You must produce a high-detail technical docset from source code only. Avoid abstract summaries. Prefer concrete inventories, tables, and evidence-anchored facts.

## Stack Focus

- Entry points: `package.json` scripts, `index.js`, `server.js`, `app.js`, `src/main.ts`, `bin/*`.
- Frameworks: Express, Fastify, Koa, NestJS, Hapi, Next.js, Remix.
- Structure: `src/`, `server/`, `apps/`, `packages/`, `services/`, `api/`.
- Data models: TypeScript interfaces/types, Zod/Joi schemas, Prisma schema, Mongoose/TypeORM models.

---

## Mandatory Workflow: Per-Document Direct Synthesis

For EACH document in order:

1. **Identify** relevant source files for that document (see reading focus table below).
2. **Read** those files directly using bounded reads.
3. **Write** the document immediately from observed evidence.
4. **Move** to next document.

**Critical:** Do NOT collect inventories globally before writing docs. Each document is synthesized directly from source reads for that document.

---

## Document-Specific Reading Focus

Before writing each document, read the files listed in the Source Focus column:

| Document | Source Focus | What to Extract |
| -------- | ------------ | --------------- |
| **Project Intent & Business Frame** | `package.json`, CLI entry (`bin/*`, `src/cli.*`), orchestration modules, README | Purpose, commands, flags, exit codes, business goals |
| **Domain Landscape** | Model files, `types.*`, `schemas.*`, domain classes, enums | Core concepts, entity relationships, boundaries |
| **Canonical Data Model** | Type definitions, validation schemas, config classes, persistence modules | Field names, types, defaults, JSON schemas |
| **Service & Capability Map** | Module `index.*` files, service classes, orchestration functions | Module responsibilities, public functions, dependencies |
| **Architectural Guardrails** | Config defaults, validation functions, error handlers, limit constants | Constraints, thresholds, validation rules, error paths |
| **API & Integration Contracts** | External imports, SDK usage, HTTP clients (`fetch`, axios), tool definitions | Provider interfaces, external dependencies, contracts |
| **User Journey & UI Intent** | CLI definitions, output formatters, logging, user-facing messages | User flows, commands, output modes, error messages |

---

## Output Format Requirements

### Evidence Citations (Required)

Every major claim must include file:line evidence. Format examples:

```text
**Evidence:** `<file>.ts:<line>` - `<relevant code snippet>`
```

Or inline: `The <functionName> function does X (path/to/file.ts:45-77).`

### Tables with Evidence Column (Required)

All inventories must use tables with an Evidence column. Format example:

```markdown
| Field | Type | Default | Evidence |
|-------|------|---------|----------|
| `<fieldName>` | `<type>` | `<default>` | `<file>:<line>` |
```

### Code Structure Trees (Required for Data Models)

Use tree notation for types and config structures. Format example:

```text
<TypeName>
├── <field1>: <type>
├── <field2>: <NestedType>
│   ├── <nested_field1>: <type>
│   └── <nested_field2>: <type>
└── <field3>: <type>
```

### ASCII Diagrams (Required for Domain Landscape)

Show module boundaries and data flow. Format example:

```text
+-----------------------------------------+
|          <Layer Name> (<file>)          |
+-----------------------------------------+
                    |
                    v
+-----------------------------------------+
|          <Layer Name> (<file>)          |
+-----------------------------------------+
```

Replace `<placeholders>` with actual names from the project being analyzed.

### JSON/Config Examples (Where Applicable)

When the project uses JSON serialization, show the schema. Format example:

```json
{
  "<field>": "<value>",
  "<nested>": {
    "<subfield>": "<value>"
  }
}
```

Include actual field names and types from the project being analyzed.

---

## Document Depth Requirements

Each document must meet these minimums:

| Document | Min Tables | Min Evidence Bullets | Required Elements |
| -------- | ---------- | -------------------- | ----------------- |
| Project Intent | 2 | 5 | Goals table, CLI commands table |
| Domain Landscape | 2 | 5 | Concepts table, ASCII boundary diagram |
| Canonical Data Model | 3 | 5 | Code tree per major type, field tables |
| Service & Capability Map | 2 | 5 | Module table with Key Functions column |
| Architectural Guardrails | 2 | 5 | Constraints table with enforcement evidence |
| API & Integration Contracts | 2 | 3 | External deps table, usage examples |
| User Journey & UI Intent | 1 | 5 | CLI flow diagram or step sequence |

---

## Section Structure Requirements

### Project Intent & Business Frame

- §1.1 Purpose (with evidence)
- §1.2 Business Goals table (Goal, Description, Evidence)
- §1.3 Target Users (bullet list)
- §1.4 Value Proposition (evidence-backed bullets)
- §1.5 CLI Commands table (Command, Flags, Defaults, Exit Codes, Evidence)

### Domain Landscape

- §2.1 Core Domain Concepts table (Concept, Description, Location)
- §2.2 Domain Boundaries (ASCII diagram)
- §2.3 Key Enums/Types table (Name, Values, Evidence)

### Canonical Data Model

- §3.1 Configuration Models (code trees + field tables)
- §3.2 Domain Models (code trees + field tables)
- §3.3 Runtime Models (code trees + field tables)
- Include JSON schema examples where serialization exists

### Service & Capability Map

- §4.1 Module Overview table (Module, Responsibility, Key Functions)
- §4.2+ One subsection per major module with function signatures

### Architectural Guardrails

- §5.1 Bounded Analysis limits table
- §5.2 Scope Isolation rules
- §5.3 Validation Rules with enforcement evidence
- §5.4 Error Handling paths

### API & Integration Contracts

- §6.1 External Dependencies table (Package, Usage, Evidence)
- §6.2 Provider/SDK Integration details
- §6.3 File Format Contracts (JSON schemas, config formats)
- Write stub with "No external APIs detected" if not applicable

### User Journey & UI Intent

- §7.1 User Interface type (CLI/Web/etc)
- §7.2 User Journeys (step sequences with evidence)
- §7.3 Output Artifacts list
- §7.4 Error Handling (user-facing messages)
- Write stub with "No UI layer detected" if not applicable

---

## Quality Rules

1. **Evidence required:** Every claim must cite file:line-range or state "Not found in bounded reads."
2. **No vague language:** Avoid "appears to", "seems to", "might". State facts with evidence or mark as unknown.
3. **Concrete symbols:** Use actual function/class/type names, not descriptions.
4. **Deep reads:** For each document, read at least 200-400 lines of relevant source files.
5. **Re-read if shallow:** If initial reads yield only surface facts, read deeper or read related files.

---

## Probe Terms

- express
- fastify
- koa
- nest
- hapi
- next
- remix
- router
- middleware
- prisma
- mongoose
- typeorm
