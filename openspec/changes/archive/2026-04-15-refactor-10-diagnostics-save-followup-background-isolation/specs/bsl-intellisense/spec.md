## MODIFIED Requirements

### Requirement: Incident bundle summary показывает didSave refresh как request-centric diagnostics cycle (MUST)
`summary.md` и `incident.json` MUST переносить diagnostics save timeline в человекочитаемом request-centric виде.

Human-readable projection MUST:

- показывать `uri`, `requested_version` и bounded first-publish outcome;
- сохранять `save_cycle_sequence` рядом с `requested_version` и `diagnostics_generation`;
- различать `save_fastlane` first publish и `idle_heavy` follow-up;
- показывать, был ли first publish `syntax_only` или `full`;
- не переименовывать aggregate metrics p95/p99 в request-level факты.

Дополнительно projection MUST:

- рендерить operator-facing save ordering через `save_cycle_sequence`, а не через `diagnostics_generation`, если два save-cycle делят один `requested_version`;
- явно различать active `in_flight` cycles и terminal cycles;
- не рендерить pending profile facts для active cycle как `unknown`, если lifecycle уже известен;
- объяснять active heavy follow-up через explicit request-centric wait reason, если сервер его уже знает;
- сохранять canonical terminal non-cancellation outcome `disabled_by_config`, когда backend его публикует для `idle_heavy`, и MUST NOT схлопывать его в `pending`, `unknown` или cancellation surrogate.

#### Scenario: Summary preserves disabled_by_config as an explicit terminal outcome
- **GIVEN** diagnostics save timeline trace already published `idle_heavy_outcome=disabled_by_config`
- **AND** cycle remains terminal from the server perspective
- **WHEN** extension формирует `summary.md` и `incident.json`
- **THEN** human-readable diagnostics save section показывает `disabled_by_config` как explicit terminal non-cancellation outcome
- **AND** не рендерит этот cycle как `pending`, `unknown` или generic cancellation
