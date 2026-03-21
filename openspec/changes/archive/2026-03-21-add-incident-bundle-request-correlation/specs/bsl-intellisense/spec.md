## ADDED Requirements
### Requirement: Observability incident bundle даёт request-centric handoff summary поверх raw evidence (MUST)
VS Code extension MUST формировать `incident.json` и `summary.md` так, чтобы типовой completion incident можно было разбирать как набор bounded request-level facts без обязательного чтения полного raw timeline JSON.

Этот derived report MUST:
- сохранять raw attachments отдельными и не подменять их;
- использовать authoritative request list только из `bsl.getCompletionTimeline`, если этот источник доступен;
- выражать capture scope (`uri` или явное отсутствие single-URI scope) без guesswork;
- выражать `request_count`;
- содержать bounded request list для authoritative completion traces;
- переносить в request list ключевые latency/verdict facts из authoritative trace;
- использовать client probes только как optional supplemental correlation layer;
- явно маркировать unavailable/unsupported/ambiguous correlation;
- не вычислять псевдо-`metrics delta` из одного cumulative snapshot.

#### Scenario: Single-document capture получает request-centric summary
- **GIVEN** export bundle содержит authoritative completion timeline, и все captured traces относятся к одному `uri`
- **WHEN** extension формирует `incident.json` и `summary.md`
- **THEN** derived report явно содержит этот `uri`
- **AND** derived report содержит `request_count`
- **AND** derived report включает bounded request list с ключевыми latency/verdict facts для каждого authoritative trace

#### Scenario: Derived report не подменяет missing authoritative request list local probes-данными
- **GIVEN** connected server не вернул authoritative completion timeline
- **WHEN** extension формирует bundle
- **THEN** bundle остаётся валидным partial export
- **AND** request-centric section явно помечается как unavailable или unsupported
- **AND** local client probes не выдаются за authoritative request list

### Requirement: Probe-to-trace correlation остаётся deterministic и fail-closed (MUST)
Если extension дополняет request-centric summary данными из local client probes, такая correlation MUST выполняться только по deterministic bounded rules.

Correlation MUST:
- использовать только уже записанные bounded fields из authoritative trace и probe;
- быть optional;
- не требовать нового server-side request или explicit shared request id;
- не создавать guessed pair, если correlation ambiguous.

При успешной correlation request summary MAY включать bounded client-side supplement, например:
- `probe_id`;
- `client_duration_ms`;
- `client_terminal_state`;
- optional client/server edge delta.

При ambiguous или unavailable correlation derived report MUST:
- оставить request summary валидным и server-centric;
- явно указать ограничение;
- не выдумывать client-side pair.

#### Scenario: Unambiguous correlation переносит bounded client-side supplement
- **GIVEN** authoritative trace и local probe можно сопоставить детерминированно
- **WHEN** extension строит request-centric summary
- **THEN** request entry MAY включать bounded client-side supplement
- **AND** supplement не подменяет authoritative server verdicts и latencies

#### Scenario: Ambiguous correlation не создаёт guessed pair
- **GIVEN** для authoritative trace существует несколько одинаково правдоподобных probe-кандидатов или недостаточно данных для уверенного сопоставления
- **WHEN** extension строит request-centric summary
- **THEN** request entry остаётся без client-side pair
- **AND** derived report явно фиксирует correlation gap
- **AND** bundle не создаёт guessed correlation
