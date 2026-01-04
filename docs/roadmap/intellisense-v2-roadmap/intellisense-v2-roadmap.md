# Roadmap: IntelliSense v2 для BSL (Полнота выражений + stdlib + metadata)

**Статус:** 🔴 ПЛАН  
**Приоритет:** HIGH  
**Цель:** довести IntelliSense до IDE‑grade полноты в VS Code: корректное автодополнение по платформенным типам и метаданным конфигурации для цепочек выражений (`Идентификатор.`, `Вызов().`, `Коллекция[...].`, `(expr).`, `Выбор/?(...)`) при неполном/сломленном коде и инкрементальном редактировании.

---

## Контекст

В рамках завершённого `docs/roadmap/intellisense-roadmap/` уже есть базовая реализация completion/resolve/signatureHelp, ранжирование, метрики и тесты.
Однако для практического использования в 1С этого недостаточно:

- основной код живёт внутри метаданных (конфигурационные типы и фасеты), поэтому stdlib без metadata не даёт реальной ценности;
- completion в редакторе вызывается по “неидеальному” коду: незакрытые строки/скобки, незавершённые выражения, куски `expr.` без следующего идентификатора;
- в VS Code позиции приходят в UTF‑16, а инкрементальные правки требуют строгой согласованности byte offsets/Point для tree‑sitter;
- `completionItem/resolve` должен быть однозначным (по `candidate_id`), иначе при дублях/перегрузках можно “разрешить” не тот элемент.

---

## Объем (definition of v2 completeness)

**Обязательно:**

- `textDocument/completion` + `completionItem/resolve` (LSP).
- Member access completion для receiver‑выражений:
  - идентификатор: `Объект.`
  - цепочка: `Объект.Свойство.`
  - вызовы: `Функция().`, `Объект.Метод().`
  - индексатор: `Коллекция[0].`, `Соответствие["Ключ"].`
  - скобки: `(Объект).`
  - тернарные/условные выражения: `?(Усл, А, Б).`, `Выбор Когда ... Тогда ... Иначе ... Конец.`
- Полная интеграция stdlib + metadata:
  - `Документы.`/`Справочники.`/`РегистрыСведений.`/... (имена объектов конфигурации)
  - фасеты (Manager/Object/Reference/Selection) и их переходы (`.Ссылка`, `.Объект`, табличные части)
  - методы/свойства платформенных типов (Table, ValueTable, Query, etc.) по синтакс‑докам.
- Детеминизм: одинаковый контекст → одинаковый список (порядок/`sortText`).

**Желательно:**

- Контекстная фильтрация/ранжирование на выражениях (не только prefix).
- Нормальные “причины деградации” (почему ушли в fallback).
- Повышение точности `activeParameter` в SignatureHelp для вложенных вызовов.

---

## Нефункциональные требования

- **Completeness‑first:** полнота приоритетнее latency, но UX должен оставаться интерактивным (не “висеть” на наборе).
- **Cancelability:** поддержка отмены (VS Code шлёт отмены часто).
- **Incremental correctness:** после `didChange` подсказки соответствуют актуальному тексту, без mixed state.
- **No blocking I/O:** в hot path completion/resolve не должно быть дисковых/сетевых операций; допускаются фоновые warmup/refresh.
- **Observability:** метрики покрытия/причин fallback + трассировка стадий pipeline.

---

## Архитектурные принципы

- **IR‑first completion:** receiver/type вычисляются по AST/IR, а не по парсингу хвоста строки; строковые эвристики только как fallback.
- **Document semantic state:** per‑document state (text + tree + IR + типизация выражений) обновляется на `didChange` и атомарно читается из completion.
- **Единый источник типов:** `TypeResolver/TypeRepository` + фасеты должны быть единым entrypoint для stdlib и metadata.
- **Однозначный resolve:** `candidate_id` в `CompletionItem.data` → точное определение сущности для resolve.
- **Дешёвые снапшоты:** чтение индексов/состояния без `clone()` больших структур в горячем пути.

---

## Milestones

### M1: Корректность позиций и инкрементального парсинга
**Цель:** устранить класс багов “completion/hover ломается после правок” из‑за рассинхронизации UTF‑16/char/byte offsets и tree‑sitter `Point`.
**План:** `docs/roadmap/intellisense-v2-roadmap/m1-implementation-plan.md`

---

### M2: Document Semantic State (AST/IR/type caches)
**Цель:** построить слой согласованного состояния документа для всех LSP фич, чтобы completion работал по готовым данным.
**План:** `docs/roadmap/intellisense-v2-roadmap/m2-implementation-plan.md`

---

### M3: CompletionTarget: определение выражения‑receiver под курсором
**Цель:** корректно извлекать receiver‑выражение для `expr.` в условиях неполного кода (включая синтетический плейсхолдер).
**План:** `docs/roadmap/intellisense-v2-roadmap/m3-implementation-plan.md`

---

### M4: Типизация выражений для completion (call/index/ternary/paren)
**Цель:** вывести тип для receiver‑выражений из M3: вызовы, индексаторы, скобки, `?()` и `Выбор`.
**План:** `docs/roadmap/intellisense-v2-roadmap/m4-implementation-plan.md`

---

### M5: Полная интеграция metadata (имена/фасеты/табличные части)
**Цель:** completion по путям метаданных и фасетам как first‑class сценарий.
**План:** `docs/roadmap/intellisense-v2-roadmap/m5-implementation-plan.md`

---

### M6: Candidate identity + корректный resolve (без угадывания по label)
**Цель:** `completionItem/resolve` всегда “разрешает” именно выбранный элемент, а не похожий.
**План:** `docs/roadmap/intellisense-v2-roadmap/m6-implementation-plan.md`

---

### M7: Индексы/снапшоты без клонирования (под полноту)
**Цель:** при росте полноты (больше данных) не убить hot path `snapshot.clone()` и lock contention.
**План:** `docs/roadmap/intellisense-v2-roadmap/m7-implementation-plan.md`

---

### M8: Тесты “как в VS Code” и регрессии полноты
**Цель:** покрыть матрицу выражений + stdlib+metadata golden/integration тестами и сделать регрессию воспроизводимой.
**План:** `docs/roadmap/intellisense-v2-roadmap/m8-implementation-plan.md`

---

## Зависимости

- Type system: `TypeResolver`, `TypeRepository`, фасеты конфигурации.
- Парсинг: tree‑sitter + инкрементальные правки (`InputEdit`).
- Источник stdlib: `syntax_helper` (типы/методы/свойства).
- Конфигурация: загрузчики metadata + fingerprint/invalidations.

---

## Definition of Done

- Completion работает для `Идентификатор.`, `Вызов().`, `Коллекция[...].`, `(expr).`, `?(...).`, `Выбор...Конец.`.
- Работает и для платформенных типов, и для метаданных (`Документы/Справочники/...`) с фасетами и табличными частями.
- `completionItem/resolve` однозначный (по `candidate_id`), без угадывания по label.
- Инкрементальное редактирование в VS Code не ломает подсказки (UTF‑16/byte offsets согласованы).
- Есть тестовый набор, фиксирующий полноту и регрессии (golden + LSP integration).

