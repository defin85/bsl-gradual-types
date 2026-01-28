## ADDED Requirements

### Requirement: Генерированные артефакты VS Code extension не версионируются
Репозиторий MUST не хранить в git генерированные артефакты сборки VS Code extension (например, `vscode-extension/out/**`, `vscode-extension/*.vsix`).

#### Scenario: Чистый diff для исходников
- **GIVEN** разработчик меняет исходники расширения
- **WHEN** он делает commit
- **THEN** в diff не должны попадать генерированные файлы сборки, если они не являются исходниками

