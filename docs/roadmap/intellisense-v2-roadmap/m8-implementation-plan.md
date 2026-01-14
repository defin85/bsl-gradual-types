# План реализации M8: Тесты и регрессии полноты (VS Code reality)

**Статус:** ✅ РЕАЛИЗОВАНО  
**Цель:** зафиксировать “полноту” как проверяемую характеристику: матрица выражений × (stdlib + metadata) покрыта golden и LSP‑интеграционными тестами, включая неполный/сломанный код.

---

## Область работ

- Golden completion snapshots:
  - выражения `id.`, `call().`, `[]`, `()`, `?()`, `Выбор`, цепочки
  - stdlib + metadata (fixture конфигурации)
- LSP integration tests для VS Code‑паттернов:
  - серия `didChange` → completion
  - проверка, что incremental не ломает подсказки (связка с M1)
- Регрессионный runner/скрипт

---

## Пошаговый план

### Шаг 1: Матрица кейсов полноты
- Описать набор кейсов как таблицу:
  - expression form
  - expected receiver type
  - expected top‑N members (минимальный набор, без “всего списка”)

**Выход:** прозрачная матрица полноты.

#### Матрица (v1)

| case id | expression form | source | receiver expr | expected top‑N (must include) |
| --- | --- | --- | --- | --- |
| m8_metadata_documents | `id.` | metadata | `Документы` | `ЗаказНаряды` |
| m8_metadata_catalogs | `id.` | metadata | `Справочники` | `Контрагенты` |
| m8_metadata_doc_manager | `id.` | metadata | `Документы.ЗаказНаряды` | `СоздатьДокумент` |
| m8_metadata_doc_object_call | `call().` | metadata | `Документы.ЗаказНаряды.СоздатьДокумент()` | `ПолучитьСсылкуНового`, `Работы` |
| m8_metadata_tabular_section_chain | цепочка | metadata | `Документы.ЗаказНаряды.СоздатьДокумент().Работы` | `Добавить` |
| m8_metadata_doc_ref_chain | цепочка | metadata | `Документы.ЗаказНаряды.СоздатьДокумент().ПолучитьСсылкуНового()` | `ПолучитьОбъект` |
| m8_stdlib_new_value_table | `call().` | stdlib | `Новый ТаблицаЗначений()` | `Колонки` |
| m8_stdlib_value_table_columns | цепочка | stdlib | `Новый ТаблицаЗначений().Колонки` | `Добавить` |
| m8_stdlib_index_after_call | `[]` | stdlib | `Новый Массив()[0]` | `Добавить` |
| m8_stdlib_parens_receiver | `()` | stdlib | `(Новый Массив())` | `Добавить` |
| m8_stdlib_conditional_receiver | `?()` | stdlib | `?(Истина, Новый Массив(), Новый Массив())` | `Добавить` |
| m8_stdlib_choice_receiver | `Выбор` | stdlib | `Выбор … КонецВыбора` | `Добавить` |

---

### Шаг 2: Golden тесты по матрице
- Добавить golden snapshots для ключевых кейсов.
- Зафиксировать детерминизм (стабильный порядок).

**Выход:** snapshot‑регрессия на полноту.

---

### Шаг 3: LSP интеграционные тесты с incremental edits
- Смоделировать VS Code сценарии:
  - набираем `ТаблЗнач.Колонки.` по символам и проверяем, что completion появляется
  - вставки/удаления вокруг `.` и внутри строк/комментариев

**Выход:** воспроизведение реального поведения VS Code.

---

### Шаг 4: Скрипт запуска и отчёт
- Обновить/добавить скрипт запуска тестов v2.
- Добавить краткий отчёт “сколько кейсов покрыто”.

**Выход:** простой локальный запуск и измеримость прогресса.

---

## Критерии завершения

- Golden + LSP tests покрывают все заявленные формы выражений.
- Есть кейсы для stdlib и metadata на fixture конфигурации.
- Регрессии воспроизводимы локально одной командой.

---

## Задачи (тикеты) по M8

### T1: Матрица выражений × источники ✅
**DoD:**
- таблица кейсов и ожидаемое поведение.

### T2: Golden snapshots полноты ✅
**DoD:**
- golden кейсы покрывают выражения и источники;
- детерминизм фиксируется.

### T3: LSP tests с incremental edits ✅
**DoD:**
- модель VS Code `didChange` → completion;
- Unicode кейсы (связка с M1).

### T4: Runner/отчёт прогресса ✅
**DoD:**
- скрипт запуска;
- краткий отчёт по покрытию.

---

## Прогресс (факты по коду)

- Матрица кейсов полноты (12) зафиксирована в `docs/roadmap/intellisense-v2-roadmap/m8-implementation-plan.md`.
- Golden регрессия по матрице: `backend/tests/m8_completion_matrix_golden_v2_test.rs` + `backend/tests/golden/m8_completion_matrix_v2.json`.
- Snapshot helper top‑N: `backend/tests/intellisense_testkit.rs` (`completion_snapshot_domain_top_n`).
- LSP incremental тесты: `backend/tests/lsp_incremental_completion_test.rs` (VS Code‑паттерны `didChange` → completion, правки вокруг `.`, Unicode, строки/комментарии).
- Для кейса `Выбор … КонецВыбора`: извлечение receiver распознаёт `КонецВыбора` / `EndCase` как `End`.
  - `backend/src/application/type_system/services/completion_target.rs`
- Member access не триггерится внутри строк и комментариев.
  - `backend/src/application/type_system/services/completion_service.rs`
  - тест: `m8_lsp_completion_inside_string_and_comment_does_not_suggest_member_access`
- Runner обновлён: `scripts/run-intellisense-tests.sh` (добавлены тесты M8 + отчёт по количеству кейсов).

**Как обновить golden:**

```bash
UPDATE_GOLDEN=1 cargo test -p bsl-backend --test m8_completion_matrix_golden_v2_test
```

**Проверка:**

```bash
./scripts/run-intellisense-tests.sh smoke
cargo test --workspace
```
