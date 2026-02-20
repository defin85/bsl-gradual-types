# Change: Повысить интерактивную надёжность completion v2 (latency + first-trigger + dot-trigger)

## Why
По фактическому поведению IDE при редактировании больших BSL-модулей наблюдаются три пользовательские проблемы:
- заметная задержка ответа `textDocument/completion`;
- первый completion после ввода часто не даёт полезного результата, и пользователь вынужден повторно вызывать completion (`Esc` -> `Ctrl+Space`);
- completion по `.` не срабатывает предсказуемо, тогда как ручной вызов работает.

Текущее поведение частично допускается существующим контрактом (stale/incomplete fallback), но не фиксирует UX-инвариант "первый интерактивный ответ должен быть полезным" и не задаёт явный trigger-aware контракт для `TriggerCharacter='.'`.

## What Changes
- Подход реализации для этого change фиксируется как **runtime-centric trigger contract + LSP adapter shadow state** (без полной реархитектуры очередей).
- **MODIFIED**: `bsl-intellisense-v2` requirement `Инкрементальность и корректность позиций в v2 pipeline (MUST)`.
  - Добавляется требование, что первый completion после `didChange` в member-access контексте не требует повторного ручного триггера для получения полезной выдачи.
- **ADDED**: новое requirement в `bsl-intellisense-v2` про trigger-aware completion контракт.
  - Сервер обязан учитывать `CompletionContext` (`TriggerCharacter`/`Invoked`) и сохранять semantic parity для completion по `.` и `Ctrl+Space` в одной ревизии документа.
- **ADDED**: новое requirement в `bsl-intellisense-v2` про bounded interactive latency и деградированный (но полезный) fallback.
  - В условиях гонки `didChange`/completion сервер обязан возвращать bounded response (без зависаний) и не отдавать terminal-empty ответ для распознаваемого member-access контекста.
- **MODIFIED**: `bsl-intellisense` requirement `VS Code extension запускает LSP и предоставляет IDE-интеграцию`.
  - Расширение обязано не ломать trigger-character completion для BSL и явно сигнализировать пользователю, если effective editor settings выключают автотриггеры completion.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
  - `bsl-intellisense`
- Affected code (planned):
  - `backend/src/bin/lsp_server/server/language_server.rs`
  - `backend/src/bin/lsp_server/server/mod.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade.rs`
  - `bsl-runtime/src/application/type_system/services/completion_service.rs`
  - `backend/src/bin/lsp_server/server/core.rs` (регрессионные/контрактные тесты)
  - `backend/tests/lsp_incremental_completion_test.rs`
  - `vscode-extension/src/lsp/client/*`

## Scope
- Change ограничен интерактивным completion pipeline (LSP + v2 runtime + VS Code client integration).
- В change не входят новые completion фичи (новые типы candidates, ranking-модель, расширение domain coverage).
- В change не входит полная event-driven rearchitecture очередей runtime/LSP; она вынесена в отдельный change: `refactor-v2-completion-event-driven-pipeline`.

## Acceptance Gates
- Интерактивный warm-path completion latency под профильной нагрузкой: `p95 <= 300ms`, `p99 <= 800ms`.
- First-trigger success rate для member-access (`expr.` сразу после `didChange`): `>= 99%` на регрессионном наборе.
- Terminal-empty rate для распознанного member-access контекста, вызванный только transient недоступностью IR: `<= 0.5%`.
- Parity mismatch rate между `TriggerCharacter='.'` и `Invoked` для одной ревизии/позиции: `<= 1%` на нагрузочном telemetry-срезе и `0` в контрактных тестах.
