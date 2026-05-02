# Change: update local variable member completion children

## Why
After `ТаблЗнач = Новый ТаблицаЗначений;`, member completion for `ТаблЗнач.` currently returns no children in the live incident. The incident bundle shows after-dot probes returning `empty_list` or being cancelled, including repeated empty results for version `22` and a later cancelled after-dot probe for version `48`.

This is a user-visible completion correctness failure. It should be fixed after snapshot terminal liveness is trustworthy, because the same incident also showed a hanging snapshot status and exact/current-head readiness gaps.

## What Changes
- Add an IntelliSense v2 contract for member completion on local variables whose type is inferred from constructor assignment inside a procedure or function.
- Require `ТаблЗнач.` to return `ТаблицаЗначений` children such as `Колонки` and `ВыгрузитьКолонку` when current-revision completion artifacts are ready.
- Distinguish three outcomes:
  - artifact unavailable: explicit bounded fail-closed/unavailable;
  - owner type unresolved with ready artifacts: bounded `owner_unresolved`-class correctness failure, not a successful empty result and not an artifact-unavailable result;
  - owner type resolved: children returned from the canonical current-revision artifact.
- Require local-variable owner type hints to be produced by the shared canonical owner-resolution path used by completion head and exact artifacts; LSP adapter code must only pass those hints through and must not reconstruct local owner truth from parse text, static receiver fallback, or adapter-local IR logic.
- Add focused regression tests using the real `examples/conf_big/CommonModules/АвансовыйОтчетФормы/Ext/Module.bsl` fixture and a minimal snippet.
- Keep variable-name completion and member-child completion separate so assertions for the local variable item do not substitute for children under `ТаблЗнач.`.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `analysis-v2` / IR local symbol and constructor-type inference
  - `analysis-v2/src/derived_artifacts.rs` completion head type entries
  - `bsl-runtime` completion head / owner-hint generation
  - `bsl-runtime/src/application/type_system/services/completion_service.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `backend/src/bin/lsp_server/handlers/completion.rs` only as a pass-through/fail-closed boundary, not as a new inference layer
  - completion trace/probe tests and representative live fixture tests

## Dependencies
- Depends on `update-snapshot-status-terminal-liveness`.
- Implementation must first prove snapshot/head/exact readiness is terminal and observable; otherwise `ТаблЗнач.` empty children can be misclassified as a readiness race.

## Non-Goals
- Do not relax current-revision fail-closed semantics.
- Do not serve stale children from an older revision.
- Do not add syntax-helper fallback or heuristic members outside canonical current-revision artifacts.
- Do not treat the presence of a `CompletionItemKind::VARIABLE` item for `ТаблЗнач` as proof that member children for `ТаблЗнач.` work.
