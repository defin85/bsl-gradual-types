## 1. Implementation
- [ ] 1.1 Introduce a low-latency passive readiness registration path for `wait_for_file_version` or semantically equivalent current-revision waits so requests become observable waiters without sitting for seconds in the generic background writer FIFO first.
- [ ] 1.2 Route interactive completion readiness and didSave heavy follow-up readiness waits through the new passive registration path while preserving current-revision correctness and fail-closed semantics.
- [ ] 1.3 Ensure passive readiness waiting does not consume additional blocking CPU permits and remains observably distinct from actual apply execution backlog.
- [ ] 1.4 Extend observability so request-centric traces and cumulative metrics distinguish waiter-registration latency, passive wait latency, actual apply execution/apply lag, and downstream semantic work.

## 2. Validation
- [ ] 2.1 Add deterministic regressions proving same-file completion no longer times out at `wait_for_file_version` only because waiter registration sat behind unrelated apply backlog.
- [ ] 2.2 Add didSave follow-up regressions proving richer follow-up can register its readiness wait without inheriting raw seconds-scale generic runtime FIFO residency before it becomes a waiter.
- [ ] 2.3 Re-run representative completion mixed-load and didSave follow-up live gates so apply-backlog and readiness-registration failure classes remain separated.
- [ ] 2.4 Capture representative evidence showing `wait_for_file_version` queue-wait tails drop independently from actual apply execution/apply lag tails.
- [ ] 2.5 Run `openspec validate refactor-12-runtime-ready-waiter-contention-bounding --strict --no-interactive`.
