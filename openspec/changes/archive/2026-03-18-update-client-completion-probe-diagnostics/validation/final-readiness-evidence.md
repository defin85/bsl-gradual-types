# Final Readiness Evidence (update-client-completion-probe-diagnostics)

## Scope
- change_id: `update-client-completion-probe-diagnostics`
- archived_change_dir: `openspec/changes/archive/2026-03-18-update-client-completion-probe-diagnostics`
- date (UTC): `2026-03-18T21:34:38Z`

## Review Closure
- Shipped extension smoke path now covers `Completion Probe Runtime Test Suite`, so the transport hook and selection observer are exercised in the same focused slice as `Completion Timeline` / `Client Probe Feed`.
- Repository docs/runbook now use the same focused smoke command as `./scripts/run-intellisense-tests.sh smoke`, including `Completion Probe Runtime`.
- Checked-in readiness evidence now exists under this archived change for lint, focused extension tests, smoke asset regression tests, shell syntax validation, and repo-wide OpenSpec strict validation.
- While closing the readiness gap, the readiness asset regression test was also fixed to read `quality-gates.json` from the archived `refactor-ir-canonical-semantic-pipeline` change path instead of the removed pre-archive path.

## Verification Evidence
- `npm --prefix vscode-extension run compile:fast` -> `ok`
- `cd vscode-extension && BSL_TEST_GREP='Completion Probe (Schema|Recorder|Runtime|Store) Test Suite|Completion Timeline (Clipboard|Model|Webview Provider) Test Suite|Client Options Test Suite|getCompletionTimeline should work via executeCommand|getCompletionTimeline should fail-closed on Method not found' node ./out/test/runTest.js` -> `31 passing`
- `npm --prefix vscode-extension run lint` -> `ok`
- `python3 scripts/test-intellisense-readiness-assets.py` -> `4 tests, ok`
- `bash -n scripts/run-intellisense-tests.sh` -> `ok`
- `openspec validate --all --strict --no-interactive` -> `Totals: 17 passed, 0 failed (17 items)`

## OpenSpec Note
- После архивирования change CLI больше не валидирует archived change по старому active-change ID.
- Для post-archive readiness используется repo-wide strict validation: `openspec validate --all --strict --no-interactive`.
