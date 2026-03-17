# lsp-completion-timeline v4

## 4.0.0

- Bumps public response envelope to `version=2`.
- Makes `prepare_details` and `turn_attribution` part of the authoritative trace surface.
- Adds bounded split-prepare routing fields under `prepare_details`:
  - `route`: `head_hit|exact_hit|null`
  - `fail_closed_cause`: `prepare_timeout|exact_deadline|null`

Migration note: timeline consumers must switch to `v4` and expect `response.version=2`.
If tooling reads split-prepare attribution from timeline, it must use the bounded
`prepare_details.route` and `prepare_details.fail_closed_cause` fields instead of inferring
them from raw stage names or private debug output.
