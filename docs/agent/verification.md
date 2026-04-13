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

## Local GitHub Actions Replica

Когда нужно прогнать тот же workflow локально до облачного runner, используй
repo-local wrapper над `act`:

```bash
./scripts/run-local-ci-with-act.sh list
./scripts/run-local-ci-with-act.sh agent_readiness_docs_gate
./scripts/run-local-ci-with-act.sh intellisense_smoke_gate --offline
./scripts/run-local-ci-with-act.sh du
```

Expected outcome:

- workflow `./.github/workflows/ci.yml` исполняется через `workflow_dispatch`
- replica покрывает hosted CI path; self-hosted `conf_big` representative gates живут отдельно в `./.github/workflows/intellisense-real-module-gates.yml`
- тяжёлые Rust/npm caches, `CARGO_TARGET_DIR`, `vscode-extension/node_modules` и `.vscode-test` уходят в Docker named volumes
- IntelliSense smoke автоматически использует локальный act runner image с Linux runtime libs и `xvfb` для VS Code
- extension-host tail запускается через `./scripts/run-vscode-extension-tests.js`, который fail-closed уводит VS Code под `xvfb-run` внутри WSL и в Linux headless без `DISPLAY`/`WAYLAND_DISPLAY`
- логи и uploaded artifacts складываются в repo-local ignored cache directory и
  автоматически подчищаются по retention policy, чтобы локальное хранилище не
  раздувалось

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

- Manual self-hosted representative gates for `conf_big` fixtures:

```bash
# GitHub Actions -> IntelliSense Real-Module Gates
# workflow input: conf_big_root=/absolute/path/to/conf_big
```

Expected outcome:

- workflow `./.github/workflows/intellisense-real-module-gates.yml` запускается только вручную на self-hosted runner с label `conf-big`
- runner передаёт absolute fixture path через `BSL_TEST_CONF_BIG_ROOT`
- hosted `./.github/workflows/ci.yml` не зависит от `examples/conf_big`

- Active checked-in readiness bundle for current-revision completion changes:

```bash
./scripts/validate-v2-completion-gates.sh
```

- Change-specific checked-in evidence bundles:

```bash
./scripts/validate-document-symbol-interactive-isolation.sh
./scripts/validate-isolate-completion-pre-dispatch-ingress.sh
./scripts/validate-completion-superseded-active-turn-release.sh
./scripts/validate-stabilize-completion-front-edge.sh
./scripts/validate-completion-turn-wait-slot-release.sh
./scripts/validate-completion-turn-wait-lifecycle.sh
CHANGE_ID=refactor-completion-prepare-lightweight-exact-split ./scripts/validate-v2-completion-gates.sh
CHANGE_ID=isolate-completion-pre-dispatch-ingress ./scripts/validate-v2-completion-gates.sh
CHANGE_ID=refactor-completion-superseded-active-turn-release ./scripts/validate-v2-completion-gates.sh
CHANGE_ID=refactor-completion-front-edge-exact-deadline-removal ./scripts/validate-v2-completion-gates.sh
CHANGE_ID=stabilize-completion-front-edge ./scripts/validate-v2-completion-gates.sh
CHANGE_ID=refactor-completion-turn-wait-slot-release ./scripts/validate-v2-completion-gates.sh
CHANGE_ID=refactor-completion-turn-wait-lifecycle ./scripts/validate-v2-completion-gates.sh
```

`./scripts/validate-document-symbol-interactive-isolation.sh` is the canonical
default entry point for the document-symbol isolation evidence bundle; it wraps
the generic readiness script with the correct `CHANGE_ID`.
If the referenced OpenSpec change has already been archived, the wrapper falls
back to `openspec validate --all --strict --no-interactive`, because the CLI no
longer resolves the archived change by its old active `change-id`.
`./scripts/validate-isolate-completion-pre-dispatch-ingress.sh` is the
canonical default entry point for the truthful pre-dispatch ingress evidence
bundle; it wraps the generic readiness script with the correct `CHANGE_ID` and
forces the representative `outline` profile that fails on
`adapter_to_dispatch_wait_ms` budgets.
`./scripts/validate-completion-superseded-active-turn-release.sh` is the
canonical default entry point for the overlap supersession evidence bundle; it
wraps the generic readiness script with the correct `CHANGE_ID` and collects
both change-specific representative profiles (`churn` and `overlap`).
`./scripts/validate-stabilize-completion-front-edge.sh` is the canonical
default entry point for the completion front-edge stabilization evidence
bundle; it wraps the generic readiness script with `CHANGE_ID=stabilize-completion-front-edge`
and collects the representative `churn` profile for deterministic correlation,
quiet observability, and trigger parity validation.
For `refactor-completion-front-edge-exact-deadline-removal`, the generic
entry point `CHANGE_ID=refactor-completion-front-edge-exact-deadline-removal
./scripts/validate-v2-completion-gates.sh` is the canonical full-bundle run;
it auto-selects representative perf profiles `large churn` plus the
change-specific real-module `front_edge` gate, so the bundle stays scoped to
the immediate post-edit/save readiness surface that the change actually fixes.
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
