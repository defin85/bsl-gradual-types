# Agent Verification Runbook

## Prerequisites

- Rust toolchain для workspace команд
- Node.js для `vscode-extension/`
- `cargo`, `python3`, `npm`
- `trunk` нужен для frontend/WASM сборок и для `./scripts/run-intellisense-tests.sh smoke`, если `target/site/` ещё не собран
- Syntax Helper и config dump нужны только для сценариев, где они явно указаны

## Runtime Surface Smoke

Эти команды проверяют, что живые бинарные surfaces существуют и стартуют с корректным CLI contract.

```bash
cargo run -p bsl-cli -- --help
cargo run -p bsl-backend --bin bsl-web-server -- --help
cargo run -p bsl-backend --bin bsl-lsp-server -- --help
cargo run -p bsl-agent -- --help
```

Expected outcome:

- каждая команда завершается `0`
- в help output используются текущие names `bsl-cli`, `bsl-web-server`, `bsl-lsp-server`, `bsl-agent`

## Default Smoke Path

```bash
./scripts/run-agent-readiness-checks.sh

python3 -m unittest \
  scripts/test-agent-readiness.py \
  scripts/test-intellisense-smoke-gate.py \
  scripts/test-intellisense-readiness-assets.py \
  scripts/test-ci-openspec-governance-workflow.py

./scripts/run-intellisense-tests.sh smoke
```

Expected outcome:

- canonical agent docs, instruction layering и onboarding commands проходят fail-closed validation
- shipped smoke selectors и readiness assets согласованы
- shipped completion supersession smoke retains branch-level `p33_same_file_completion_supersession_releases_active_turn_at_format_checkpoint` proof alongside the response-build regression
- cross-adapter smoke suite проходит без внешних фикстур
- если `target/site/index.html` отсутствует, smoke script сам собирает embedded `bsl-agent` UI assets через `trunk build --release`

## Manual Or Broader Validation

- VS Code extension compile/lint/tests:

```bash
npm --prefix ./vscode-extension run compile:fast
npm --prefix ./vscode-extension run lint
npm --prefix ./vscode-extension test
```

- Frontend/WASM build:

```bash
(cd frontend && NO_COLOR=true trunk build --release)
```

- Workspace-wide Rust verification:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

## Heavy Or Readiness Gates

- Canonical perf/readiness path:

```bash
./scripts/run-intellisense-perf.sh
```

- Active checked-in readiness bundle for current-revision completion changes:

```bash
./scripts/validate-v2-completion-gates.sh
```

- Change-specific checked-in evidence bundles:

```bash
./scripts/validate-document-symbol-interactive-isolation.sh
./scripts/validate-completion-superseded-active-turn-release.sh
./scripts/validate-completion-turn-wait-slot-release.sh
./scripts/validate-completion-turn-wait-lifecycle.sh
CHANGE_ID=refactor-completion-prepare-lightweight-exact-split ./scripts/validate-v2-completion-gates.sh
CHANGE_ID=refactor-completion-superseded-active-turn-release ./scripts/validate-v2-completion-gates.sh
CHANGE_ID=refactor-completion-turn-wait-slot-release ./scripts/validate-v2-completion-gates.sh
CHANGE_ID=refactor-completion-turn-wait-lifecycle ./scripts/validate-v2-completion-gates.sh
```

`./scripts/validate-document-symbol-interactive-isolation.sh` is the canonical
default entry point for the document-symbol isolation evidence bundle; it wraps
the generic readiness script with the correct `CHANGE_ID`.
`./scripts/validate-completion-superseded-active-turn-release.sh` is the
canonical default entry point for the overlap supersession evidence bundle; it
wraps the generic readiness script with the correct `CHANGE_ID` and collects
both change-specific representative profiles (`churn` and `overlap`).
`./scripts/validate-completion-turn-wait-slot-release.sh` is the canonical
default entry point for the completion transport-slot-release evidence bundle;
it wraps the generic readiness script with the correct `CHANGE_ID` and collects
both change-specific representative profiles (`churn` and `preactive_overlap`).
Targeted lifecycle safety for this handoff path is covered by the backend tests
`transport_adapter_emits_single_terminal_response_for_handoff_cancel_race` and
`transport_adapter_aborts_blocked_completion_handoff_on_transport_shutdown`.
`./scripts/validate-completion-turn-wait-lifecycle.sh` is the canonical
default entry point for the pre-active `turn_wait` lifecycle evidence bundle;
it wraps the generic readiness script with the correct `CHANGE_ID` and collects
both change-specific representative profiles (`churn` and `preactive_overlap`).

Use these only when the task explicitly needs readiness/perf evidence.
