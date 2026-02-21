## 1. Schema and Runtime Foundations

- [x] 1.1 Add new UI domain modules for schema, runtime, registry, events, and theme (`src/ui/*`, `src/theme.rs`) and wire them in `src/main.rs`
- [x] 1.2 Define typed `UiSchema`/component/form-field/output-contract models with serde support and enum-based component kinds
- [x] 1.3 Add embedded deterministic schema fixture loading via `include_str!` + deserialization into typed `UiSchema`
- [x] 1.4 Define typed `UiEvent` variants (button + form field) with structured payloads including originating component IDs

## 2. Validation and Registry

- [x] 2.1 Implement `ComponentRegistry` allowlist for `markdown`, `form`, `code`/`diff`, and `button`
- [x] 2.2 Implement form-field type enforcement for `text`, `number`, `select`, and `checkbox`
- [x] 2.3 Implement schema validation for max component count and max nesting depth limits
- [x] 2.4 Implement validation for unique actionable IDs and button output-contract mapping requirements
- [x] 2.5 Ensure render APIs accept only validated schema types (never raw schema)

## 3. Canvas Rendering and Event Propagation

- [x] 3.1 Implement enum/registry-driven render dispatch for all supported component variants in declared schema order
- [x] 3.2 Render markdown, form, code/diff, and button components in the right Canvas panel from validated schema
- [x] 3.3 Emit typed `UiEvent` values on button clicks with mapped output event IDs
- [x] 3.4 Emit typed `UiEvent` values on committed form-field updates with typed values
- [x] 3.5 Render right-panel append-only UI event debug log in deterministic interaction order
- [x] 3.6 Render non-interactive validation/runtime error state in Canvas when fixture load/validation fails

## 4. App Shell and Styling Refactor

- [x] 4.1 Replace right-panel static placeholder with runtime-backed Canvas + debug log while preserving panel layout and hierarchy
- [x] 4.2 Refactor shell styling to centralized `Theme` tokens (surfaces, text, accents, spacing, radii, shadows) and remove inline color literals in app UI code
- [x] 4.3 Update top bar to centered status area with semantic marker and subtle gradient while keeping passive mode indicators visible and active-mode control disabled
- [x] 4.4 Apply constrained spacing/radius/control styles to sidebar, chat bubbles, buttons, diff lines, and input container without changing component IDs or interaction logic
- [x] 4.5 Keep chat transcript/send/session behavior functional after Canvas-first visual refactor

## 5. Tests and Verification

- [x] 5.1 Add validation unit tests for valid schema, unknown component, unsupported field type, max-count/max-depth violations, and missing button output contract
- [x] 5.2 Add runtime/event tests proving deterministic `UiEvent` sequence for repeated interaction order
- [x] 5.3 Run `cargo check` and `cargo test` and fix any issues introduced by SPEC-2 changes
