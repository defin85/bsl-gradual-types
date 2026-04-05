# Temporary Agent Override

Reason: 2026-04-05 completion observability bundles show `client_before_transport_write_wait_ms`
at 1-2ms while dominant latency remains in server/transport ingress and egress handoff. Until new
evidence contradicts this, UI-first investigation is a waste of time for this class of incident.

## Scope

- Applies only to completion latency / observability investigations.
- Does not apply to explicit UI bugs, UX work, or intentional `vscode-extension/` feature changes.

## Temporary Rule

- Do not treat VS Code UI rendering or extension pre-send work as the primary suspect for
  completion latency by default.
- Do not start investigations in `vscode-extension/` request dispatch code unless a fresh bundle
  shows materially elevated `client_before_transport_write_wait_ms` or other direct contradictory
  evidence.
- Prioritize backend transport/inbound/outbound path analysis, especially:
  - `backend/src/bin/lsp_server/server/transport_adapter.rs`
  - ingress before `adapter_read_at_ms`
  - egress around `response_output_handoff_send_wait_ms`

## Removal Condition

- Remove this override once a newer authoritative bundle shows that UI / extension pre-send latency
  is materially contributing again, or once the current transport-side investigation is complete.
