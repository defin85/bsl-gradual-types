## ADDED Requirements
### Requirement: Current-revision completion head публикуется как detached immutable snapshot (MUST)
Для completion first-response path система MUST публиковать отдельный detached immutable current-revision head snapshot как canonical derived read model.

Этот snapshot MUST:
- быть keyed по `(file_id, file_version, deps_id, settings_id)`;
- быть safe для concurrent readers;
- не являться shared `AnalysisV2`;
- не удерживать writer-owned mutable runtime state;
- содержать только bounded payload, необходимый для first completion response.

#### Scenario: Concurrent readers используют один detached head snapshot
- **GIVEN** current-revision detached head snapshot уже опубликован
- **WHEN** несколько completion readers запрашивают first response для той же revision
- **THEN** они читают один и тот же detached immutable snapshot
- **AND** не требуют shared mutable runtime state как read boundary

#### Scenario: Более новая revision supersede-ит detached head snapshot
- **GIVEN** detached head snapshot опубликован для revision `V`
- **AND** приходит более новая revision `V+1`
- **WHEN** система публикует новый current-revision snapshot
- **THEN** detached snapshot `V` не используется как substitute для `V+1`
- **AND** latest-wins semantics сохраняются

## MODIFIED Requirements
### Requirement: v2 pipeline является единственным источником истины для вывода типов (MUST)
Система MUST использовать canonical IR как единственный semantic source of truth для IDE-функций (`completion`, `hover`, `signatureHelp`, `definition`, `diagnostics`, `type-at-position`).

Bounded set canonical derived semantic artifacts MUST строиться только из canonical IR snapshot:
- `CompletionHeadArtifact` или его detached immutable current-revision read model для initial completion response;
- `ExactSemanticArtifact` (`derived semantic index`) — full semantic artifact для exact completion и остальных interactive semantic запросов.

Detached current-revision head snapshot допустим только если:
- он строится только из canonical IR snapshot той же revision;
- invalidated по `(file_version, deps_id, settings_id)`;
- не использует stale payload другой revision как substitute;
- не маскирует shared runtime snapshot под immutable published read model.

Legacy-пути вывода типов MUST быть удалены (не поддерживаются), включая parse-result-based semantic inference paths, которые существуют параллельно canonical IR.

#### Scenario: Detached head snapshot остается canonical derived artifact
- **GIVEN** система публикует detached current-revision head snapshot для completion
- **WHEN** IDE запрашивает completion на этой revision
- **THEN** ответ происходит из canonical derived artifact той же revision
- **AND** detached snapshot не использует альтернативный semantic inference path вне canonical IR contract

#### Scenario: Detached snapshot не заменяет exact semantic truth для других операций
- **GIVEN** detached current-revision head snapshot уже опубликован
- **WHEN** IDE запрашивает `hover` или `definition`
- **THEN** сервер продолжает использовать exact semantic artifact
- **AND** не materialize-ит non-exact ответ из detached completion snapshot

### Requirement: Interactive latency budget защищается canonical fast path, а не fallback semantics (MUST)
Система MUST удовлетворять согласованным representative latency budgets для interactive semantic queries с использованием canonical IR и canonical derived semantic artifacts.

Для completion latency budget MAY соблюдаться через current-revision `CompletionHeadArtifact` или detached immutable head snapshot, но MUST NOT соблюдаться через stale, degraded или discovery-backed semantic substitute.

Если detached immutable head snapshot уже опубликован для current revision, first-response completion MUST иметь возможность читать его без обязательной зависимости от shared writer-owned mutable runtime state.

Если latency budget нарушен, система MUST оптимизировать canonical semantic path и MUST NOT возвращать stale, degraded или discovery-backed semantic substitute как механизм соблюдения latency.

#### Scenario: Published detached snapshot используется как canonical fast path
- **GIVEN** current-revision detached head snapshot уже опубликован
- **WHEN** IDE запрашивает completion
- **THEN** first-response completion может быть обслужен из detached canonical head snapshot
- **AND** системе не требуется masquerade shared runtime snapshot как immutable fast path

#### Scenario: Отсутствие detached snapshot не разрешает stale substitute
- **GIVEN** detached current-revision head snapshot еще не опубликован
- **AND** exact semantic artifact тоже не ready в пределах bounded policy
- **WHEN** IDE запрашивает completion
- **THEN** сервер завершает запрос fail-closed
- **AND** не использует stale или degraded semantic substitute
