## ADDED Requirements

### Requirement: Symbol Browser supports a BSL TypeRepository mode
For BSL projects, Symbol Browser SHALL be able to use the BSL language server TypeRepository list surface instead of generic `workspace/symbol`.

The BSL mode SHALL be selected only when at least one language server in the active project/workspace advertises support for `bsl.getAllTypes` through `workspace/executeCommand` or a tested equivalent `bsl/getAllTypes` alias. For unsupported servers, Symbol Browser SHALL fall back to generic `workspace/symbol`.

When more than one open-worktree language server advertises the BSL getAllTypes surface, Symbol Browser SHALL request BSL TypeRepository data from each capable server and merge results in a deterministic order.

#### Scenario: BSL project uses getAllTypes
- **GIVEN** a BSL project is open in Zed
- **AND** the active BSL language server advertises `bsl.getAllTypes`
- **WHEN** the user opens Symbol Browser
- **THEN** the panel requests BSL TypeRepository data through getAllTypes
- **AND** it does not use generic `SymbolKind` grouping for that BSL result

#### Scenario: Multiple capable BSL servers merge deterministically
- **GIVEN** a Zed workspace has multiple open worktrees with language servers advertising `bsl.getAllTypes`
- **WHEN** the user opens Symbol Browser
- **THEN** the panel requests TypeRepository data from each capable server
- **AND** the merged BSL result order is deterministic across refreshes

#### Scenario: Unsupported BSL server falls back to generic symbols
- **GIVEN** a BSL language server does not advertise `bsl.getAllTypes`
- **WHEN** the user opens Symbol Browser
- **THEN** the panel uses the existing generic `workspace/symbol` data source
- **AND** generic Rust and other-language behavior is unchanged

### Requirement: BSL TypeRepository results are grouped by source and category
When Symbol Browser renders BSL getAllTypes data, it SHALL group entries using TypeRepository semantics instead of LSP `SymbolKind`.

The grouping SHALL be deterministic and SHALL include counts in group headings. The initial display SHALL include at least the type name for each entry.

Group membership SHALL be derived from each returned type entry's `source` and `category` fields. Response-level `categories` metadata MAY be used for decoration or diagnostics, but SHALL NOT be the source of truth for group membership.

#### Scenario: Configuration and platform types are grouped separately
- **GIVEN** getAllTypes returns types with `source=Platform` and `source=Configuration`
- **WHEN** Symbol Browser renders the result
- **THEN** platform and configuration entries appear in distinct source/category groups
- **AND** each visible group heading includes the number of entries in that group

#### Scenario: Response metadata does not override item grouping
- **GIVEN** getAllTypes returns item-level `source` and `category` fields
- **AND** response-level `categories` metadata is absent, incomplete, or summarized differently
- **WHEN** Symbol Browser renders the result
- **THEN** each entry is grouped by its own `source` and `category`

#### Scenario: Empty TypeRepository result is not loading forever
- **GIVEN** getAllTypes returns a successful response with no `types`
- **WHEN** Symbol Browser renders the result
- **THEN** the panel shows the completed empty state
- **AND** it does not show `Loading symbols...` indefinitely

### Requirement: BSL getAllTypes failures are fail-closed and visible
After Symbol Browser selects BSL mode, getAllTypes request failures SHALL be surfaced to the panel as an error state.

The panel SHALL log the failure reason and SHALL NOT silently merge partial stale data into the displayed result.

BSL pagination SHALL use checked-in page-size, max-page, and max-item bounds. Reaching the bound SHALL stop fetching deterministically instead of continuing unbounded requests.

#### Scenario: getAllTypes request fails
- **GIVEN** Symbol Browser selected BSL mode
- **WHEN** a getAllTypes page request fails
- **THEN** the panel shows `Symbols unavailable` or an equivalent error placeholder
- **AND** the failure reason is logged

#### Scenario: Pagination cap is reached
- **GIVEN** getAllTypes keeps returning additional pages
- **WHEN** Symbol Browser reaches the checked-in maximum page or item bound
- **THEN** the panel stops requesting more pages
- **AND** the result state remains deterministic and non-loading
