# План реализации M1: Позиции и инкрементальный парсинг (UTF‑16/byte/tree‑sitter)

**Статус:** ✅ РЕАЛИЗОВАНО  
**Цель:** обеспечить строгую согласованность позиций между VS Code (UTF‑16), внутренними вычислениями (byte offsets) и tree‑sitter (`InputEdit`/`Point`), чтобы completion/hover/diagnostics не деградировали после `didChange`.

**Код:** `backend/src/system/positioning.rs`  
**Интеграция:** `backend/src/system/parser_coordinator.rs`, `backend/src/bin/lsp_server/handlers/text_document.rs`  
**Тесты:** `backend/tests/incremental_parsing_test.rs`  

**Проверка:**  
- `cargo test -p bsl-backend --test incremental_parsing_test`  
- `cargo test -p bsl-backend --lib positioning`  
- `cargo test -p bsl-backend --test utf16_span_extraction_test`  
- `./scripts/run-intellisense-tests.sh smoke`  

---

## Область работ

- Единая стратегия конвертации позиций:
  - `Position (line, character UTF‑16)` ↔ byte offset в UTF‑8 строке/документе
  - `TextDocumentContentChangeEvent` → корректный `InputEdit` для tree‑sitter
- Согласованность `Point.column` (byte column) и byte offsets
- Тесты на кириллицу/emoji и многострочные правки

---

## Пошаговый план

### Шаг 1: Аудит текущих конвертеров и инвариантов
- Собрать все места конвертации (LSP position ↔ byte offset) и описать, что именно “считается колонкой” (UTF‑16/char/byte).
- Зафиксировать инварианты для tree‑sitter:
  - `start_byte/old_end_byte/new_end_byte` — byte offsets
  - `Point.row` — line
  - `Point.column` — byte column в строке

**Выход:** документированный набор инвариантов + список точек рассинхронизации.

---

### Шаг 2: Единый модуль позиционирования
- Выделить один модуль/набор функций, которые используются везде (completion, hover, incremental parsing).
- Запретить “локальные” конвертации через `.len()`/`chars()` без явного контекста (UTF‑16 vs byte).

**Выход:** единый API для позиционных конвертаций.

---

### Шаг 3: Исправление `TextEdit → InputEdit`
- Обеспечить корректный расчёт `new_end_line/new_end_column` для `didChange`:
  - `new_end_*` должны соответствовать фактическому новому тексту в координатах, которые ожидает следующий слой.
- Привести `position_to_byte()` и расчёт `Point.column` к byte columns.

**Выход:** `InputEdit` соответствует tree‑sitter ожиданиям на любых Unicode строках.

---

### Шаг 4: Тесты инкрементального парсинга
- Набор кейсов:
  - кириллица + emoji в одной строке
  - вставка/удаление в середине строки
  - многострочная вставка/удаление
  - правки вокруг точки `expr.` и внутри строк/комментариев
- Проверка: incremental parse даёт эквивалентный результат “полного” парса (на уровне дерева или AST/IR).

**Выход:** тесты, которые ловят регрессии позиционирования.

---

## Критерии завершения

- Для Unicode‑кейсов `didChange` не ломает tree‑sitter дерево.
- Completion/hover продолжают работать после серии правок, без “случайных” провалов в fallback.
- Есть тестовое покрытие, которое воспроизводит проблемные кейсы.

---

## Задачи (тикеты) по M1

### T1: Инвентаризация конвертеров позиций ✅
**Цель:** перечислить и классифицировать все преобразования позиций.  
**DoD (выполнено):**
- **LSP UTF‑16 → byte offsets (UTF‑8):** `backend/src/system/positioning.rs` (`utf16_to_byte_offset`, `LineIndex::utf16_position_to_byte_offset`).
- **tree‑sitter byte offsets/byte columns → LSP UTF‑16:** `backend/src/system/tree_sitter_adapter/span.rs`, `backend/src/system/tree_sitter_adapter/syntax_errors.rs`.
- **didChange → инкрементальный парсинг:** `backend/src/bin/lsp_server/handlers/text_document.rs` (формирование `TextEdit`), `backend/src/system/parser_coordinator.rs` (`TextEdit → InputEdit`).

### T2: Единый API позиционирования ✅
**Цель:** один источник истины для позиционных преобразований.  
**DoD (выполнено):**
- добавлен `backend/src/system/positioning.rs` и переиспользован в LSP/инкрементальном парсинге/span extraction;
- убраны “локальные” реализации `LineIndex` и пересчёты колонок через `.len()` без учёта UTF‑16/bytes.

### T3: Корректный `InputEdit` для tree‑sitter ✅
**Цель:** `TextEdit → InputEdit` корректен для Unicode и многострочных правок.  
**DoD (выполнено):**
- исправлен расчёт edits в `didChange` (LSP передаёт UTF‑16 start/end + `new_text`, остальное вычисляется ниже);
- `TextEdit → InputEdit` считает byte offsets + byte columns через `LineIndex` и учитывает многострочные вставки/удаления;
- добавлены тесты Unicode/multiline/2 edits: `backend/tests/incremental_parsing_test.rs`.

### T4: Документация инвариантов ✅
**Цель:** чтобы будущие изменения не ломали позиционирование.  
**DoD (выполнено):**
- инварианты зафиксированы в `backend/src/system/positioning.rs` (LSP UTF‑16, byte offsets, tree‑sitter `Point.column` = byte column);
- примеры и проверки покрыты тестами: `backend/src/system/positioning.rs`, `backend/tests/incremental_parsing_test.rs`, `backend/tests/utf16_span_extraction_test.rs`.
