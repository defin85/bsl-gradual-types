# lsp-completion-timeline v15

## 15.0.0

- Bumps public response envelope to `version=18`.
- Preserves all `v14` authoritative timing fields and adds root-level
  per-request `client_probe_id` to each trace when the incoming completion
  request carried `bslProbeId` on the default VS Code path.
- Keeps `client_probe_id` optional so legacy payloads and non-probe completion
  requests remain valid, but makes deterministic probe-to-trace correlation a
  first-class contract field for incident tooling.

Migration note: timeline consumers must switch to `v15` and expect
`response.version=18`. Tooling that validates or documents the
server-generated payload must read optional root-level `client_probe_id` and
degrade gracefully for `v17` payloads where request-bound probe correlation is
unavailable by design.
