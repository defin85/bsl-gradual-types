## MODIFIED Requirements

### Requirement: IntelliSense v2 обеспечивает IDE‑grade completion по выражениям (MUST)
Система SHALL обеспечивать completion v2, который корректно работает для member access в выражениях и цепочках, включая неполный код:
- `Идентификатор.`
- `Вызов().`
- `Коллекция[...].`
- `(expr).`
- цепочки вида `a.b().c[d].e.`

Syntax extraction для неполного кода MAY использовать parse/syntax helpers, но semantic candidates для completion MUST происходить только из canonical IR snapshot текущей revision и его canonical derived completion artifacts.

Для completion допускается bounded set canonical current-revision artifacts:
- `CompletionHeadArtifact` — fast artifact для initial completion response;
- `ExactSemanticArtifact` (`derived semantic index`) — full exact semantic artifact для enriched completion и других interactive semantic операций.

Оба артефакта MUST:
- строиться только из canonical IR snapshot той же revision;
- invalidated по `(file_version, deps_id, settings_id)`;
- не использовать stale payload другой revision как substitute.

`CompletionHeadArtifact` для текущей revision MUST быть publishable и queryable независимо от ready-state `ExactSemanticArtifact` той же revision. Completion MUST NOT оставаться effectively `exact-only` только потому, что exact artifact ещё не достроен после нового `didChange`.

Если current-revision `CompletionHeadArtifact` и `ExactSemanticArtifact` недоступны, completion MUST работать fail-closed и MUST NOT синтезировать semantic candidates из stale cache, keyword fallback или альтернативного inference path.
Система MUST NOT возвращать semantic candidates другой revision под видом current-revision completion ответа.

#### Scenario: Completion после новой revision может вернуться из current-revision completion head artifact
- **GIVEN** пользователь только что создал новую requested revision через `didChange`
- **AND** exact semantic artifact текущей revision ещё не ready
- **AND** current-revision `CompletionHeadArtifact` уже ready
- **WHEN** IDE запрашивает completion на позиции после `.`
- **THEN** сервер возвращает semantic completion response из `CompletionHeadArtifact` той же revision
- **AND** не использует stale semantic payload другой revision

#### Scenario: Недоступность current-revision completion artifacts не превращается в semantic fallback
- **GIVEN** для текущей revision недоступны и `CompletionHeadArtifact`, и `ExactSemanticArtifact`
- **WHEN** IDE запрашивает completion на позиции после `.`
- **THEN** сервер возвращает explicit empty/unavailable fail-closed response для этой revision
- **AND** сервер не возвращает stale, degraded или keyword-only semantic substitute

### Requirement: v2 pipeline является единственным источником истины для вывода типов (MUST)
Система MUST использовать canonical IR как единственный semantic source of truth для IDE-функций (`completion`, `hover`, `signatureHelp`, `definition`, `diagnostics`, `type-at-position`).

Bounded set canonical derived semantic artifacts MUST строиться только из canonical IR snapshot:
- `CompletionHeadArtifact` — fast query artifact только для initial completion response;
- `ExactSemanticArtifact` (`derived semantic index`) — full semantic artifact для exact completion и остальных interactive semantic запросов.

Legacy-пути вывода типов MUST быть удалены (не поддерживаются), включая parse-result-based semantic inference paths, которые существуют параллельно canonical IR.

#### Scenario: Completion head и exact artifact используют один canonical snapshot
- **GIVEN** пользователь работает в IDE с `.bsl` файлом
- **WHEN** IDE запрашивает completion, а затем hover в том же current-revision контексте
- **THEN** completion head и exact semantic artifact опираются на один canonical IR snapshot той же revision
- **AND** не используют альтернативные semantic inference пути вне canonical IR contract

### Requirement: LSP interactive операции v2 используют bounded wait + fail-closed freshness policy (MUST)
Для `completion`, `hover`, `signatureHelp` система MUST применять freshness policy:
- сначала пытаться обслужить `requested file version` по фактически `applied_version`;
- ждать не дольше `intellisense_v2_interactive_wait_budget_ms` (дефолт `120ms`, если ключ не задан);
- после исчерпания wait budget завершать запрос fail-closed для текущей revision без stale semantic substitute.

Runtime knob MUST валидироваться и приводиться к диапазону:
- `intellisense_v2_interactive_wait_budget_ms` в диапазон `[10, 2000]`.

Snapshot с несовпадающими `deps_id` или `settings_id`, а также snapshot предыдущей revision, MUST NOT использоваться как semantic substitute для interactive ответа.

Дополнительно для completion:
- completion MUST ждать bounded время current-revision `CompletionHeadArtifact` или `ExactSemanticArtifact`;
- readiness/publish path для current-revision `CompletionHeadArtifact` MUST NOT блокироваться ожиданием ready exact semantic artifact той же revision;
- если `CompletionHeadArtifact` ready внутри wait budget, completion MAY вернуть current-revision semantic response из него;
- если `ExactSemanticArtifact` ready внутри wait budget, completion MAY использовать exact semantic response напрямую;
- если внутри wait budget не ready ни один current-revision completion artifact, completion MUST завершиться fail-closed;
- exact precompute MAY продолжаться после first response, но MUST NOT менять revision ответа задним числом, MUST NOT маскировать stale semantic path как acceptable substitute и MUST NOT превращать completion под `revision-churn` обратно в `exact-only` wait path, если head artifact уже ready.

#### Scenario: Completion после правки использует current-revision head artifact без stale substitute
- **GIVEN** пользователь ввёл новую строку и `received_version=V+1`, но exact semantic artifact для `V+1` ещё не ready
- **AND** current-revision `CompletionHeadArtifact` для `V+1` успел построиться в wait budget
- **WHEN** IDE запрашивает completion
- **THEN** сервер возвращает non-empty semantic completion response для версии `V+1`
- **AND** не возвращает semantic payload версии `V` под видом текущего результата

#### Scenario: Последовательные didChange не возвращают completion к exact-only зависимости
- **GIVEN** пользователь последовательно создаёт новые requested revisions `V+1` и `V+2`
- **AND** для `V+2` current-revision `CompletionHeadArtifact` ready внутри wait budget
- **AND** `ExactSemanticArtifact` для `V+2` ещё не ready
- **WHEN** IDE запрашивает completion на `V+2`
- **THEN** сервер возвращает current-revision completion response из `CompletionHeadArtifact` для `V+2`
- **AND** не продолжает ждать exact artifact только потому, что completion выполняется после очередного `didChange`

#### Scenario: Нет current-revision completion artifacts в пределах wait budget
- **GIVEN** requested версия ещё не ready ни по `CompletionHeadArtifact`, ни по exact semantic artifact
- **WHEN** IDE запрашивает completion
- **THEN** сервер не блокируется дольше wait budget
- **AND** сервер не использует snapshot предыдущей revision как semantic substitute

### Requirement: Canonical semantic queries fail-closed при недоступности артефактов (MUST)
Interactive semantic queries (`completion`, `hover`, `signatureHelp`, `definition`, `type-at-position`) MUST завершаться fail-closed, если для них недоступен необходимый canonical current-revision artifact.

Требуемые артефакты:
- `completion` -> `CompletionHeadArtifact` ИЛИ `ExactSemanticArtifact`;
- `hover`, `signatureHelp`, `definition`, `type-at-position` -> `ExactSemanticArtifact`.

Fail-closed path MUST NOT:
- использовать stale semantic artifacts как substitute;
- возвращать semantic payload предыдущей revision под видом ответа для текущей revision;
- возвращать keyword fallback как semantic answer;
- запускать альтернативный parse-result-based semantic inference path;
- усиливать semantic truth локальной adapter logic.

#### Scenario: Hover miss current revision остаётся fail-closed
- **GIVEN** exact semantic artifact текущей revision недоступен
- **WHEN** IDE запрашивает hover в позиции с member access
- **THEN** сервер возвращает empty/unavailable hover response
- **AND** не materialize-ит semantic ответ из `CompletionHeadArtifact` или другого non-exact пути

#### Scenario: Completion head current revision допустим, stale exact другой revision недопустим
- **GIVEN** `CompletionHeadArtifact` для текущей revision ready
- **AND** exact semantic artifact ready только для предыдущей revision
- **WHEN** IDE запрашивает completion
- **THEN** сервер использует только current-revision `CompletionHeadArtifact` или fail-closed
- **AND** не использует exact artifact предыдущей revision как substitute

### Requirement: Interactive latency budget защищается canonical fast path, а не fallback semantics (MUST)
Система MUST удовлетворять согласованным representative latency budgets для interactive semantic queries с использованием canonical IR и canonical derived semantic artifacts.

Для completion latency budget MAY соблюдаться через current-revision `CompletionHeadArtifact`, но MUST NOT соблюдаться через stale, degraded или discovery-backed semantic substitute.

Если latency budget нарушен, система MUST оптимизировать canonical semantic path и MUST NOT возвращать stale, degraded или discovery-backed semantic substitute как механизм соблюдения latency.

#### Scenario: Representative large-module completion использует canonical head path, а не stale rescue
- **GIVEN** representative large real module
- **WHEN** команда исправляет latency interactive completion
- **THEN** first-response completion приходит из current-revision `CompletionHeadArtifact` или `ExactSemanticArtifact`
- **AND** merge-state не вводит stale/degraded/search-backed semantic substitute как perf workaround

## ADDED Requirements

### Requirement: Representative real-module gate проверяет current-revision first-response availability для completion (MUST)
Acceptance для архитектурных изменений completion MUST включать representative gate на реальном workspace module, а не только synthetic URI harness.

Этот gate MUST:
- открывать реальный модуль из representative large configuration;
- проверять отдельно `same-revision warm` member-access completion и `revision-churn` completion после нового `didChange` перед каждым measured sample;
- отдельно учитывать first-response availability и exact upgrade latency;
- fail-ить, если completion после новой revision снова деградирует в `fail_closed`, несмотря на наличие current-revision canonical fast path.

#### Scenario: Real-module gate ловит регрессию first-response availability
- **GIVEN** representative real module из большой конфигурации открыт в live gate
- **AND** gate применяет новый `didChange` перед каждым measured completion в `revision-churn` профиле
- **WHEN** выполняется member-access completion
- **THEN** gate требует `ok_non_empty` first response из current-revision canonical artifact
- **AND** gate фиксирует exact upgrade отдельно, не маскируя им first-response availability
