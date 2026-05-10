## ADDED Requirements

### Requirement: LSP exposes TypeRepository data through getAllTypes
The BSL language server SHALL expose a bounded TypeRepository listing for IDE clients.

The current required transport is standard LSP `workspace/executeCommand` with command `bsl.getAllTypes`, and initialization capabilities SHALL advertise that command through `execute_command_provider.commands`. Request parameters SHALL support:
- `limit`;
- `offset`;
- optional `category`.

The response SHALL be a well-formed `AnalysisResultDto` with `types`, `categories`, `metrics`, and optional `pagination`.

Each returned type entry SHALL include item-level `name`, `source`, and `category` fields suitable for IDE grouping. The response-level `categories` map is advisory metadata and SHALL NOT be required by clients to determine an individual entry's source/category group.

If a direct custom request alias `bsl/getAllTypes` is added, it SHALL return the same response semantics as `bsl.getAllTypes`.

#### Scenario: Server advertises getAllTypes execute command
- **GIVEN** a BSL language server initializes successfully
- **WHEN** the client inspects server capabilities
- **THEN** `execute_command_provider.commands` includes `bsl.getAllTypes`

#### Scenario: Client requests the first getAllTypes page
- **GIVEN** TypeRepository data is available
- **WHEN** the client sends `workspace/executeCommand` with command `bsl.getAllTypes` and `limit=100`, `offset=0`
- **THEN** the server returns an `AnalysisResultDto`
- **AND** the response includes `types`
- **AND** each returned type includes item-level `name`, `source`, and `category`
- **AND** pagination metadata is present when the result is paged

#### Scenario: Client filters getAllTypes by category
- **GIVEN** TypeRepository data is available
- **WHEN** the client sends `bsl.getAllTypes` with a `category`
- **THEN** every returned type belongs to that category
- **AND** the pagination totals reflect the filtered result

#### Scenario: Domain bundle is unavailable
- **GIVEN** the TypeRepository domain bundle is not available
- **WHEN** the client sends `bsl.getAllTypes`
- **THEN** the server returns a well-formed empty `AnalysisResultDto`
- **AND** the request does not fail with an internal error solely because the bundle is missing
