## ADDED Requirements

### Requirement: Auxiliary LSP CPU work stays isolated from interactive transport/runtime loops (MUST)
CPU-heavy auxiliary LSP work, не являющаяся primary semantic body текущего interactive ответа, MUST выполняться через bounded blocking или эквивалентную isolated CPU boundary и MUST NOT выполняться inline на async runtime threads, которые обслуживают:
- transport read/write loops;
- admission и service scheduling;
- first polling service futures;
- completion handoff/output progression.

Этот contract MUST покрывать как минимум:
- documentSymbol ready-cache materialization и same-version outline refresh, инициированные document-sync path;
- parse/context derivation для auxiliary request path `bsl.getCurrentContext`, когда для ответа нужен полный parse текущего текста файла.

Auxiliary jobs MAY оставаться bounded, cancellable и coalesced, но MUST NOT вызывать seconds-scale `client_to_transport_wait_ms`, `service_future_to_first_poll_wait_ms` или `response_output_handoff_send_wait_ms` regressions для same-file interactive completion, если primary completion path уже hot/ready.

#### Scenario: Background outline materialization не выполняет symbol building inline на async runtime
- **GIVEN** document-sync worker уже завершил bounded parse для requested revision
- **WHEN** сервер materializes latest-ready outline cache для того же файла
- **THEN** CPU-heavy symbol derivation выполняется через bounded auxiliary CPU boundary
- **AND** newer same-file completion не теряет runtime progress только из-за этого auxiliary work

#### Scenario: `bsl.getCurrentContext` parse не starvation-ит concurrent completion
- **GIVEN** extension почти одновременно вызывает `bsl.getCurrentContext` и `textDocument/completion` для крупного модуля
- **AND** current-context request требует parse/context derivation
- **WHEN** сервер обслуживает оба запроса
- **THEN** current-context auxiliary CPU work не выполняется inline на async transport/runtime loop
- **AND** completion trace не получает seconds-scale ingress или output-handoff delay только из-за `bsl.getCurrentContext`

### Requirement: Representative mixed-load guard budgets truthful ingress and handoff seams (MUST)
Representative mixed-load regression coverage для completion MUST budget-ить truthful latency seams, которые остаются user-visible после probe/egress split, а не только legacy pre-dispatch ingress split.

Guard MUST как минимум:
- использовать same-file profile `didChange + didSave + documentSymbol burst + completion` на representative large-module fixture;
- собирать authoritative fields `client_to_transport_wait_ms`, `service_future_to_first_poll_wait_ms` и `response_output_handoff_send_wait_ms`;
- fail-ить, если auxiliary runtime work уводит trace в seconds-scale ingress или handoff backlog, даже если `adapter_to_dispatch_wait_ms` остаётся в бюджете;
- сохранять existing correctness checks для non-empty completion, fail-closed counters и `documentSymbol latest_ready` behavior.

#### Scenario: Truthful mixed-load gate ловит starvation, скрытую от legacy pre-dispatch split
- **GIVEN** representative same-file mixed-load profile на крупном модуле
- **AND** completion handler hot path уже ready или fast
- **WHEN** auxiliary outline/context work regression-ит и stall-ит transport ingress или completion handoff
- **THEN** representative gate завершается ошибкой по truthful `client_to_transport_wait_ms` или `response_output_handoff_send_wait_ms`
- **AND** regression не маскируется только потому, что `adapter_to_dispatch_wait_ms` остался в бюджете
