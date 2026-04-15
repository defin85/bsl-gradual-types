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

### Requirement: didSave heavy follow-up isolation exposes runtime-configurable permit quota with explicit zero semantics (MUST)
Система SHALL описывать в runtime-config registry stable key для quota/permits dedicated non-interactive lane, который обслуживает post-fastlane `didSave + idle_heavy` follow-up.

Этот key MUST:

- иметь machine-readable metadata в registry snapshot;
- быть runtime-mutable без рестарта процесса;
- влиять на последующие admission decisions follow-up lane;
- при отсутствии override иметь default effective value `1`;
- управлять dedicated admission lane, отдельной от бинарной taxonomy `CpuWorkClass`, а не переопределять `Interactive` / `Background` в третий work class;
- обозначать global process-wide count end-to-end didSave-follow-up slots, охватывающих outer admission boundary, writer/runtime preparation, blocking CPU execution и final pre-publish supersession/quota/disposition decision одного heavy follow-up, но MUST NOT включающих outbound publish/output wait, а не набор independently configurable writer-vs-CPU quotas или per-file multiplicative capacity;
- регулировать долю dedicated didSave-follow-up lane внутри существующего bounded runtime/CPU budget и MUST NOT создавать net-new total process-wide parallelism;
- трактовать effective value `0` как explicit disable новых `didSave + idle_heavy` admissions;
- не clamp-ить `0` к `1` и не возвращать didSave heavy follow-up в generic background lane молча;
- применяться на outer admission boundary для future admissions; already admitted work MAY finish under already acquired slot и MUST NOT подвергаться retroactive revocation/reclassification mid-flight;
- не менять contract `save_fastlane`;
- не менять contract interactive lane.

#### Scenario: Operator changes follow-up permit quota without restart
- **GIVEN** сервер уже работает и didSave follow-up isolation lane включён
- **WHEN** runtime override меняет permit quota этого lane
- **THEN** новое effective значение видно в runtime-config snapshot
- **AND** последующие didSave follow-up admissions используют новую quota без рестарта

#### Scenario: Default follow-up permit quota is one bounded slot
- **GIVEN** сервер запущен без operator override для dedicated didSave follow-up lane
- **WHEN** runtime-config snapshot строится из default registry values
- **THEN** effective permit quota этого lane equals `1`
- **AND** default behavior remains bounded without introducing net-new save-storm parallelism

#### Scenario: Positive quota change affects subsequent admissions only
- **GIVEN** один heavy follow-up уже прошёл outer admission boundary dedicated lane
- **AND** оператор runtime override меняет positive permit quota этого lane во время выполнения уже admitted work
- **WHEN** сервер принимает последующие didSave heavy follow-up admissions
- **THEN** новое effective значение governs only those subsequent outer-admission decisions
- **AND** already admitted work does not require retroactive revocation or reclassification

#### Scenario: Admitted slot lifetime ends before outbound publish wait
- **GIVEN** didSave heavy follow-up уже владеет одним admitted slot dedicated lane
- **AND** heavy branch дошёл до final pre-publish supersession/quota/disposition decision
- **WHEN** дальнейший progress упирается только в outbound publish/output wait
- **THEN** quota contract больше не считает этот follow-up владельцем scarce slot
- **AND** slot lifetime не продолжается через publish/output wait

#### Scenario: Operator sets follow-up permit quota to zero
- **GIVEN** сервер уже работает и `save_fastlane` semantics остаются доступными
- **WHEN** runtime override устанавливает permit quota didSave follow-up lane в `0`
- **THEN** runtime-config snapshot явно показывает effective value `0`
- **AND** новые `didSave + idle_heavy` follow-up admissions отключаются без silent fallback в generic background lane

#### Scenario: Zero quota also disables queued-but-not-started follow-up at admission time
- **GIVEN** didSave heavy follow-up was queued before the operator changed the dedicated lane quota
- **AND** effective permit quota becomes `0` before that queued work acquires scarce lane capacity
- **WHEN** the admission boundary is reached
- **THEN** the queued work re-checks the effective runtime-config value instead of relying on stale pre-disable assumptions
- **AND** the server does not silently let that work enter the lane

