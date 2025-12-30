# План реализации M8: Тесты и регрессии

**Статус:** ✅ РЕАЛИЗОВАНО (T1/T2/T3/T4/T5/T6 ✅)  
**Цель:** обеспечить надежность изменений в IntelliSense и детерминизм выдачи подсказок.

---

## Область работ

- unit‑тесты фильтрации/ранжирования/контекста completion
- golden‑тесты для стабилизации output completion
- интеграционные тесты LSP (completion/resolve/signatureHelp)
- регрессионная стабильность в CI

---

## Пошаговый план

### Шаг 1: Тестовый каркас и фикстуры ✅
- Ввести общий testkit для IntelliSense (загрузка фикстур, подготовка индексов).
- Зафиксировать набор файлов/конфигураций для тестов (mini‑workspace).
- Добавить утилиту сериализации результата completion (для golden).

**Выход:** единый тестовый каркас и набор стабильных фикстур.

---

### Шаг 2: Unit‑тесты ранжирования и фильтрации ✅
- Prefix match (exact/starts/contains/none).
- Дедупликация и стабильный порядок.
- Скоринг и влияние source_priority/owner_type.
- Граничные случаи (пустой prefix, лимит COMPLETION_MAX_ITEMS).

**Выход:** покрытие базовой логики ранжирования и фильтрации.

---

### Шаг 3: Unit‑тесты контекста completion ✅
- Member access vs non‑member.
- Определение current_word и trigger_char.
- Корректные line/column позиции (UTF‑16).

**Выход:** стабильный парсинг контекста запросов.

---

### Шаг 4: Golden‑тесты completion output ✅
- JSON‑снимки (labels/kind/flags/is_incomplete) на фиксированных кейсах.
- Политика обновления golden (явный режим обновления).
- Проверка детерминизма сортировки.

**Выход:** snapshot‑регрессии для итогового списка подсказок.

---

### Шаг 5: Интеграционные LSP‑тесты ✅
- Completion request → корректный ответ + resolve.
- SignatureHelp для типовых кейсов.
- Проверка устойчивости при пустом индексе/частичных данных.

**Выход:** end‑to‑end проверка LSP contract.

---

### Шаг 6: Регрессионный запуск и стабильность в CI ✅
- Набор тестов, пригодный для shared CI‑раннеров.
- Отдельный профиль для quick‑run (smoke) и full‑suite локально.

**Выход:** стабильные тесты без флапов.

---

## Критерии завершения

- >= 30 unit‑тестов, >= 10 golden‑тестов.
- Есть LSP‑интеграционные тесты (completion/resolve/signatureHelp).
- Детеминизм результата для фиксированных входов.
- Тесты стабильны в CI.

---

## Фактический статус (по коду)

- Есть testkit для фикстур/snapshots + smoke‑тест: `backend/tests/intellisense_testkit.rs`.
- Есть unit‑тесты для ранжирования и completion контекста (31 шт.).
- Есть golden‑тесты completion output (10 кейсов).
- Добавлены LSP‑интеграционные тесты: `backend/tests/lsp_intellisense_tests.rs`.
- Есть smoke/full сценарии запуска: `scripts/run-intellisense-tests.sh`.

---

## Чек-лист задач для завершения M8

- Добавить testkit/fixtures для IntelliSense.
- Расширить unit‑тесты ранжирования/фильтрации и контекста.
- Ввести golden‑тесты с контролируемым обновлением.
- Добавить интеграционные LSP‑тесты (completion/resolve/signatureHelp).
- Зафиксировать режимы запуска (smoke/full) для стабильности.

---

## Задачи (тикеты) по M8

### T1: Testkit и фикстуры ✅
**Цель:** единый каркас для тестов IntelliSense.  
**Где:** `backend/tests/...` + вспомогательные модули.  
**DoD:**
- фикстуры и loader доступны из тестов;
- helper для подготовки индексов/TypeSystemService;
- утилита сериализации результата completion.

### T2: Unit‑тесты ранжирования/фильтрации ✅
**Цель:** покрыть базовую логику ranking/filtering.  
**Где:** `backend/src/application/type_system/services/completion_ranking.rs`.  
**DoD:**
- >= 15 новых unit‑тестов;
- покрыты edge cases (empty prefix, dedupe, ordering).

### T3: Unit‑тесты контекста completion ✅
**Цель:** зафиксировать детерминизм context extraction.  
**Где:** `backend/src/application/type_system/services/completion_service.rs`.  
**DoD:**
- >= 5 новых unit‑тестов;
- покрыты member_access и UTF‑16 позиции.

### T4: Golden‑тесты completion output ✅
**Цель:** регрессии по итоговому списку подсказок.  
**Где:** `backend/tests/intellisense_golden_tests.rs` + `backend/tests/fixtures/...`.  
**DoD:**
- >= 10 golden‑кейсов;
- есть режим обновления snapshots;
- сравнение включает label/kind/is_incomplete.

### T5: Интеграционные LSP‑тесты ✅
**Цель:** проверка end‑to‑end LSP contract.  
**Где:** `backend/tests/lsp_intellisense_tests.rs`.  
**DoD:**
- completion + resolve + signatureHelp;
- тесты проходят на shared CI‑раннерах;
- устойчивость к неполным данным индекса.

### T6: Регрессионный запуск и стабильность ✅
**Цель:** быстрый smoke и полный прогон для локальной проверки.  
**Где:** `scripts/run-intellisense-tests.sh`.  
**DoD:**
- есть smoke профиль для shared CI;
- есть full профиль для локального запуска;
- smoke включает unit + golden + LSP интеграцию.
