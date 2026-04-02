## Context

После `refactor-completion-turn-wait-slot-release` server-side transport slot release уже работает на default path, но в front-edge completion остаются три независимых источника operator pain:

1. extension строит client/server correlation через эвристику по `uri + trigger_mode + response_sent_at_ms ± window`, что ломается при overlap и быстро даёт `multiple_probe_candidates`;
2. `Completion Timeline` panel сама продолжает опрашивать `bsl.getCompletionTimeline` во время active completion probes;
3. request-side exact wait для member-access может увидеть `NoMatchingTask`, хотя `didChange` producer task для той же версии уже был запланирован и успел закончиться до того, как readiness стала наблюдаемой.

Технические границы подтверждаются текущим кодом:

- middleware отмечает probe start до `next(...)`, а transport instrumentation отдельно отмечает момент реального LSP dispatch;
- `observabilityIncidentBundleRequests.ts` сначала ищет кандидатов только по URI/trigger mode/timestamp window;
- `Completion Timeline` panel имеет постоянный polling loop;
- `transport_adapter.rs` уже видит raw `Request` и может безопасно читать vendor fields до typed dispatch;
- `deps_and_precompute.rs` удаляет exact precompute task entry сразу после одного run, а waiter трактует `Missing` как terminal `NoMatchingTask`.

## Goals / Non-Goals

### Goals

- Сделать correlation между client probe и server trace request-bound и deterministic на default VS Code path.
- Убрать observability self-noise из active completion path.
- Закрыть race между `didChange`-produced exact task и same-version member-access waiter без ослабления bounded fail-closed поведения.
- Сохранить parity `TriggerCharacter='.'` vs `Invoked` и existing current-revision guarantees.

### Non-Goals

- Не делать новый rewrite observability/perf pipeline.
- Не пересматривать transport handoff / slot-release дизайн.
- Не вводить stale semantic fallback, request-side exact precompute spawn или sync rebuild на completion request path.
- Не менять LSP/public UX за пределами completion front-edge и observability surfaces, связанных с ним.

## Decisions

### 1. Request-bound deterministic probe correlation

Рекомендуемое решение: extension генерирует opaque `client_probe_id` для каждого completion probe и передаёт его как BSL-vendor field в исходный `textDocument/completion` request. Сервер читает это поле на raw transport boundary и сохраняет его в per-request timeline trace.

Почему так:

- timestamp/window correlation уже показала себя недетерминированной при overlap;
- `transport_adapter.rs` уже является точкой, где доступны raw JSON-RPC request params до типизированного service path;
- echo того же opaque id обратно в timeline позволяет correlation без guessed attribution.

Последствия:

- outbound completion request несёт vendor field `bslProbeId`;
- per-request timeline trace получает root-level field `client_probe_id`, а не server-edge timing field;
- timeline contract получает contiguous version bump `17 -> 18`, а baseline поднимается `v14 -> v15`;
- extension использует `client_probe_id` как primary correlation key;
- старые payload'ы остаются backward-compatible через fail-closed fallback на прежнюю эвристику, но без invented verdicts.
- `client_probe_id` не попадает в агрегированные observability counters/histograms и остаётся только per-request correlation marker.

### 2. Quiet mode для Completion Timeline panel и incident export

Рекомендуемое решение: panel перестаёт auto-refresh'иться, пока есть active completion probes, и использует короткое quiet window после их завершения. Webview export по умолчанию использует последний уже захваченный snapshot, а не инициирует новый fetch во время churn; fallback fetch сохраняется только в explicit export command path, когда cached capture отсутствует.

Почему так:

- observability surface не должна сама вмешиваться в completion front-edge;
- текущий polling loop конкурирует за те же execution windows и искажает operator picture;
- cached snapshot уже достаточно информативен для incident handoff и не требует fresh fetch в критический момент.

Последствия:

- panel остаётся usable, но становится чуть менее "живой" под активной нагрузкой;
- нужен явный manual refresh contract и визуально понятное quiet/backoff поведение.
- не требуется переписывать existing cached export path, уже существующий в webview provider.

### 3. Producer-side exact-task visibility до observable readiness

Рекомендуемое решение: completed exact type-index producer task entry должен оставаться observable/joinable для same-version waiter до момента, когда `serve_only_ready` для той же версии реально наблюдаем, либо пока задача не superseded, файл не закрыт (`didClose`) или сервер не завершает shutdown. Немедленное удаление task entry после единственного run больше не допускается для same-version handoff path.

Почему так:

- существующий waiter корректно остаётся fail-closed и bounded, но сейчас может наблюдать ложный `Missing` в race-окне между завершением producer task и materialization readiness;
- request-side spawn exact task противоречит текущему current-revision/fail-closed contract;
- stale fallback разрушил бы parity semantics и ослабил бы truthful observability.

Последствия:

- registry lifecycle станет чуть сложнее и потребует явного terminal phase/cleanup;
- нужны дополнительные tests на leak-free supersession/shutdown cleanup.
- cleanup policy должна быть одной и той же для background и promoted interactive task entry, чтобы waiter не терял matching visibility после promotion.

## Alternatives Considered

### A. Улучшить только timestamp/window correlation

Отклонено. Это не делает correlation request-bound и продолжает ломаться при burst overlap с одинаковыми URI, trigger mode и близкими response timestamps.

### B. Разрешить request path самому spawn exact precompute task при `Missing`

Отклонено. Это ослабляет fail-closed contract, смешивает producer и waiter responsibilities и открывает путь к latency spikes на интерактивном completion path.

### C. Разрешить stale/degraded semantic fallback для `TriggerCharacter='.'`

Отклонено. Это прямо конфликтует с current-revision parity intent и маскирует producer-side race вместо его исправления.

### D. Полностью переписать observability/perf pipeline

Отклонено как слишком широкий scope. Для текущей проблемы достаточно точечных изменений в request-bound correlation, quiet polling и exact-task lifecycle.

### E. Перенести `client_probe_id` в aggregated observability вместо timeline trace

Отклонено. Correlation нужен на per-request уровне; агрегаты не дают request-bound truth и только поднимут cardinality pressure.

## Risks / Trade-offs

- Request vendor field повышает coupling между extension и сервером.
  - Mitigation: использовать optional namespaced field `bslProbeId`, root-level echoed `client_probe_id` и backward-compatible contiguous contract bump.
- Удержание exact-task entry дольше повышает риск registry leak.
  - Mitigation: cleanup только на supersession, `didClose`, shutdown и явные tests на bounded lifecycle и retention after completion.
- Quiet mode делает panel менее live под нагрузкой.
  - Mitigation: manual refresh и явный UI-state, показывающий quiet/backoff режим.
- Если после deterministic correlation останется большой `client_to_transport_wait_ms`, следующий bottleneck может оказаться в VS Code / `vscode-languageclient`, а не в backend.
  - Mitigation: этот change должен сначала устранить ambiguity и self-noise, чтобы следующий шаг опирался на truthful evidence.

## External References

- LSP 3.17 completion: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_completion
- LSP 3.17 cancellation support: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#cancellation-support
- VS Code `CompletionItemProvider`: https://code.visualstudio.com/api/references/vscode-api#CompletionItemProvider
- VS Code programmatic completion: https://code.visualstudio.com/api/language-extensions/programmatic-language-features#show-code-completion-proposals
