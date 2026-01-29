# P3: TODO list — ParseResult как query (tree-sitter)

**Дата:** 2026-01-09  
**Статус:** 🟢 DONE  
**Основание:** Фаза P3 из `docs/roadmap/intellisense-v2-roadmap/architecture-intermediate/salsa-migration-plan.md`

## Цель P3

- Ввести salsa query `parse_result(FileId) -> ParseResult`, которая является **чистой функцией** от:
  - `file_text(FileId)` (и связанных с ним инвариантов позиционирования),
  - `settings` (минимум — dependency на `settings_id`, лучше — на конкретные parse options).
- Убрать I/O и скрытую мутабельность из синтаксического парсинга на пути v2.
- Сохранить поведение “partial recovery”: даже при синтаксических ошибках возвращаем AST, плюс `syntax_errors`.

Не-цели P3:
- Инкрементальный tree-sitter по `InputEdit` (можно добавить позже, если потребуется по профилированию).
- Disk cache/AST cache на уровне query (salsa memoization достаточно для начала).

## Контракт (инварианты)

- **Determinism:** одинаковые `(text, settings)` → одинаковый `ParseResult` (включая порядок ошибок).
- **No I/O:** внутри query нельзя читать файлы/метаданные/конфиги; всё внешнее — inputs.
- **UTF-16 корректность:** `ParseError.span` хранится как **UTF-8 byte offsets**; на границе (LSP/web)
  конвертируется в UTF‑16 позиции через единый `LineIndex`.
- **Snapshot safety:** query не читает/пишет глобальные mutable кэши; никаких “скрытых” `static mut`/singleton caches.

## Внешние референсы

- tree-sitter editing / incremental parsing (`InputEdit`):
  - https://tree-sitter.github.io/tree-sitter/using-parsers#editing
- tree-sitter Rust bindings:
  - https://docs.rs/tree-sitter/latest/tree_sitter/

## Локальные референсы (в репо)

- Legacy парсинг + кэши + инкрементальность:
  - `backend/src/system/parser_coordinator.rs`
  - `backend/src/system/tree_cache.rs`, `backend/src/system/ast_cache.rs`, `backend/src/system/disk_cache.rs`
- Конвертация tree-sitter дерева в `ParseResult` (+ синтаксические ошибки/эвристики):
  - `backend/src/system/tree_sitter_adapter/mod.rs`
  - `backend/src/system/tree_sitter_adapter/syntax_errors.rs`
- Текущий `ParseResult` и AST структуры (сейчас в backend):
  - `backend/src/parsing/bsl/mod.rs`
- v2 salsa DB и паттерны `Arc<...>` результатов:
  - `analysis-v2/src/lib.rs`
- Позиционирование (единая реализация): `line-index/src/lib.rs`

## Решение по реализации (фиксируем перед кодом)

### 1) Где живёт `ParseResult` и AST

Чтобы `bsl-analysis-v2` мог возвращать `ParseResult`, но при этом не зависеть от `bsl-backend`,
нужно вынести syntax AST и tree-sitter adapter **из backend** в отдельный crate.

Рекомендация: создать workspace-crate `bsl-syntax` (или аналог), который:
- содержит AST (`Program/Statement/Expression`) и `ParseResult`,
- содержит tree-sitter adapter (конвертацию дерева в AST + сбор `ParseError`),
- зависит от `bsl-shared` (Span/ParseError/ErrorType) и `tree-sitter/tree-sitter-bsl`.

`bsl-backend` и `bsl-analysis-v2` зависят от `bsl-syntax` и используют один и тот же парсер/AST.

### 2) Что именно возвращает query

Query `parse_result` возвращает **только** данные синтаксиса:
- `program` (AST),
- `syntax_errors` (`Vec<ParseError>`).

Возвращаем `Arc<ParseResult>` (как `line_index`), чтобы не копировать большие структуры.

### 3) От чего зависит query

`parse_result(FileId)` зависит от:
- `file_text(FileId)`
- `settings` (минимум — читаем `settings_id()` внутри query, чтобы гарантировать инвалидацию при смене настроек).

Query НЕ зависит от `deps_id` (иначе лишняя инвалидация и риск “пересчёта всего”).

### 4) Как создаём tree-sitter Parser (без contention)

Избегаем глобального `Mutex<Parser>` в query (иначе параллельные запросы сериализуются).

Рекомендация для P3:
- создавать новый `tree_sitter::Parser` внутри вычисления query (простая корректность),
- либо использовать `thread_local!` Parser (оптимизация без shared contention).

## TODO (шаги)

### 1) Вынести syntax AST + tree-sitter adapter в `bsl-syntax`

- [x] Добавить новый workspace member (например, `syntax/`), package: `bsl-syntax`, lib: `bsl_syntax`.
- [x] Перенести типы AST и `ParseResult` из `backend/src/parsing/bsl/mod.rs` в новый crate:
  - [x] сохранить публичные имена (через re-export в backend на переходный период).
- [x] Перенести `backend/src/system/tree_sitter_adapter/` в новый crate и разорвать зависимости от backend:
  - [x] использовать `bsl-line-index` для UTF‑16 ↔ byte conversions,
  - [x] оставить сбор ошибок (`collect_syntax_errors_cached`, semicolon/new эвристики) внутри `bsl-syntax`.
- [x] Определить API `bsl-syntax`:
  - [x] `parse(source: &str, options: &ParseOptions) -> Result<ParseResult, ParseFatalError>`
  - [x] `parse_fast(source: &str) -> Result<ParseResult, ParseFatalError>` (для индексаторов, если нужно)
- [x] Обновить backend-использования:
  - [x] оставлен thin-wrapper `backend/src/system/tree_sitter_adapter/mod.rs`, который re-export'ит `bsl_syntax::tree_sitter_adapter::*` (минимальный дифф по импортам).

### 2) Добавить `parse_result` salsa query в `bsl-analysis-v2`

- [x] Добавить зависимости в `analysis-v2/Cargo.toml`:
  - [x] `bsl-syntax` (tree-sitter остаётся внутренней деталью `bsl-syntax`).
- [x] Добавить tracked query:
  - [x] `parse_result(db, file: SourceFile, settings: SettingsSnapshot) -> ParseResultSnapshot(Arc<ParseResult>)`
  - [x] гарантировать dependency на settings (прочитать `settings.id(db)`).
- [x] Добавить публичный метод в `AnalysisV2`:
  - [x] `parse_result(FileId) -> Cancellable<Option<Arc<ParseResult>>>`.

### 3) Минимальная интеграция / smoke usage

- [x] Добавить “smoke” использование query в v2 ветке (под флагом), например:
  - [x] логировать количество `syntax_errors` (без публикации диагностик).
  - (цель — убедиться, что query реально считается и инвалидация работает).

### 4) Тесты (обязательная часть P3)

- [x] Юнит‑тесты в `bsl-syntax`:
  - [x] валидный код → `syntax_errors.is_empty()`.
  - [x] заведомо сломанный код → есть ошибки, но `program` всё равно возвращается (partial recovery).
  - [x] UTF‑16 edge cases (кириллица/emoji) → `Span` попадает в ожидаемую позицию (минимум smoke).
  - [x] детерминизм: одинаковый вход → одинаковый порядок ошибок.
- [x] Интеграционные тесты в `bsl-analysis-v2`:
  - [x] изменение `file_text` инвалидирует `parse_result`.
  - [x] смена `settings_id` инвалидирует `parse_result` (даже если options пока не материализованы).
  - [x] `RemoveFile` → `parse_result` возвращает `None`.

## DoD (P3 считается закрытым, если)

- [x] В `bsl-analysis-v2` есть query `parse_result` и публичный read API.
- [x] Query не делает I/O и не использует скрытую мутабельность (проверено по коду и `rg`).
- [x] `bsl-syntax` покрыт тестами на partial recovery + UTF‑16 spans.
- [x] `cargo test -p bsl-analysis-v2` проходит.
- [x] `cargo test --workspace` проходит.

## Реализация (где смотреть в коде)

- `syntax/src/lib.rs`: API `parse/parse_fast`, `ParseOptions`, `ParseFatalError`.
- `syntax/src/ast.rs`: AST и `ParseResult`.
- `syntax/src/tree_sitter_adapter/`: адаптер tree-sitter → AST + `ParseError` (UTF‑16 spans через `bsl-line-index`).
- `analysis-v2/src/lib.rs`: tracked query `parse_result` + `AnalysisV2::parse_result`.
- `backend/src/system/tree_sitter_adapter/mod.rs`: thin-wrapper re-export для совместимости импортов backend.
- `backend/src/bin/lsp_server/server/language_server.rs`: smoke-лог под `BSL_INTELLISENSE_V2_P3_SMOKE`.
