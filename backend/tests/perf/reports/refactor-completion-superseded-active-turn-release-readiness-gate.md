# refactor-completion-superseded-active-turn-release readiness gates

## Cross-adapter smoke
- command: `./scripts/run-intellisense-tests.sh smoke`
- pass: yes

## Representative-matrix perf gate
- command: `CHANGE_ID=refactor-completion-superseded-active-turn-release BSL_V2_PERF_GATE_BLOCKING=1 ./scripts/run-intellisense-perf.sh`
- pass: yes

| profile | pass | matrix entries | failing entries | fail_closed budget failures | reason codes |
|---|---|---:|---:|---:|---|
| small | yes | 21 | 0 | 0 | - |
| large | yes | 21 | 0 | 0 | - |
| churn | yes | 21 | 0 | 0 | - |

## Real-module representative gates
| profile | pass | report change_id | measured samples | head_hit traces | exact_hit traces | prepare_timeout delta | exact_deadline delta |
|---|---|---|---:|---:|---:|---:|---:|
| revision-churn/post-handoff readiness | yes | refactor-completion-superseded-active-turn-release | 10 | 10 | 0 | 0 | 0 |
| same-file overlap supersession | yes | refactor-completion-superseded-active-turn-release | 5 | 5 | 0 | 0 | 0 |

## OpenSpec
- command: `openspec validate refactor-completion-superseded-active-turn-release --strict --no-interactive`
- pass: yes
- log: `/home/egor/code/bsl-gradual-types/backend/tests/perf/reports/refactor-completion-superseded-active-turn-release-openspec-validate.log`
