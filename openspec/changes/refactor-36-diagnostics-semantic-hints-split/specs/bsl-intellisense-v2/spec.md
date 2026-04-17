## ADDED Requirements

### Requirement: Semantic diagnostics MUST support diagnostics-only type-hint materialization for the current exact target

The system MUST support a diagnostics-only semantic path that materializes only the type-hint
artifact required by semantic diagnostics for the current exact target instead of always
materializing full `SemanticFacts`.

At minimum this diagnostics-only artifact MUST support:

- `assignment_value_type_by_span`;
- `call_receiver_type_by_span`;
- `call_arg_types_by_span`;
- `member_access_object_type_by_span`.

This behavior MUST:

- preserve semantic diagnostics parity with the full semantic path for supported cases;
- fall back to the full semantic path fail-closed when parity cannot be proven for a case;
- avoid performing diagnostics-irrelevant full semantic-facts work on supported same-file
  save-follow-up targets.

#### Scenario: Representative semantic diagnostics use diagnostics-only hints instead of full semantic facts

- **GIVEN** a same-file save-follow-up requests semantic diagnostics for a current exact target
- **AND** that target falls within the supported diagnostics-only contract
- **WHEN** the runtime materializes semantic inputs for diagnostics
- **THEN** it builds diagnostics-only type hints instead of full `SemanticFacts`
- **AND** the resulting semantic diagnostics remain equivalent to the full semantic path

#### Scenario: Unsupported diagnostics case falls back to the full semantic path

- **GIVEN** semantic diagnostics encounter a case whose parity is not proven under the
  diagnostics-only contract
- **WHEN** the runtime prepares semantic inputs for diagnostics
- **THEN** it falls back to full `SemanticFacts`
- **AND** it does not silently publish reduced diagnostics from an unsupported narrowed path

### Requirement: Diagnostics-only semantic artifacts MUST remain isolated from the full exact semantic artifact cache

The system MUST NOT store diagnostics-only semantic artifacts under the current full exact semantic
cache identity for the same `(file, version, deps, settings)` target.

Diagnostics-only artifacts MUST be ephemeral or stored under a separate diagnostics cache namespace
so later interactive exact consumers cannot mistake them for full `SemanticFacts`.
This isolation requirement also applies to any cached `SemanticProgram`, completion-head artifact,
or equivalent exact IR-derived artifact that interactive exact consumers reuse.
The diagnostics-only path MUST NOT publish a trimmed semantic artifact into the current exact
interactive slot for that target.

#### Scenario: Diagnostics-only query does not poison later hover or completion

- **GIVEN** a diagnostics-only semantic query already ran for the current exact target
- **WHEN** a later interactive exact request such as hover or completion needs full semantic facts
- **THEN** the runtime does not treat the diagnostics-only artifact as a cache hit for the full
  semantic contract
- **AND** the interactive request still reads or builds full `SemanticFacts`

### Requirement: Representative diagnostics evidence MUST distinguish diagnostics-only hints from full semantic-facts fallback

The system MUST export low-cardinality attribution showing whether representative semantic
diagnostics used diagnostics-only hint materialization or fell back to full `SemanticFacts`.

This evidence MUST include at least:

- the diagnostics semantic path identity for the traced target;
- the bounded latency for diagnostics-hint materialization or full semantic-facts fallback;
- the remaining diagnostics collection/query latency for that same traced target.

#### Scenario: Representative report explains the diagnostics semantic path truthfully

- **GIVEN** a representative same-file save-follow-up exports semantic diagnostics evidence
- **WHEN** the diagnostics path finishes or exports a checked-in report
- **THEN** the report distinguishes diagnostics-only hint materialization from full semantic-facts
  fallback for that traced target
- **AND** operators can attribute the residual to the correct semantic path instead of inferring it
  indirectly
