## ADDED Requirements
### Requirement: Existing completion surfaces различают strong и weak pre-method attribution без invented findings (MUST)
Completion Timeline panel, clipboard export и request-centric incident bundle summary MUST переносить `v8` pre-method attribution provenance в человекочитаемом виде.

Human-readable projection MUST:
- явно показывать provenance для pre-method attribution, если connected server возвращает `v8` payload;
- считать `server_before_method_entry_dominant` сильным verdict только для same-request authoritative provenance;
- явно деградировать на `v7`, не выдумывая provenance для старого payload.

#### Scenario: Panel и clipboard показывают provenance рядом с pre-method split
- **GIVEN** extension получает completion timeline `v8` с pre-method provenance
- **WHEN** оператор открывает Completion Timeline panel или копирует visible traces
- **THEN** human-readable output показывает pre-method split вместе с provenance
- **AND** оператор может отличить strong same-request attribution от best-effort fallback

#### Scenario: Incident bundle findings не агрегируют weak attribution как сильный ingress bottleneck
- **GIVEN** incident bundle строится по `v8` payload, где trace использует best-effort fallback provenance
- **WHEN** extension формирует `incident.json` и `summary.md`
- **THEN** request summary сохраняет bounded raw fact
- **AND** derived findings не считают такой trace сильным `server_before_method_entry` bottleneck

#### Scenario: Extension явно деградирует на `v7`
- **GIVEN** connected server возвращает completion timeline `v7`
- **WHEN** extension формирует panel, clipboard или incident bundle
- **THEN** extension не выдумывает `v8` provenance
- **AND** человекочитаемый output явно отмечает, что trustworthy pre-method attribution fields unavailable by design
