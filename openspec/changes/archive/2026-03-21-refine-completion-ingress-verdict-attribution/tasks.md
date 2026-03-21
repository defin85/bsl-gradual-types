## 1. Контракт truthful ingress verdicts
- [x] 1.1 Зафиксировать conservative positive-dominance semantics для ingress и handler-prelude verdicts.
- [x] 1.2 Зафиксировать fail-closed правило: без deterministic correlation client-side ingress verdict не появляется.
- [x] 1.3 Зафиксировать truthful aggregation semantics для request-centric incident summary и findings.

## 2. Projection и wiring
- [x] 2.1 Обновить shared drilldown verdict helper и mirrored webview projection до одной ingress vocabulary.
- [x] 2.2 Обновить request-centric incident bundle projection для client/server ingress verdicts без переоценки hot traces.
- [x] 2.3 Сохранить существующие raw latency lines и partial-export semantics без нового API/contract bump.

## 3. Проверка и фиксация
- [x] 3.1 Добавить extension tests для hot path без ingress verdict, client-before-transport dominance, server-before-method dominance и uncorrelated fail-closed path.
- [x] 3.2 Обновить smoke/runbook expectations для новой ingress vocabulary и truthful findings.
- [x] 3.3 Зафиксировать `Requirement -> Code -> Test` traceability для truthful ingress attribution.
- [x] 3.4 Провалидировать change через `openspec validate refine-completion-ingress-verdict-attribution --strict --no-interactive`.
