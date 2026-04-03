## MODIFIED Requirements

### Requirement: VS Code extension ведёт bounded client-side completion probe buffer (MUST)
VS Code extension MUST вести bounded in-memory ring buffer последних client-side completion probes на основном activation/runtime path.

Probe buffer MUST:

- быть wired на default `LanguageClient` path, используемый обычной активацией extension;
- использовать deterministic oldest-first eviction;
- хранить только bounded/redacted probe fields;
- оставаться session-local и in-memory only.

Каждый probe MUST включать только bounded metadata:

- `probe_id`;
- `uri`;
- `document_version`;
- `document_version_at_terminal`;
- `trigger_mode` и optional `trigger_character`;
- `request_started_at_ms`;
- `request_completed_at_ms`;
- explicit transport-phase milestones, достаточные для отделения:
  - client enter;
  - LSP request dispatch;
  - raw transport response receive;
  - LSP promise resolve;
  - client terminal;
- terminal status/result summary;
- bounded `result_kind` vocabulary;
- bounded `item_count_bucket`;
- `is_incomplete`, только если этот сигнал доступен без guesswork;
- `time_since_last_local_edit_ms`;
- `time_since_last_did_change_sent_ms` либо явное значение `unknown`, если этот сигнал недоступен;
- bounded cancellation diagnostics: `cancel_reason_hint` из vocabulary `superseded_same_version|superseded_newer_version|editor_state_changed|unknown`, optional `superseded_by_probe_id`, optional `superseded_after_ms`;
- bounded overlap/drift diagnostics: `did_change_count_during_probe`, `cursor_moved_during_probe`, `active_completion_count_at_start`, `same_uri_probe_overlap_count`, `newer_probe_started_before_terminal`;
- derived context flags вроде `is_after_dot` и `identifier_tail_length`.

Если raw transport response receive boundary недоступна на конкретном runtime path, probe MUST фиксировать это explicit bounded marker'ом unavailable/unknown и MUST NOT silently подменять receive timestamp временем promise resolution.

Probe buffer MUST NOT:

- хранить raw document text, line prefixes или произвольные snippets;
- хранить unbounded free-form labels;
- требовать отдельного persistent telemetry pipeline в рамках этой capability;
- требовать protocol-level `client_probe_id` или trace-level correlation с `Server Timeline`.

#### Scenario: Probe отделяет raw receive от promise resolution

- **GIVEN** completion probe завершился успешным LSP response
- **WHEN** extension записывает transport-phase milestones
- **THEN** probe отдельно фиксирует raw transport response receive и LSP promise resolve
- **AND** не смешивает эти две границы в один timestamp

#### Scenario: Недоступный receive boundary не подменяется guessed timestamp

- **GIVEN** на конкретном runtime seam raw transport receive boundary нельзя наблюдать детерминированно
- **WHEN** extension завершает запись client-side probe
- **THEN** probe явно помечает receive boundary как unavailable или unknown
- **AND** не записывает promise-resolution timestamp под видом raw receive

## ADDED Requirements

### Requirement: Existing completion surfaces переносят `v21` post-response gap split без guessed root cause (MUST)
Completion Timeline panel, clipboard export и request-centric incident bundle summary MUST переносить `v21` flush-aware server egress split и новый client probe receive/resolve split в человекочитаемом виде.

Human-readable projection MUST:

- показывать `response_ready_to_flush_wait_ms`, если connected server возвращает `v21` payload с flush-aware boundary;
- при deterministic correlation и наличии нового probe split показывать отдельно `transport_to_client_receive_wait_ms`, `client_receive_to_resolve_wait_ms` и existing `client_post_response_ms`;
- сохранять existing `client_to_transport_wait_ms` как отдельный ingress bucket;
- MAY сохранять compatibility umbrella вроде `server_to_client_post_response_ms`, но MUST NOT использовать её как единственный evidence bucket, если новый split доступен;
- явно деградировать на `v20` и на legacy probe paths, не выдумывая flush или raw-receive boundaries.

#### Scenario: Panel и clipboard показывают split post-response tail

- **GIVEN** extension получает completion timeline `v21`
- **AND** correlated probe содержит raw receive и promise resolve milestones
- **WHEN** оператор открывает Completion Timeline panel или копирует visible trace
- **THEN** output показывает server egress wait отдельно от transport-after-flush и client-after-receive waits
- **AND** оператору не нужно читать raw JSON, чтобы увидеть этот split

#### Scenario: Incident bundle summary не обвиняет одну сторону при incomplete split

- **GIVEN** connected server возвращает `v20` payload или correlated probe не имеет raw receive boundary
- **WHEN** extension формирует `incident.json` и `summary.md`
- **THEN** summary явно отмечает, что post-response gap split unavailable by design для этой evidence version
- **AND** derived handoff не переименовывает opaque tail в точный server-side или client-side виновник
