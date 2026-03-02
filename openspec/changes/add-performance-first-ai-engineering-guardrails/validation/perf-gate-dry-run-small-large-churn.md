# Dry-run: small/large/churn

Date: March 2, 2026

## Command

```bash
UPDATE_BASELINE=1 PERF_WARMUP=1 PERF_ITERATIONS=5 THRESHOLD_P95=50 THRESHOLD_P99=50 THRESHOLD_RESOURCE=50 PERF_PROFILES="small large churn" ./scripts/run-intellisense-perf.sh
PERF_WARMUP=1 PERF_ITERATIONS=5 THRESHOLD_P95=50 THRESHOLD_P99=50 THRESHOLD_RESOURCE=50 PERF_PROFILES="small large churn" ./scripts/run-intellisense-perf.sh
PERF_WARMUP=1 PERF_ITERATIONS=5 THRESHOLD_P95=50 THRESHOLD_P99=50 THRESHOLD_RESOURCE=50 PERF_PROFILES="small large churn" ./scripts/run-intellisense-perf.sh
```

## Dry-run #1

- small: verdict=`pass`, reason_codes=`[]`, ratio_p95=`0.624426`, ratio_p99=`0.632759`
- large: verdict=`pass`, reason_codes=`[]`, ratio_p95=`1.477986`, ratio_p99=`1.478944`
- churn: verdict=`pass`, reason_codes=`[]`, ratio_p95=`1.734161`, ratio_p99=`1.724571`

## Dry-run #2

- small: verdict=`pass`, reason_codes=`[]`, ratio_p95=`0.681426`, ratio_p99=`0.714421`
- large: verdict=`pass`, reason_codes=`[]`, ratio_p95=`0.977546`, ratio_p99=`0.999755`
- churn: verdict=`pass`, reason_codes=`[]`, ratio_p95=`1.048908`, ratio_p99=`1.034840`

## Reproducibility verdict

- Verdict/reason-codes are stable across repeated dry-runs for all three profiles.
