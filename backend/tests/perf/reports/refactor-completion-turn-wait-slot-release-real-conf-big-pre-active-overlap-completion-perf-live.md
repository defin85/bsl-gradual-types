# refactor-completion-turn-wait-slot-release real-module readiness gate

- profile: `p41_real_conf_big_pre_active_turn_wait_overlap_completion_perf_report_live`
- profile title: `same-file pre-active turn_wait overlap`
- report: `/home/egor/code/bsl-gradual-types/backend/tests/perf/reports/refactor-completion-turn-wait-slot-release-real-conf-big-pre-active-overlap-completion-perf-live.json`
- report change_id: `refactor-completion-turn-wait-slot-release`
- measured samples: `5`
- head_hit traces: `5`
- exact_hit traces: `0`
- prepare_timeout delta: `0`
- exact_deadline delta: `0`
- first cancelled/superseded traces: `5`
- first empty responses: `5`
- first registry cleared: `5`
- second non-empty responses: `5`
`p95(service_future_to_first_poll_wait_ms)=0ms`
`max(service_future_to_first_poll_wait_ms)=0ms`
- first pre-active `turn_wait` ready traces: `5`
- stranded pre-active `turn_wait` samples: `0`
- trace-linked samples: `5`
