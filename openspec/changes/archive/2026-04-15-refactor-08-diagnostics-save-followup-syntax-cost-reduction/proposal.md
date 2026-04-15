# Change: Reduce didSave follow-up syntax cost

## Why

Live bundle `bsl-observability-incident-2026-04-07T23-20-28Z` shows that `didSave`
`save_fastlane` is already bounded, but the remaining `idle_heavy` tail on `conf_big`
is now dominated by full follow-up syntax work rather than by hidden queueing.

## What Changes

- Rework `didSave + idle_heavy` follow-up so it avoids redoing expensive full-file syntax work
  when same-version syntax artifacts already exist.
- Keep request-centric diagnostics save timeline truthful for the optimized follow-up path.
- Add regression and live validation on representative `conf_big` save flow.

## Impact

- Affected specs: `bsl-intellisense`, `bsl-intellisense-v2`
- Affected code: diagnostics runtime, save follow-up path, observability timeline, diagnostics
  regression/perf validation
