# План реализации M6: Candidate identity + корректный completionItem/resolve

**Статус:** 🔴 ПЛАН  
**Цель:** сделать `completionItem/resolve` однозначным и корректным при дублях/перегрузках/нескольких источниках, используя стабильный `candidate_id` вместо угадывания по `label`.

---

## Область работ

- Дизайн `candidate_id`:
  - source kind + ключ сущности (type/member/metadata path)
  - версия payload/snapshot
  - минимальная сериализация в JSON (`CompletionItem.data`)
- Resolve по `candidate_id`:
  - detail/documentation
  - snippet text (если поддерживается клиентом)
- Дедуп/merge не должны терять `candidate_id` и origin metadata

---

## Пошаговый план

### Шаг 1: Спецификация candidate_id
- Определить, какие типы кандидатов существуют (method/property/type/metadata object/etc.).
- Определить формат `data` и схему версионирования.

**Выход:** контракт `candidate_id` и правила обратной совместимости.

---

### Шаг 2: Передача candidate_id в LSP CompletionItem
- `textDocument/completion` возвращает лёгкие items + `data` с `candidate_id`.

**Выход:** клиент всегда может запросить resolve без дополнительных поисков.

---

### Шаг 3: Resolve по candidate_id
- `completionItem/resolve` использует `candidate_id` для точного lookup.
- Fallback: если `candidate_id` отсутствует (legacy) — best‑effort по текущей логике.

**Выход:** resolve корректен при дублях.

---

## Критерии завершения

- Resolve не зависит от одного `label`.
- Дедуп/ранжирование сохраняют корректную связь item ↔ сущность.
- Есть тесты на “две одинаковые метки из разных источников”.

---

## Задачи (тикеты) по M6

### T1: Спецификация candidate_id ⏳
**DoD:**
- определён формат `data`;
- добавлена версия/схема миграции.

### T2: Проброс candidate_id в completion ⏳
**DoD:**
- `data` содержит candidate_id для всех item‑ов;
- тесты на сериализацию/совместимость.

### T3: Resolve по candidate_id ⏳
**DoD:**
- resolve корректен при дублях;
- fallback‑ветка не ломает legacy.

### T4: Тесты на корректность resolve ⏳
**DoD:**
- unit + integration тесты;
- фиксация регрессий.

