## Контекст

После remediation completion path bundle `2026-04-07T12:42:23Z` перестал показывать hidden wait в
completion, но пользовательский симптом сместился в diagnostics-after-save.

По authoritative metrics:

- `wait_for_file_version_diagnostics_ms.p95=13931`;
- `apply_changes_batch.runtime_queue_wait.p95=13764`;
- `apply_change_set_file.runtime_exec.p95=14323`;
- `diagnostics.runtime_snapshot_with_deps.p95=5`;
- `semantic_diagnostics_query.p95=716`.

Это указывает, что first diagnostics refresh после save блокируется не на snapshot acquisition и не
на UI, а на том, что diagnostics ждёт пока writer thread реально применит `SetFile` к
`AnalysisHostV2`.

В коде есть два разных источника truth:

- `latest_document_shadow_state_v2` и `latest_received_file_versions_v2` уже знают same-version
  saved text/version;
- writer thread владеет `AnalysisHostV2` и `applied_file_revisions`, а diagnostics сейчас ждёт именно
  их readiness через `wait_for_file_version`.

## Цели

- Сделать первое обновление diagnostics после `didSave` bounded и user-visible быстрым.
- Сохранить same-version correctness: first publish не должен брать stale older revision.
- Не ломать heavy/final semantic diagnostics и flow-sensitive path.
- Сделать root cause отличимым по observability и acceptance assets.

## Не-цели

- Не переписывать весь writer thread scheduler.
- Не обещать, что first publish после save всегда уже содержит final flow-sensitive semantic output.
- Не менять completion / hover / current-context contracts.

## Ownership / Concurrency Map

- `latest_document_shadow_state_v2` владеет latest same-version text для opened document.
- writer thread остаётся единственным владельцем mutable `AnalysisHostV2`, `applied_file_revisions`
  и wake-up pending `wait_for_file_version` waiters.
- diagnostics scheduler владеет generation/token supersession и publish gate.

Следствие: bounded save fastlane должен читать same-version data без нарушения writer-thread
ownership и без попытки мутировать `AnalysisHostV2` вне writer thread.

## Решение

### 1. Ввести отдельный didSave first-publish fastlane

`didSave` больше не должен запускать только `IdleHeavy`.

Он должен инициировать отдельный first-publish path (`save_fastlane`), который:

- стартует сразу на `didSave`;
- использует bounded readiness budget вместо небюджетного ожидания `wait_for_file_version`;
- предпочитает same-version источники в таком порядке:
  1. уже applied analysis snapshot той же версии;
  2. version-bound parse snapshot той же версии;
  3. rebuild из shadow text той же версии, если version-bound parse snapshot ещё не готов.

### 2. First publish может быть частичным, но обязан быть same-version truthful

Если full semantic/latest applied analysis не готовы в пределах bounded save-freshness budget,
система не должна ждать seconds-scale apply lag.

Допустим первый publish:

- syntax-only для сохранённой версии;
- либо syntax + bounded semantic subset для той же версии, если он уже готов без долгого ожидания.

Недопустимы:

- publish diagnostics от older revision;
- unbounded wait перед первым publish только ради final heavy completeness.

### 3. `IdleHeavy` остаётся вторым проходом

`IdleHeavy` сохраняется как follow-up publish для same generation/version и может дополнять первый
publish final flow-sensitive semantic diagnostics.

Тем самым contract делится на два user-visible этапа:

- `save_fastlane`: bounded first refresh;
- `idle_heavy`: eventual heavy completion того же save.

### 4. Observability должна различать fastlane и heavy follow-up

Нужен новый low-cardinality profile `save_fastlane` в diagnostics event model.

Acceptance и observability должны позволять отдельно увидеть:

- latency до first publish после `didSave`;
- был ли first publish `syntax_only` или `syntax_plus_semantic`;
- ушёл ли delay в `wait_for_file_version`/apply lag или уже в heavy diagnostics query.

## Alternatives Considered

### Только оптимизировать `apply_change(SetFile)` и writer queue

Полезно, но недостаточно как первичный change. Даже заметное ускорение writer thread не даёт
гарантию bounded first publish после save.

### Просто заменить `DidSave -> IdleHeavy` на `DidSave -> DebouncedFull`

Недостаточно. `DebouncedFull` идёт через тот же `prepare_stateful_operation(min_file_version=...)`
и наследует тот же unbounded `wait_for_file_version`.

### Добавить только observability без нового fastlane

Сделает задержку лучше видимой, но не уберёт пользовательский симптом.

## Риски и trade-offs

### Риск: первый publish будет неполным

Это допустимо только если он same-version truthful и явным образом supersede-ится final heavy
publish той же revision.

### Риск: flicker между first publish и heavy publish

Нужно проверить acceptance на representative fixture и не допустить regressions, где temporary
partial publish живёт слишком долго или перетирается stale result.

### Риск: shadow-path логика начнёт дублировать analysis logic

Fastlane должен ограничиваться тем, что реально можно безопасно построить из same-version shadow
artifacts, а не становиться вторым полным semantic engine.

## Rollout / Validation

1. Зафиксировать новый save-freshness contract в OpenSpec.
2. Добавить failing regression на delayed apply + didSave.
3. Реализовать `save_fastlane` без stale older-version publish.
4. Подтвердить perf evidence на representative `conf_big` scenario и checked-in incident/report.
