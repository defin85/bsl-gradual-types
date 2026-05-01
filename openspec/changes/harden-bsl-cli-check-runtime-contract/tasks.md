## 1. Baseline and Contract

- [ ] 1.1 Capture current `bsl-cli check` behavior for default output, `--verbose`, `--format json`, and human formats on a tiny fixture.
- [ ] 1.2 Capture current `bsl-cli check` behavior on `examples/conf_big/CommonModules/АвансовыйОтчетФормы/Ext/Module.bsl`, including which diagnostics remain after known global collection fixes.
- [ ] 1.3 Decide whether `bsl-cli check` needs a new config-root/workspace flag or can reuse an existing explicit input without ambiguity.
- [ ] 1.4 Document the intended exit-code contract for errors, warnings, strict mode, and internal runtime failures.

## 2. CLI Output Hardening

- [ ] 2.1 Add failing tests proving `--format json` emits parseable diagnostics output without requiring `--verbose`.
- [ ] 2.2 Add failing tests proving human diagnostic output is documented and does not hide diagnostics behind an accidental verbose-only path.
- [ ] 2.3 Implement the JSON report shape with counts, diagnostics, checked path, exit classification, and runtime evidence metadata.
- [ ] 2.4 Keep stdout/stderr separation strict enough that JSON stdout can be consumed by automation.
- [ ] 2.5 Ensure `--format` and `--verbose` compose predictably, with tests for the supported combinations.

## 3. Runtime and Configuration Evidence

- [ ] 3.1 Pass explicit configuration root/workspace input through CLI runtime when provided.
- [ ] 3.2 Expose configuration status in the CLI report, including no-config, loaded, and failed states.
- [ ] 3.3 Expose syntax-helper and rules-config status in the CLI report using the existing runtime loader facts where possible.
- [ ] 3.4 Expose exact type-index status truthfully; diagnostics must not imply exact snapshot readiness unless it was actually prepared.
- [ ] 3.5 Fail closed for malformed explicit config input instead of silently downgrading to no-config mode.

## 4. Smoke Coverage and Documentation

- [ ] 4.1 Add CLI integration coverage for JSON output parsing, human output, and exit codes.
- [ ] 4.2 Add regression smoke for `examples/conf_big/CommonModules/АвансовыйОтчетФормы/Ext/Module.bsl` proving `Командировка` and `Выбрать` do not reappear as high-confidence unknown member diagnostics in default mode.
- [ ] 4.3 Update agent/developer verification docs with `bsl-cli check` commands and explicit caveats about single-file, config-backed, exact-index, and LSP/live evidence.
- [ ] 4.4 Add the CLI smoke to the repo-owned verification path if it remains fast and deterministic.

## 5. Validation

- [ ] 5.1 Run `cargo test -p bsl-cli --locked`.
- [ ] 5.2 Run targeted runtime/analysis tests affected by config-root and runtime metadata plumbing.
- [ ] 5.3 Run `cargo fmt --all -- --check`.
- [ ] 5.4 Run `openspec validate harden-bsl-cli-check-runtime-contract --strict --no-interactive`.
