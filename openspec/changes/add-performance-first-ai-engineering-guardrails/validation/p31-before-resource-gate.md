# p31 before resource gate

Date: March 2, 2026

Failing evidence captured before bootstrap baseline refresh:

- command: `PERF_WARMUP=1 PERF_ITERATIONS=5 THRESHOLD_P95=5 THRESHOLD_P99=5 THRESHOLD_RESOURCE=5 PERF_PROFILES=\"small large churn\" ./scripts/run-intellisense-perf.sh`
- failure verdict: `fail`
- reason_codes: `latency_relative_ratio_exceeded`
