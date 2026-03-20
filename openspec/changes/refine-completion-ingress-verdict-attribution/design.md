## Контекст
Сейчас completion observability уже даёт:
- server-driven `Completion Timeline v6`;
- request-centric `incident.json` и `summary.md`;
- optional deterministic probe correlation для части requests.

Однако human-readable ingress verdict layer построен слишком грубо:
- в shared helper verdict `ingress_before_method_entry` появляется при условии `transport_to_method_wait_ms >= method_prelude_exec_ms`;
- при `0 >= 0` это даёт ложный ingress verdict даже на горячем пути;
- aggregated findings в bundle наследуют эту ошибку и завышают масштаб ingress-проблемы.

На практике это мешает следующему шагу root-cause анализа: сырой payload показывает одно, а summary/verdict layer — другое.

## Цели
- Сделать ingress verdicts truthful и conservative.
- Различать хотя бы три понятных случая:
  - client-side lag до `transport_received`;
  - server-side lag до `method_entered`;
  - handler prelude dominance.
- Не менять raw transport и не вводить новый contract version.
- Держать derived report fail-closed: если данных недостаточно, новый verdict не появляется.

## Не-цели
- Не добавлять новые telemetry поля.
- Не менять correlation rules и не пытаться коррелировать больше traces, чем сейчас.
- Не чинить сами runtime latency bottlenecks.

## Решение

### 1. Positive-dominance policy
Ingress verdict строится только при положительной и действительно доминирующей задержке.

Предлагаемая модель:
- `client_before_transport_dominant`
  - только если есть deterministic correlation;
  - только если `client_to_transport_wait_ms > 0`;
  - и этот wait доминирует над `transport_to_method_wait_ms` и `method_prelude_exec_ms`.
- `server_before_method_entry_dominant`
  - только если `transport_to_method_wait_ms > 0`;
  - и этот wait доминирует над `method_prelude_exec_ms`.
- `handler_prelude_dominant`
  - только если `method_prelude_exec_ms > 0`;
  - и он доминирует над `transport_to_method_wait_ms`.

Если все соответствующие waits равны `0` или данных недостаточно:
- ingress verdict не должен появляться вообще.

### 2. Fail-closed client supplement
Client-side ingress verdict не должен строиться из общих probe counts или loose heuristics.

Если correlation unavailable или ambiguous:
- request остаётся server-centric;
- summary может указать correlation gap;
- verdict `client_before_transport_dominant` не появляется.

### 3. Truthful aggregation в bundle
`summary.md` и `incident.json.findings` должны считать ingress verdicts по refined rules:
- hot traces без положительного ingress wait не попадают в ingress findings;
- client-side и server-side ingress считаются отдельно;
- общий текст findings не должен скрывать, на каком ingress-подслое наблюдается bottleneck.

### 4. Existing completion surfaces
`Completion Timeline` panel, clipboard и incident bundle должны использовать одну и ту же семантику verdicts.

Практически это означает:
- shared helper остаётся источником истины для TypeScript-side derived verdicts;
- inline webview projection обновляется до тех же правил и vocabulary;
- smoke/tests фиксируют одинаковый смысл verdicts на одинаковом input.

## Риски и trade-offs

### Риск: смена vocabulary сломает текущие smoke/assertions
Смягчение:
- change явно обновляет smoke, docs и regression tests;
- новые verdict names фиксируются в spec delta.

### Риск: слишком строгие правила скроют реальную ingress-проблему
Смягчение:
- raw latency lines никуда не исчезают;
- change затрагивает только human-readable verdict layer;
- при сомнении предпочитается отсутствие verdict, а не ложноположительный bottleneck.

### Риск: drift между shared helper и webview JS
Смягчение:
- change требует coverage на оба consumer path;
- acceptance фиксирует одинаковый смысл verdicts на одинаковых traces.

## Acceptance-направление
- Hot trace с нулевыми ingress/prelude waits не получает ingress verdict.
- Correlated trace с доминирующим `client_to_transport_wait_ms` получает client-side ingress verdict.
- Trace с положительным `transport_to_method_wait_ms`, доминирующим над `method_prelude_exec_ms`, получает server-side ingress verdict.
- Incident bundle findings не формулируют общий ingress bottleneck для traces, где positive ingress wait отсутствует.
