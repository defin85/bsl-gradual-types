# Проверка same-file overlap gate для transport slot release

- Канонический wrapper: `./scripts/validate-completion-turn-wait-slot-release.sh`
- Нижележащая generic команда с явным override: `CHANGE_ID=refactor-completion-turn-wait-slot-release ./scripts/validate-v2-completion-gates.sh`
- Revision-churn companion profile: `CHANGE_ID=refactor-completion-turn-wait-slot-release cargo test -p bsl-backend --bin bsl-lsp-server p38_real_conf_big_revision_churn_completion_perf_report_live -- --nocapture`
- Точечная команда pre-active overlap-профиля: `CHANGE_ID=refactor-completion-turn-wait-slot-release cargo test -p bsl-backend --bin bsl-lsp-server p41_real_conf_big_pre_active_turn_wait_overlap_completion_perf_report_live -- --nocapture`
- Отчёт профиля: `backend/tests/perf/reports/refactor-completion-turn-wait-slot-release-real-conf-big-pre-active-overlap-completion-perf-live.json`
- Checked-in summary профиля: `backend/tests/perf/reports/refactor-completion-turn-wait-slot-release-real-conf-big-pre-active-overlap-completion-perf-live.md`
- Итоговый readiness gate: `backend/tests/perf/reports/refactor-completion-turn-wait-slot-release-readiness-gate.md`

## Итог

- Разогревочный same-file completion на representative module остаётся non-empty: `1/1`.
- Measured pre-active overlap samples: `5`.
- Старый request завершился `cancelled/superseded` во всех `5/5` измерениях.
- Старый request дал bounded empty response во всех `5/5` измерениях.
- Registry entry старого request очищен во всех `5/5` измерениях.
- `turn_wait` older request boundedly резолвится до active registration во всех `5/5` измерениях.
- Stranded pre-active `turn_wait` samples: `0/5`.
- Новый request дал non-empty completion во всех `5/5` измерениях.
- Transport slot release recorded for newer request во всех `5/5` измерениях.
- Slot release happened before passive `turn_wait` во всех `5/5` измерениях.
- Новый request сохранил route attribution: `5` `head_hit`, `0` `exact_hit`.
- `prepare_timeout` delta: `0`.
- `exact_deadline` delta: `0`.
- `cancelled` delta: `5`.
- `fail_closed` delta: `0`.
- `p95(transport_to_slot_release_wait_ms)=0ms`.
- `max(transport_to_slot_release_wait_ms)=0ms`.
- `p95(service_future_to_first_poll_wait_ms)=0ms`.
- `max(service_future_to_first_poll_wait_ms)=0ms`.

## Вывод

Same-file overlap profile на реальном модуле подтверждает целевой контракт change:

- current completion освобождает transport slot до длительного passive `turn_wait` за older same-file owner;
- same-file overlap не превращает current request в seconds-scale pre-first-poll backlog;
- evidence позволяет отличить transport-slot regression от stale pre-active contender и от обычного latest-wins/cancel cleanup.
