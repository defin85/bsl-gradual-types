# Design: `bsl-cli check` runtime contract

## Context

Current observed behavior:

- `bsl-cli check <file>` can run v2 diagnostics on a real BSL file and return non-zero when errors or warnings are present.
- `--verbose` exposes diagnostic details, while the default path mostly reports counts.
- `--format` is accepted by the CLI, but the diagnostics check path does not yet provide a reliable machine-readable diagnostics contract.
- The CLI runtime starts `SystemCoordinator` with syntax-helper/runtime resources, but no configuration root is passed for the single-file check path.
- Exact type-index precompute is used for completion-style operations, not for diagnostics.

That behavior is useful, but it must be described and tested as a limited single-file diagnostics surface unless the user explicitly opts into a richer workspace/config mode.

## Goals

- Make `bsl-cli check` automation-friendly and truthful.
- Preserve a fast default single-file diagnostics mode.
- Expose enough runtime metadata to prevent overclaiming evidence.
- Allow an explicit config-root/workspace path when a check is intended to prove configuration-backed behavior.
- Add repo-owned smoke coverage that catches output-format drift and known false diagnostics.

## Non-Goals

- Replace LSP/live workspace verification with CLI single-file checks.
- Make diagnostics always require exact type-index warmup.
- Parse every file under `examples/conf_big` by default.
- Hide remaining unsupported globals behind permissive unknowns without targeted evidence.

## Decisions

### Diagnostics output

`--format json` should emit parseable JSON on stdout with no human progress logs mixed into the JSON stream. The payload should include:

- checked path,
- exit status classification,
- error/warning counts,
- diagnostics with severity, message, source/rule id when available, and range,
- runtime evidence metadata.

Human formats may stay optimized for terminal use, but diagnostics should not be discoverable only through an undocumented side effect. If the default format remains summary-oriented, the detailed human mode must be explicit and documented.

### Runtime evidence metadata

The check report should distinguish at least:

- syntax helper status,
- rules config status and effective path/hash when applicable,
- configuration root status (`not_provided`, `loaded`, `failed`, or equivalent),
- exact type-index status (`not_requested`, `ready`, `failed`, or equivalent),
- analysis mode (`single_file`, `workspace_config`, or equivalent).

The names can be adjusted during implementation, but the semantics must remain stable enough for tests and docs.

### Configuration-backed mode

`bsl-cli check` needs an explicit way to receive a configuration root/workspace root for configuration-backed diagnostics. If an existing flag already provides this contract after review, the implementation should reuse it and document that path. Otherwise, add a narrow `check` input rather than relying on ambient cwd inference.

Missing or malformed configuration input should fail closed with a clear diagnostic/report status. It should not silently produce no-config output while implying that configuration metadata was used.

### Exact index policy

Diagnostics should report exact type-index readiness truthfully. The default diagnostics path may keep using the lighter diagnostics-ready dependency snapshot. Exact warmup should be opt-in or tied to a mode that genuinely needs it. The report must not imply exact readiness when no exact snapshot was prepared.

### Regression smoke

The smoke suite should include:

- a small deterministic fixture for output-format behavior,
- a JSON parse/assertion test,
- a regression run against `examples/conf_big/CommonModules/АвансовыйОтчетФормы/Ext/Module.bsl` proving that known global collection manager chains such as `Перечисления.ВидыОперацийАвансовыйОтчет.Командировка` and `Документы.АвансовыйОтчет.Выбрать()` do not regress into high-confidence unknown member diagnostics in the default single-file mode.
