# mcp-bsl-agent-semantic v1

## 1.0.0

- Introduce the first machine-readable baseline for MCP semantic result envelopes.
- Lock public payload shapes and fail-closed invariants for `bsl_type_at_position`, `bsl_members`, and `bsl_definition`.
- Keep shared fail-closed reason codes in observability; MCP payloads remain transport-only and do not expose synthetic semantic fallback states.
