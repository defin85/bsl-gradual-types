## 1. Contract и truthful query-body attribution

- [ ] 1.1 Поднять authoritative completion timeline до response version `20` и contiguous baseline `contracts/lsp-completion-timeline/v17`, зафиксировав bounded `query_bundle` stage taxonomy для pool-wait / snapshot / ir-query / retry / other split.
- [ ] 1.2 Зафиксировать в contract fail-closed semantics: если request вошёл в query-body path, `query_bundle*` stage MUST публиковаться и на `cancelled/failed` path, а spent time MUST NOT полностью исчезать в `unattributed_overhead`.
- [ ] 1.3 Обновить extension-facing spec delta так, чтобы derived verdicts использовали authoritative `dominant_stage`/`stages` для `query_bundle` dominance и явно деградировали на `v19`.

## 2. Backend runtime и timeline instrumentation

- [ ] 2.1 Добавить observed blocking helper для interactive query path, который возвращает request-local split между pool queue wait и blocking exec без потери existing global metrics.
- [ ] 2.2 Перевести `impl_completion.rs` на truthful `query_bundle` stage accounting с bounded sub-stages и stage guard на success/cancel/fail paths.
- [ ] 2.3 Сохранить bounded cancellation semantics для superseded/cancelled completion: trace показывает truthful query-body stage status, но не выдумывает hard preemption там, где её реально нет.

## 3. Human-readable projection

- [ ] 3.1 Свести verdict builder для Completion Timeline/webview/clipboard/incident bundle к единому bounded source of truth.
- [ ] 3.2 Убрать ложный verdict `adapter_before_dispatch_dominant` для traces, где dominant latency находится в `query_bundle*`, и добавить focused projection для query-body dominance.
- [ ] 3.3 Зафиксировать graceful degradation для `v19`: old payload остаётся читаемым, но detailed query-body split помечается unavailable by design.

## 4. Validation

- [ ] 4.1 Добавить backend tests, которые доказывают: cancelled request внутри query-body публикует `query_bundle*` stage, а не оставляет seconds-scale tail в одном `unattributed_overhead`.
- [ ] 4.2 Добавить backend tests на request-local split `query_bundle_pool_wait` vs `query_bundle_ir_query`, чтобы saturation и actual compute различались в authoritative trace.
- [ ] 4.3 Добавить extension tests на truthful verdicts и explicit `v19` degradation для Completion Timeline, clipboard и incident bundle.
- [ ] 4.4 Обновить versioned contract baseline и прогнать `openspec validate refactor-completion-query-bundle-root-cause-attribution --strict --no-interactive`.
