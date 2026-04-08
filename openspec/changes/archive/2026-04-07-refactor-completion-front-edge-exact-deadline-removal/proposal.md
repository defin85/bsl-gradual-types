# Change: устранить скрытый `exact_deadline` в front-edge completion window

## Почему
Acceptance-review change `refactor-completion-front-edge-readiness-window` показал, что front-edge regression фактически не закрыт.

Новый fast path убрал `prepare_timeout`, но checked-in evidence по `p42` всё ещё показывает immediate same-file front-edge completion, которая исчерпывает bounded `wait_exact_type_index` и завершает запрос fail-closed:
- `type_index_wait_outcome=deadline`;
- `dominant_stage=wait_exact_type_index`;
- `measured_successful_traces=0`;
- `measured_fail_closed_traces=10`.

Дополнительно regression теперь маскируется:
- `fail_closed_cause` остаётся `null`;
- public reason схлопывается в generic `missing_semantic_index`;
- representative gate считается зелёным даже если все measured samples fail-closed и ни один sample не проходит readiness до cold `query_bundle_pool_wait`.

Это означает, что текущая реализация не доставила requirement "front-edge readiness window bounded current-revision first response" и ослабила truthful regression surface вместо его устранения.

## Что меняется
- Front-edge same-file completion сразу после `didChange/didSave` MUST NOT уходить в скрытый `exact_deadline`, если current-revision handoff уже зарегистрирован и truthful transport seams healthy.
- Если exact artifact всё ещё не стал наблюдаемым в этом окне, outcome MUST оставаться explicit regression signal, а не маскироваться под generic `missing_semantic_index` без причины.
- Representative front-edge gate MUST доказывать не только отсутствие `prepare_timeout`, но и наличие как минимум одного successful current-revision sample в том же profile.
- Cold `query_bundle_pool_wait` после успешного readiness MUST report-иться отдельным diagnostic bucket и не может считаться подтверждённым, если gate не увидел ни одного successful sample.
- Validation/evidence должны включать durable checked-in artifact, по которому воспроизводимо видно:
  - исчезновение front-edge `exact_deadline`;
  - наличие successful current-revision sample;
  - отдельное состояние downstream cold `query_bundle_pool_wait`, если оно ещё остаётся.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `backend/src/bin/lsp_server/server/core/deps_and_precompute.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/core/tests.rs`
  - `scripts/validate-v2-completion-gates.sh`
  - checked-in perf/incident evidence for front-edge profile

## Не цели
- Не оптимизировать широкий cold query-body path вне front-edge remediation.
- Не переоткрывать transport/UI investigation.
- Не ослаблять fail-closed semantics и не вводить stale substitute.
