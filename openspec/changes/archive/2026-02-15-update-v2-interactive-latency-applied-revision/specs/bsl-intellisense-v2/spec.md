## ADDED Requirements

### Requirement: LSP runtime отслеживает received и applied ревизии файла раздельно (MUST)
Система MUST вести для каждого открытого `file_id` две независимые ревизии:
- `received_version`: последняя версия, полученная transport-слоем из `didOpen/didChange`;
- `applied_version`: последняя версия, реально применённая runtime writer path к semantic snapshot.

Latency-critical orchestration для interactive операций MUST использовать `applied_version` как критерий фактической готовности snapshot. `received_version` MUST NOT считаться эквивалентом готовности semantic состояния.

#### Scenario: Received версия опережает applied версию
- **GIVEN** сервер получил `didChange` до версии `V+1`
- **AND** runtime ещё применил только версию `V`
- **WHEN** интерактивный completion запрошен для `V+1`
- **THEN** состояние рассматривается как "latest ещё не applied"
- **AND** orchestration использует bounded wait/stale policy, а не assumes-ready по received версии

## MODIFIED Requirements

### Requirement: LSP interactive операции v2 используют latency-priority freshness policy с явными лимитами (MUST)
Для `completion`, `hover`, `signatureHelp` система MUST применять latency-priority policy:
- сначала пытаться обслужить `requested file version` по фактически `applied_version`;
- ждать не дольше `intellisense_v2_interactive_wait_budget_ms` (дефолт `120ms`, если ключ не задан);
- после исчерпания wait budget допускать stale fallback только на snapshot того же `file_id`, который удовлетворяет обоим ограничениям:
  - `version_gap <= intellisense_v2_interactive_max_stale_version_gap` (дефолт `1`);
  - `stale_age_ms <= intellisense_v2_interactive_max_stale_age_ms` (дефолт `1000ms`).

Runtime knobs MUST валидироваться и приводиться к допустимым диапазонам:
- `intellisense_v2_interactive_wait_budget_ms` в диапазон `[10, 2000]`;
- `intellisense_v2_interactive_max_stale_version_gap` в диапазон `[0, 10]`;
- `intellisense_v2_interactive_max_stale_age_ms` в диапазон `[0, 10000]`.

Stale fallback MUST использовать snapshot, согласованный по `deps_id` и `settings_id` с текущим запросом. Snapshot с несовпадающими `deps_id` или `settings_id` MUST NOT быть использован как stale fallback.

Дополнительно для completion:
- при timeout/cancel на latest-path и наличии допустимого stale snapshot система MUST возвращать stale completion как частичный ответ (`isIncomplete=true`);
- при timeout/cancel и отсутствии допустимого stale snapshot система MUST завершать запрос быстро (без блокировки сверх wait budget) и MAY вернуть empty/partial ответ;
- completion MUST NOT деградировать в "пусто" исключительно из-за transient latest cancel, если доступен допустимый stale snapshot.

Система MUST явно сигнализировать stale-serving в observability.

#### Scenario: Первый completion после правки отдаёт частичный stale ответ
- **GIVEN** пользователь ввёл новую строку и `received_version=V+1`, но `applied_version=V`
- **AND** latest-path запрос для `V+1` не завершился в wait budget
- **WHEN** IDE запрашивает completion
- **THEN** сервер возвращает stale-compatible completion по версии `V`
- **AND** ответ помечен `isIncomplete=true`
- **AND** запрос завершается без ожидания секундного хвоста

#### Scenario: Нет подходящего stale snapshot
- **GIVEN** requested версия ещё не ready по `applied_version`
- **AND** последний snapshot превышает допустимый `version_gap` или `stale_age_ms`, либо несовместим по `deps_id/settings_id`
- **WHEN** IDE запрашивает hover/signatureHelp/completion
- **THEN** сервер не блокируется дольше wait budget
- **AND** сервер не использует несовместимый stale snapshot

### Requirement: CPU планирование отделяет interactive и background бюджеты с fairness-гарантией (MUST)
Система MUST планировать CPU-bound semantic работу так, чтобы background diagnostics не могли полностью занять вычислительную ёмкость, необходимую для интерактивных операций.

При общем числе permits `>= 2` система MUST резервировать как минимум:
- `1` permit для interactive-класса;
- `1` permit для background-класса.

Система MUST приоритизировать control-path orchestration операции (apply changes, wait-for-version coordination) относительно тяжёлых query-path задач.

Background-класс MUST NOT заимствовать interactive reserve при наличии interactive waiters.
Interactive-класс MAY заимствовать background reserve только когда background queue пуста и это не нарушает гарантированный минимум background-прогресса.

#### Scenario: Background load не вытесняет interactive waiters
- **GIVEN** в системе идёт интенсивный поток background diagnostics/query задач
- **AND** есть ожидающий интерактивный completion/hover запрос
- **WHEN** планировщик выбирает следующую задачу
- **THEN** интерактивный запрос получает слот без ожидания завершения всего background хвоста
- **AND** background не забирает interactive reserve при наличии interactive waiters

#### Scenario: Background сохраняет прогресс под interactive нагрузкой
- **GIVEN** идёт непрерывный поток interactive запросов
- **WHEN** запланирован diagnostics task
- **THEN** diagnostics получает минимум background-прогресс
- **AND** система не уходит в starvation diagnostics

### Requirement: Observability контракт отражает stale/singleflight/priority поведение фиксированными ключами (MUST)
Система MUST предоставлять в observability snapshot следующие ключи метрик.

Counter keys:
- `intellisense_v2_interactive_wait_budget_exhausted_total`
- `intellisense_v2_interactive_stale_served_total`
- `intellisense_v2_interactive_knob_clamped_total`
- `intellisense_v2_singleflight_leader_total`
- `intellisense_v2_singleflight_shared_total`
- `intellisense_v2_runtime_queue_wait_interactive_total`
- `intellisense_v2_runtime_queue_wait_background_total`
- `intellisense_v2_runtime_exec_interactive_total`
- `intellisense_v2_runtime_exec_background_total`
- `intellisense_v2_completion_stale_fallback_total`
- `intellisense_v2_completion_fallback_unavailable_total`
- `intellisense_v2_revision_lag_sample_total`

Histogram keys:
- `intellisense_v2_singleflight_wait_ms`
- `intellisense_v2_runtime_queue_wait_interactive_ms`
- `intellisense_v2_runtime_queue_wait_background_ms`
- `intellisense_v2_runtime_exec_interactive_ms`
- `intellisense_v2_runtime_exec_background_ms`
- `intellisense_v2_revision_lag_versions`

#### Scenario: Метрики показывают lag и fallback причину
- **GIVEN** completion обслуживается через stale fallback из-за отставания applied revision
- **WHEN** запрашивается snapshot observability
- **THEN** snapshot содержит обязательные stale/fallback/lag ключи
- **AND** `revision_lag_versions` и fallback counters отражают факт lag-driven ответа

### Requirement: Interactive latency quality gate фиксирует warm-path SLO (MUST)
Система MUST удовлетворять интерактивным latency SLO на warm-path профиле `examples/conf_big` при предзагруженных deps/settings:
- `p95(intellisense_v2_wait_for_file_version_completion_ms) <= intellisense_v2_interactive_wait_budget_ms + 20ms`;
- `p95(completion_duration_ms) <= 1500ms`.

Дополнительно warm-path quality gate MUST проверять устойчивость completion outcomes:
- `completion_cancelled_rate <= 0.10`, где `completion_cancelled_rate = intellisense_v2_completion_result_total_cancelled / completion_total`;
- тестовый прогон MUST включать не менее `50` последовательных completion-запросов в рамках одной сессии.

#### Scenario: Warm-path SLO и cancel-rate выдерживаются после latency fix
- **GIVEN** сервис работает в warm состоянии на `examples/conf_big`
- **WHEN** выполняется perf smoke из 50 последовательных completion-запросов
- **THEN** `p95` wait-for-version и `p95` completion-duration укладываются в заявленные SLO
- **AND** `completion_cancelled_rate` не превышает 10%
