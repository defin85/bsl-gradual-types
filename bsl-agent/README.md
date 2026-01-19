# bsl-agent (MCP)

Local MCP server (stdio) that provides semantic context for BSL projects.

## Environment variables

Disk cache:
- `BSL_CACHE_DIR` - cache root directory (overrides XDG/HOME defaults)
- `BSL_CACHE_DISABLE=1|true|yes` - disable disk cache reads/writes
- `BSL_CACHE_STRICT_FINGERPRINT=1` - stricter fingerprinting for cache keys

In-memory AST cache:
- `BSL_AST_CACHE_CAPACITY` - LRU capacity (default: 64)

Logging:
- `RUST_LOG` - e.g. `bsl_agent=debug,info`

