# Runtime BSL_* inventory (draft)

This file documents the current `BSL_*` keys discovered in the repo.

Notes:
- Scope: keys referenced by Rust code as string literals (runtime reads via `std::env::var(...)` and helper wrappers).
- `dev-only` keys are intended to be isolated behind `devEnvOverrides` and a single opt-in toggle, so they can be removed later with minimal blast radius.
- `build-only` keys come from `option_env!` / build.rs and are not runtime-mutable.

## Runtime keys

| Key | Type | Default (effective) | Tier | Primary usage |
|---|---:|---|---|---|
| `BSL_CACHE_DIR` | path | XDG/HOME fallback | stable | `bsl-runtime/src/system/disk_cache.rs`, `bsl-agent/src/state.rs` |
| `BSL_CACHE_DISABLE` | bool | false | stable | `bsl-runtime/src/system/disk_cache.rs`, `bsl-runtime/src/system/parser_coordinator.rs` |
| `BSL_CACHE_TTL_SECS` | u64? | none | stable | `bsl-runtime/src/system/disk_cache.rs` |
| `BSL_CACHE_TTL_MODE` | string | `created` | stable | `bsl-runtime/src/system/disk_cache.rs` |
| `BSL_CACHE_MAX_BYTES` | u64? | none | stable | `bsl-runtime/src/system/disk_cache.rs` |
| `BSL_CACHE_CLEANUP_INTERVAL_SECS` | u64 | 300 | stable | `bsl-runtime/src/system/disk_cache.rs` |
| `BSL_CACHE_TOUCH_INTERVAL_SECS` | u64 | 60 | stable | `bsl-runtime/src/system/disk_cache.rs` |
| `BSL_CACHE_SWR` | bool | true | stable | `bsl-runtime/src/system/disk_cache.rs` |
| `BSL_CACHE_STRICT_FINGERPRINT` | bool | false | stable | `bsl-runtime/src/system/system_coordinator/coordinator.rs` |
| `BSL_AST_CACHE_CAPACITY` | usize | 64 | stable | `bsl-runtime/src/system/ast_cache.rs` |
| `BSL_INDEX_WARMUP` | bool | true | stable | `bsl-runtime/src/system/system_coordinator/config_loader.rs` |
| `BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS` | u64(ms) | 250 | stable | `backend/src/bin/lsp_server/server/core.rs` |
| `BSL_INTELLISENSE_V2_SLOW_CLIENT_LOG_MS` | u64(ms) | 2000 | stable | `backend/src/bin/lsp_server/server/mod.rs`, `vscode-extension/src/lsp/client/server-options.ts` |
| `BSL_INTELLISENSE_V2_SLOW_WAIT_WARN_MS` | u64(ms)? | none | stable | `backend/src/bin/lsp_server/server/mod.rs` |
| `BSL_INTELLISENSE_V2_SLOW_SNAPSHOT_WARN_MS` | u64(ms)? | none | stable | `backend/src/bin/lsp_server/server/mod.rs` |
| `BSL_INTELLISENSE_V2_SLOW_QUERY_WARN_MS` | u64(ms)? | none | stable | `backend/src/bin/lsp_server/server/mod.rs` |
| `BSL_AGENT_HTTP_ADDR` | string | unset (disabled) | stable | `bsl-agent/src/main.rs` |
| `BSL_AGENT_HTTP_STATIC_DIR` | path? | none | stable | `bsl-agent/src/main.rs` |
| `BSL_AGENT_STATE_TTL_SECS` | u64 | 604800 (7d) | stable | `bsl-agent/src/jobs/mod.rs` |
| `BSL_WEB_HOST` | string | 127.0.0.1 | stable | `backend/src/config/mod.rs` |
| `BSL_WEB_PORT` | u16 | 8080 | stable | `backend/src/config/mod.rs` |
| `BSL_STATIC_PATH` | path? | none | stable | `backend/src/config/mod.rs` |
| `BSL_PROJECT_PATH` | path? | none | stable | `backend/src/config/mod.rs` |
| `BSL_PLATFORM_VERSION` | string? | none | stable | `backend/src/config/mod.rs` |
| `BSL_ENABLE_CORS` | bool | true | stable | `backend/src/config/mod.rs` |
| `BSL_LOG_LEVEL` | string | info | stable | `backend/src/config/mod.rs` |
| `BSL_SYNTAX_HELPER_PATH` | path? | none | dev-only | `backend/src/bin/lsp_server/handlers/hover.rs` |

## Dev-only + test-only keys

| Key | Type | Default (effective) | Tier | Primary usage |
|---|---:|---|---|---|
| `BSL_COMPLETION_TRACE` | bool | false | dev-only | `bsl-runtime/src/application/type_system/services/completion_service.rs` |
| `BSL_COMPLETION_QUALITY` | bool | false | dev-only | `backend/src/bin/lsp_server/server/language_server.rs` |
| `BSL_INTELLISENSE_V2_P3_SMOKE` | bool | false | dev-only | `backend/src/bin/lsp_server/server/language_server.rs` |
| `BSL_INTELLISENSE_V2_P4_SMOKE` | bool | false | dev-only | `backend/src/bin/lsp_server/server/language_server.rs` |
| `BSL_SLOW_MODULE_THRESHOLD_MS` | u64(ms) | 3000 | dev-only | `bsl-runtime/src/data/loaders/config_bsl_modules/metrics.rs` |
| `BSL_SLOW_MODULE_TOP_N` | usize | 5 | dev-only | `bsl-runtime/src/data/loaders/config_bsl_modules/metrics.rs` |
| `BSL_MODULE_PARSE_LOG_EACH` | bool | false | dev-only | `bsl-runtime/src/data/loaders/config_bsl_modules/metrics.rs` |
| `BSL_RUN_WEB_API_TESTS` | bool | false | dev-only | `backend/tests/pagination_integration_test.rs` |
| `BSL_WEB_API_BASE_URL` | string? | none | dev-only | `backend/tests/pagination_integration_test.rs` |

## Build-only keys (not runtime-mutable)

| Key | Source | Notes |
|---|---|---|
| `BSL_AGENT_PROFILE` | build.rs / `option_env!` | compile-time info |
| `BSL_AGENT_TARGET` | build.rs / `option_env!` | compile-time info |
| `BSL_AGENT_GIT_SHA` | build.rs / `option_env!` | compile-time info |
| `BSL_AGENT_GIT_DESCRIBE` | build.rs / `option_env!` | compile-time info |
| `BSL_AGENT_BUILD_UNIX_SECS` | build.rs / `option_env!` | compile-time info |

