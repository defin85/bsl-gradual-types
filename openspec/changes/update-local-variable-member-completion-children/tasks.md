## 1. Preconditions
- [ ] 1.1 Confirm `update-snapshot-status-terminal-liveness` is implemented or explicitly mocked in tests so readiness state cannot remain stale `building`.
- [ ] 1.2 Capture the current failing behavior for `ТаблЗнач.` after-dot completion in the real fixture or a deterministic backend test.

## 2. Completion Correctness
- [ ] 2.1 Add a minimal BSL snippet test for `Лок = Новый ТаблицаЗначений; Лок.` returning `ТаблицаЗначений` children.
- [ ] 2.2 Add a real-fixture regression for `examples/conf_big/CommonModules/АвансовыйОтчетФормы/Ext/Module.bsl` around `ТаблЗнач.`.
- [ ] 2.3 Ensure tests assert member children such as `Колонки` and `ВыгрузитьКолонку`, not only the local variable label.
- [ ] 2.4 Fix local variable constructor-type propagation through canonical IR / completion head / exact artifact as needed.
- [ ] 2.5 Fix member-access owner-hint lookup so local-scope owner expressions are resolved for current revision.
- [ ] 2.6 Ensure artifact-unavailable, owner-unresolved, and resolved-zero-members outcomes are distinguishable in traces or test assertions.
- [ ] 2.7 Add artifact-level assertions that `completion_member_access_owner_type_hints_from_analysis` and completion-head owner hints both resolve `Лок`/`ТаблЗнач` to `ТаблицаЗначений` when their respective artifacts are ready.
- [ ] 2.8 Preserve the LSP adapter boundary: do not add adapter-local owner inference in `handlers/completion.rs`; it may pass shared hints through or fail closed with the bounded owner-unresolved classification.

## 3. Regression Guards
- [ ] 3.1 Preserve fail-closed behavior when current-revision artifacts are unavailable.
- [ ] 3.2 Preserve non-member local variable completion behavior and `CompletionItemKind::VARIABLE` assertions separately from member-child assertions.
- [ ] 3.3 Ensure no stale previous-revision children are served during didChange churn.
- [ ] 3.4 Ensure an artifact-ready owner-hint miss is not reported as `exact_deadline`, `wait_not_ready`, or successful empty children.

## 4. Validation
- [ ] 4.1 Run `openspec validate update-local-variable-member-completion-children --strict --no-interactive`.
- [ ] 4.2 Run targeted completion/member-access backend tests.
- [ ] 4.3 Run the smallest representative live or integration completion probe for `ТаблЗнач.` after snapshot liveness is fixed.
- [ ] 4.4 Run formatting/checks required by touched Rust/TypeScript paths.
