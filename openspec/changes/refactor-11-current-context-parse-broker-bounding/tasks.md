## 1. Implementation
- [ ] 1.1 Introduce a server-owned latest-only parse broker for `bsl.getCurrentContext` before `spawn_bounded_blocking`, keyed by same-file same-revision/text identity and allowing at most one leader parse/context derivation per key.
- [ ] 1.2 Route ready-snapshot hits, broker-leader execution, broker-follower waiting, superseded generations, and bounded empty over-budget outcomes through one explicit current-context contract without returning stale context for newer generations.
- [ ] 1.3 Ensure follower requests no longer acquire independent blocking CPU permits just to wait behind the same leader parse, while preserving reusable artifact warmup for later requests.
- [ ] 1.4 Export dedicated observability for broker role/outcome and wall/parse latency so incident bundles can distinguish `ready_snapshot`, leader parse, follower wait, supersession, and budget exhaustion.

## 2. Validation
- [ ] 2.1 Add deterministic regressions proving concurrent same-key `bsl.getCurrentContext` bursts share one leader parse instead of spawning multiple blocking parse holders.
- [ ] 2.2 Add supersession regressions proving obsolete generations are discarded or coalesced before independent expensive parse work starts.
- [ ] 2.3 Add mixed-load regressions proving `bsl.getCurrentContext` bursts no longer create extra blocking CPU contention for same-file completion beyond one leader parse.
- [ ] 2.4 Capture representative observability evidence showing current-context requests shift from repeated parser-coordinator parse holders toward ready-snapshot or brokered leader/follower outcomes.
- [ ] 2.5 Run `openspec validate refactor-11-current-context-parse-broker-bounding --strict --no-interactive`.
