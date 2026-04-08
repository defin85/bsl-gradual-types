# Change: шаг 2 сделать current-context latest-only и supersession-aware

## Почему

После step-1 completion path больше не должен скрыто ждать aged exact re-probe, но incident bundle всё
ещё показывает фоновую cursor-driven нагрузку без пользовательской ценности:

- concurrent contenders класса `bsl.getCurrentContext` доходят до `4`;
- `runtime_queue_wait_background_ms.p95=33516`;
- extension шлёт `bsl.getCurrentContext` по cursor/selection updates с одним только debounce.

Change `refactor-lsp-auxiliary-runtime-isolation` уже вынес тяжёлый parse/context derivation с async
runtime loop, но не ввёл latest-only policy. В результате устаревшие current-context запросы могут
накопляться и конкурировать за bounded auxiliary ресурсы даже тогда, когда пользователь уже ушёл на
новую позицию курсора.

## Что меняется

- VS Code extension MUST сделать `bsl.getCurrentContext` latest-only per editor session/generation и не применять stale responses к status-bar surface.
- Протокол current-context request MUST нести bounded generation hints, достаточные для server-side supersession/coalescing obsolete auxiliary work.
- Backend MUST обрывать или коалесцировать obsolete same-session current-context work до expensive parse/context derivation, когда более новая generation уже известна.

## Impact

- Спецификация: `bsl-intellisense`, `bsl-intellisense-v2`
- VS Code extension: `contextProvider` и status-bar update path
- Backend: `bsl.getCurrentContext` admission/supersession policy
- Validation: cursor-burst mixed-load coverage и bounded auxiliary inflight assertions

## Не цели

- замена `workspace/executeCommand` на другой transport в этом шаге
- изменение completion semantic contract или aged completion remediation из step-1
