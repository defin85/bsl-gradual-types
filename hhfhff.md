Рекомендую такой порядок реализации (от самого блокирующего к менее блокирующему).

  1. Закрыть текущий почти завершённый change

  1. prioritize-completion-under-large-module-churn
     Причина: уже 8/9, нужно формально закрыть 4.2 (отчёт уже есть: backend/openspec/changes/prioritize-completion-under-large-module-churn/validation/scale-aware-large-small-live.json).

  2. Сначала зафиксировать контрактную инфраструктуру
  2. add-versioned-contracts-layer
  Причина: дальше будет много изменений в completion/diagnostics/observability; без versioned contracts вырастет риск дрейфа интерфейсов.

  3. Критический perf-путь (зависимости уже указаны в proposals)
  3. add-incremental-parse-snapshot-for-analysis-v2
  Основание: это фундамент для следующих двух (Dependencies в openspec/changes/add-incremental-parse-snapshot-for-analysis-v2/proposal.md).
  4. add-cancellable-diagnostics-supersession
  Основание: снижает waste CPU на superseded heavy задачах; опирается на п.3 (openspec/changes/add-cancellable-diagnostics-supersession/proposal.md).
  5. add-bounded-stale-completion-fastpath
  Основание: зависит от 3+4 (openspec/changes/add-bounded-stale-completion-fastpath/proposal.md).

  4. Затем закрепить гейты и убрать дубли
  6. add-large-module-completion-acceleration-gate
  Причина: сделать окончательный enforce/CI после 3–5.
  7. add-incremental-syntax-diagnostics-for-large-modules
  Причина: сейчас пересекается с add-incremental-parse-snapshot-for-analysis-v2 по сути. Рекомендация: не реализовывать отдельно “как есть”, а сузить до остатка или заархивировать как superseded после п.3.

  5. Функциональная точность IntelliSense (доменные типы)
  8. update-form-thisobject-platform-context-filtering
  9. add-v2-valuetable-column-resolution
  10. add-v2-structure-field-schema-resolution
  11. add-v2-map-index-value-resolution

  6. Исследование покрытия типов (можно параллельно с п.8–11, но лучше перед финальным GA)
  12. add-platform-config-parser-type-coverage-research
  Причина: даст объективную карту пробелов для финальной стабилизации.

  7. GA/коммерческий слой в конце
  13. add-lsp-functional-ga-readiness
  Важно: в tasks ссылка на update-v2-contextual-implicit-variables, а активный change у вас update-form-thisobject-platform-context-filtering — лучше синхронизировать зависимость до старта.
  14. add-tpm-lease-licensing
  15. add-sales-readiness-ga
  Причина: sales readiness логично закрывать после функционального GA + licensing policy.

  Если хочешь, следующим шагом могу сделать конкретный execution-план по неделям/итерациям (что в какой PR, какие тесты-гейты на каждом шаге).

  Что запускать теперь (один прогон):

  BSL_V2_SCALE_AWARE_PROGRESS_EVERY=1 BSL_V2_SCALE_AWARE_GATE_REPORT=/tmp/p31-live-diag.json cargo test -p bsl-backend --bin bsl-lsp-server p31_scale_aware_large_small_completion_gate_live -- --nocapture --test-threads=1

  И сразу топ p95 по large warm:

  jq -r '.profiles.large.warm.metrics | to_entries[] | select(.value|type=="object" and has("p95")) | [.key, .value.p95, .value.p99, .value.count] | @tsv' /tmp/p31-live-diag.json | sort -t $'\t' -k2,2nr | head
  -n 25

  Если хочешь, после этого я по выводу сразу укажу конкретный проблемный участок кода, уже без новых “часовых” угадываний.

  [egor@DESKTOP-SS5D6MM bsl-gradual-types]$ BSL_V2_SCALE_AWARE_PROGRESS_EVERY=1 BSL_V2_SCALE_AWARE_GATE_REPORT=/tmp/p31-live-diag.json cargo test -p bsl-backend --bin bsl-lsp-server p31_scale_aware_large_small_c
ompletion_gate_live -- --nocapture --test-threads=1
   Compiling bsl-backend v0.4.105 (/home/egor/code/bsl-gradual-types/backend)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 5.54s
     Running unittests src/bin/lsp_server/main.rs (target/debug/deps/bsl_lsp_server-cf9cd9861cbd5697)

running 1 test
[p31] profile=large phase=start done progress=1/1 (100.0%) elapsed_ms=71309 eta_ms=0 completion_total=1 cancelled_total=0 cancelled_rate=0.0000 churn_edits=0
[p31] profile=large phase=cold done progress=5/5 (100.0%) elapsed_ms=70940 eta_ms=0 completion_total=5 cancelled_total=0 cancelled_rate=0.0000 churn_edits=0
[p31] profile=large phase=warm done progress=55/55 (100.0%) elapsed_ms=3552202 eta_ms=0 completion_total=55 cancelled_total=0 cancelled_rate=0.0000 churn_edits=50
[p31] profile=small phase=start done progress=1/1 (100.0%) elapsed_ms=12 eta_ms=0 completion_total=1 cancelled_total=0 cancelled_rate=0.0000 churn_edits=0
[p31] profile=small phase=cold done progress=5/5 (100.0%) elapsed_ms=34 eta_ms=0 completion_total=5 cancelled_total=0 cancelled_rate=0.0000 churn_edits=0
[p31] profile=small phase=warm done progress=55/55 (100.0%) elapsed_ms=112 eta_ms=0 completion_total=55 cancelled_total=0 cancelled_rate=0.0000 churn_edits=0
p31_scale_aware_gate_report=/tmp/p31-live-diag.json
ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 112 filtered out; finished in 3694.63s

Отлично, теперь причина локализована предметно.

  1. Реальный узкий участок: query_bundle
     completion_stage_query_bundle_ms.p95 = 35490ms при completion_duration_ms.p95 = 35500ms.
  2. Это совпадает с runtime execution:
     intellisense_v2_runtime_exec_interactive_ms.p95 = 35007ms.
  3. Всё остальное маленькое и не объясняет 35с:
     semantic_diagnostics_query.p95 = 285ms, ir_query.p95 = 10ms, response_build.p95 = 9ms, prepare_stateful.p95 = 2ms, queue-wait ~2ms.
  4. Значит bottleneck внутри closure, который исполняется в spawn_bounded_blocking_with_class_observed_origin:

  - вызов в language_server.rs (/home/egor/code/bsl-gradual-types/backend/src/bin/lsp_server/server/language_server.rs:2739)
  - замер runtime_exec_interactive_ms делается в policy.rs (/home/egor/code/bsl-gradual-types/bsl-runtime/src/application/intellisense_v2/policy.rs:631)

  5. По коду внутри этого closure уже измеренные куски (ir/parse) не тяжелые. Остаётся практически один кандидат: расчёт member_access_owner_type_hint (вызовы flow_type_at_byte_offset / type_at_byte_offset) в
     language_server.rs (/home/egor/code/bsl-gradual-types/backend/src/bin/lsp_server/server/language_server.rs:2925).

  Что делать дальше (чтобы не гонять много раз по часу):

  1. Я добавлю ещё 2 подэтапа внутри query_bundle: owner_hint_ms и deps_and_file_snapshot_ms.
  2. Один следующий p31 прогон даст точный виновник уже без догадок.