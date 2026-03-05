# Change: Устранить двойной full-index на старте VS Code extension

## Why
Сейчас при активации extension могут запускаться два тяжёлых full-flow процесса подряд:
1) startup загрузка типов в LSP (`bsl-load-types`),
2) отдельный `bsl/buildIndex` из extension.

Это увеличивает холодный старт, создаёт лишнюю нагрузку на CPU/IO и путает пользователя (визуально выглядит как повторная индексация без явной причины).

Дополнительно extension использует файловый sentinel (`project_indices/.../unified_index.json`) как сигнал готовности, но фактическое состояние индекса живёт на стороне runtime/LSP. Из-за этого локальная проверка может давать ложный cache-miss и провоцировать лишний full build.

## What Changes
- Добавить явный server-driven контракт состояния индекса для LSP клиента (`index state`), пригодный для принятия решения на старте.
- Зафиксировать single-flight поведение full-index: при уже идущем startup/build повторный full build не запускается.
- Зафиксировать startup-орchestration в extension: решение о запуске full build принимается по серверному состоянию, а не по локальному файловому sentinel.
- Зафиксировать, что auto-reindex контур остаётся incremental-only и не подменяется full build на старте без необходимости.

## Resolved Decisions (2026-03-05)
- Контракт `bsl/getIndexState` фиксируется в версии `v1` с machine-readable полями:
  - `version`, `state`, `ready`, `active_operation`, `operation_id`, `message`, `updated_at_ms`.
  - `active_operation`, `operation_id`, `message` всегда присутствуют в payload; при отсутствии значения передаются как `null`.
- Политика гонки `startup` vs `bsl/buildIndex` фиксируется как strict single-flight:
  - один leader;
  - повторные запросы `bsl/buildIndex` во время `running` attach к текущей операции и не запускают новый full-index.
- Политика совместимости с legacy LSP (без `bsl/getIndexState`) фиксируется как fail-closed для startup auto-index:
  - extension не запускает silent full build на старте;
  - extension показывает явное предупреждение;
  - ручной `Build Index` остаётся доступен.
- Fail-safe антизалипание `running` фиксируется через watchdog timeout:
  - default: `hard_timeout_ms = 1200000` (20 минут);
  - по timeout состояние переводится в `failed`.
- UX для ручного `Build Index` во время `running`:
  - показывается информационный статус "already running (attached)";
  - второй progress и второй full-index не создаются.

## Impact
- Affected specs: `bsl-intellisense`
- Affected code (implementation follow-up):
  - `backend/src/bin/lsp_server/server/language_server/impl_init_config.rs`
  - `backend/src/bin/lsp_server/server/command_handlers.rs`
  - `backend/src/bin/lsp_server/main.rs`
  - `backend/src/bin/lsp_server/types.rs`
  - `vscode-extension/src/extension.ts`
  - `vscode-extension/src/lsp/customRequests.ts`
  - `vscode-extension/src/commands/index-commands.ts`

## Non-Goals
- Переписывание всего pipeline индексации или миграция на новый формат кэша в рамках этого change.
- Изменение семантики ручной команды `Build Index` (кроме дедупликации параллельного full build).
- Изменение бизнес-логики диагностик/резолюции типов.
