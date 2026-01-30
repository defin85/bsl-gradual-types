## ADDED Requirements

### Requirement: Границы зависимостей workspace (library vs application)
Репозиторий MUST поддерживать слоистую архитектуру зависимостей (layered dependencies), где application/adapter крейты (например, web/LSP/MCP) зависят от библиотечных крейтов (domain/runtime), но не наоборот.

В частности, `bsl-agent` MUST NOT зависеть от `bsl-backend`. Общая логика startup/deps snapshot/кэша, необходимая и backend, и agent, MUST жить в отдельном библиотечном крейте (например, `bsl-runtime`).

#### Scenario: `bsl-agent` не тянет `bsl-backend` как зависимость
- **GIVEN** разработчик собирает workspace
- **WHEN** он проверяет дерево зависимостей для `bsl-agent`
- **THEN** `bsl-backend` отсутствует в зависимостях `bsl-agent` (прямых и транзитивных)

### Requirement: Декомпозиция `bsl-shared` (этап 1)
Система SHALL постепенно уменьшать связанность `bsl-shared`, выделяя базовые компоненты в отдельные library crates. На первом этапе:
- базовые доменные типы типовой системы SHOULD быть вынесены в `bsl-types`,
- интерфейсы/структуры репозитория типов и индексов SHOULD быть вынесены в `bsl-repository`.
- DTO для публичных контрактов (например, для HTTP/MCP parity) SHOULD быть вынесены в отдельный library crate (например, `bsl-api-dtos`), чтобы не смешивать контракты и доменную/инфраструктурную логику.

Миграция MUST быть поэтапной и сопровождаться тестами/quality gates, чтобы не ломать поведение анализа и внешние адаптеры.

#### Scenario: Workspace собирается и тесты проходят после выделения новых крейтов
- **GIVEN** выделены новые library crates и обновлены зависимости
- **WHEN** разработчик запускает `cargo test --workspace`
- **THEN** сборка проходит, а поведение (вывод и контракты) остаётся совместимым с текущими спецификациями
