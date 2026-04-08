## 1. Contract / Design
- [x] 1.1 Зафиксировать diagnostics-after-save latency contract и acceptance budgets в proposal/design/spec delta без изменения production code.
- [x] 1.2 Явно описать ownership map между `latest_document_shadow_state_v2`, diagnostics scheduler и writer-thread-owned `AnalysisHostV2`.

## 2. Runtime / Backend
- [x] 2.1 Добавить отдельный `didSave` first-publish fastlane path, который не зависит от seconds-scale `wait_for_file_version` перед первым publish.
- [x] 2.2 Гарантировать, что first publish после save использует только same-version truthful artifacts и не публикует older revision diagnostics.
- [x] 2.3 Сохранить `IdleHeavy` как follow-up path для final heavy/flow-sensitive publish той же generation/version.
- [x] 2.4 Не допустить, чтобы `save_fastlane` silently превращался в полный дублирующий semantic engine поверх shadow text.

## 3. Observability / Validation
- [x] 3.1 Добавить low-cardinality observability для `save_fastlane` отдельно от `idle_heavy`.
- [x] 3.2 Добавить regression, воспроизводящий delayed apply / didSave symptom и fail-ящий без bounded first publish.
- [x] 3.3 Добавить checked-in acceptance evidence на representative `conf_big` scenario, подтверждающее, что first diagnostics refresh после save больше не уходит в seconds-scale apply-lag wait.
- [x] 3.4 Прогнать `openspec validate refactor-03-diagnostics-save-freshness-fastlane --strict --no-interactive`.
