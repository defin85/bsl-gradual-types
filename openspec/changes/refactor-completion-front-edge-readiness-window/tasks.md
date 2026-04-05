## 1. Specification
- [ ] 1.1 Добавить в `bsl-intellisense-v2` contract, что immediate same-file post-edit/save completion window не должна завершаться `prepare_timeout`, если current-revision handoff уже зарегистрирован и truthful transport seams healthy.
- [ ] 1.2 Добавить в `bsl-intellisense-v2` representative front-edge acceptance, которая разводит front-edge `prepare_timeout` и cold `query_bundle_pool_wait`.

## 2. Design
- [ ] 2.1 Зафиксировать front-edge readiness window как отдельную boundary между handoff и fully prepared current-revision snapshot path.
- [ ] 2.2 Зафиксировать, что `prepare_timeout@snapshot_with_deps` сразу после handoff относится к тому же regression surface, что и `prepare_timeout@wait_for_file_version`.
- [ ] 2.3 Зафиксировать, что cold `query_bundle_pool_wait` после успешного readiness остаётся отдельным follow-up и не входит в remediation target.

## 3. Implementation
- [ ] 3.1 Устранить backend path, из-за которого первые same-file completion сразу после `didChange/didSave` всё ещё уходят в `prepare_timeout` при healthy truthful transport seams.
- [ ] 3.2 Обновить representative live gate/report так, чтобы front-edge profile fail-ил на любом `prepare_timeout`, а `query_bundle_pool_wait` report-ился отдельным diagnostic bucket.
- [ ] 3.3 Сохранить strict current-revision semantics: без stale substitute, без ослабления fail-closed policy и без regression truthful ingress/egress seams.

## 4. Validation
- [ ] 4.1 Провалидировать change: `openspec validate refactor-completion-front-edge-readiness-window --strict --no-interactive`.
- [ ] 4.2 Прогнать representative live gate для real `conf_big` immediate post-edit/save front-edge профиля и подтвердить отсутствие `prepare_timeout`.
- [ ] 4.3 Переснять incident bundle и подтвердить, что front-edge readiness regressions исчезли, а residual `query_bundle_pool_wait` остаётся отдельным диагностическим bucket.
