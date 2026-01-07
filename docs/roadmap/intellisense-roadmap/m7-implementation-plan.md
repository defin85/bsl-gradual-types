# План реализации M7: Производительность и телеметрия

**Статус:** ✅ РЕАЛИЗОВАНО  
**Цель:** измеримость качества и latency для IntelliSense, трассировка pipeline и регрессионные проверки.

---

## Область работ

- Метрики latency/coverage для completion/signatureHelp/resolve
- Трассировка pipeline (debug mode)
- Экспорт метрик и отчеты для CI/локально
- Нагрузочные/регрессионные тесты производительности

---

## Пошаговый план

### Шаг 1: Метрики и схема данных ✅
- Зафиксировать набор метрик и их имена (latency P50/P95/P99, coverage).
- Ввести уровни детализации (debug/perf).
- Определить источники данных в pipeline (completion, resolve, signatureHelp).

**Выход:** единая схема метрик и точки сбора.

---

### Шаг 2: Трассировка completion pipeline ✅
- Добавить trace spans для этапов: сбор → фильтрация → ранжирование → форматирование.
- Включение через env/config (без влияния на prod).
- Корреляция с request id и временем выполнения.

**Выход:** трассировка для диагностики регрессий.

---

### Шаг 3: Экспорт метрик и отчеты ✅
- Экспорт сводки (JSON) и/или лог‑отчетов.
- Поддержка агрегатов (P50/P95/P99, count, error rate).
- Документирование формата и сценариев использования.

**Выход:** стандартизированный отчет для локального/CI запуска.

---

### Шаг 4: Нагрузочные и регрессионные тесты ✅
- Бенчмарки на типовых и крупных конфигурациях.
- Тесты на деградацию latency/coverage (baseline + threshold).
- Сбор результатов в артефакты CI.

**Выход:** регрессионный performance suite.

---

### Шаг 5: Критерии качества и алерты ✅
- Зафиксировать пороги P95/P99 (NFR).
- Механизм fail‑fast в CI при деградации.
- Сводный отчет о качестве подсказок.

**Выход:** автоматическое обнаружение регрессий.

---

## Критерии завершения

- Метрики completion/signatureHelp/resolve доступны и стабильны.
- Есть трассировка pipeline в debug‑режиме.
- Есть performance/regression тесты с порогами.
- Отчеты доступны локально и в CI.

---

## Фактический статус (по коду)

- Есть BasicObservability с метриками completion latency/quality: `backend/src/system/basic_observability.rs`.
- Есть запись latency completion в LSP: `backend/src/bin/lsp_server/server/language_server.rs`.
- Есть `/api/metrics` с агрегатами P50/P95/P99 и rates: `backend/src/presentation/web/handlers.rs`.
- Есть метрики для загрузки BSL модулей (parse metrics).
- Есть трассировка этапов completion pipeline (debug + request id).
- Есть контрактные тесты `/api/metrics` и trace‑логов.
- Есть perf‑harness с baseline/threshold для small/medium и manual large.
- Есть fail‑fast по порогам (P95/P99 + error/incomplete rate) и summary отчет.

---

## Чек-лист задач для завершения M7

- Ввести метрики для resolve/signatureHelp и coverage.
- ✅ Добавить trace spans на этапы pipeline.
- ✅ Добавить отчеты/экспорт агрегатов (P95/P99).
- ✅ Настроить perf/regression тесты и пороги.
- ✅ Обновить документацию по запуску метрик.
- ✅ Пороговые проверки и fail‑fast (скрипт/harness).

---

## Задачи (тикеты) по M7

### T1: Схема метрик и сбор данных ✅
**Цель:** единый набор метрик для IntelliSense.  
**Где:** `backend/src/system/basic_observability.rs`, pipeline completion/signatureHelp/resolve.  
**DoD:**
- метрики latency и coverage для completion/resolve/signatureHelp;
- агрегаты P50/P95/P99 доступны в JSON;
- нет блокирующего I/O в hot path.

### T2: Трассировка pipeline ✅
**Цель:** пошаговая трассировка completion pipeline.  
**Где:** `backend/src/application/type_system/services/completion_service.rs`.  
**DoD:**
- spans для этапов: сбор → фильтр → ранжирование → форматирование;
- включение через env/config;
- trace id в логах.

### T3: Экспорт/отчеты метрик ✅
**Цель:** стабильный формат отчета.  
**Где:** web handler `/api/metrics` + docs.  
**DoD:**
- отчеты содержат P50/P95/P99, counts и rates;
- документирован формат;
- есть пример использования для CI.

### T4: Нагрузочные тесты IntelliSense ✅
**Цель:** регрессионный perf‑suite.  
**Где:** `backend/tests/...` или отдельный perf harness.  
**DoD:**
- сценарии `small`/`medium` + manual `large`;
- фиксация baseline (JSON);
- скрипт с порогами (P95/P99) для CI.

### T5: Пороговые проверки и алерты ✅
**Цель:** автоматическое обнаружение регрессий.  
**Где:** CI pipeline + docs.  
**DoD:**
- thresholds по P95/P99 + error/incomplete rate;
- fail‑fast при ухудшении (perf harness);
- summary отчет (markdown).
