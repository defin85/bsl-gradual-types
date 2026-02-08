## ADDED Requirements

### Requirement: Runtime Overrides должны быть явно доступны в Settings UI extension (MUST)
VS Code extension MUST публиковать canonical runtime override settings в `contributes.configuration` так, чтобы пользователь мог найти их через стандартный поиск настроек IDE.

Минимальный набор MUST включать:
- stable overrides (`envOverrides`),
- dev-only overrides (`devEnvOverrides`),
- gate для dev-only (`allowDevOverrides`).

Каждый ключ MUST иметь понятные title/description и пример формата значения.

#### Scenario: Пользователь находит Runtime Overrides в Settings без ручного JSON editing
- **GIVEN** extension установлен в VS Code
- **WHEN** пользователь ищет настройки по `runtime overrides` или `BSL`
- **THEN** IDE отображает canonical ключи overrides с описаниями и ожидаемым форматом

### Requirement: Документация по runtime settings синхронизирована с фактической схемой (MUST)
Документация extension MUST отражать фактический список runtime settings из `contributes.configuration` без устаревших или пропущенных ключей.

Если поддерживаются legacy aliases, документация MUST явно маркировать их как compatibility-only и MUST указывать canonical ключ.

#### Scenario: README и schema настроек не расходятся
- **GIVEN** релизный кандидат extension
- **WHEN** reviewer сравнивает runtime settings в README и в `package.json`
- **THEN** список ключей и их назначение совпадают, а legacy aliases помечены как compatibility-only
