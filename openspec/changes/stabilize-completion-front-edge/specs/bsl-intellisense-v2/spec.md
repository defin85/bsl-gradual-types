## ADDED Requirements

### Requirement: Completion timeline использует request-bound probe correlation key (MUST)
Если default VS Code completion path отправляет namespaced vendor correlation key `bslProbeId` для completion probe, authoritative LSP completion timeline MUST переносить этот opaque key в root-level field `client_probe_id` соответствующего per-request trace.

Derived extension surfaces, которые коррелируют local completion probes с server traces, MUST использовать этот key как primary correlation source. Timestamp/window эвристика MAY использоваться только как backward-compatible fallback для старых payload'ов и MUST оставаться fail-closed при отсутствии deterministic correlation.

Этот requirement дополняет существующий server-driven timeline contract и existing fail-closed правила для client-side ingress supplement; он MUST NOT заменяться guessed attribution по одним только local probe timings.

Реализация этого requirement MUST поднять authoritative completion timeline response version `17 -> 18` и contiguous contract baseline `contracts/lsp-completion-timeline/v14 -> v15`.

`client_probe_id` MUST оставаться только per-request correlation marker и MUST NOT становиться частью агрегированных observability counters, histograms или human-readable guessed summaries без authoritative trace.

#### Scenario: Overlap completion requests коррелируются без ambiguity
- **GIVEN** два completion probe для одного `uri` и одинакового `trigger_mode` перекрываются по времени
- **AND** оба request'а несут разные request-bound correlation keys
- **WHEN** сервер публикует authoritative completion timeline и extension строит incident bundle
- **THEN** каждый trace коррелируется с ровно одним probe по echoed correlation key
- **AND** incident bundle не получает `multiple_probe_candidates` только из-за overlap и близких timestamps

#### Scenario: Старый payload без echoed key деградирует fail-closed
- **GIVEN** extension получает timeline payload старой версии без request-bound correlation key
- **WHEN** derived surface пытается дополнить trace client-side ingress verdict
- **THEN** fallback MAY использовать legacy timestamp/window heuristic
- **AND** verdict не публикуется, если deterministic correlation всё равно не доказана

### Requirement: Completion Timeline panel остаётся quiet во время active completion (MUST)
VS Code `Completion Timeline` panel MUST NOT создавать дополнительный front-edge noise на default completion path.

Auto-refresh/polling MUST приостанавливаться или переходить в bounded backoff, пока есть active completion probes, и в течение короткого quiet window после их завершения. Incident export и panel rendering MUST по умолчанию использовать последний уже захваченный authoritative snapshot, а не форсировать fresh `bsl.getCompletionTimeline` в момент churn.

Explicit export command MAY делать fresh fetch только когда cached capture отсутствует; он MUST NOT обходить quiet policy в случае, когда authoritative snapshot уже захвачен webview path.

Manual refresh MAY оставаться доступным, но он MUST быть явным действием оператора, а не скрытым side effect active observability view.

#### Scenario: Видимая panel не мешает активному completion
- **GIVEN** `Completion Timeline` panel открыта и видима
- **AND** пользователь вызывает completion во время typing/load
- **WHEN** extension отслеживает active completion probes
- **THEN** panel не инициирует обычный polling `bsl.getCompletionTimeline`, пока active probe ещё не завершён
- **AND** incident/export path использует последний already-captured snapshot вместо forced fresh fetch

#### Scenario: Manual refresh остаётся explicit после quiet window
- **GIVEN** active completion probes уже завершились и quiet window истёк
- **WHEN** оператор вручную инициирует refresh panel
- **THEN** extension делает новый timeline fetch явным образом
- **AND** этот refresh не маскируется под background auto-polling

### Requirement: Same-version member-access completion не теряет didChange-produced exact-task visibility (MUST)
Если `didChange` уже запланировал exact type-index producer task для текущей версии файла, member-access completion на той же версии MUST наблюдать либо matching producer task, либо уже опубликованное `serve_only_ready` состояние до terminal decision request path.

Race-окно, в котором producer task завершился и исчез из registry раньше, чем readiness стала наблюдаемой, MUST NOT приводить к spurious `NoMatchingTask` для same-version `TriggerCharacter='.'` или `Invoked` request'ов.

Completed matching same-version task entry MUST оставаться observable/joinable до одного из bounded terminal cleanup events:
- `serve_only_ready` для той же версии стал наблюдаемым;
- task superseded новой версией;
- файл закрыт через `didClose`;
- сервер выполняет shutdown cleanup.

При этом request path MUST NOT сам создавать exact producer task, MUST NOT переходить на stale semantic fallback и MUST сохранять bounded fail-closed behaviour для genuine cold miss, wrong-version и deadline cases.

#### Scenario: Same-version `TriggerCharacter='.'` ждёт producer вместо spurious `NoMatchingTask`
- **GIVEN** `didChange` уже запланировал exact type-index producer task для версии `V`
- **AND** completion по `TriggerCharacter='.'` приходит для той же версии `V` до публикации observable `serve_only_ready`
- **WHEN** request path ожидает current-revision exact readiness
- **THEN** waiter видит matching producer task или готовый exact artifact для версии `V`
- **AND** request не завершается `NoMatchingTask` только из-за короткого race между producer completion и readiness publication

#### Scenario: Genuine cold miss остаётся bounded fail-closed
- **GIVEN** matching producer task для текущей версии не существует либо уже superseded другой версией
- **WHEN** member-access completion ждёт exact readiness в пределах bounded wait budget
- **THEN** request path остаётся fail-closed и bounded
- **AND** система не создаёт exact producer task на request path и не возвращает stale semantic substitute

#### Scenario: Completed matching task очищается только по bounded cleanup rules
- **GIVEN** exact producer task для текущей версии уже завершил compute, но matching same-version waiter ещё может обратиться к registry
- **WHEN** `serve_only_ready` ещё не наблюдаем и не произошло supersession, `didClose` или shutdown
- **THEN** completed task entry остаётся observable для same-version waiter
- **AND** cleanup не происходит преждевременно только из-за факта single-run completion
