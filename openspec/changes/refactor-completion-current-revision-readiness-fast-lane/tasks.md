## 1. Specification
- [ ] 1.1 Добавить в `bsl-intellisense-v2` contract, что same-file save-triggered auxiliary churn не должна возвращать completion к `prepare_timeout@wait_for_file_version` после current-revision handoff.
- [ ] 1.2 Добавить в `bsl-intellisense-v2` representative post-edit/save churn acceptance, которая разводит readiness regression и cold `query_bundle_ir_query`.

## 2. Design
- [ ] 2.1 Описать границу между current-revision readiness remediation и long-term detached snapshot architecture.
- [ ] 2.2 Зафиксировать, что truthful transport seams считаются prerequisite evidence, а не текущим remediation target.
- [ ] 2.3 Зафиксировать acceptance distinction между `prepare_timeout@wait_for_file_version` и latency после успешного readiness.

## 3. Implementation
- [ ] 3.1 Устранить backend path, из-за которого same-file completion после edit/save churn повторно уходит в `prepare_timeout@wait_for_file_version` при healthy truthful transport seams.
- [ ] 3.2 Обновить representative live gate/report так, чтобы post-edit/save churn profile fail-ил на readiness timeout отдельно от cold `query_bundle_ir_query`.
- [ ] 3.3 Сохранить strict current-revision semantics: без stale substitute, без переопределения `applied_version`, без regression truthful ingress/egress seams.

## 4. Validation
- [ ] 4.1 Провалидировать change: `openspec validate refactor-completion-current-revision-readiness-fast-lane --strict --no-interactive`.
- [ ] 4.2 Прогнать representative live gate для real `conf_big` same-file churn/completion профиля и подтвердить отсутствие `prepare_timeout@wait_for_file_version`.
- [ ] 4.3 Переснять incident bundle и подтвердить, что readiness regressions исчезли или стали отдельными от cold `query_bundle_ir_query` samples.
