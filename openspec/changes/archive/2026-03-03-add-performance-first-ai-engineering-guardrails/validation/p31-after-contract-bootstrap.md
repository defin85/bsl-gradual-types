# p31 after contract bootstrap

Date: March 2, 2026

Passing evidence after contract-driven baseline bootstrap:

- bootstrap command: `UPDATE_BASELINE=1 PERF_WARMUP=1 PERF_ITERATIONS=5 THRESHOLD_P95=50 THRESHOLD_P99=50 THRESHOLD_RESOURCE=50 PERF_PROFILES=\"small large churn\" ./scripts/run-intellisense-perf.sh`
- validation commands (x2): `PERF_WARMUP=1 PERF_ITERATIONS=5 THRESHOLD_P95=50 THRESHOLD_P99=50 THRESHOLD_RESOURCE=50 PERF_PROFILES=\"small large churn\" ./scripts/run-intellisense-perf.sh`
- all profiles `small|large|churn` returned `verdict=pass` with empty `reason_codes`.
