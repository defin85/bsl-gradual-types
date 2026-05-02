## 1. Preconditions
- [x] 1.1 Confirm `update-snapshot-status-terminal-liveness` is implemented or explicitly mocked in tests so readiness state cannot remain stale `building`.
- [x] 1.2 Capture the current failing behavior for `ТаблЗнач.` after-dot completion in the real fixture or a deterministic backend test.

## 2. Completion Correctness
- [x] 2.1 Add a minimal BSL snippet test for `Лок = Новый ТаблицаЗначений; Лок.` returning `ТаблицаЗначений` children.
- [x] 2.2 Add a real-fixture regression for `examples/conf_big/CommonModules/АвансовыйОтчетФормы/Ext/Module.bsl` around `ТаблЗнач.`.
- [x] 2.3 Ensure tests assert member children such as `Колонки` and `ВыгрузитьКолонку`, not only the local variable label.
- [x] 2.4 Fix local variable constructor-type propagation through canonical IR / completion head / exact artifact as needed.
- [x] 2.5 Fix member-access owner-hint lookup so local-scope owner expressions are resolved for current revision.
- [x] 2.6 Ensure artifact-unavailable, owner-unresolved, and resolved-zero-members outcomes are distinguishable in traces or test assertions.
- [x] 2.7 Add artifact-level assertions that `completion_member_access_owner_type_hints_from_analysis` and completion-head owner hints both resolve `Лок`/`ТаблЗнач` to `ТаблицаЗначений` when their respective artifacts are ready, whether the artifact stores this as a general type entry or as a dedicated owner-hint projection.
- [x] 2.8 Preserve the LSP adapter boundary: do not add adapter-local owner inference in `handlers/completion.rs`; it may pass shared hints through or fail closed with the bounded owner-unresolved classification. Existing static receiver fallback may remain only for non-local/static receivers and must not satisfy the local-variable owner scenarios.

## 3. Regression Guards
- [x] 3.1 Preserve fail-closed behavior when current-revision artifacts are unavailable.
- [x] 3.2 Preserve non-member local variable completion behavior and `CompletionItemKind::VARIABLE` assertions separately from member-child assertions.
- [x] 3.3 Ensure no stale previous-revision children are served during didChange churn.
- [x] 3.4 Ensure an artifact-ready owner-hint miss is not reported as `exact_deadline`, `wait_not_ready`, or successful empty children.
- [x] 3.5 Ensure artifact-unavailable/degraded `isIncomplete=true` responses do not synthesize local-variable children and are not counted as successful `Лок.`/`ТаблЗнач.` member-child completion.

## 4. Validation
- [x] 4.1 Run `openspec validate update-local-variable-member-completion-children --strict --no-interactive`.
- [x] 4.2 Run targeted completion/member-access backend tests.
- [x] 4.3 Run the smallest representative live or integration completion probe for `ТаблЗнач.` after snapshot liveness is fixed.
- [x] 4.4 Run formatting/checks required by touched Rust/TypeScript paths.
