# Change: Unify runtime-config contract (payload + mutability + observability tools)

## Why
- После `add-unified-runtime-config` unified runtime-config появился, но контракт между LSP и `bsl-agent` не унифицирован:
  - LSP принимает camelCase (`envOverrides`, `devEnvOverrides`, `enableDevEnvOverrides`),
  - `bsl-agent` принимает snake_case (`env_overrides`, `dev_env_overrides`, `allow_dev_overrides`).
- “Runtime update без рестарта” на практике неоднороден: часть ключей влияет сразу, часть — только при startup (например, корень дискового кэша).
- Нужен единый способ получать observability-метрики из `bsl-agent`, аналогичный LSP `bsl.getObservabilityMetrics`.

## What Changes
- Утвердить **канонический JSON payload** для runtime overrides (camelCase), единый для:
  - LSP settings (`workspace/didChangeConfiguration`),
  - MCP tools `bsl-agent` (`workspace_update_settings`, `workspace_get_settings`).
- Сохранить обратную совместимость в `bsl-agent`:
  - вход: принимать и camelCase, и legacy snake_case (как alias),
  - выход: гарантировать camelCase поля в ответах (legacy snake_case допускается только как временная совместимость, отдельным пунктом в спеках).
- Ввести в runtime-registry понятие **mutability**: `runtime` vs `startup_only`, и возвращать это в snapshot, плюс репортить ключи, требующие рестарта для эффекта.
- Добавить MCP tool `workspace_get_observability_metrics(session_id)` в `bsl-agent`, возвращающий observability snapshot (как в LSP).

## Impact
- Affected specs:
  - `bsl-runtime-config` (schema: payload + mutability),
  - `mcp-bsl-agent` (tools contract + observability tool).
- Affected code:
  - `bsl-agent` (serde contracts, response DTO, tests),
  - `bsl-runtime` (registry metadata: mutability + snapshot),
  - (в меньшей степени) `backend` (только если потребуется расширить `bsl.getRuntimeConfig` форматом mutability).

