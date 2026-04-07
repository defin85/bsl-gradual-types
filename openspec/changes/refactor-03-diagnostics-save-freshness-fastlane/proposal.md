# Change: шаг 3 убрать didSave apply-lag из first diagnostics refresh

## Почему

Incident bundle `2026-04-07T12:42:23Z` показывает, что заметная задержка обновления ошибок после
сохранения сидит не в UI и не в semantic query как таковом, а раньше, в ожидании пока analysis
runtime догонит сохранённую версию файла:

- `intellisense_v2_wait_for_file_version_diagnostics_ms.p95=13931`;
- `intellisense_v2_drilldown_stage_latency_ms_origin_lsp_operation_apply_changes_batch_stage_runtime_queue_wait.p95=13764`;
- `intellisense_v2_drilldown_stage_latency_ms_origin_lsp_operation_apply_change_set_file_stage_runtime_exec.p95=14323`;
- при этом `intellisense_v2_drilldown_stage_latency_ms_origin_lsp_operation_diagnostics_stage_semantic_diagnostics_query.p95=716`.

Текущий `didSave` path запускает только `IdleHeavy`, а diagnostics runtime затем идёт через
`prepare_stateful_operation(min_file_version=requested_version)` и может неограниченно ждать
`wait_for_file_version` для analysis host. Пользователь видит это как "сохранил файл, а ошибки
обновились сильно позже".

## Что меняется

- `didSave` diagnostics MUST иметь bounded first-publish fastlane для сохранённой версии документа и
  MUST NOT сидеть на seconds-scale `wait_for_file_version`, если same-version shadow/parse artifacts
  уже доступны.
- bounded first publish MAY быть неполным по сравнению с final heavy publish, но MUST оставаться
  same-version truthful и MUST NOT публиковать diagnostics от более старой revision.
- heavy flow-sensitive/final diagnostics MAY завершаться вторым проходом после first publish, но
  MUST NOT быть обязательным prereq для быстрого обновления ошибок после save.
- observability и acceptance MUST отдельно различать:
  - save fastlane first publish;
  - wait/apply lag до latest revision;
  - heavy follow-up publish.

## Impact

- Спецификация: `bsl-intellisense-v2`
- Backend/LSP: `didSave` document sync path и diagnostics runtime
- Runtime facade: apply/wait-for-version contract и ownership между shadow text и analysis host
- Validation: perf/regression coverage для same-file save latency на representative fixture

## Не цели

- перепроектирование completion path или current-context auxiliary load;
- полная оптимизация `AnalysisHostV2::apply_change(SetFile)` для всех операций;
- UI-first workaround в `vscode-extension/`.
