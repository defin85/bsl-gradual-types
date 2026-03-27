# stabilize-completion-front-edge readiness gates

## Cross-adapter smoke
- command: `./scripts/run-intellisense-tests.sh smoke`
- pass: yes

## Representative-matrix perf gate
- command: `CHANGE_ID=stabilize-completion-front-edge BSL_V2_PERF_GATE_BLOCKING=1 ./scripts/run-intellisense-perf.sh`
- pass: yes

| profile | pass | matrix entries | failing entries | fail_closed budget failures | reason codes |
|---|---|---:|---:|---:|---|
| small | yes | 21 | 0 | 0 | - |
| large | yes | 21 | 0 | 0 | - |
| churn | yes | 21 | 0 | 0 | - |

## Real-module representative gates
| profile | pass | report change_id | measured samples | head_hit traces | exact_hit traces | prepare_timeout delta | exact_deadline delta |
|---|---|---|---:|---:|---:|---:|---:|
| revision-churn/post-handoff readiness | yes | stabilize-completion-front-edge | 10 | 10 | 0 | 0 | 0 |

## OpenSpec
- command: `openspec validate stabilize-completion-front-edge --strict --no-interactive`
- pass: yes
- log: `/home/egor/code/bsl-gradual-types/backend/tests/perf/reports/stabilize-completion-front-edge-openspec-validate.log`
