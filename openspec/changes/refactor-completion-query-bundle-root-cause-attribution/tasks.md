## 1. Contract, taxonomy и migration

- [ ] 1.1 Поднять authoritative completion timeline до response version `20` и contiguous baseline `contracts/lsp-completion-timeline/v17`, зафиксировав canonical grouped `query_bundle` taxonomy (`pool_wait`, `deps_and_file_snapshot`, `owner_hint`, `ir_query`, `ir_retry`, `other`).
- [ ] 1.2 Зафиксировать full migration policy для timeline/metrics/report consumers: `dominant_stage`, scale-aware reports, incident summary и acceptance gates используют только grouped taxonomy; legacy aggregate `query_bundle` допускается лишь как transitional mirror и не входит в canonical `v20/v17`.
- [ ] 1.3 Зафиксировать в contract fail-closed semantics: если request вошёл в query-body path, grouped `query_bundle*` stage MUST публиковаться и на `cancelled/failed` path, а spent time MUST NOT полностью исчезать в `unattributed_overhead`.
- [ ] 1.4 Обновить extension-facing spec delta так, чтобы derived verdicts использовали authoritative `dominant_stage`/`stages` для grouped `query_bundle` dominance, имели exact bounded verdict vocabulary и явно деградировали на `v19`.

## 2. Backend runtime и timeline instrumentation

- [ ] 2.1 Добавить `ObservedBlockingCall`-style helper для interactive query path, который возвращает request-local split между pool queue wait и blocking exec без потери existing global metrics.
- [ ] 2.2 Вынести из blocking closure structured `QueryBundleTraceReport`, который переносит grouped substage attribution на cancelled/failed path наружу в request-level timeline builder.
- [ ] 2.3 Перевести `impl_completion.rs` на truthful grouped `query_bundle` stage accounting со stage guards, remainder accounting через `query_bundle_other` и total accounting на success/cancel/fail/join-error paths.
- [ ] 2.4 Сохранить bounded cancellation semantics для superseded/cancelled completion: trace показывает truthful query-body stage status, но не выдумывает hard preemption там, где её реально нет.

## 3. Human-readable projection

- [ ] 3.1 Свести verdict builder для Completion Timeline/webview/clipboard/incident bundle к единому bounded source of truth.
- [ ] 3.2 Перевести human-readable projection на canonical verdict vocabulary `query_bundle_dominant` + leaf verdicts и убрать ложный `adapter_before_dispatch_dominant` для query-dominant traces.
- [ ] 3.3 Обновить incident/report consumers и scale-aware summaries так, чтобы grouped query-body taxonomy была canonical также вне panel/webview.
- [ ] 3.4 Зафиксировать graceful degradation для `v19`: old payload остаётся читаемым, но detailed query-body split помечается unavailable by design.

## 4. Validation

- [ ] 4.1 Добавить backend tests, которые доказывают: cancelled request внутри query-body публикует grouped `query_bundle*` stage, а не оставляет seconds-scale tail в одном `unattributed_overhead`.
- [ ] 4.2 Добавить backend tests на carrier path `ObservedBlockingCall + QueryBundleTraceReport`, включая join-error и remainder accounting через `query_bundle_other`.
- [ ] 4.3 Добавить backend tests на request-local split `query_bundle_pool_wait` vs `query_bundle_ir_query`, чтобы saturation и actual compute различались в authoritative trace.
- [ ] 4.4 Добавить extension/tests/report-gates на canonical verdict vocabulary и explicit `v19` degradation для Completion Timeline, clipboard, incident bundle и scale-aware summaries.
- [ ] 4.5 Обновить versioned contract baseline, docs/runbook references и прогнать `openspec validate refactor-completion-query-bundle-root-cause-attribution --strict --no-interactive`.
