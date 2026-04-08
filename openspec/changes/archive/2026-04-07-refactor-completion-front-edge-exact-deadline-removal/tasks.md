## 1. Specification
- [x] 1.1 Добавить в `bsl-intellisense-v2` contract, что hidden `exact_deadline` в immediate same-file post-edit/save window не считается допустимым bounded outcome при healthy truthful transport seams.
- [x] 1.2 Уточнить representative front-edge acceptance так, чтобы gate требовал хотя бы один successful current-revision sample и отдельно report-ил cold `query_bundle_pool_wait` только после успешного readiness.

## 2. Design
- [x] 2.1 Зафиксировать, что `wait_exact_type_index=deadline` после зарегистрированного same-file handoff относится к тому же unresolved front-edge regression surface, а не к generic availability miss.
- [x] 2.2 Зафиксировать, что all-fail-closed measured profile не закрывает remediation intent.
- [x] 2.3 Зафиксировать durable evidence contract для checked-in front-edge artifacts.

## 3. Implementation
- [x] 3.1 Устранить backend path, из-за которого immediate same-file front-edge completion всё ещё уходит в `wait_exact_type_index=deadline` вместо truthful successful current-revision sample.
- [x] 3.2 Сделать fail-closed attribution truthful: front-edge exact wait exhaustion не должна маскироваться под generic `missing_semantic_index` без explicit regression signal.
- [x] 3.3 Обновить representative gate/report так, чтобы green state требовал хотя бы один successful sample и zero hidden `exact_deadline` в measured front-edge window.

## 4. Validation
- [x] 4.1 Провалидировать change: `openspec validate refactor-completion-front-edge-exact-deadline-removal --strict --no-interactive`.
- [x] 4.2 Прогнать front-edge representative live gate и подтвердить: `prepare_timeout=0`, `exact_deadline=0`, `measured_successful_traces>0`.
- [x] 4.3 Зафиксировать durable checked-in evidence, где отдельно видны successful front-edge traces и residual `query_bundle_pool_wait` bucket, если он остаётся.
