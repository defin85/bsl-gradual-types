## 1. Specification
- [ ] 1.1 Уточнить в `bsl-intellisense-ide-grade`, что auxiliary outline maintenance обязана оставаться runtime-isolated от interactive semantic path.
- [ ] 1.2 Добавить в `bsl-intellisense-v2` contract для bounded/isolated execution CPU-heavy auxiliary LSP work вместо inline async-runtime execution.
- [ ] 1.3 Добавить в `bsl-intellisense-v2` requirement для representative mixed-load gate с truthful ingress/egress seams.

## 2. Design
- [ ] 2.1 Описать исполнительную границу между async LSP transport/runtime loop и CPU-heavy auxiliary work.
- [ ] 2.2 Зафиксировать remediation scope для `documentSymbol` ready-cache materialization, same-version outline refresh и `bsl.getCurrentContext`.
- [ ] 2.3 Зафиксировать acceptance metrics и gate signals для `client_to_transport_wait_ms`, `service_future_to_first_poll_wait_ms` и `response_output_handoff_send_wait_ms`.
- [ ] 2.4 Явно зафиксировать non-goals: без redesign transport fairness, без semantic change `documentSymbol` outcomes.

## 3. Implementation
- [ ] 3.1 Перенести CPU-heavy post-parse `build_document_symbols(...)` из async runtime path в bounded auxiliary execution path для background outline materialization.
- [ ] 3.2 Перенести parse/context derivation в `bsl.getCurrentContext` из inline async handler path в bounded auxiliary execution path без изменения user-visible result contract.
- [ ] 3.3 Обновить representative mixed-load perf gate и related live artifacts так, чтобы они fail-или на truthful ingress/egress starvation, даже если legacy pre-dispatch split остаётся в бюджете.
- [ ] 3.4 Подтвердить, что `latest_ready` / same-version outline refresh сохраняют bounded publish semantics и не деградируют completion correctness.

## 4. Validation
- [ ] 4.1 Провалидировать change: `openspec validate refactor-lsp-auxiliary-runtime-isolation --strict --no-interactive`.
- [ ] 4.2 Прогнать focused regression: `cargo test -p bsl-backend p33_document_symbol_burst_does_not_delay_completion_first_poll_under_parse_gap -- --nocapture`.
- [ ] 4.3 Прогнать representative mixed-load gate: `cargo test -p bsl-backend p39_real_conf_big_document_symbol_mixed_load_gate_live -- --nocapture`.
- [ ] 4.4 Прогнать focused regression для concurrent `bsl.getCurrentContext` и completion на крупном модуле (existing или new targeted test).
