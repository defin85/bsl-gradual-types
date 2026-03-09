## MODIFIED Requirements

### Requirement: IntelliSense v2 обеспечивает IDE‑grade completion по выражениям (MUST)
Система SHALL обеспечивать completion v2, который корректно работает для member access в выражениях и цепочках, включая неполный код:
- `Идентификатор.`
- `Вызов().`
- `Коллекция[...].`
- `(expr).`
- цепочки вида `a.b().c[d].e.`

Syntax extraction для неполного кода MAY использовать parse/syntax helpers, но semantic candidates MUST происходить только из canonical IR snapshot текущей revision и его derived semantic index.

Если canonical semantic artifacts для текущей revision недоступны, completion MUST работать fail-closed и MUST NOT синтезировать semantic candidates из stale cache, keyword fallback или альтернативного inference path.

#### Scenario: Completion на неполном коде использует canonical semantic path
- **GIVEN** пользователь набирает `expr.` и код может быть синтаксически неполным
- **WHEN** IDE запрашивает completion на позиции после `.`
- **THEN** система извлекает receiver-выражение syntax-aware способом
- **AND** semantic candidates читаются только из canonical IR snapshot и derived semantic index текущей revision

#### Scenario: Недоступность canonical artifacts не превращается в semantic fallback
- **GIVEN** для текущей revision canonical IR или derived semantic index ещё недоступны
- **WHEN** IDE запрашивает completion на позиции после `.`
- **THEN** сервер возвращает explicit empty/unavailable fail-closed response для этой revision
- **AND** сервер не возвращает stale, degraded или keyword-only semantic substitute

### Requirement: Инкрементальность и корректность позиций в v2 pipeline (MUST)
Система SHALL обеспечивать согласованность позиций между LSP (UTF-16), внутренними byte offsets и tree-sitter incremental parsing, чтобы completion не использовал semantic truth от другой revision после `didChange`.

Система SHALL гарантировать, что interactive semantic ответы после `didChange` опираются только на canonical artifacts текущей revision или fail-closed для этой revision.

#### Scenario: Первый completion после `didChange` не использует semantic truth предыдущей revision
- **GIVEN** пользователь вводит `expr.` и IDE отправляет `didChange` для новой версии документа
- **WHEN** IDE немедленно отправляет `textDocument/completion` в позиции после `.`
- **THEN** сервер отвечает exact semantic результатом для новой revision или fail-closed response для новой revision
- **AND** не использует stale semantic candidates от предыдущей revision как substitute

### Requirement: v2 pipeline является единственным источником истины для вывода типов (MUST)
Система MUST использовать canonical IR как единственный semantic source of truth для IDE-функций (`completion`, `hover`, `signatureHelp`, `definition`, `diagnostics`, `type-at-position`).

`derived semantic index` MUST быть единственным fast query артефактом для интерактивных semantic запросов и MUST строиться только из canonical IR snapshot.

Legacy-пути вывода типов MUST быть удалены (не поддерживаются), включая parse-result-based semantic inference paths, которые существуют параллельно canonical IR.

#### Scenario: Hover и completion используют canonical IR и derived semantic index
- **GIVEN** пользователь работает в IDE с `.bsl` файлом
- **WHEN** IDE запрашивает hover и completion в одной и той же позиции/контексте
- **THEN** ответы опираются на один canonical IR snapshot и derived semantic index той же revision
- **AND** не используют альтернативные semantic inference пути вне canonical IR contract

## ADDED Requirements

### Requirement: Canonical IR и derived semantic index образуют единый semantic core v2 (MUST)
Система MUST иметь единый semantic core вида `canonical IR -> derived semantic index`.

Canonical IR MUST содержать или однозначно порождать semantic facts, достаточные для:
- owner/member resolution;
- type-at-position;
- completion candidate semantics;
- definition/reference anchors, где требуется semantic ownership;
- diagnostics;
- flow-sensitive overlays через CFG.

`derived semantic index` MUST:
- строиться только из canonical IR snapshot текущей revision;
- быть детерминированной projection того же snapshot;
- не выполнять самостоятельный semantic inference;
- не читать `parse_result.program` как самостоятельный semantic source of truth.

#### Scenario: Один canonical IR snapshot даёт один semantic index для всех consumers
- **GIVEN** построен canonical IR snapshot конкретной revision
- **WHEN** система материализует derived semantic index для interactive queries
- **THEN** индекс является projection того же snapshot
- **AND** все consumers читают semantic facts из одного и того же IR-derived contract

#### Scenario: Derived semantic index не переизобретает semantic truth
- **GIVEN** canonical IR snapshot уже содержит owner/member/type truth для позиции
- **WHEN** derived semantic index строится для этой revision
- **THEN** индекс лишь денормализует lookup для fast queries
- **AND** не вычисляет альтернативный semantic результат из parse tree или отдельной эвристики

### Requirement: Canonical semantic queries fail-closed при недоступности артефактов (MUST)
Interactive semantic queries (`completion`, `hover`, `signatureHelp`, `definition`, `type-at-position`) MUST завершаться fail-closed, если canonical IR или derived semantic index текущей revision недоступны.

Fail-closed path MUST NOT:
- использовать stale semantic artifacts как substitute;
- возвращать keyword fallback как semantic answer;
- запускать альтернативный parse-result-based semantic inference path;
- усиливать semantic truth локальной adapter logic.

Observability MAY фиксировать bounded reason-code недоступности, но не MAY вводить отдельный fallback semantic path.

#### Scenario: Hover miss current revision остаётся fail-closed
- **GIVEN** canonical IR или derived semantic index текущей revision недоступны
- **WHEN** IDE запрашивает hover в позиции с member access
- **THEN** сервер возвращает empty/unavailable hover response
- **AND** не materialize-ит semantic ответ из альтернативного non-IR path

### Requirement: Applied-owner bare identifier fallback удалён из v2 semantics (MUST)
Система MUST NOT резолвить bare identifiers в `ObjectModule` / `RecordSetModule` через special applied-owner fallback вне canonical IR semantic binding model.

Если implicit module-context identifier semantics нужны продукту, они MUST быть представлены в canonical IR/binding model и одинаково доступны всем consumers.

#### Scenario: Bare identifier без canonical binding остаётся unresolved
- **GIVEN** код в `ObjectModule` или `RecordSetModule` содержит bare identifier, который не имеет canonical binding в текущем snapshot
- **WHEN** система выполняет type-at-position, hover или diagnostics для этого identifier
- **THEN** identifier остаётся unresolved согласно canonical semantic contract
- **AND** система не резолвит его через applied-owner fallback branch
