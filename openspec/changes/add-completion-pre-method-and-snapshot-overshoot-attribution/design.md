## Контекст
Сейчас authoritative `Completion Timeline v6` уже умеет показывать:
- split `transport_received -> method_entered -> handler_entered`;
- bounded `prepare_progress`;
- bounded `timeout_attribution`;
- bounded `wait_for_file_version_runtime`, `snapshot_with_deps_runtime` и `artifact_poll`.

Однако последние bundle всё ещё оставляют два недостающих разреза:

1. `server_before_method_entry`
- large wait уже виден как `transport_to_method_wait_ms`;
- но непонятно, сидит ли запрос до первого poll service future или уже после первого poll, но до входа в `lsp_completion`;
- это мешает локализовать bottleneck между tower/service scheduling и actual LSP method entry.

2. `prepare_timeout@prepare_guard` на `snapshot_with_deps`
- timeout attribution уже знает `source=prepare_guard` и `phase=snapshot_with_deps`;
- но timeout path не возвращает bounded runtime split для snapshot stage;
- поэтому нельзя различить, где именно budget уходит: queue wait, writer exec или wake wait после готового reply.

## Цели
- Уточнить root-cause attribution без нового диагностического канала.
- Держать payload bounded и additive.
- Сохранить extension surfaces человекочитаемыми, но не допускающими invented data.
- Дать оператору типовой разбор без обязательного чтения raw JSON.

## Не-цели
- Не лечить runtime latency.
- Не менять probe schema.
- Не добавлять free-text event log.
- Не трогать `exact_wait` contract, кроме уже существующей проекции в summary.

## Решение

### 1. Pre-method ingress split в server edge
Добавить один bounded timestamp внутри service future scope до входа в `lsp_completion`:
- `service_scope_entered_at_ms`

И derived waits:
- `transport_to_service_scope_wait_ms`
- `service_scope_to_method_wait_ms`

Смысл:
- `transport_to_service_scope_wait_ms` показывает задержку от момента приёма request сервисом до первого poll под request context;
- `service_scope_to_method_wait_ms` показывает задержку уже после первого poll service future, но до первой строки `lsp_completion`.

Старые поля сохраняются:
- `transport_to_method_wait_ms`
- `method_prelude_exec_ms`
- `transport_to_handler_wait_ms`

Это позволяет:
- не ломать существующих consumers;
- читать новый split как уточнение уже известного `server_before_method_entry`.

### 2. Timeout-safe snapshot overshoot attribution
Для `snapshot_with_deps` нужен отдельный bounded timeout-safe object, доступный даже если outer `prepare_guard` победил раньше завершения future.

Предлагаемая модель:
- `snapshot_with_deps_timeout_runtime`
  - `queue_wait_ms?`
  - `exec_ms?`
  - `wake_wait_ms?`
  - `resolution`

`resolution` из fixed vocabulary:
- `queue_wait`
- `exec`
- `wake_wait`
- `unavailable`

Смысл:
- если timeout случился во время queue wait, payload показывает это без invented exec data;
- если writer выполнил команду, но completion future проснулся поздно, это видно как `wake_wait`;
- если partial bounded data ещё нет, consumer получает `unavailable`, а не guessed split.

### 3. Existing completion surfaces
Новые поля не должны оставаться raw-only.

`Completion Timeline`, clipboard и incident bundle summary должны:
- печатать новый pre-method split отдельными fact lines;
- печатать bounded snapshot timeout runtime attribution, если она доступна;
- на `v6` payload явно отмечать отсутствие `v7` fields, не реконструируя их.

### 4. Contract discipline
- Новый contract version нужен, потому что меняется authoritative payload shape.
- Все новые поля additive и bounded.
- Legacy `v6` path остаётся supported в extension с explicit degradation.

## Риски и trade-offs

### Риск: новый split всё ещё не укажет root cause до executor level
Смягчение:
- change не обещает final root cause;
- он обещает один дополнительный bounded diagnostic cut, достаточный для narrowing.

### Риск: timeout-safe snapshot attribution усложнит runtime path
Смягчение:
- scope ограничен bounded summary, а не event log;
- допускается `resolution=unavailable`, если partial trace нельзя получить корректно.

### Риск: drift между raw contract и human-readable projections
Смягчение:
- change включает extension regression coverage и smoke/runbook updates.

## Acceptance-направление
- Trace с большим `server_before_method_entry` показывает, где именно сидит лаг: до первого poll service future или уже после него.
- `prepare_timeout@prepare_guard` на `snapshot_with_deps` показывает bounded timeout-safe split или явно `unavailable`, но не молчит.
- Panel/clipboard/incident summary переносят новые `v7` facts без invented reconstruction.
- `v6` payload деградирует явно и не выдаёт фиктивные `v7` поля.
