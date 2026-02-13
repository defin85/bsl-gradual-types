## Context
В текущем LSP v2 pipeline интерактивные операции и diagnostics используют общий строгий паттерн синхронизации по версии файла. Это обеспечивает консистентность, но при тяжёлом синтаксическом проходе приводит к заметным задержкам в completion/hover/signatureHelp.

Наблюдаемая картина:
- тяжёлые `syntax_diagnostics` и `parse_result` формируют latency floor даже на warm path;
- интерактивные ответы ждут ту же “свежесть”, что и background diagnostics;
- есть отмены, но CPU уже потрачен на запущенные blocking-задачи.

## Goals / Non-Goals
- Goals:
  - снизить tail-latency интерактивных LSP ответов без потери корректности diagnostics.
  - убрать дубли дорогих query для одинаковой ревизии.
  - ограничить влияние background diagnostics на интерактивный путь.
  - сохранить единый v2 facade/orchestration контракт.
- Non-Goals:
  - миграция на pull diagnostics.
  - изменение публичных API контрактов клиентов.
  - глобальная переработка flow-sensitive логики.

## Architecture Drivers
- Latency: completion/hover/signatureHelp должны оставаться отзывчивыми под нагрузкой.
- Correctness: diagnostics должны публиковаться только для актуальной версии.
- Determinism: одинаковая ревизия должна давать одинаковые результаты.
- Operability: поведение должно быть прозрачно в observability-метриках.

## Options Considered

### Option A: Сохранить strict wait для всех операций (status quo)
- Плюсы: максимальная консистентность “всегда latest”.
- Минусы: высокий tail-latency интерактивных операций при тяжёлых diagnostics.

### Option B (Recommended): Fast interactive path + strict diagnostics
- Идея:
  - интерактивные операции получают bounded wait и controlled stale fallback;
  - diagnostics сохраняют strict latest policy;
  - дорогие query дедуплицируются singleflight;
  - CPU scheduling резервирует емкость под interactive path.
- Плюсы: заметное снижение p95/p99 для интерактивных запросов без потери строгой корректности diagnostics.
- Минусы: усложняется orchestrator логика и observability.

### Option C: Полный переход на pull diagnostics
- Плюсы: более явная модель приоритизации у клиента.
- Минусы: слишком широкий scope, затрагивает протокол и клиентские интеграции.

## Decisions
- Выбран Option B как минимальный практически полезный шаг.
- Diagnostics остаются strict latest; stale-публикация недопустима.
- Для интерактивных операций допускается controlled stale read из последнего доступного snapshot, если latest версия не готова в пределах bounded wait.
- Дорогие query для одинакового ключа ревизии выполняются singleflight.
- Планировщик CPU разделяет интерактивные и background бюджеты, чтобы не допускать starvation интерактивных запросов.
- Runtime knobs фиксируются в change:
  - `intellisense_v2_interactive_wait_budget_ms` (default `120`);
  - `intellisense_v2_interactive_max_stale_version_gap` (default `1`);
  - `intellisense_v2_interactive_max_stale_age_ms` (default `1000`).

## Proposed Architecture

### 1) Freshness policy by operation class
- `completion`, `hover`, `signatureHelp`: latency-priority policy.
- `diagnostics`: strict-version policy.

Для interactive-path действует bounded wait. После исчерпания wait budget:
- stale fallback разрешён только если snapshot проходит ограничения по `version_gap` и `stale_age_ms`;
- stale fallback должен быть согласован по `deps_id` и `settings_id`;
- если подходящего stale snapshot нет, операция завершается без дальнейшего блокирующего ожидания latest.

Результат stale-serving и факт исчерпания wait budget отражаются в observability.

### 2) Singleflight for expensive revision-bound queries
Ключ дедупликации: `(file_id, file_version, deps_id, settings_id, query_kind)`, где `query_kind` минимум:
- `parse_result`
- `syntax_diagnostics`
- `ir`

Lifecycle singleflight:
- на ключ одновременно существует только один `leader`;
- `followers` получают shared-результат лидера;
- отмена одного follower не отменяет leader, если leader уже исполняется;
- in-flight запись обязана удаляться после завершения leader (success/error/cancel), чтобы не было зависших ключей.

### 3) Priority-aware CPU scheduling
- Интерактивный и background пути используют раздельные классы permits.
- При total permits `>= 2` резервируются минимум `1` interactive permit и `1` background permit.
- Background diagnostics не могут занять всю емкость blocking-пула.
- Interactive поток не должен полностью starvation background-диагностики.

### 4) Observability contract expansion
Нужны метрики для:
- stale-serving интерактивных операций;
- исчерпания interactive wait budget;
- singleflight роли (`leader`/`shared`);
- queue wait по классам (`interactive` vs `background`).

Контракт фиксирует обязательные ключи:
- `intellisense_v2_interactive_wait_budget_exhausted_total`
- `intellisense_v2_interactive_stale_served_total`
- `intellisense_v2_singleflight_leader_total`
- `intellisense_v2_singleflight_shared_total`
- `intellisense_v2_singleflight_wait_ms`
- `intellisense_v2_runtime_queue_wait_interactive_total`
- `intellisense_v2_runtime_queue_wait_interactive_ms`
- `intellisense_v2_runtime_queue_wait_background_total`
- `intellisense_v2_runtime_queue_wait_background_ms`
- `intellisense_v2_runtime_exec_interactive_total`
- `intellisense_v2_runtime_exec_interactive_ms`
- `intellisense_v2_runtime_exec_background_total`
- `intellisense_v2_runtime_exec_background_ms`

## Test Strategy
- Integration:
  - интерактивный completion/hover/signatureHelp не блокируется на долгом diagnostics для более новой версии.
  - при отсутствии допустимого stale snapshot интерактивный запрос завершается без долгого ожидания.
  - diagnostics не публикуются для stale версии после прихода новой requested version.
- Concurrency:
  - параллельные эквивалентные запросы делят один expensive query (singleflight).
  - отмена follower не ломает leader и не оставляет in-flight утечек.
- Fairness:
  - under load interactive получает слот без ожидания завершения background очереди;
  - diagnostics продолжает прогрессировать при высокой interactive-нагрузке.
- Regression:
  - cold/warm perf smoke на `examples/conf_big` с проверкой улучшения интерактивных latency хвостов.
  - проверка наличия новых observability метрик и их согласованности.

## Risks / Trade-offs
- Риск stale-подсказок в интерактивном пути.
  - Mitigation: bounded stale policy + явная observability сигнализация.
- Риск усложнения конкуррентной логики.
  - Mitigation: узкий scope изменений, отдельные интеграционные тесты на race/cancellation.
- Риск starvation diagnostics при сильной интерактивной нагрузке.
  - Mitigation: гарантированный минимальный background budget.
