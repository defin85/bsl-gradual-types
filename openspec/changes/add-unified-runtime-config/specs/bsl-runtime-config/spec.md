## ADDED Requirements

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

