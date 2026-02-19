## MODIFIED Requirements

### Requirement: Инкрементальность и корректность позиций в v2 pipeline (MUST)
Система SHALL обеспечивать согласованность позиций между LSP (UTF-16), внутренними byte offsets и tree-sitter incremental parsing, чтобы completion не деградировал после `didChange`.

Система SHALL гарантировать, что первый completion-запрос после `didChange` в member-access контексте не требует повторного ручного триггера только из-за transient рассинхронизации ревизий.

#### Scenario: Первый completion после `didChange` не требует повторного `Ctrl+Space`
- **GIVEN** пользователь вводит `expr.` и IDE отправляет `didChange` для новой версии документа
- **WHEN** IDE немедленно отправляет `textDocument/completion` в позиции после `.`
- **THEN** сервер возвращает полезный completion-ответ в первом запросе (релевантные candidates или деградированный `isIncomplete=true` fallback)
- **AND** пользователь не обязан повторно вызывать completion вручную, чтобы получить первый осмысленный список

## ADDED Requirements

### Requirement: Completion v2 учитывает trigger context LSP и сохраняет parity между trigger modes (MUST)
Система MUST использовать `CompletionParams.context` (`TriggerCharacter`, `Invoked`, `TriggerForIncompleteCompletions`) как часть completion policy.

Для одной и той же ревизии документа и позиции member-access:
- completion по `TriggerCharacter='.'` MUST использовать тот же semantic контекст owner/member resolution, что и `Invoked`;
- `TriggerCharacter='.'` MUST NOT деградировать в нерелевантный keyword-only ответ только из-за trigger mode;
- различия между trigger modes допускаются только в пределах explicit degraded semantics (`isIncomplete=true`).

#### Scenario: `TriggerCharacter='.'` и `Invoked` дают согласованный member-access контекст
- **GIVEN** курсор стоит в позиции после `expr.`
- **AND** текст/ревизия документа не менялись между запросами
- **WHEN** IDE запрашивает completion сначала как `TriggerCharacter='.'`, затем как `Invoked`
- **THEN** оба ответа используют согласованный receiver/member semantic контекст
- **AND** ответ по `TriggerCharacter='.'` не сводится к нерелевантной keyword-only выдаче

### Requirement: Интерактивный completion v2 имеет bounded latency и полезную деградацию (MUST)
Система MUST обеспечивать bounded completion response при typing-load (`didChange` bursts) и MUST NOT зависать из-за ожидания консистентности ревизий.

Если fresh semantic данные временно недоступны, система MUST возвращать полезный degraded completion для распознанного member-access контекста с `isIncomplete=true`, вместо terminal-empty ответа, вызванного только transient недоступностью IR.

Система MUST сохранять observability по стадиям latency/fallback, достаточную для регрессионного контроля интерактивного пути, включая:
- разрез по trigger mode (`TriggerCharacter`, `Invoked`, `TriggerForIncompleteCompletions`, `None`),
- parity drift индикаторы между `TriggerCharacter='.'` и `Invoked`,
- счётчики transient member-access terminal-empty и fallback_unavailable.

#### Scenario: Под серией `didChange` completion остаётся bounded и не даёт transient terminal-empty
- **GIVEN** пользователь быстро печатает, и IDE отправляет серию `didChange`
- **WHEN** IDE запрашивает completion в member-access контексте во время transient нагрузки
- **THEN** completion возвращается в bounded интерактивном времени (без зависания)
- **AND** при временной недоступности fresh IR сервер возвращает degraded `isIncomplete=true` ответ вместо terminal-empty результата, если контекст позволяет полезные candidates

#### Scenario: Trigger-aware observability доступна для контроля parity
- **GIVEN** IDE выполняет completion запросы в режимах `TriggerCharacter='.'`, `Invoked` и `TriggerForIncompleteCompletions`
- **WHEN** система публикует observability метрики completion
- **THEN** метрики содержат trigger mode разрез и позволяют сравнить parity между `.` и `Invoked`
- **AND** transient member-access terminal-empty и fallback_unavailable отражаются отдельными счётчиками
