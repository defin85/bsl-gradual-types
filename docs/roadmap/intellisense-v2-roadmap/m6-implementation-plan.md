# План реализации M6: Candidate identity + корректный completionItem/resolve

**Статус:** ✅ РЕАЛИЗОВАНО  
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

### T1: Спецификация candidate_id ✅
**DoD:**
- определён формат `data`;
- добавлена версия/схема миграции.

### T2: Проброс candidate_id в completion ✅
**DoD:**
- `data` содержит candidate_id для всех item‑ов;
- тесты на сериализацию/совместимость.

### T3: Resolve по candidate_id ✅
**DoD:**
- resolve корректен при дублях;
- fallback‑ветка не ломает legacy.

### T4: Тесты на корректность resolve ✅
**DoD:**
- unit + integration тесты;
- фиксация регрессий.

---

## Прогресс (факты по коду)

- `candidate_id` добавлен в `CompletionItem.data`:
  - schema: `{"v": 1, "t": "...", ...}` (версионирование через `v`)
  - базовые типы: `method`, `property`, `function`, `type`, `metadata`, `keyword`, `other`
  - для `function` добавлено поле `resolve` (не все функции должны резолвиться через SignatureIndex, чтобы не подменять локальные символы)
    - `resolve` учитывает дедуп источников: берётся лучший источник как `min(origin_sources)`, чтобы `origin_sources=[0,1]` не приводил к ошибочному resolve глобальной сигнатуры
  - для `method`/`function` добавляется `sig_hash` (hash сигнатуры `MethodSignature`, включая `SignatureSource`)
- `completionItem/resolve` теперь использует `candidate_id` как первичный источник истины; если `candidate_id` отсутствует — fallback на legacy логику (`kind/owner_type`).

**Код:**
- `backend/src/bin/lsp_server/handlers/completion.rs`

**Тесты:**
- `cargo test -p bsl-backend --test lsp_intellisense_tests`
  - `completion_handler::tests::m6_completion_resolve_uses_candidate_id_for_function_origin`
  - `completion_handler::tests::m6_completion_resolve_uses_candidate_id_for_property`
  - `completion_handler::tests::m6_completion_resolve_uses_candidate_id_for_metadata`
  - `completion_handler::tests::m6_completion_resolve_dedup_sources_prefers_local_function`
  - `completion_handler::tests::m6_completion_resolve_legacy_fallback_works_without_candidate_id`
