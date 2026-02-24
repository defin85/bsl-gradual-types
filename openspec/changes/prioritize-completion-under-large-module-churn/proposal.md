# Change: Приоритет интерактивного completion при churn на больших модулях

## Why
На больших модулях warm-path completion остается дорогим: в текущем baseline зафиксировано `p95(completion_duration_ms)=3910ms` и `p95(wait_for_file_version_completion_ms)=3024ms`.

При этом heavy diagnostics в `didChange` контуре продолжают создавать конкурентную нагрузку в момент активного ввода, из-за чего интерактивный путь completion получает длинный хвост ожидания.

## What Changes
- **ADDED**: scale-aware policy для `large + churn` режима в LSP/runtime orchestration.
  - В режиме активного churn на большом документе `didChange` обслуживается только fast-профилем.
  - Heavy diagnostics переносится на `idle`/`didSave` и не конкурирует с completion на каждый символ.
- **ADDED**: явный интерактивный приоритет для completion/hover/signatureHelp в runtime scheduling.
  - Интерактивные stateful операции обслуживаются раньше background diagnostics задач.
  - Сохраняется fairness для background-потока, чтобы не допустить starvation diagnostics.
- **ADDED**: observability-контракт для scale-aware режима и причин деградации.
  - Метрики фиксируют вход/выход из режима `large + churn`, а также причины отложенного heavy-path.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (planned):
  - `backend/src/bin/lsp_server/server/language_server.rs`
  - `backend/src/bin/lsp_server/server/core.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade.rs`
  - `bsl-runtime/src/application/intellisense_v2/policy.rs`
  - `bsl-runtime/src/system/basic_observability.rs`

## Out of Scope
- Алгоритмическая оптимизация синтаксического парсинга (`tree-sitter old_tree reuse`).
- Изменение семантики completion candidates или ranking.
