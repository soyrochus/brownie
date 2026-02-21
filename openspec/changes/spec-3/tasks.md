## 1. Catalog Domain & Providers

- [x] 1.1 Add catalog domain types for template metadata, match rules, provider identity, and resolution trace structures.
- [x] 1.2 Implement a `CatalogProvider` abstraction with `BuiltinCatalogProvider` (embedded read-only templates) and `UserCatalogProvider` (filesystem-backed load + persistence operations).
- [x] 1.3 Add load-time template validation (`meta`, `match`, `schema`) and exclude invalid templates with explicit diagnostics.

## 2. Deterministic Intent Resolution

- [x] 2.1 Implement `CatalogManager` that aggregates provider templates and resolves by deterministic precedence (`org -> user -> builtin` when org enabled, otherwise `user -> builtin`).
- [x] 2.2 Implement deterministic matching/ranking rules: exact `primary` match gate, secondary overlap scoring, and stable tie-breakers.
- [x] 2.3 Emit structured resolution diagnostics for both selected-template and no-match outcomes.

## 3. Runtime & App Integration

- [x] 3.1 Replace hardcoded runtime schema bootstrap with catalog-selected template schema input while preserving existing `UiSchema` runtime validation semantics.
- [x] 3.2 Update app-shell canvas area to show selected template context, explicit no-match state (`No matching UI template found`), and keep existing diagnostics behavior intact.
- [x] 3.3 Ensure agent flow remains `UiIntent`-driven only (no direct template-id forcing path).

## 4. Tests

- [x] 4.1 Add provider tests for builtin load behavior, user filesystem load/persistence behavior, and read-only enforcement for builtin mutation attempts.
- [x] 4.2 Add resolver tests for deterministic precedence, ranking, tie-breakers, and repeated identical intents yielding identical outcomes.
- [x] 4.3 Add integration tests for runtime/app outcomes: selected-template render path, malformed selected schema error path, and explicit no-match behavior.
