## MODIFIED Requirements

### Requirement: didSave heavy follow-up избегает apply-lag как primary gate (MUST)

После successful same-version `save_fastlane` first publish система MUST стремиться к richer heavy
follow-up того же `save_cycle_sequence` без unbounded зависимости от writer/apply lag как primary
gate, если same-version ready artifacts уже доступны или представлены exact same-version
ready-snapshot worker, который уже находится in-flight для той же revision.

Система MAY использовать writer-owned applied state, когда он уже готов, но MUST:

- предпочитать already-ready same-version artifacts поверх blind `wait_for_file_version`;
- если ready artifacts ещё не materialized, но существует exact same-version worker для matching
  `(file_id, requested_version, text_hash)`, предпочитать promotion и bounded wait за этим worker
  перед `shadow_state` fallback;
- переиспользовать существующий bounded wait budget и MUST NOT превращать promotion в более долгий
  или неограниченный stall;
- MUST NOT запускать duplicate `didSave` ready-snapshot worker для идентичного
  `(file_id, requested_version, text_hash)`;
- truthful fallback to `shadow_state`/generic path, если exact worker superseded, cancelled,
  mismatched или не успел materialize exact snapshot к дедлайну;
- не публиковать older-version diagnostics;
- сохранять supersession semantics для newer save cycles.

#### Scenario: didSave promotes exact same-version worker and wins the same save cycle

- **GIVEN** `didSave` already completed `save_fastlane` first publish for revision `V`
- **AND** exact same-version ready-snapshot worker from `didChange` for revision `V` is already
  in flight with matching text/version
- **WHEN** heavy follow-up chooses between `ready_artifacts` and `shadow_state`
- **THEN** the server promotes and boundedly waits for that exact worker before `shadow_state`
  fallback
- **AND** does not start a duplicate `didSave` snapshot worker for the same text/version
- **AND** if the promoted worker materializes within budget, the same save cycle can publish richer
  follow-up from exact same-version artifacts

#### Scenario: didSave falls back truthfully when the promoted exact worker cannot become usable

- **GIVEN** `didSave` observes an in-flight exact same-version worker for revision `V`
- **AND** that worker becomes superseded, cancelled, mismatched, or remains not-ready until the
  bounded wait budget expires
- **WHEN** heavy follow-up completes branch selection
- **THEN** the server falls back truthfully to `shadow_state` or generic pipeline for revision `V`
- **AND** does not publish older-version diagnostics
- **AND** does not keep waiting unboundedly for the unusable exact worker

## ADDED Requirements

### Requirement: Background ready-snapshot workers are cooperatively superseded and exact-task promotable (MUST)

Background ready-snapshot workers for `didOpen`/`didChange`/`didSave` MUST behave as controllable
tasks instead of abort-only fire-and-forget jobs.

For obsolete or superseded workers, the system MUST:

- signal cooperative cancellation through shared task state that is observable before and during
  debounce / parse-build execution;
- MUST NOT rely solely on outer async task abort once blocking parse work has already started;
- stop obsolete identical or older-version workers before they continue consuming parser or
  blocking capacity after a newer requested revision has superseded them.

For exact same-version waiters, the system MAY promote an existing worker, but MUST:

- support promotion of an exact same-version worker into `did_save_followup` priority for the
  materialization stage;
- MUST NOT duplicate parse work for identical `(file_id, requested_version, text_hash)`;
- MUST NOT move snapshot-backed `SetFileWithSnapshot` install onto the interactive writer queue
  merely to win the wait.

#### Scenario: Newer didChange supersedes obsolete exact worker before it keeps burning parse capacity

- **GIVEN** a ready-snapshot worker is already running for revision `V`
- **AND** a newer requested revision `V+1` supersedes that file before the older worker finishes
- **WHEN** the system updates worker control state for the file
- **THEN** the older worker observes cooperative cancellation before continuing obsolete parse/build
  work
- **AND** the system does not rely only on outer-task abort to stop already-started blocking parse
  execution

#### Scenario: didSave promotes existing exact worker instead of spawning duplicate parse work

- **GIVEN** `didSave` heavy follow-up needs exact same-version artifacts for revision `V`
- **AND** an exact same-version worker for matching `(file_id, requested_version, text_hash)` is
  already in flight
- **WHEN** the server requests higher priority for that exact worker
- **THEN** the existing worker becomes the promoted producer for that revision
- **AND** the server does not start a second same-version parse worker just because `didSave`
  joined the wait

### Requirement: bsl.getCurrentContext reuses exact same-version snapshot workers before independent parse (MUST)

Backend MUST prefer bounded reuse of an exact same-version ready-snapshot worker before launching
an independent `parser_coordinator` parse for `bsl.getCurrentContext`, when a same-file request
already has a matching in-flight worker for the same text/version.

The backend MUST:

- consume ready exact snapshot state immediately if it is already materialized;
- otherwise wait only a short bounded reuse budget for the exact worker's materialization before
  starting independent parse work;
- preserve latest-generation-wins supersession/cancellation semantics for current-context
  generations;
- fall back to the existing broker/leader parse path if no matching exact task exists, the task no
  longer matches the text/version, or the reuse budget expires.

#### Scenario: currentContext reuses same-file exact worker instead of racing parser_coordinator

- **GIVEN** `didChange` already started an exact same-version ready-snapshot worker for file `F`
  and revision `V`
- **AND** `bsl.getCurrentContext` arrives for the same file text/revision before that worker
  materializes
- **WHEN** the backend decides whether to parse current context independently
- **THEN** it first reuses or briefly awaits the exact worker's materialization
- **AND** only falls back to independent `parser_coordinator` parse if the reuse budget expires or
  the worker stops matching the request
- **AND** newest-generation current-context semantics remain authoritative for the client
