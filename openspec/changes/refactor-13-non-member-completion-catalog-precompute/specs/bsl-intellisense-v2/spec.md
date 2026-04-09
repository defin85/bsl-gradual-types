## ADDED Requirements
### Requirement: Warm non-member completion reuses immutable deps-scoped candidate catalogs (MUST)

Warm non-member completion MUST reuse immutable deps-scoped candidate catalogs for candidate families whose semantic content depends on deps/settings snapshot rather than on the current cursor-local revision state.

Для таких immutable families система MUST:

- prebuild or reuse a deps/settings-scoped catalog (or semantically equivalent immutable snapshot artifact);
- avoid rebuilding the full family on every warm non-member request under the same deps/settings snapshot;
- apply prefix-aware filtering before full `Candidate` materialization when a usable request prefix is already known;
- preserve existing source-priority and ranking semantics for the surviving candidates.

При этом система MUST keep revision/context-sensitive sources separate:

- local symbols;
- contextual implicit symbols;
- module routines and other cursor-local sources.

#### Scenario: Warm non-member completion не rebuild-ит immutable deps-wide catalogs на каждый request
- **GIVEN** deps/settings snapshot не менялся между двумя warm non-member completion requests
- **AND** request path уже имеет current-revision state для файла
- **WHEN** IDE запрашивает non-member completion повторно
- **THEN** immutable deps-scoped families переиспользуются из snapshot-scoped catalog вместо полного rebuild
- **AND** warm collect latency не доминируется повторной materialization тех же global functions / repository types / metadata items

#### Scenario: Prefix-aware filtering materialize-ит только нужный subset immutable catalog
- **GIVEN** non-member completion request содержит usable prefix
- **AND** immutable deps-scoped catalog уже готов для текущего deps/settings snapshot
- **WHEN** handler формирует collect-stage candidates
- **THEN** сервер сначала фильтрует immutable catalog по prefix
- **AND** materialize-ит полные `Candidate` только для surviving subset
- **AND** итоговый candidate set остаётся эквивалентен текущему correctness contract
