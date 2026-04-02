# Проверка pre-active overlap-профиля

- Канонический wrapper: `./scripts/validate-completion-turn-wait-lifecycle.sh`
- Нижележащая generic команда с явным override: `CHANGE_ID=refactor-completion-turn-wait-lifecycle ./scripts/validate-v2-completion-gates.sh`
- Revision-churn companion profile: `CHANGE_ID=refactor-completion-turn-wait-lifecycle cargo test -p bsl-backend --bin bsl-lsp-server p38_real_conf_big_revision_churn_completion_perf_report_live -- --nocapture`
- Точечная команда pre-active overlap-профиля: `CHANGE_ID=refactor-completion-turn-wait-lifecycle cargo test -p bsl-backend --bin bsl-lsp-server p41_real_conf_big_pre_active_turn_wait_overlap_completion_perf_report_live -- --nocapture`
- Отчёт профиля: `backend/tests/perf/reports/refactor-completion-turn-wait-lifecycle-real-conf-big-pre-active-overlap-completion-perf-live.json`
- Checked-in summary профиля: `backend/tests/perf/reports/refactor-completion-turn-wait-lifecycle-real-conf-big-pre-active-overlap-completion-perf-live.md`
- Итоговый readiness gate: `backend/tests/perf/reports/refactor-completion-turn-wait-lifecycle-readiness-gate.md`

## Итог

- Разогревочный same-file completion на representative module остаётся non-empty: `1/1`.
- Measured pre-active overlap samples: `5`.
- Старый request завершился `cancelled/superseded` во всех `5/5` измерениях.
- Старый request дал bounded empty response во всех `5/5` измерениях.
- Registry entry старого request очищен во всех `5/5` измерениях.
- `turn_wait` older request boundedly резолвится до active registration во всех `5/5` измерениях.
- Stranded pre-active `turn_wait` samples: `0/5`.
- Новый request дал non-empty completion во всех `5/5` измерениях.
- Новый request сохранил route attribution: `5` `head_hit`, `0` `exact_hit`.
- `prepare_timeout` delta: `0`.
- `exact_deadline` delta: `0`.
- `cancelled` delta: `5`.
- `fail_closed` delta: `0`.
- `p95(service_future_to_first_poll_wait_ms)=0ms`.
- `max(service_future_to_first_poll_wait_ms)=0ms`.

## Вывод

Pre-active overlap profile на реальном модуле подтверждает целевой контракт change:
- same-file completion в `turn_wait` не становится orphaned между queue exit и active registration;
- newer same-file completion не ждёт stranded pre-active predecessor до first poll;
- fix остаётся локальным completion lifecycle hardening на existing path и не требует transport-priority workaround или общего scheduler redesign.
