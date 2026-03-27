## 1. Contract и deterministic correlation

- [x] 1.1 Зафиксировать contract seam `bslProbeId -> client_probe_id` и поднять timeline response version `17 -> 18` вместе с contiguous baseline `contracts/lsp-completion-timeline/v15`.
- [x] 1.2 Протянуть request-bound correlation key через default completion path: extension probe lifecycle -> outbound completion request -> raw LSP request boundary -> authoritative timeline trace.
- [x] 1.3 Обновить extension degradation так, чтобы primary correlation использовала `client_probe_id`, а timestamp/window эвристика оставалась только backward-compatible fail-closed fallback для legacy payload.
- [x] 1.4 Добавить focused tests на overlapping same-uri completion probes без `multiple_probe_candidates` на default path.

## 2. Quiet observability path

- [x] 2.1 Перевести `Completion Timeline` panel в quiet/backoff mode во время active completion probes и короткого idle окна после них.
- [x] 2.2 Сохранить cached export path как default для webview и ограничить fresh `bsl.getCompletionTimeline` explicit refresh/fallback path, когда cached capture отсутствует.
- [x] 2.3 Добавить extension tests/smoke на отсутствие polling noise и на явный manual refresh contract.

## 3. Same-version exact-task visibility

- [x] 3.1 Изменить lifecycle exact type-index producer task так, чтобы completed matching same-version task entry оставался observable/joinable до `serve_only_ready` либо до supersession, `didClose` или shutdown.
- [x] 3.2 Сохранить bounded fail-closed поведение для genuine cold miss, wrong-version и deadline cases без stale fallback и без request-side exact-task spawn.
- [x] 3.3 Добавить backend tests на same-version `TriggerCharacter='.'` после `didChange`, подтверждающие отсутствие spurious `NoMatchingTask` и сохранение parity с `Invoked`.

## 4. Validation и traceability

- [x] 4.1 Обновить runbook/contracts/changelog и зафиксировать traceability `Requirement -> Code -> Test` для новых mandatory требований, включая version bump `18/v15`.
- [x] 4.2 Прогнать targeted backend/extension verify set и representative acceptance path на `conf_big`, подтверждая deterministic correlation, quiet observability и trigger parity.
