## 1. Authoritative Contract
- [x] 1.1 Зафиксировать `v5` contract shape для per-request completion timeline bottleneck drilldown.
- [x] 1.2 Спроектировать bounded prepare drilldown для `wait_for_file_version` и `snapshot_with_deps`, без free-text логов и high-cardinality полей.
- [x] 1.3 Спроектировать bounded exact-wait drilldown для waiter/task state, совместимый с текущим `exact_wait`.
- [x] 1.4 Зафиксировать инварианты ingress/disptacher attribution и fail-open поведение instrumentation.

## 2. Human-Readable Projections
- [x] 2.1 Спроектировать отображение новых drilldown-полей в Completion Timeline panel без raw-only semantics.
- [x] 2.2 Спроектировать clipboard export так, чтобы он переносил ключевые bottleneck facts без необходимости открывать raw JSON.
- [x] 2.3 Спроектировать derived summary для incident handoff (`summary.md` / `incident.json`) поверх authoritative timeline, без отдельного лог-файла.
- [x] 2.4 Зафиксировать graceful degradation для старого backend payload (`v4`) и частично отсутствующих drilldown-полей.

## 3. Validation
- [x] 3.1 Определить backend contract coverage для `v5` payload, bounded vocabulary и fail-open paths.
- [x] 3.2 Определить extension coverage для panel, clipboard и incident summary projections.
- [x] 3.3 Обновить smoke/runbook expectations для нового drilldown handoff.
- [x] 3.4 Провалидировать change через `openspec validate add-completion-bottleneck-drilldown --strict --no-interactive`.
