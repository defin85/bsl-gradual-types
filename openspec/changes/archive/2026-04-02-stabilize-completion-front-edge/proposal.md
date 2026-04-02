# Change: Стабилизировать completion front-edge после slot-release

## Почему

`refactor-completion-turn-wait-slot-release` убрал server-side удержание transport slot во время пассивного `turn_wait`, но оставил вне scope три front-edge проблемы, которые теперь доминируют в incident bundles и operator UX:

- extension коррелирует client probes с server traces по `uri + trigger_mode + response timestamp ± window`, из-за чего overlap/churn даёт ambiguous или unavailable correlation вместо request-bound truth;
- `Completion Timeline` panel продолжает polling `bsl.getCompletionTimeline` во время активных completion probes и может добавлять собственный шум в front-edge картину;
- member-access completion по `TriggerCharacter='.'` всё ещё может увидеть `NoMatchingTask` после `didChange`, если producer exact-task уже завершился, но его readiness ещё не наблюдаем на request path.

Следующий change должен закрыть именно эти gaps, не открывая заново transport handoff, не ослабляя fail-closed policy и не превращаясь в общий rewrite observability pipeline.

## Что меняется

- Добавляется request-bound deterministic correlation между client probe и server completion trace на default VS Code path:
  - extension отправляет vendor field `bslProbeId` в completion request;
  - authoritative timeline trace получает echoed field `client_probe_id` в корне trace;
  - response version поднимается `17 -> 18`, а contiguous baseline в `contracts/lsp-completion-timeline/` поднимается `v14 -> v15`.
- `Completion Timeline` panel и incident export переводятся в quiet mode:
  - auto-polling не должен мешать active completion и не должен форсировать свежий timeline fetch в момент churn;
  - webview export по умолчанию переиспользует уже захваченный snapshot;
  - fallback fetch в explicit export command допускается только когда cached capture отсутствует.
- Для same-version member-access completion сохраняется producer-side visibility exact type-index task:
  - completed matching task entry остаётся observable/joinable до момента, когда `serve_only_ready` для той же версии становится наблюдаемым, либо до supersession, `didClose` или shutdown;
  - request path не должен получать spurious `NoMatchingTask` в этом race-окне.
- Сохраняются current-revision, fail-closed и parity-инварианты между `TriggerCharacter='.'` и `Invoked`.

## Impact

- Affected specs: `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/transport_adapter.rs`
  - `backend/src/bin/lsp_server/server/request_context.rs`
  - `backend/src/bin/lsp_server/server/core/deps_and_precompute.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `backend/src/bin/lsp_server/types.rs`
  - `vscode-extension/src/lsp/client/*`
  - `vscode-extension/src/providers/observabilityIncidentBundle*.ts`
  - `vscode-extension/src/providers/completionTimeline*.ts`
  - `vscode-extension/src/commands/observability.ts`
  - `contracts/lsp-completion-timeline/v15/*`
- External references:
  - LSP completion: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_completion
  - LSP cancellation: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#cancellation-support
  - VS Code CompletionItemProvider: https://code.visualstudio.com/api/references/vscode-api#CompletionItemProvider
  - VS Code programmatic completion: https://code.visualstudio.com/api/language-extensions/programmatic-language-features#show-code-completion-proposals
