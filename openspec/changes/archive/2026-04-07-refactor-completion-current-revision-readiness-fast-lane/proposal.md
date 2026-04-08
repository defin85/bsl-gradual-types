# Change: stabilize current-revision completion readiness after edit/save churn

## Why
После `refactor-lsp-auxiliary-runtime-isolation` truthful transport seams вернулись в норму: в representative incident bundle `client_to_transport_wait_ms`, `service_future_to_first_poll_wait_ms` и `response_output_handoff_send_wait_ms` больше не объясняют пользовательскую задержку.

Новый dominant failure mode другой: same-file completion после edit/save churn повторно упирается в `prepare_timeout@wait_for_file_version` внутри bounded wait budget, хотя transport ingress и output handoff уже здоровы. В том же bundle отдельно видно, что когда current revision все же догоняет запрос, cold path все еще может тратить секунды в `query_bundle_ir_query`; это отдельная проблема и она не должна маскировать readiness regression.

Текущий spec уже запрещает post-handoff readiness starvation, но representative acceptance пока недостаточно явно фиксирует именно post-edit/save churn profile с auxiliary same-file noise как отдельный regression surface. Нужен узкий change, который зафиксирует этот профиль и разведет две независимые проблемы:
- current-revision readiness после handoff;
- cold semantic/query-body cost после успешного readiness.

## What Changes
- Добавить в `bsl-intellisense-v2` explicit contract, что same-file save-triggered auxiliary churn не должна возвращать completion к `prepare_timeout@wait_for_file_version` после current-revision handoff.
- Добавить в `bsl-intellisense-v2` representative post-edit/save churn acceptance, которая:
  - использует same-file `didChange + didSave + auxiliary same-file noise + completion` профиль;
  - трактует `prepare_timeout@wait_for_file_version` как readiness regression, если truthful transport seams уже в бюджете;
  - отделяет readiness failures от холодного `query_bundle_ir_query`, который диагностируется отдельно.
- Зафиксировать rollout scope как backend current-revision readiness remediation, а не transport/UI investigation и не detached-snapshot architecture work.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `backend/src/bin/lsp_server/server/core/execution_context.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/core/tests.rs`
  - representative live perf reports / incident-based readiness artifacts

## Non-Goals
- Не переоткрывать UI/extension investigation без нового прямого контрдоказательства.
- Не менять transport/handoff architecture, если truthful ingress/egress seams уже остаются в бюджете.
- Не оптимизировать `query_bundle_ir_query` и `collect` в рамках этого change.
- Не внедрять detached immutable snapshot architecture; это остается отдельным long-term change.
