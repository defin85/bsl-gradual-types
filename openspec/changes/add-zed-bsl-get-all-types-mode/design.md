## Context

The current Symbol Browser panel is generic: it calls `project.symbols("", cx)`, which sends LSP `workspace/symbol` and groups the result by `SymbolKind`.

The BSL backend already has a TypeRepository list surface used by the VS Code extension:
- transport: standard LSP `workspace/executeCommand`;
- command: `bsl.getAllTypes`;
- request fields: `limit`, `offset`, optional `category`;
- response shape: `AnalysisResultDto` with `types`, `categories`, `metrics`, and optional `pagination`;
- type entries include `name`, `category`, `source`, `methods`, `properties`, `tabularSections`, and description fields.

The Beads follow-up used the slash spelling `bsl/getAllTypes`. Current checked-in backend code exposes the dot command spelling. This change should normalize the product contract without pretending the slash method already exists.

## Goals / Non-Goals

- Goals: BSL-specific Symbol Browser mode, TypeRepository-backed grouped type list, generic fallback preserved.
- Non-Goals: click navigation, expanding type members, replacing VS Code Type Repository, or relying on a non-existent custom request without a compatibility path.

## Decisions

### Decision 1: Transport contract

Initial Zed implementation MUST call `workspace/executeCommand` with command `bsl.getAllTypes`, because that command is currently advertised by `execute_command_provider` and is already used by VS Code.

If implementation adds a direct custom request alias `bsl/getAllTypes`, it MUST delegate to the same handler and return the same payload shape. The alias is optional compatibility work, not an initial-mode prerequisite. Zed MAY prefer the alias only after it is implemented and covered by backend and Zed integration tests.

### Decision 2: BSL mode detection

The panel MUST choose BSL mode only when the active Zed project/workspace has at least one language server whose capabilities advertise `workspace/executeCommand` with `bsl.getAllTypes`, or an equivalent tested `bsl/getAllTypes` alias.

If no BSL getAllTypes surface is available, the panel MUST fall back to the generic `workspace/symbol` data source instead of showing a false BSL error.

When BSL mode is selected, the project-layer helper MUST query every open-worktree language server that advertises the command, then merge results in a deterministic order. If multiple servers return the same `(source, category, name)` tuple, the initial implementation MAY preserve duplicates when they come from different worktrees, but it MUST keep ordering stable and MUST NOT hide an origin needed to disambiguate displayed entries.

### Decision 3: Pagination and bounds

Zed MUST fetch getAllTypes in bounded pages. The initial implementation SHOULD use a large page size and stop when:
- `pagination.hasNext == false`;
- the response returns fewer items than requested;
- the response returns no items;
- a checked-in maximum item/page cap is reached.

The implementation MUST define visible constants for page size, maximum pages, and maximum total items, and MUST test the cap path.

A failed page after choosing BSL mode MUST surface as `Symbols unavailable` and log the reason; it MUST NOT silently merge stale partial data.

### Decision 4: Grouping model

BSL mode MUST group by TypeRepository semantics rather than `SymbolKind`:
- top-level display groups SHOULD distinguish `source` (`Platform`, `Configuration`, `UserDefined`) and `category`;
- configuration categories SHOULD use metadata-oriented names where possible;
- item labels SHOULD use type `name` for the first iteration.

Generic mode MUST keep its current `SymbolKind` grouping unchanged.

Group membership MUST be derived from each returned type entry's `source` and `category` fields. The response-level `categories` map is advisory metadata only and MUST NOT be the source of truth for building groups, because current backend responses summarize categories separately from item-level membership.

### Decision 5: Settings

The default behavior SHOULD be automatic:
- BSL project with getAllTypes support -> BSL TypeRepository mode;
- otherwise -> generic `workspace/symbol` mode.

If a setting is added, it SHOULD be explicit and fail-safe, for example `symbol_browser.mode = "auto" | "generic" | "bsl"`, where forced `bsl` still reports unsupported/error states honestly.

### Decision 6: Zed crate boundary

LSP capability inspection, `workspace/executeCommand` calls, BSL DTO parsing, pagination, and multi-server merge belong in `crates/project` or a narrow adjacent project-layer helper. The `crates/symbol_browser` UI should consume a UI-neutral symbol/group model and should not construct raw LSP execute-command requests directly.

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| Current server exposes `bsl.getAllTypes`, not `bsl/getAllTypes` | Specify the dot-command as the initial transport and make any slash alias an explicit backend task |
| BSL mode accidentally runs for non-BSL projects | Gate on advertised command/alias capability and keep generic fallback |
| Large TypeRepository responses hurt UI responsiveness | Use pagination, caps, and focused tests around bounded fetch behavior |
| Generic Rust/other-language behavior regresses | Keep generic code path separate and repeat Rust live-smoke |

## Deferred Questions

- Whether to add the direct `bsl/getAllTypes` alias in the backend after the initial `workspace/executeCommand` implementation.
- Exact display naming for platform categories should follow existing VS Code Type Repository conventions where practical, but the initial acceptance only requires deterministic source/category grouping.
