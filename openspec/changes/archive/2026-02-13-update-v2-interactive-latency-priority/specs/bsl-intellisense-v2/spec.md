## ADDED Requirements

### Requirement: LSP interactive операции v2 используют latency-priority freshness policy с явными лимитами (MUST)
Для `completion`, `hover`, `signatureHelp` система MUST применять latency-priority policy:
- сначала пытаться обслужить `requested file version`;
- ждать не дольше `intellisense_v2_interactive_wait_budget_ms` (дефолт `120ms`, если ключ не задан);
- после исчерпания wait budget допускать stale fallback только на snapshot того же `file_id`, который удовлетворяет обоим ограничениям:
  - `version_gap <= intellisense_v2_interactive_max_stale_version_gap` (дефолт `1`);
  - `stale_age_ms <= intellisense_v2_interactive_max_stale_age_ms` (дефолт `1000ms`).

Runtime knobs MUST валидироваться и приводиться к допустимым диапазонам:
- `intellisense_v2_interactive_wait_budget_ms` в диапазон `[10, 2000]`;
- `intellisense_v2_interactive_max_stale_version_gap` в диапазон `[0, 10]`;
- `intellisense_v2_interactive_max_stale_age_ms` в диапазон `[0, 10000]`.

Stale fallback MUST использовать snapshot, согласованный по `deps_id` и `settings_id` с текущим запросом. Snapshot с несовпадающими `deps_id` или `settings_id` MUST NOT быть использован как stale fallback. Если подходящего stale snapshot нет, система MUST вернуть ответ без блокировки на дальнейшее ожидание latest.

Система MUST явно сигнализировать stale-serving в observability.

#### Scenario: Completion не блокируется до окончания долгого diagnostics
- **GIVEN** для файла уже доступен snapshot версии `V`
- **AND** пользователь редактирует файл до версии `V+1`, и `syntax_diagnostics` для `V+1` выполняется долго
- **WHEN** IDE запрашивает `completion` для `V+1`
- **THEN** сервер завершает ожидание latest не позднее configured wait budget
- **AND** ответ использует latest доступный snapshot (включая controlled stale fallback, если `V+1` ещё недоступна и stale snapshot удовлетворяет лимитам)
- **AND** observability фиксирует факт stale-serving

#### Scenario: Нет подходящего stale snapshot
- **GIVEN** requested версия ещё не готова
- **AND** последний доступный snapshot превышает допустимый `version_gap` или `stale_age_ms`
- **WHEN** IDE запрашивает `hover`
- **THEN** сервер не блокируется дольше wait budget
- **AND** сервер не использует просроченный stale snapshot
- **AND** сервер возвращает пустой/частичный результат без ошибки протокола

#### Scenario: Stale snapshot отклоняется из-за несовпадения deps/settings
- **GIVEN** requested версия ещё не готова
- **AND** доступен stale snapshot того же `file_id`, но с другим `deps_id` или `settings_id`
- **WHEN** IDE запрашивает `signatureHelp`
- **THEN** такой stale snapshot не используется
- **AND** сервер завершает обработку в пределах wait budget без stale ответа с несовместимой ревизией

### Requirement: Diagnostics publish остаётся strict latest-version и monotonic по ревизии (MUST)
Система MUST публиковать `diagnostics` только для актуальной requested version документа.

Результаты, вычисленные для stale версии или stale ревизии зависимостей, MUST NOT быть опубликованы и MUST NOT перезаписывать diagnostics более новой ревизии.

Ревизия для publish MUST валидироваться как минимум по:
- `file_version`;
- `deps_id`;
- `settings_id`.

#### Scenario: Вычисленный stale diagnostics не публикуется
- **GIVEN** diagnostics для версии `V` уже запущен
- **AND** до публикации приходит новая requested version `V+1`
- **WHEN** вычисление для `V` завершается позже
- **THEN** результат для `V` не публикуется в IDE
- **AND** публикуется только результат, соответствующий актуальной requested version

### Requirement: Revision-bound expensive queries дедуплицируются singleflight с корректным lifecycle (MUST)
Для одинакового ключа ревизии `(file_id, file_version, deps_id, settings_id, query_kind)` система MUST выполнять не более одного дорогого query одновременно и делиться результатом между конкурентными запросами.

`query_kind` MUST включать минимум:
- `parse_result`
- `syntax_diagnostics`
- `ir`

Followers MUST получать тот же терминальный outcome, что и leader (`success`, `empty`, `error`, `cancelled`) для данного flight.

Система MUST NOT выполнять автоматический повтор внутри того же flight при `error/cancelled`; новый flight может быть создан только новым входящим запросом на тот же ключ после завершения предыдущего.

#### Scenario: Параллельные completion и diagnostics делят один parse_result
- **GIVEN** два конкурентных запроса требуют `parse_result` для одного и того же ключа ревизии
- **WHEN** оба запроса обрабатываются одновременно
- **THEN** `parse_result` вычисляется один раз
- **AND** оба запроса получают согласованный результат этой единственной вычислительной операции

#### Scenario: Отмена follower не ломает shared вычисление
- **GIVEN** один запрос является `leader`, второй подключён как `shared follower` к тому же singleflight-ключу
- **AND** follower отменяется клиентом
- **WHEN** лидерное вычисление уже запущено
- **THEN** лидерное вычисление не прерывается из-за отмены follower
- **AND** запись in-flight singleflight очищается после завершения leader (success/error/cancel)

#### Scenario: Ошибка leader распространяется на followers без auto-retry в том же flight
- **GIVEN** для singleflight-ключа запущен leader
- **AND** к flight подключены followers
- **WHEN** leader завершается с ошибкой
- **THEN** followers получают тот же ошибочный outcome этого flight
- **AND** внутри текущего flight не запускается повторное вычисление
- **AND** новый leader может появиться только на следующий входящий запрос после очистки in-flight записи

### Requirement: CPU планирование отделяет interactive и background бюджеты с fairness-гарантией (MUST)
Система MUST планировать CPU-bound semantic работу так, чтобы background diagnostics не могли полностью занять вычислительную емкость, необходимую для интерактивных операций.

При общем числе permits `>= 2` система MUST резервировать как минимум:
- `1` permit для interactive-класса;
- `1` permit для background-класса.

Если одна из очередей пуста, система MAY временно давать её свободные permits другой очереди (borrow). При возвращении конкуренции между классами система MUST восстановить гарантированный минимум permits для каждого класса.

#### Scenario: Background diagnostics не вызывает starvation интерактивного пути
- **GIVEN** в системе выполняется серия background diagnostics задач
- **WHEN** поступает интерактивный `hover` или `completion` запрос
- **THEN** интерактивный запрос получает вычислительный слот без ожидания завершения всех background задач
- **AND** интерактивный latency путь не блокируется из-за полного захвата permits background-потоком

#### Scenario: Interactive нагрузка не блокирует diagnostics полностью
- **GIVEN** в системе идёт непрерывный поток interactive-запросов
- **WHEN** запланирован diagnostics для того же процесса
- **THEN** diagnostics получает background permit и выполняет прогресс
- **AND** система не уходит в полное starvation diagnostics

#### Scenario: Borrow permits повышает throughput без потери fairness
- **GIVEN** background-очередь пуста, а interactive-очередь содержит задачи
- **WHEN** доступные background permits временно заимствуются interactive-классом
- **THEN** общая пропускная способность растёт за счёт borrow
- **AND** при появлении background-задач минимум `1` permit возвращается background-классу

### Requirement: Observability контракт отражает stale/singleflight/priority поведение фиксированными ключами (MUST)
Система MUST предоставлять в observability snapshot следующие ключи метрик:

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

Histogram keys:
- `intellisense_v2_singleflight_wait_ms`
- `intellisense_v2_runtime_queue_wait_interactive_ms`
- `intellisense_v2_runtime_queue_wait_background_ms`
- `intellisense_v2_runtime_exec_interactive_ms`
- `intellisense_v2_runtime_exec_background_ms`

#### Scenario: Метрики показывают причину ускорения интерактивного ответа
- **GIVEN** интерактивный запрос обслужен через stale fallback и shared singleflight
- **WHEN** запрашивается snapshot observability
- **THEN** snapshot содержит обязательные stale/singleflight ключи
- **AND** snapshot содержит queue/exec метрики в разрезе `interactive` и `background`

### Requirement: Interactive latency quality gate фиксирует warm-path SLO (MUST)
Система MUST удовлетворять интерактивным latency SLO на warm-path профиле `examples/conf_big` при предзагруженных deps/settings:
- `p95(intellisense_v2_wait_for_file_version_completion_ms) <= intellisense_v2_interactive_wait_budget_ms + 20ms`;
- `p95(completion_duration_ms) <= 1500ms`.

SLO MUST проверяться автоматизированным perf smoke-тестом не менее чем на `50` последовательных completion-запросах в рамках одной сессии.

#### Scenario: Warm-path SLO выдерживается после включения latency-priority policy
- **GIVEN** сервис работает в warm состоянии на `examples/conf_big`
- **WHEN** выполняется perf smoke из 50 последовательных completion-запросов
- **THEN** `p95` wait-for-version и `p95` completion-duration укладываются в заявленные SLO
