## ADDED Requirements

### Requirement: Exact completion IR work для current revision шарится между request path и prewarm (MUST)
Для одинакового revision key `(file_id, file_version, deps_id, settings_id)` система MUST выполнять не более одного exact IR build, пригодного для current-revision completion `ExactSemanticArtifact`, даже если compute инициирован разными orchestration path.

Этот contract MUST означать, что:

- interactive exact completion request и background current-revision prewarm используют один и тот же revision-bound flight для IR/exact prerequisite;
- request MAY присоединяться follower'ом к уже запущенному prewarm flight той же revision;
- prewarm MAY reuse request-started flight той же revision вместо запуска duplicate compute;
- same-revision duplicate exact IR build MUST NOT выполняться конкурентно только из-за различного caller path (`request` vs `prewarm`).

#### Scenario: Request присоединяется к уже идущему same-revision prewarm

- **GIVEN** background prewarm уже запустил exact IR flight для revision key `K`
- **WHEN** interactive completion request для той же revision требует exact semantic artifact
- **THEN** request attach-ится к уже идущему flight как follower
- **AND** система не запускает второй exact IR build для `K`

#### Scenario: Background prewarm не дублирует request-started exact IR build

- **GIVEN** interactive completion request уже стал leader exact IR flight для revision key `K`
- **WHEN** background current-revision prewarm для той же revision стартует позже
- **THEN** prewarm reuse-ит уже идущий flight
- **AND** не создаёт второй concurrent exact IR build для `K`

### Requirement: Superseded exact IR build boundedly сворачивается до publish и не пишет partial artifacts (MUST)
Если same-file exact IR / exact semantic build потерял latest-wins из-за более новой revision или explicit cancel, система MUST boundedly остановить устаревший build на cooperative checkpoints внутри exact compute envelope до публикации результата.

Этот contract MUST означать, что:

- stale build MAY дойти только до ближайшего internal checkpoint внутри AST->IR, exact facts traversal или эквивалентной крупной exact stage;
- superseded/cancelled build MUST NOT публиковать `ExactSemanticArtifact` или IR stale revision как latest result;
- partial или unfinished IR / semantic facts MUST NOT записываться в shared derived cache как successful artifact;
- более новая revision MAY стартовать свой revision flight независимо от того, успел ли stale build завершить unwind.

#### Scenario: Более новая revision supersede-ит exact IR build до publish

- **GIVEN** exact IR build для revision `V` уже выполняется
- **AND** для того же файла приходит более новая revision `V+1`
- **WHEN** build для `V` достигает ближайшего cooperative checkpoint внутри exact compute
- **THEN** build для `V` завершает stale unwind без publish current result
- **AND** latest serving не использует artifact для `V` как substitute для `V+1`

#### Scenario: Explicit cancel не оставляет partial exact artifact в cache

- **GIVEN** interactive exact completion уже вошёл в exact IR / facts build
- **WHEN** request получает explicit cancel
- **THEN** request завершает exact path terminal cancelled outcome на ближайшем cooperative checkpoint
- **AND** shared cache не получает partial successful artifact от этого build
