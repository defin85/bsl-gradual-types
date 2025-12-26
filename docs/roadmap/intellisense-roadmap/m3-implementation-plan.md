# План реализации M3: LSP Completion MVP

**Статус:** 🟡 ЧАСТИЧНО РЕАЛИЗОВАНО  
**Цель:** базовое автодополнение через LSP с минимальным набором источников и стабильной задержкой.

---

## Область работ

- LSP completion handler + resolve
- Формирование `CompletionContext`
- Источники: `KeywordIndex`, `TypeIndex` (типы и методы)
- Фильтрация, ранжирование, лимиты, `isIncomplete`
- Метрики latency и базовые тесты

---

## Пошаговый план

### Шаг 1: LSP capabilities и эндпоинты 🟡
- Включить `completionProvider.resolveProvider = true`.
- Установить `triggerCharacters = [".", "("]`.
- Добавить обработчик `textDocument/completion`.
- Добавить `completionItem/resolve` (только для одного item).

**Выход:** LSP сервер объявляет completion и принимает запросы.

---

### Шаг 2: Контекст запроса (`CompletionContext`) 🟡
- Определить префикс, trigger, позицию.
- Учесть базовый сценарий: после `.` и внутри выражений.
- Резать контекст до безопасного окна (например, 128–256 символов).

**Выход:** стабильное извлечение контекста без парсинга всего файла.

---

### Шаг 3: Источники кандидатов 🟡
- `KeywordIndex` → ключевые слова и директивы.
- `TypeIndex` → типы платформы и их методы.
- Для `.` приоритет: методы типов; без типа — только keywords.

**Выход:** набор кандидатов от индексов без I/O.

---

### Шаг 4: Фильтрация и ранжирование 🟡
- Префикс‑фильтр (case‑insensitive).
- Ранжирование по источнику: типы/методы выше keywords.
- Стабильный `sortText`.

**Выход:** упорядоченный список без мусора.

---

### Шаг 5: Маппинг в `CompletionItem` 🟡
- Минимальный набор полей: `label`, `kind`, `sortText`, `filterText`, `insertText`, `insertTextFormat`.
- `detail`/`documentation` только в resolve.

**Выход:** корректные LSP‑items без тяжёлых данных.

---

### Шаг 6: Лимиты, isIncomplete, метрики ⏳
- Ограничение `max_items = 200`.
- При превышении — `isIncomplete = true`.
- Метрики latency P95/P99 (без resolve).

**Выход:** стабильная задержка, контроль качества.

---

### Шаг 7: Тесты ⏳
- Unit: фильтрация по префиксу, лимит, ранжирование.
- Integration: keywords + platform types в типовом файле.
- Regression: корректная работа при пустом индексе.

**Выход:** базовый набор тестов M3.

---

## Критерии завершения

- completion отвечает < 50ms P95 на типовых файлах;
- в ответе только минимальные поля, тяжелые данные через resolve;
- корректные подсказки для ключевых слов и типов платформы.

---

## Фактический статус (по коду)

- Обработчик LSP completion отсутствует или неполный.
- `KeywordIndex` и `TypeIndex` доступны, но не используются в completion.
- `completionItem/resolve` не реализован.
- Метрики и тесты для completion отсутствуют.

---

## Задачи (тикеты) по M3

### T1: LSP completion endpoints ✅/⏳
**Статус:** ⏳
**Цель:** подключить `textDocument/completion` и `completionItem/resolve`.
**Где:** `backend/src/bin/lsp_server/...`.
**DoD:**
- capabilities объявлены;
- обработчик completion подключен;
- resolve работает для одного item.

### T2: CompletionContext ✅/⏳
**Статус:** ⏳
**Цель:** безопасное извлечение контекста и префикса.
**Где:** новый модуль `backend/src/system/completion_context.rs` или рядом с LSP handler.
**DoD:**
- корректный prefix для обычных идентификаторов;
- поддержка `.` и `(`.

### T3: Источники кандидатов ✅/⏳
**Статус:** ⏳
**Цель:** соединить `KeywordIndex` + `TypeIndex`.
**Где:** completion pipeline.
**DoD:**
- keywords всегда доступны;
- types и методы доступны в контексте `.`.

### T4: Фильтрация/ранжирование ✅/⏳
**Статус:** ⏳
**Цель:** убрать мусор и стабилизировать порядок.
**Где:** completion pipeline.
**DoD:**
- prefix filter;
- стабильный `sortText`;
- типы/методы выше keywords.

### T5: CompletionItem mapping + resolve ✅/⏳
**Статус:** ⏳
**Цель:** минимальный item в hot path + resolve.
**Где:** completion pipeline.
**DoD:**
- минимальные поля в ответе;
- detail/documentation только в resolve.

### T6: Лимиты и метрики ✅/⏳
**Статус:** ⏳
**Цель:** соблюдение latency и ограничений.
**Где:** completion pipeline + метрики.
**DoD:**
- max_items=200;
- `isIncomplete=true` при превышении;
- метрика P95/P99.

### T7: Тесты ✅/⏳
**Статус:** ⏳
**Цель:** покрыть базовый функционал.
**Где:** `backend/tests/...`.
**DoD:**
- unit‑тесты фильтрации/лимитов;
- integration‑тест с keywords и platform types;
- regression‑тест для пустого индекса.
