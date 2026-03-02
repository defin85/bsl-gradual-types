# Dry-run: small/large/churn

Date: March 2, 2026

## Commands

```bash
# baseline refresh after enabling deterministic churn profile
UPDATE_BASELINE=1 PERF_WARMUP=1 PERF_ITERATIONS=5 THRESHOLD_P95=50 THRESHOLD_P99=50 THRESHOLD_RESOURCE=50 PERF_PROFILES="small large churn" ./scripts/run-intellisense-perf.sh

# reproducibility runs
PERF_WARMUP=1 PERF_ITERATIONS=5 THRESHOLD_P95=50 THRESHOLD_P99=50 THRESHOLD_RESOURCE=50 PERF_PROFILES="small large churn" ./scripts/run-intellisense-perf.sh
PERF_WARMUP=1 PERF_ITERATIONS=5 THRESHOLD_P95=50 THRESHOLD_P99=50 THRESHOLD_RESOURCE=50 PERF_PROFILES="small large churn" ./scripts/run-intellisense-perf.sh
```

## Dry-run #1

- small: verdict=`pass`, reason_codes=`[]`, p95=`0.251119ms`, p99=`0.483235ms`, ratio_p95=`2.079146`, ratio_p99=`3.415078`
- large: verdict=`pass`, reason_codes=`[]`, p95=`5.554810ms`, p99=`5.642816ms`, ratio_p95=`0.917005`, ratio_p99=`0.925415`
- churn: verdict=`pass`, reason_codes=`[]`, p95=`22.052037ms`, p99=`24.303614ms`, ratio_p95=`1.089641`, ratio_p99=`1.163535`

## Dry-run #2

- small: verdict=`pass`, reason_codes=`[]`, p95=`0.132210ms`, p99=`0.149811ms`, ratio_p95=`1.095242`, ratio_p99=`1.055470`
- large: verdict=`pass`, reason_codes=`[]`, p95=`5.408089ms`, p99=`5.537298ms`, ratio_p95=`0.892788`, ratio_p99=`0.907282`
- churn: verdict=`pass`, reason_codes=`[]`, p95=`19.615911ms`, p99=`19.875630ms`, ratio_p95=`0.969267`, ratio_p99=`0.951536`

## Reproducibility verdict

- Verdict/reason-codes are stable across repeated dry-runs for all three profiles (`pass`, empty reason-codes).
- Profiles are distinguishable after churn instrumentation:
  - `small` remains low-latency/low-resource (`p95 < 1ms`, allocations `~435`).
  - `large` is medium (`p95 ~5.4-5.6ms`, allocations `~17533`).
  - `churn` is the highest and materially different (`p95 ~19.6-22.1ms`, allocations `~81483`, lock_wait `~13ms`).
