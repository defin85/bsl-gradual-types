# План реализации M2: Document Semantic State (AST/IR/type caches)

**Статус:** 🔴 ПЛАН  
**Цель:** создать согласованное состояние документа (text/tree/IR/типизация), обновляемое на `didOpen/didChange` и читаемое из completion/hover/signatureHelp без mixed state и без повторного тяжёлого анализа в hot path.

---

## Область работ

- Per‑document state: `text`, `version`, `tree`, `AST`, `IR`, “known types at spans”
- Обновление state на `didChange` (дебаунс, отмена предыдущих задач)
- Атомарное чтение state из LSP handlers
- Интеграция с существующими кэшами (IR cache, AST cache, индексы)

---

## Пошаговый план

### Шаг 1: Модель данных DocumentState
- Определить структуру: что хранить, какие версии/снапшоты, как связывать с `IndexSnapshotId`.
- Определить `state_id`/`doc_version` для согласованности LSP запросов.

**Выход:** `DocumentState` контракт и правила согласованности.

---

### Шаг 2: Пайплайн обновления (didOpen/didChange)
- На `didChange`:
  - обновлять текст и версию
  - запускать (или планировать) обновление tree/IR
  - отменять устаревшие задачи (cancel previous)
- Обеспечить, что completion читает последний “готовый” state, а не partially‑updated.

**Выход:** предсказуемый и отменяемый update‑pipeline.

---

### Шаг 3: Использование state в completion/hover/signatureHelp
- completion: читает receiver/type из state
- hover: читает узел/тип из state
- signatureHelp: использует state для определения receiver и активного параметра

**Выход:** единый путь получения семантики во всех LSP фичах.

---

### Шаг 4: Метрики и причины деградации
- Ввести причины fallback:
  - “state not ready”
  - “parse error”
  - “unknown receiver type”
  - “metadata not loaded”
- Экспортировать это в метрики/trace (в debug/perf режимах).

**Выход:** измеримость полноты и причин потерь.

---

## Критерии завершения

- Completion/hover/signatureHelp не строят IR “на лету” в горячем пути, а используют state.
- Нет mixed state: snapshot/версия согласованы.
- Есть отмена устаревших задач обновления.
- Метрики/логи показывают причины fallback.

---

## Задачи (тикеты) по M2

### T1: Контракт DocumentState ⏳
**DoD:**
- определены поля и правила версии;
- определено, что является “готовым” состоянием;
- описан `state_id`/`doc_version`.

### T2: Update‑pipeline для didChange ⏳
**DoD:**
- дебаунс и отмена устаревших задач;
- обновление state атомарно для читателей.

### T3: Интеграция с LSP handlers ⏳
**DoD:**
- completion/hover/signatureHelp используют state;
- есть тесты на согласованность и отсутствие mixed state.

### T4: Observability причин fallback ⏳
**DoD:**
- причины деградации фиксируются в лог/метрику;
- есть базовый отчёт/дамп для диагностики.

