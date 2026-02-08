## ADDED Requirements

### Requirement: Tag-driven release pipeline публикует integrity артефакты extension (MUST)
Система MUST иметь release workflow, который запускается по релизному тегу и собирает VS Code extension артефакт (`.vsix`) вместе с integrity артефактами:
- checksum файл (минимум SHA-256),
- SBOM для релизного пакета (формат machine-readable).

Артефакты MUST публиковаться как часть релиза и быть доступны для проверки downstream пользователями.

#### Scenario: Релизный тег производит проверяемый пакет поставки
- **GIVEN** создан релизный тег
- **WHEN** выполняется release workflow
- **THEN** в релизе присутствуют `.vsix`, checksum и SBOM, пригодные для независимой верификации

### Requirement: Release checks включают валидацию docs/settings консистентности (MUST)
CI MUST включать автоматическую проверку консистентности между:
- runtime settings schema (`contributes.configuration`),
- пользовательской документацией extension.

При рассинхронизации ключей/описаний проверка MUST падать до публикации релиза.

#### Scenario: Drift настроек ловится до публикации
- **GIVEN** изменены ключи runtime settings в extension schema
- **WHEN** CI запускает release checks
- **THEN** релиз блокируется, если документация не обновлена и не соответствует новой схеме
