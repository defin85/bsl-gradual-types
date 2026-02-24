# Change: Инкрементальный syntax diagnostics pipeline для больших модулей

## Why
`syntax_diagnostics` на больших модулях остается тяжелой стадией, потому что текущий путь опирается на полный parse для каждой новой ревизии текста.

Это создает алгоритмический bottleneck и ограничивает верхнюю границу ускорения completion/diagnostics при активном редактировании больших файлов.

## What Changes
- **ADDED**: инкрементальный синтаксический путь для последовательных ревизий одного файла с reuse предыдущего parse tree.
  - Используются edit-aware обновления дерева и incremental parse вместо полного parse на каждое `didChange`.
- **ADDED**: fail-safe policy для некорректных/неприменимых edits.
  - При невозможности корректного incremental update система MUST fallback на полный parse текущей ревизии без нарушения корректности diagnostics.
- **ADDED**: observability для incremental hit/miss/fallback причин и latency эффекта.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (planned):
  - `syntax/src/lib.rs`
  - `analysis-v2/src/lib.rs`
  - `backend/src/bin/lsp_server/server/language_server.rs`
  - `backend/src/bin/lsp_server/server/core.rs`
  - `bsl-runtime/src/system/basic_observability.rs`

## Out of Scope
- Изменение семантики синтаксических сообщений (rewrite rules и текст diagnostics).
- Изменение LSP wire-контракта для diagnostics/completion.
