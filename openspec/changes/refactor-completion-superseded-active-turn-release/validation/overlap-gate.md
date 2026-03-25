# Проверка overlap-профиля

- Каноническая команда: `CHANGE_ID=refactor-completion-superseded-active-turn-release ./scripts/validate-v2-completion-gates.sh`
- Точечная команда профиля: `CHANGE_ID=refactor-completion-superseded-active-turn-release cargo test -p bsl-backend --bin bsl-lsp-server p40_real_conf_big_same_file_overlap_completion_perf_report_live -- --nocapture`
- Отчёт: `backend/tests/perf/reports/refactor-completion-superseded-active-turn-release-real-conf-big-overlap-completion-perf-live.json`
- Сводка: `backend/tests/perf/reports/refactor-completion-superseded-active-turn-release-real-conf-big-overlap-completion-perf-live.md`
- Итоговый readiness gate: `backend/tests/perf/reports/refactor-completion-superseded-active-turn-release-readiness-gate.md`

## Итог

- Разогревочный same-file completion на representative module остаётся non-empty: `1/1`.
- Measured overlap samples: `5`.
- Старый request завершился `cancelled/superseded` во всех `5/5` измерениях.
- Старый request дал bounded empty response во всех `5/5` измерениях.
- Registry entry старого request очищен во всех `5/5` измерениях.
- Новый request дал non-empty completion во всех `5/5` измерениях.
- Новый request сохранил route attribution: `5` `head_hit`, `0` `exact_hit`.
- `prepare_timeout` delta: `0`.
- `exact_deadline` delta: `0`.
- `cancelled` delta: `5`.
- `fail_closed` delta: `0`.
- `p95(service_future_to_first_poll_wait_ms)=0ms`.
- `max(service_future_to_first_poll_wait_ms)=0ms`.

## Вывод

Overlap profile на реальном модуле подтверждает целевой контракт change:
- superseded active completion boundedly сворачивается;
- newer same-file completion не ждёт stale active turn до first poll;
- fix работает на existing completion path без возврата к seconds-scale pre-poll starvation.
