# План реализации M4: Контекстное ранжирование и качество

**Статус:** 🟡 ЧАСТИЧНО РЕАЛИЗОВАНО  
**Цель:** повысить релевантность подсказок через контекстное ранжирование, дедупликацию и стабильную сортировку без ухудшения latency.

---

## Область работ

- Контекстные сигналы (тип/фасет/контекст выполнения/область видимости)
- Дедуп подсказок из разных источников
- Стабильная сортировка и детерминизм
- Базовые метрики качества выдачи (локально)

---

## Пошаговый план

### Шаг 1: Контракт данных и модель ранжирования ✅
- Ввести структуру `RankingSignals`/`CandidateFeatures` (prefix match, source, scope, type match, facet match, execution context, deprecation, length).
- Зафиксировать веса/правила (таблица приоритетов + числовой score).
- Описать политику tie‑breakers (score -> source priority -> label).

**Выход:** спецификация и код контракта для ранжирования.

---

### Шаг 2: Извлечение сигналов (feature extraction) ✅
- Из `CompletionContext` выделить:
  - prefix‑match type: exact/starts/contains;
  - member access и owner type;
  - execution_context (Server/Client/Universal);
  - scope (local/module/global);
  - facet_context (Manager/Object/Reference/Selection).
- Привязать к данным индексов: SymbolIndex/ModuleIndex/TypeIndex/MetadataIndex.
- Запретить I/O и тяжелые вычисления в hot‑path.

**Выход:** функция `extract_features(candidate, context) -> RankingSignals`.

---

### Шаг 3: Модель score и политика сортировки ✅
- Реализовать вычисление `score` по весам (например, prefix + type‑compat + scope + source).
- Нормализовать значения (0..1) для предсказуемости.
- Обеспечить стабильный `sortText` и детерминизм.

**Выход:** ранжирование, дающее стабильный порядок на одинаковом контексте.

---

### Шаг 4: Дедуп и merge кандидатов 🟡
- Ключ дедупа: `(label_lower, kind, scope)` с приоритетом источников.
- Политика merge: сохраняем лучший score, переносим detail/doc из best‑source.
- Сохранять origin‑metadata для resolve.

**Выход:** уникальный список кандидатов без дублей (detail/doc переносится на resolve‑этап).

---

### Шаг 5: Интеграция в completion pipeline ✅
- Встроить ранжирование между fetch и mapping.
- Учитывать `max_items` и `isIncomplete`.
- Поддержать fallback: если нет контекста → source‑priority сортировка.

**Выход:** completion использует ранжирование в hot‑path.

---

### Шаг 6: Метрики качества ✅
- Локальные метрики: distribution score, top‑N coverage, доля dedup.
- Логи/метрики только в debug/локальном режиме.

**Выход:** измеримость качества без влияния на latency.

---

### Шаг 7: Тесты ✅
- Unit: feature extraction, scoring, tie‑breakers, dedup.
- Golden: фиксированные входы → стабильный output.
- Regression: стабильный порядок при одинаковом контексте.

**Выход:** тесты M4 защищают качество и стабильность выдачи (golden/regression — отдельно).

---

## Критерии завершения

- Стабильный порядок выдачи при одинаковых входах.
- Видимое улучшение релевантности (локальные метрики/ручные проверки).
- Дедуп работает для источников `symbols/types/metadata/modules`.
- Latency не ухудшается (P95 в пределах M3).

---

## Архитектурные решения

- Ранжирование реализовать как отдельный модуль, например:
  - `backend/src/application/type_system/services/completion_ranking.rs`
- `CompletionCandidate` дополняется `RankingSignals` и `score`.
- Не допускается I/O, запрет на кэш‑промахи в hot‑path.

---

## Задачи (тикеты) по M4

### T1: Контракт данных для ранжирования
**Статус:** ✅  
**Цель:** определить `RankingSignals` и набор feature‑полей.  
**Где:** `backend/src/application/type_system/services/...`.  
**DoD:**
- есть структура сигналов;
- веса и правила в одном месте;
- покрыто unit‑тестом.

### T2: Feature extraction из CompletionContext
**Статус:** ✅  
**Цель:** собрать сигналы без I/O.  
**Где:** completion pipeline.  
**DoD:**
- context signals вычислены;
- tests на prefix/type/scope.

### T3: Score модель и сортировка
**Статус:** ✅  
**Цель:** детерминированный score и стабильный порядок.  
**Где:** completion pipeline.  
**DoD:**
- score вычисляется;
- tie‑breakers стабильны.

### T4: Dedup и merge кандидатов
**Статус:** 🟡  
**Цель:** убрать дубли, не теряя полезных данных.  
**Где:** completion pipeline.  
**DoD:**
- ключ дедупа согласован;
- merge учитывает best‑candidate (detail/doc дополняются через resolve).

### T5: Интеграция в LSP completion
**Статус:** ✅  
**Цель:** включить ранжирование в hot‑path.  
**Где:** LSP handler + completion service.  
**DoD:**
- ранжирование применяется;
- `max_items` и `isIncomplete` корректны.

### T6: Метрики качества
**Статус:** ✅  
**Цель:** измеримость качества.  
**Где:** BasicObservability / debug‑лог.  
**DoD:**
- метрики доступны локально;
- нет влияния на latency.

### T7: Набор тестов M4
**Статус:** ✅  
**Цель:** стабильность качества и детерминизма.  
**Где:** `backend/tests/...`.  
**DoD:**
- unit тесты есть;
- golden/regression добавлены.
