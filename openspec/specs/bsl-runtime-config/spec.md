# bsl-runtime-config Specification

## Purpose
TBD - created by archiving change add-unified-runtime-config. Update Purpose after archive.
## Requirements
### Requirement: Единый реестр runtime `BSL_*` ключей с tiering
Система SHALL иметь единый реестр всех runtime `BSL_*` переменных окружения, которые читаются в рантайме (через `std::env::var`/эквивалент), включающий:
- тип значения,
- значение по умолчанию,
- описание/назначение,
- tier: `stable` или `dev-only`.

#### Scenario: Реестр содержит все runtime `BSL_*` из кода
- **GIVEN** исходники проекта
- **WHEN** выполняется проверка списка ключей (test/validation)
- **THEN** каждый runtime `std::env::var("BSL_*")` имеет запись в реестре

### Requirement: Runtime overrides применяются без рестарта
Система SHALL поддерживать runtime overrides значений ключей реестра и SHALL применять их без перезапуска процесса/сессии.

#### Scenario: Изменение порога производительности влияет без рестарта
- **GIVEN** система запущена и использует runtime-config store
- **WHEN** клиент обновляет значение ключа через runtime overrides
- **THEN** новое значение используется в последующих операциях без рестарта

### Requirement: Раздельные stable и dev-only overrides
Система SHALL принимать overrides в двух каналах:
- `envOverrides` (stable),
- `devEnvOverrides` (dev-only).

Dev-only overrides SHALL быть логически изолированы так, чтобы поддержку dev-only можно было удалить без изменения stable-контракта.

#### Scenario: Отключение dev-only не ломает stable overrides
- **GIVEN** система поддерживает stable и dev-only overrides
- **WHEN** поддержка dev-only overrides отключается/удаляется
- **THEN** stable overrides продолжают работать без изменений

### Requirement: Bootstrap-совместимость с env
Система MUST сохранять совместимость: если `BSL_*` env заданы при старте процесса, они используются как bootstrap значения, но могут быть переопределены runtime overrides.

#### Scenario: Overrides переопределяют env
- **GIVEN** процесс стартовал с `BSL_CACHE_DISABLE=1`
- **WHEN** runtime overrides устанавливают `BSL_CACHE_DISABLE=false`
- **THEN** кэш включается без рестарта

### Requirement: Валидация unknown keys в overrides
Система SHALL валидировать ключи overrides по реестру.
Unknown ключи SHALL не приводить к падению процесса и SHALL быть отражены как diagnostic (warning) с игнорированием значения.

#### Scenario: Unknown key игнорируется с warning
- **GIVEN** клиент отправляет override `BSL_NOT_A_REAL_KEY=1`
- **WHEN** система применяет overrides
- **THEN** значение игнорируется и возвращается/логируется warning о неизвестном ключе

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

