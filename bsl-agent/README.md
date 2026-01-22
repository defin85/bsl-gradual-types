# bsl-agent (MCP)

Local MCP server (stdio) that provides semantic context for BSL projects.

## MCP inputs (workspace_open)

Most behavior is configured via MCP tool inputs (not env):
- `roots[]` - workspace roots (sandbox)
- `platform_docs_archive` - path to 1C Syntax Helper (file or directory)
- `platform_version` - e.g. `8.3.27`
- `configuration_path` - path to config dump (optional)
- `mode` - optional mode switch

Notes:
- Single-session: a single `bsl-agent` process allows at most one active workspace session.
  - Calling `workspace_open` again with the same params is idempotent (returns the same `session_id`).
  - Calling `workspace_open` with different params returns `INVALID_PARAMS`; call `workspace_close` first to switch.

## Environment variables

Logging:
- `RUST_LOG` - e.g. `bsl_agent=info` or `bsl_agent=debug,info`

Disk cache (shared with backend startup):
- `BSL_CACHE_DIR` - cache root directory (overrides XDG/HOME defaults)
- `BSL_CACHE_DISABLE=1|true|yes` - disable disk cache reads/writes
- `BSL_CACHE_STRICT_FINGERPRINT=1` - stricter fingerprinting for cache keys
- `BSL_CACHE_TTL_SECS` - TTL in seconds for cache entries (optional)
- `BSL_CACHE_TTL_MODE=created|idle` - TTL base timestamp (default: `created`)
- `BSL_CACHE_MAX_BYTES` - max cache size, triggers cleanup (optional)
- `BSL_CACHE_CLEANUP_INTERVAL_SECS` - cleanup loop period (default: `300`)
- `BSL_CACHE_TOUCH_INTERVAL_SECS` - keep-alive touch interval (default: `60`)
- `BSL_CACHE_SWR=1|true|yes|0|false|no` - stale-while-revalidate mode (default: enabled)

In-memory AST cache:
- `BSL_AST_CACHE_CAPACITY` - LRU capacity (default: 64)

Persisted state (sessions/jobs, for resume):
- Stored under `<cache_root>/bsl-agent-state/v1/` (derived from `BSL_CACHE_DIR` or XDG/HOME fallbacks).
- `BSL_AGENT_STATE_TTL_SECS` - TTL for persisted jobs (default: 7 days)

Optional HTTP UI (read-only, unified SPA):
- `BSL_AGENT_HTTP_ADDR=127.0.0.1:0` - enable UI and bind to loopback (use `:0` to auto-pick a port)
- `BSL_AGENT_HTTP_STATIC_DIR=/path/to/site` - optional override: serve SPA from a directory on disk (useful for development)

Notes:
- HTTP UI is localhost-only and rejects non-loopback bind addresses.
- UI is read-only: `bsl-agent` does not expose write HTTP endpoints under `/api/mcp/*`.
- By default, `bsl-agent` serves an embedded SPA baked into the binary at build time.
- UI parity endpoints for types:
  - `GET /api/mcp/types`, `GET /api/mcp/search`, `GET /api/mcp/metrics` (read-only).

## Building embedded UI assets

`bsl-agent` embeds the SPA output directory `target/site/` into the binary. Build the frontend first:

```bash
cd frontend
NO_COLOR=true trunk build --release
```

Then build `bsl-agent` as usual.

## Discovering the HTTP UI URL

When `BSL_AGENT_HTTP_ADDR` uses an auto-port (e.g. `127.0.0.1:0`), `bsl-agent` writes a runtime discovery record under:

`<cache_root>/bsl-agent-state/v1/runtime/http-ui/<instance_id>.json`

The `cache_root` is derived from `BSL_CACHE_DIR` (or XDG/HOME fallbacks).

List discovered instances (live-only by default):

```bash
BSL_CACHE_DIR=/tmp/bsl-cache bsl-agent ui list
```

Include stale records:

```bash
BSL_CACHE_DIR=/tmp/bsl-cache bsl-agent ui list --all
```

Get the UI URL (prints plain `http://localhost:<port>`):

```bash
BSL_CACHE_DIR=/tmp/bsl-cache bsl-agent ui url
```

If multiple instances are running in the same `BSL_CACHE_DIR`, select one explicitly:

```bash
BSL_CACHE_DIR=/tmp/bsl-cache bsl-agent ui url --roots /path/to/workspace/root
BSL_CACHE_DIR=/tmp/bsl-cache bsl-agent ui url --instance-id <instance_id>
BSL_CACHE_DIR=/tmp/bsl-cache bsl-agent ui url --pid <pid>
```

### Via MCP (read-only)

If your MCP client can call tools but cannot run shell commands, use the MCP tool:

- `ui_url` → `{ enabled: bool, ui_url: string | null }`

## Example Codex MCP config (stdio)

```toml
[mcp_servers.bsl_agent]
command = "/home/egor/code/bsl-gradual-types/target/release/bsl-agent"
cwd = "/home/egor/code/bsl-gradual-types"
env = {
  RUST_LOG = "bsl_agent=info",
  BSL_CACHE_DIR = "/tmp/bsl-cache",
  BSL_AGENT_HTTP_ADDR = "127.0.0.1:0",
}
```
