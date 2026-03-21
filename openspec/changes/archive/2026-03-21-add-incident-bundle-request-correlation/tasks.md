## 1. Контракт derived report
- [x] 1.1 Зафиксировать request-centric incident report contract для `uri`, `request_count` и bounded request list.
- [x] 1.2 Зафиксировать политику authoritative source: request list строится от server timeline, а probes только дополняют его.
- [x] 1.3 Зафиксировать явную политику для unavailable/unsupported/ambiguous correlation без invented data.

## 2. Correlation и summary semantics
- [x] 2.1 Спроектировать deterministic правила optional probe-to-trace correlation.
- [x] 2.2 Спроектировать bounded per-request latency/verdict projection для `incident.json`.
- [x] 2.3 Спроектировать compact request-centric section для `summary.md`.
- [x] 2.4 Зафиксировать non-goal для `metrics delta` и single-snapshot semantics.

## 3. Проверка и фиксация
- [x] 3.1 Добавить extension tests для single-uri request summary, prepare/exact findings и ambiguous correlation.
- [x] 3.2 Обновить smoke/runbook expectations для richer `incident.json` и `summary.md`.
- [x] 3.3 Зафиксировать `Requirement -> Code -> Test` traceability для request-centric bundle report.
- [x] 3.4 Провалидировать change через `openspec validate add-incident-bundle-request-correlation --strict --no-interactive`.
