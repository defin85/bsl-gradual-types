# Final Readiness Evidence (extension completion probe monitoring)

## Scope
- change_id: `add-extension-completion-probe-monitoring`
- archived_change_dir: `openspec/changes/archive/2026-03-18-add-extension-completion-probe-monitoring`
- date (UTC): `2026-03-18T15:43:27Z`

## Review Closure
- Default `LanguageClient` completion middleware now records `client_terminal_state=error` for non-cancelled exceptions instead of collapsing them into `ok_empty`.
- Shipped smoke path now includes a focused extension-host slice for `Completion Timeline` / `Client Probe Feed`.
- Acceptance commands for lint/tests/OpenSpec validation are now recorded as checked-in evidence under this archived change.

## Verification Evidence
- `npm --prefix vscode-extension run lint` -> `ok`
- `npm --prefix vscode-extension run compile:fast && (cd vscode-extension && BSL_TEST_GREP='Completion Probe (Schema|Recorder|Store) Test Suite|Completion Timeline (Clipboard|Model|Webview Provider) Test Suite|Client Options Test Suite|getCompletionTimeline should work via executeCommand|getCompletionTimeline should fail-closed on Method not found' node ./out/test/runTest.js)` -> `26 passing`
- `npm --prefix vscode-extension run compile:fast && (cd vscode-extension && node ./out/test/runTest.js)` -> `170 passing`
- `python3 -c "import importlib.util, pathlib; p=pathlib.Path('scripts/test-intellisense-readiness-assets.py'); spec=importlib.util.spec_from_file_location('readiness_assets', p); m=importlib.util.module_from_spec(spec); spec.loader.exec_module(m); case=m.IntellisenseReadinessAssetsTest('test_shipped_smoke_script_covers_extension_completion_observability_slice'); result=case.defaultTestResult(); case.run(result); print('failures=', len(result.failures), 'errors=', len(result.errors)); raise SystemExit(1 if result.failures or result.errors else 0)"` -> `failures= 0 errors= 0`
- `bash -n scripts/run-intellisense-tests.sh` -> `ok`
- `openspec validate --all --strict --no-interactive` -> `Totals: 17 passed, 0 failed (17 items)`

## OpenSpec Note
- После архивирования change CLI больше не разрешает `openspec validate add-extension-completion-probe-monitoring --strict --no-interactive` по старому active-change ID (`Unknown item 'add-extension-completion-probe-monitoring'`).
- Для post-archive readiness используется repo-wide strict validation: `openspec validate --all --strict --no-interactive`.
