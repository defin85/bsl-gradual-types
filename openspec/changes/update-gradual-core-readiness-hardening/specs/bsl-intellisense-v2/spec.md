## ADDED Requirements

### Requirement: Bootstrap-only implicit module-context fallback MUST NOT become a second semantic truth (MUST)
The system MUST ensure that any transitional bootstrap-only path for implicit module-context symbols is directly bounded by automated evidence and converges to the same shared owner/member truth as the canonical resolved path.

The fallback MUST NOT:
- materialize structural truth that other consumers cannot observe;
- keep reviewed typed `Структура` or typed-row scenarios working through consumer-local reconstruction when shared owner hints are absent;
- survive indefinitely without explicit exit criteria.

#### Scenario: Supported implicit module-context completion stays within the shared semantic contract
- **GIVEN** completion resolves a supported implicit module-context symbol through the transitional fallback
- **WHEN** the owner/member result is compared against the shared semantic path
- **THEN** the resulting owner/type contract is equivalent
- **AND** the fallback does not create consumer-only structural truth

#### Scenario: Reviewed structural scenarios still fail closed without shared owner hints
- **GIVEN** a reviewed typed `Структура` or typed-row completion scenario without shared owner hint input
- **WHEN** completion is executed
- **THEN** the request fails closed instead of reconstructing structural member truth locally
- **AND** the transitional implicit module-context fallback does not mask this drift
