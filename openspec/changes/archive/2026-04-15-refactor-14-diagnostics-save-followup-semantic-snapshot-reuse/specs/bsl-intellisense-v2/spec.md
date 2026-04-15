## MODIFIED Requirements

### Requirement: didSave heavy follow-up избегает apply-lag как primary gate (MUST)
После successful same-version `save_fastlane` first publish система MUST стремиться к richer heavy
follow-up того же `save_cycle_sequence` без unbounded зависимости от writer/apply lag как primary
gate, если same-version ready artifacts уже доступны.

Система MAY использовать writer-owned applied state, когда он уже готов, но MUST:

- предпочитать same-version ready artifacts немедленно, если same-version ready parse snapshot уже
  materialized к моменту старта heavy follow-up;
- использовать `shadow_state` semantic path только как fallback, когда same-version ready parse
  snapshot ещё не доказан или stale;
- при snapshot-backed follow-up semantic diagnostics использовать snapshot-aware parse-result и
  IR accessors, а не direct salsa parse/IR path, который bypass-ит version-bound parse snapshot
  reuse;
- не публиковать older-version diagnostics;
- сохранять supersession semantics для newer save cycles;
- оставаться fail-closed и truthful, когда same-version snapshot отсутствует, stale или mismatch.

#### Scenario: already-ready parse snapshot is preferred before shadow-state semantic work
- **GIVEN** `didSave` already materialized same-version ready parse snapshot for requested revision
- **AND** `save_fastlane` first publish already completed
- **WHEN** server starts `idle_heavy` follow-up for the same `save_cycle_sequence`
- **THEN** follow-up prefers the `ready_artifacts` path immediately
- **AND** does not first spend semantic work on the `shadow_state` path for the same revision

#### Scenario: snapshot-backed semantic follow-up does not force direct parse_result recompute
- **GIVEN** heavy didSave follow-up runs on analysis state seeded with same-version
  `SetFileWithSnapshot`
- **WHEN** semantic diagnostics are profiled for that follow-up
- **THEN** parse input is taken from snapshot-aware parse-result accessors
- **AND** the implementation does not force a direct full salsa `parse_result(...)` path as the
  default same-version semantic input

#### Scenario: stale or missing snapshot falls back truthfully
- **GIVEN** same-version ready parse snapshot is absent, stale, or mismatched for the requested
  save revision
- **WHEN** heavy didSave follow-up attempts to build richer diagnostics
- **THEN** the server falls back to the existing truthful shadow/generic path
- **AND** does not publish stale diagnostics from another revision

### Requirement: didSave diagnostics публикует request-centric save refresh timeline (MUST)
Система MUST публиковать bounded authoritative trace для каждого diagnostics refresh, инициированного
`textDocument/didSave`.

Этот trace MUST:

- быть server-authored;
- быть request-centric, а не derived из cumulative metrics;
- содержать `uri`, `requested_version`, `save_cycle_sequence`, `diagnostics_generation`,
  `trigger=did_save`;
- фиксировать bounded stage/runtime facts, достаточные для разбора first publish и heavy
  follow-up;
- не содержать raw document text, snippets или high-cardinality payload.

Дополнительно trace MUST:

- не создавать второй trace identity для уже terminal `(requested_version, save_cycle_sequence)`;
- не заставлять operator-facing cycle ordering выводиться из `diagnostics_generation`, если у двух
  save-cycle совпадает `requested_version`;
- публиковать `blocking_queue_wait_ms` только как factual wait перед shared blocking gate, а не как
  synthetic surrogate для direct save-fastlane bypass path;
- различать `save_fastlane` first publish и heavy follow-up stall;
- не оставлять active heavy follow-up в состоянии просто `pending`, если сервер уже знает, что
  primary blocker это `apply_lag` / `wait_for_file_version`;
- при semantic heavy follow-up публиковать bounded path/source attribution, достаточную чтобы
  оператор видел:
  - какой semantic path использовался (`ready_artifacts|shadow_state|generic_pipeline`);
  - откуда пришёл semantic parse input (`snapshot|salsa`);
  - откуда пришёл semantic IR input (`exact_cache|snapshot_build|salsa`).
- публиковать эту semantic path/source attribution через versioned authoritative diagnostics save
  timeline contract так, чтобы older contract versions деградировали явно как
  unavailable-by-design, а не silently выглядели как отсутствие semantic reuse.

#### Scenario: timeline explains snapshot-backed semantic follow-up sources
- **GIVEN** `didSave` heavy follow-up publishes richer diagnostics via same-version ready artifacts
- **WHEN** operator reads diagnostics save timeline
- **THEN** trace keeps one save-cycle identity for that follow-up
- **AND** trace shows semantic path `ready_artifacts`
- **AND** trace shows semantic parse source `snapshot`
- **AND** trace shows semantic IR source as one of bounded canonical values rather than leaving the
  operator to infer reuse from aggregate latency only

#### Scenario: timeline explains truthful fallback when semantic reuse is unavailable
- **GIVEN** heavy didSave follow-up cannot prove same-version ready parse snapshot reuse
- **WHEN** operator reads diagnostics save timeline
- **THEN** trace keeps explicit semantic path/source attribution for the fallback path
- **AND** operator can distinguish missing semantic reuse from unrelated queue or publish delay
