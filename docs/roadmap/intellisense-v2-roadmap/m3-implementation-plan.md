# План реализации M3: CompletionTarget (expression receiver под курсором)

**Статус:** ✅ РЕАЛИЗОВАНО  
**Цель:** корректно определять receiver‑выражение для completion в позициях вида `expr.` (в том числе для незавершённого/частично сломанного кода) и выдавать подсказки по реальному типу receiver’а.

**Примечание:** базовый “document semantic state” уже реализован через v2 engine (см. M2):
`AnalysisV2` snapshot + queries (`parse_result/ir/line_index`). M3 строится поверх этих данных.

---

## Область работ

- Определение вида completion запроса:
  - member access (`.`)
  - call context (`(`)
  - другие контексты (statement, type position)
- Извлечение receiver‑выражения:
  - `Идентификатор.`
  - `Вызов().`
  - `Коллекция[...].`
  - `(expr).`
  - цепочки `a.b().c[d].e.`
- Устойчивость к неполному синтаксису (трейлинг `.` без следующего токена)

---

## Пошаговый план

### Шаг 1: CompletionTarget контракт
- Ввести структуру `CompletionTarget`:
  - kind (MemberAccess/Call/Statement/TypePosition)
  - receiver выражение (в терминах AST/IR, не строка)
  - позиция/диапазон для диагностики

**Выход:** единый контракт для дальнейшей типизации.

---

### Шаг 2: IR‑first извлечение receiver
- Использовать узел AST/IR под курсором (и рядом) для определения receiver’а.
- Для случая `expr.` без следующего токена:
  - применить синтетический плейсхолдер (добавить фиктивный идентификатор), но без “побочного” изменения документа:
    это должен быть локальный pure helper (или отдельная query), который работает на `file_text` из снапшота и возвращает `CompletionTarget`.

**Выход:** `CompletionTarget` строится из IR, а не из “хвоста строки”.

---

### Шаг 3: Поддержка цепочек выражений
- Нормализовать receiver‑выражение в форму “путь”:
  - segment: property/method/index/call
  - каждый segment позже типизируется (M4)

**Выход:** универсальное представление цепочки выражений.

---

## Критерии завершения

- Receiver корректно извлекается для перечисленных форм (`id.`, `call().`, `[]`, `()` и цепочек).
- Работает на неполном коде (трейлинг `.`) и в присутствии синтакс‑ошибок.
- Есть тесты на извлечение `CompletionTarget`.

---

## Задачи (тикеты) по M3

### T1: Контракт CompletionTarget ✅
**DoD:**
- структура и виды контекстов определены;
- есть unit‑тесты на построение target.

### T2: Синтетический плейсхолдер для `expr.` ✅
**DoD:**
- корректная обработка trailing dot;
- не ломает позиции и incremental pipeline (M1).

### T3: Представление цепочек выражений ✅
**DoD:**
- receiver нормализован в сегменты;
- тесты на сложные цепочки `a.b().c[d].e.`.

---

## Прогресс (факты по коду)

- Receiver цепочка для member access с trailing `.` извлекается через локальный синтетический snippet (парсинг выражения слева от точки без модификации документа): `backend/src/application/type_system/services/completion_target.rs`.
- `(expr).` (скобки) поддерживается: перед синтетическим парсингом снимаются внешние обрамляющие скобки receiver’а: `backend/src/application/type_system/services/completion_target.rs`.
- Index access (`obj[expr]`) в цепочках теперь корректно конвертируется в `Expression::IndexAccess` в синтаксическом слое (нужно для сегментов `Index`): `syntax/src/tree_sitter_adapter/expression_converter.rs`.
- Completion v2 теперь может учитывать `ParseResult` из v2 snapshot и корректно обрабатывать `obj.Method().`: `backend/src/application/type_system/services/completion_service.rs`, `backend/src/bin/lsp_server/handlers/completion.rs`.

**Проверка:** `cargo test -p bsl-backend member_access_receiver_chain` (5/5).
