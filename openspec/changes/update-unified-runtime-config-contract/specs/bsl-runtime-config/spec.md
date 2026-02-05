## ADDED Requirements

### Requirement: Канонический JSON payload overrides (camelCase) един для LSP и bsl-agent
Система SHALL иметь единый канонический JSON payload для runtime overrides ключей `BSL_*`:
- `envOverrides` (stable),
- `devEnvOverrides` (dev-only),
- `allowDevOverrides` (gate dev-only).

`bsl-agent` MUST принимать этот payload в MCP tools, совместимых с LSP settings.

#### Scenario: Один payload применяется и в LSP, и в bsl-agent
- **GIVEN** клиент имеет JSON payload overrides
- **WHEN** клиент применяет payload через LSP settings и через MCP tool `workspace_update_settings`
- **THEN** effective runtime-config в обоих интерфейсах отражает одинаковые значения

### Requirement: bsl-agent поддерживает legacy snake_case как input-совместимость
Система SHALL обеспечивать обратную совместимость на входе `bsl-agent`:
- `env_overrides` является alias для `envOverrides`,
- `dev_env_overrides` является alias для `devEnvOverrides`,
- `allow_dev_overrides` является alias для `allowDevOverrides`.

#### Scenario: legacy snake_case payload принимается
- **GIVEN** клиент отправляет legacy payload (snake_case)
- **WHEN** `bsl-agent` применяет overrides
- **THEN** overrides применяются так же, как если бы payload был в camelCase

### Requirement: Registry описывает mutability ключей
Система SHALL описывать mutability каждого ключа реестра `BSL_*` как одно из:
- `runtime` — изменения MUST влиять без рестарта процесса/сессии,
- `startup_only` — изменения отражаются в effective snapshot немедленно, но для эффекта MAY требоваться рестарт coordinator/session.

#### Scenario: Snapshot содержит mutability карту
- **GIVEN** система запущена
- **WHEN** клиент запрашивает runtime-config snapshot
- **THEN** snapshot содержит machine-readable `mutability` для каждого ключа

### Requirement: ApplyOverridesReport сообщает ключи, требующие рестарта
Система SHALL возвращать в результате применения overrides список `requires_restart_keys`, включающий ключи, которые:
- изменили effective значение (override поменялся),
- но помечены `startup_only`.

#### Scenario: Override startup_only ключа попадает в requires_restart_keys
- **GIVEN** ключ `BSL_CACHE_DIR` помечен как `startup_only`
- **WHEN** клиент применяет override `BSL_CACHE_DIR`
- **THEN** report содержит `requires_restart_keys=["BSL_CACHE_DIR"]`

