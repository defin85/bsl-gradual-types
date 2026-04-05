# Change: стабилизировать current-revision completion front-edge readiness window

## Почему
Свежий incident bundle `2026-04-05T15:40:13Z` показывает новый остаточный failure mode после `refactor-completion-current-revision-readiness-fast-lane`.

Truthful transport seams остаются здоровыми:
- `client_to_transport_wait_ms=0..6ms`;
- `service_future_to_first_poll_wait_ms=0..1ms`;
- `response_output_handoff_send_wait_ms=0ms`.

При этом same-file completion в самом начале post-edit/save окна всё ещё может завершаться `prepare_timeout`:
- traces `1..4` падают на `prepare_timeout@wait_for_file_version`;
- traces `1` и `5` доходят до `snapshot_with_deps` только на самом краю prepare budget и завершаются `prepare_timeout@snapshot_with_deps`;
- все эти samples происходят сразу после `didChange/didSave`, когда `time_since_last_did_change_sent_ms` остаётся в пределах единиц миллисекунд;
- позже тот же файл уже успешен: trace `6` проходит readiness и показывает отдельный cold bottleneck `query_bundle_pool_wait`, а trace `7` полностью hot и занимает `1ms`.

Это означает, что предыдущий fix убрал interleaved interactive apply backlog, но не закрыл весь front-edge gap между same-file handoff и первой полностью готовой current-revision prepare path. Следующий change должен лечить именно это окно и отдельно не смешивать его с cold `query_bundle_pool_wait`.

## Что меняется
- Добавляется explicit contract для front-edge current-revision readiness window:
  - same-file completion сразу после `didChange/didSave` MUST NOT завершаться `prepare_timeout` только потому, что front-edge current-revision prepare path ещё не стал наблюдаемым;
  - regression surface включает и `prepare_timeout@wait_for_file_version`, и `prepare_timeout@snapshot_with_deps`, если truthful transport seams уже healthy.
- Добавляется representative front-edge acceptance для same-file профиля `didChange + didSave + immediate completion burst`:
  - gate проверяет именно первые completion samples сразу после handoff;
  - gate fail-ит на любом `prepare_timeout` в этом окне;
  - cold `query_bundle_pool_wait` после успешного readiness report-ится отдельно и не считается объяснением front-edge timeout.
- Rollout scope остаётся узким:
  - backend readiness/publication boundary между handoff, `wait_for_file_version` и `snapshot_with_deps`;
  - без reopening transport/UI scope;
  - без оптимизации `query_bundle_pool_wait` в рамках этого change.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `backend/src/bin/lsp_server/server/core/execution_context.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade/runtime.rs`
  - `backend/src/bin/lsp_server/server/core/tests.rs`
  - representative live reports / incident-bundle evidence

## Не цели
- Не переоткрывать transport, output handoff или UI investigation.
- Не оптимизировать `query_bundle_pool_wait`, `collect` или другой cold query-body path в рамках этого change.
- Не вводить stale substitute или ослаблять current-revision truth.
- Не подменять этот remediation long-term detached snapshot architecture.
