# bsl-agent (MCP)

Local MCP server (stdio) that provides semantic context for BSL projects.

## Crate boundaries

- `bsl-agent` is an adapter (MCP stdio + optional read-only HTTP UI).
- Core startup/deps/cache wiring and analysis helpers live in `bsl-runtime`.
- `bsl-agent` MUST NOT depend on `bsl-backend` (HTTP/LSP adapter crate).

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
- `BSL_AGENT_LOG_FILE` - explicit file path for the MCP stdio log (highest priority)
- `BSL_AGENT_LOG_DIR` - directory override; `bsl-agent` writes `<dir>/mcp.log` when `BSL_AGENT_LOG_FILE` is not set

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

## MCP stdio file log

For MCP stdio mode, `bsl-agent` always creates a persistent operator-visible log file before the normal MCP lifecycle starts. This log is independent from `workspace_open`, so it is available even if transport or process startup fails first.

Path precedence:
- `BSL_AGENT_LOG_FILE`
- `BSL_AGENT_LOG_DIR` + `/mcp.log`
- `<cwd>/.bsl-agent/mcp.log`

Default example:

```text
/home/egor/code/DO_Rolf_PT/.bsl-agent/mcp.log
```

Notes:
- `stdout` stays reserved for MCP transport frames.
- `stderr` may still contain the same diagnostics, but the file log is the primary path to inspect after a crash or `Transport closed`.
- If file log bootstrap fails, `bsl-agent` prints the attempted path and the OS error to `stderr` and exits without starting the stdio MCP server.
- Startup records include build/version info, `pid`, `cwd`, effective log path, `BSL_CACHE_DIR`, and `BSL_AGENT_HTTP_ADDR`.

## Building embedded UI assets

`bsl-agent` embeds the SPA output directory `target/site/` into the binary. Build the frontend first:

```bash
cd frontend
NO_COLOR=true trunk build --release
```

Then build `bsl-agent` as usual.

The canonical repository smoke path `./scripts/run-intellisense-tests.sh smoke`
rebuilds `target/site/` automatically when the embedded UI assets are missing,
as long as `trunk` and the `wasm32-unknown-unknown` target are available.

## Repo policy check

CI enforces crate boundaries (no `bsl-agent` -> `bsl-backend` dependency) via `scripts/check-bsl-agent-backend-dep.sh`.

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

The checked-in repository [`.mcp.json`](../.mcp.json) is a portable example-only config. The canonical onboarding path for Codex lives in [`docs/agent/codex-setup.md`](../docs/agent/codex-setup.md).

```toml
[mcp_servers.bsl_agent]
command = "cargo"
args = ["run", "-p", "bsl-agent", "--"]
env = {
  RUST_LOG = "bsl_agent=info",
  BSL_AGENT_HTTP_ADDR = "127.0.0.1:0",
}
```

Keep machine-specific absolute paths, extra servers and secrets in personal local config, not in tracked repo files.

If your MCP client supports `cwd`, point it at the target project root to keep the default log file under `<project>/.bsl-agent/mcp.log`; otherwise use `BSL_AGENT_LOG_DIR`.

With this config the default log path is:

```text
<project>/.bsl-agent/mcp.log
```

## Smoke-check for stdio logging

Create the log file without `workspace_open`:

```bash
repo_root="$(pwd)" && tmpdir="$(mktemp -d)" && cd "$tmpdir" && RUST_LOG=bsl_agent=info "$repo_root/target/debug/bsl-agent" </dev/null >/tmp/bsl-agent-stdout.txt 2>/tmp/bsl-agent-stderr.txt; test -f "$tmpdir/.bsl-agent/mcp.log" && tail -n 20 "$tmpdir/.bsl-agent/mcp.log"
```

Check the explicit file override:

```bash
repo_root="$(pwd)" && tmpdir="$(mktemp -d)" && logfile="$tmpdir/custom-mcp.log" && cd "$tmpdir" && RUST_LOG=bsl_agent=info BSL_AGENT_LOG_FILE="$logfile" "$repo_root/target/debug/bsl-agent" </dev/null >/tmp/bsl-agent-stdout.txt 2>/tmp/bsl-agent-stderr.txt; test -f "$logfile" && tail -n 20 "$logfile"
```

Reproduce the fail-fast path:

```bash
repo_root="$(pwd)" && tmpdir="$(mktemp -d)" && blocker="$tmpdir/not-a-dir" && printf blocker >"$blocker" && cd "$tmpdir" && BSL_AGENT_LOG_DIR="$blocker" "$repo_root/target/debug/bsl-agent" </dev/null >/tmp/bsl-agent-stdout.txt 2>/tmp/bsl-agent-stderr.txt; cat /tmp/bsl-agent-stderr.txt
```

## LLM usage notes

- `workspace_open.mode="default"` is accepted as default mode (no warning).
- If `configuration_path` is set and `platform_version` is omitted, `bsl-agent` tries to infer it from the config dump (CompatibilityMode). If it cannot infer, `workspace_open` fails with `INVALID_PARAMS`.
- Multi-root file references: `DocumentRef` / `FileRef.doc` accept absolute paths (string or `{ "path": "/abs/..." }`). The server resolves them via deterministic longest-prefix match against `roots[]`.
- `workspace_documents_set.files[]` also accepts plain absolute paths (strings) to mark documents as hot (no overlay).
- `bsl_diagnostics_start.scope` accepts `"project"` / `"hot"` as strings; for a single file use `{ "kind": "file", "document": <DocumentRef> }` (string `"file"` is invalid).
- `job_status.progress.percent=100` is reserved for terminal states; running jobs report `0..99`.

## Runtime settings and observability tools

- `workspace_update_settings` canonical payload uses camelCase:
  - `envOverrides` (stable overrides),
  - `devEnvOverrides` (dev-only overrides),
  - `allowDevOverrides` (gate for dev-only layer).
- Legacy snake_case payload is still accepted as input aliases:
  - `env_overrides`, `dev_env_overrides`, `allow_dev_overrides`.
- `workspace_get_settings` and `workspace_update_settings` responses return canonical camelCase fields.
- Startup-only semantics:
  - `BSL_CACHE_DIR` is `startup_only`: override is reflected in runtime snapshot immediately, but requires session/coordinator restart to take effect.
  - `report.requiresRestartKeys[]` lists startup-only keys whose effective values changed.
- `workspace_get_observability_metrics(session_id)` is read-only and requires `ready=true`; non-ready sessions are rejected with `INVALID_PARAMS`.

### Example MCP payloads

Canonical payload:

```json
{
  "session_id": "<session_id>",
  "envOverrides": { "BSL_CACHE_DISABLE": true },
  "allowDevOverrides": true,
  "devEnvOverrides": { "BSL_COMPLETION_TRACE": true }
}
```

Legacy-compatible payload (accepted on input):

```json
{
  "session_id": "<session_id>",
  "env_overrides": { "BSL_CACHE_DISABLE": true },
  "allow_dev_overrides": true,
  "dev_env_overrides": { "BSL_COMPLETION_TRACE": true }
}
```

### Example VS Code settings payload (`workspace/didChangeConfiguration`, section `bsl`)

Canonical form:

```json
{
  "bsl": {
    "envOverrides": { "BSL_CACHE_DISABLE": true },
    "allowDevOverrides": true,
    "devEnvOverrides": { "BSL_COMPLETION_TRACE": true }
  }
}
```

Legacy compatibility (still supported by LSP while migrating clients):

```json
{
  "bsl": {
    "envOverrides": { "BSL_CACHE_DISABLE": true },
    "devEnvOverrides": { "BSL_COMPLETION_TRACE": true },
    "dev": {
      "enableDevEnvOverrides": true
    }
  }
}
```

## MCP type discovery tools (stdio)

Read-only, async tools for navigating platform/configuration types. All follow the `*_start` pattern and return a `job_id`.

### List types

```json
{ "name": "bsl_types_list_start", "arguments": { "session_id": "<session_id>", "page": 1, "limit": 50, "source": "configuration", "view": "names_only" } }
```

### Search types

```json
{ "name": "bsl_types_search_start", "arguments": { "session_id": "<session_id>", "query": "Document", "limit": 200, "view": "summary" } }
```

### Get type details (properties + tabular sections)

```json
{ "name": "bsl_type_get_start", "arguments": { "session_id": "<session_id>", "type_name": "DocumentObject.<DocName>", "source": "configuration", "include_methods": false } }
```

Then use:
- `job_wait(job_id, timeout_ms)` until `state="succeeded"`
- `job_result(job_id)` to fetch the JSON payload
