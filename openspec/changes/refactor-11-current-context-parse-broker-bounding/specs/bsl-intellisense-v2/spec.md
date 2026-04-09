## MODIFIED Requirements
### Requirement: Auxiliary LSP CPU work stays isolated from interactive transport/runtime loops (MUST)
CPU-heavy auxiliary LSP work, не являющаяся primary semantic body текущего interactive ответа, MUST выполняться через bounded blocking или эквивалентную isolated CPU boundary и MUST NOT выполняться inline на async runtime threads, которые обслуживают:
- transport read/write loops;
- admission и service scheduling;
- first polling service futures;
- completion handoff/output progression.

Этот contract MUST покрывать как минимум:
- documentSymbol ready-cache materialization и same-version outline refresh, инициированные document-sync path;
- parse/context derivation для auxiliary request path `bsl.getCurrentContext`, когда для ответа нужен полный parse текущего текста файла.

Для `bsl.getCurrentContext` same-file same-revision/text bursts MUST дополнительно:
- broker-иться до входа в blocking CPU boundary;
- допускать не более одного leader parse/context derivation на эквивалентный key;
- не заставлять follower requests получать independent blocking CPU permits только ради ожидания leader parse;
- завершаться через shared async wait или bounded empty outcome при supersession/budget exhaustion.

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

#### Scenario: Current-context burst делит один leader parse вместо нескольких blocking holders
- **GIVEN** несколько same-file `bsl.getCurrentContext` requests попадают на один и тот же current document text без ready snapshot
- **WHEN** первый request начинает parse/context derivation
- **THEN** сервер допускает только один leader parse для этого key
- **AND** follower requests не получают отдельные blocking CPU permits только ради ожидания leader parse
- **AND** parse fan-out остаётся bounded одним leader parse

### Requirement: `bsl.getCurrentContext` honors client latest-only generations with bounded supersession (MUST)

Server MUST honor bounded client latest-only generations for `bsl.getCurrentContext`.

Если client current-context surface передаёт bounded generation hints для `bsl.getCurrentContext`,
server MUST использовать их для bounded supersession/coalescing obsolete auxiliary work.

Для одного editor session backend:

- MUST NOT позволять obsolete older generations неограниченно накапливать independent expensive parse/context derivation;
- MUST supersede older generation до expensive parse/context derivation или коалесцировать её с эквивалентным newer work;
- MUST prefer exact ready parse snapshot текущей revision, когда он уже доступен;
- MUST NOT запускать independent parse followers для того же same-file same-revision/text key, если leader parse уже существует;
- MUST NOT делать obsolete response источником current context для newer generation;
- MAY по-прежнему возвращать bounded auxiliary response для superseded request, если это не нарушает newest-generation-wins semantics на client side;
- MAY завершать superseded или over-budget follower пустым response, пока leader продолжает прогрев reusable parse artifact.

#### Scenario: Cursor burst supersede-ит obsolete current-context work до expensive parse

- **GIVEN** extension отправляет несколько `bsl.getCurrentContext` requests одного editor session с монотонно растущими generation hints
- **AND** более новая generation становится известна серверу до завершения expensive parse для older request
- **WHEN** backend обслуживает этот burst
- **THEN** older request не доходит независимо до полного expensive parse/context derivation
- **AND** auxiliary path остаётся bounded по obsolete work
- **AND** newer generation остаётся единственным current candidate для client-visible context surface

#### Scenario: Same-revision burst коалесцируется за одним leader parse
- **GIVEN** extension отправляет несколько current-context requests для одной и той же revision/text до появления ready snapshot
- **WHEN** backend уже запустил leader parse для самого нового запроса
- **THEN** остальные эквивалентные запросы не запускают independent expensive parse/context derivation
- **AND** либо ждут shared result bounded образом, либо получают empty response при supersession/budget exhaustion
- **AND** newest-generation-wins semantics остаётся сохранённой
