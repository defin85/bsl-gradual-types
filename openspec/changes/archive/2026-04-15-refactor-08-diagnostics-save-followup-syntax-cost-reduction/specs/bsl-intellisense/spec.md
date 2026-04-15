## MODIFIED Requirements
### Requirement: Incident bundle экспортирует diagnostics save timeline как отдельный authoritative source (MUST)
When the backend supports diagnostics save timeline export, the extension MUST surface that timeline
as an authoritative server-authored source in observability incident bundles.

The surfaced timeline MUST:

- preserve `save_fastlane` vs `idle_heavy` save-cycle attribution;
- stay explicit about unsupported / unavailable server capability;
- remain request-centric for a single `didSave` cycle;
- surface whether `idle_heavy` syntax work was reused from same-version artifacts or recomputed,
  when the backend provides that distinction.

#### Scenario: bundle keeps diagnostics save follow-up projection truthful
- **GIVEN** the backend returns diagnostics save timeline traces
- **WHEN** the extension exports an incident bundle
- **THEN** the bundle summary keeps `save_fastlane` and `idle_heavy` attribution explicit
- **AND** syntax reuse vs recompute is shown when provided by the server
