## ADDED Requirements

### Requirement: Canonical local-function-summary inference MUST short-circuit singleton non-recursive SCCs

The system MUST, when canonical semantic-facts materialization derives local routine summaries for
the current exact target revision, detect singleton SCCs that have no self-edge and compute their
summaries without entering the general recursive fixed-point loop.

This behavior MUST:

- preserve the same exact semantic contract for return types and local call targets;
- rely only on already stabilized out-of-SCC summaries plus the current routine body;
- keep self-recursive singleton SCCs off the fast path.

#### Scenario: Singleton non-recursive local routine resolves without recursive fixed-point

- **GIVEN** one local routine belongs to an SCC of size `1`
- **AND** that SCC has no self-edge
- **AND** callees outside the SCC are already stabilized by reverse-topological processing
- **WHEN** canonical semantic-facts materialization computes local-function summaries
- **THEN** the runtime computes that routine summary in one bounded pass
- **AND** it does not enter the general recursive fixed-point loop for that SCC
- **AND** the resulting summary remains equivalent to the exact semantic contract

#### Scenario: Self-recursive singleton stays on the convergence path

- **GIVEN** one local routine belongs to an SCC of size `1`
- **AND** that routine calls itself, so the SCC has a self-edge
- **WHEN** canonical semantic-facts materialization computes local-function summaries
- **THEN** the singleton fast path does not apply
- **AND** the routine summary is still derived through a convergence-safe recursive path

### Requirement: Recursive local-summary SCC solving MUST iterate SCC-locally rather than rebuilding file-wide snapshots

The system MUST, when canonical semantic-facts materialization solves a recursive local-routine
SCC, preserve a stable base view for out-of-SCC summaries and restrict per-iteration rebuild work
to the active SCC overlay rather than rebuilding a full-file local-summary snapshot.

This behavior MUST:

- let in-SCC lookups observe the latest current-SCC overlay values;
- let out-of-SCC lookups observe stable already-finalized summaries;
- preserve deterministic ordering and convergence behavior for recursive SCCs.

#### Scenario: Recursive SCC iterations reuse stable out-of-SCC summaries

- **GIVEN** a file contains one recursive local-routine SCC and many unrelated local routines
- **WHEN** the runtime iterates that SCC to convergence
- **THEN** each iteration reuses stable summaries outside the active SCC from a base lookup
- **AND** only the active SCC overlay participates in per-iteration rebuild work
- **AND** the runtime does not rebuild a full-file local-summary snapshot on each iteration

### Requirement: Representative save-follow-up evidence MUST expose local-summary convergence attribution

The system MUST export low-cardinality local-summary convergence attribution for representative
same-file save-follow-up evidence whenever canonical semantic diagnostics report
`local_function_summaries` cost.

This evidence MUST include at least:

- total `local_function_summaries` latency;
- `prep`, `fixed_point`, `snapshot_build`, and `body_infer` subphases;
- `function_count`, `scc_count`, and fixed-point iteration count.

#### Scenario: Representative report distinguishes singleton fast-path wins from recursive residual

- **GIVEN** a representative large-module same-file save-follow-up exports canonical semantic
  diagnostics evidence
- **WHEN** `local_function_summaries` remains visible in that report
- **THEN** the report includes local-summary convergence attribution and bounded workload counts
- **AND** an operator can distinguish singleton fast-path wins from remaining recursive-SCC work
