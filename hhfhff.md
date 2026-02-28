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

  Результат однозначный: bottleneck не в построении индекса, а в ожидании при его получении.

  - large/warm:
      - completion_duration_ms p95 = 156302
      - ...type_lookup_index_fetch_ms p95 = 156207
      - ...index_build_total_ms p95 = 97
      - ...index_build_visit_statements_ms p95 = 87
      - ...index_parse_result_ms p95 = 0
      - ...index_scan_ms p95 = 0
  - Оценка «чистого ожидания» внутри fetch: 156207 - 97 - 0 = 156110 ms (p95).
  - Путь owner_hint стабильный: direct=55, hit=55, fallback=0, cancelled=0.
  - Gate закономерно pass=false, large_completion_ratio ≈ 39.97.

  Вывод: under churn мы в основном ждём выдачу type_index (контеншн/синхронизация/инвалидации), а не тратим время на сам build_type_index.

  Если нужна одна команда именно для проверки новой метрики под churn, запускай эту:

  BSL_V2_SCALE_AWARE_CHURN_MODE=large_warm BSL_V2_SCALE_AWARE_CHURN_EVERY=10 BSL_V2_SCALE_AWARE_PROGRESS_EVERY=1 BSL_V2_SCALE_AWARE_GATE_REPORT=/tmp/p31-churn10.json cargo test -p bsl-backend --bin bsl-lsp-
  server p31_scale_aware_large_small_completion_gate_live -- --nocapture --test-threads=1

  Потом смотри новую метрику так:

  jq '.profiles.large.warm.metrics.completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_wait_ms' /tmp/p31-churn10.json

  Если хочешь сравнение с baseline без churn, запускай ещё и это:

  BSL_V2_SCALE_AWARE_CHURN_MODE=off BSL_V2_SCALE_AWARE_PROGRESS_EVERY=1 BSL_V2_SCALE_AWARE_GATE_REPORT=/tmp/p31-no-churn.json cargo test -p bsl-backend --bin bsl-lsp-server
  p31_scale_aware_large_small_completion_gate_live -- --nocapture --test-threads=1

  И смотри:

  jq '.profiles.large.warm.metrics.completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_wait_ms' /tmp/p31-no-churn.json