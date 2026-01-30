# Design: границы крейтов и миграция зависимостей

## Контекст
В текущем workspace:
- `bsl-agent` (MCP stdio) импортирует `bsl-backend` как библиотеку (`bsl-agent/Cargo.toml`), хотя `bsl-backend` содержит application-слой (web/LSP) и системную инициализацию.
- `bsl-shared` содержит широкий набор разнотипных модулей (`shared/src/domain/**` и др.), что ухудшает cohesion и увеличивает область инвалидации при изменениях.

Цель изменения — сделать зависимости более "слоистыми" (layered) и уменьшить coupling между компонентами.

## Goals
- Убрать прямую зависимость `bsl-agent` → `bsl-backend`, сохранив parity поведения (MCP/HTTP/LSP) и текущие DTO/сервисы.
- Начать декомпозицию `bsl-shared` на меньшие крейты, начиная с самых базовых и широко используемых компонентов.
- Сделать миграцию поэтапной и проверяемой через тесты/quality gates.

## Non-goals
- Полная декомпозиция `bsl-shared` в рамках одного PR.
- Переименование `analysis-v2` → `bsl-analysis`.
- Редизайн алгоритмов анализа/инференса: только перенос кода и стабилизация границ.

## Предлагаемая структура зависимостей (целевое состояние)
Принцип: application crates зависят от library crates, но не наоборот.

- Library crates (ядро):
  - `bsl-api-dtos`: DTO для контрактов API (HTTP/MCP/LSP/CLI), не зависящие от application wiring.
  - `bsl-types`: базовые доменные типы типовой системы.
  - `bsl-repository`: интерфейсы/реализации репозитория типов и индексов (в объеме, не зависящем от application).
  - `bsl-analysis-v2`: движок анализа и deps snapshot.
  - `bsl-runtime`: общий runtime для startup/deps/cache wiring, который нужен и backend, и agent.
- Application/adapter crates:
  - `bsl-backend`: web/LSP адаптеры (axum/tower-lsp), CLI/HTTP wiring.
  - `bsl-agent`: MCP stdio адаптер, jobs/session wiring.

## Миграционная стратегия
### 1) Выделение `bsl-runtime`
1. Найти точку импорта `bsl-backend` из `bsl-agent` и минимизировать требуемую поверхность (тип(ы) результата startup, доступ к deps snapshot и т.п.).
2. Вынести эту поверхность в `bsl-runtime`:
   - не тянуть web-слой (`axum` handlers и т.п.) в `bsl-runtime`,
   - не тянуть MCP-specific слой в `bsl-runtime`.
3. Переключить `bsl-agent` на `bsl-runtime` и убрать `bsl-backend` из зависимостей.
4. Переключить `bsl-backend` на использование `bsl-runtime` (через публичный API), чтобы избежать дублирования.

#### Инвентаризация: что `bsl-agent` реально брал из `bsl-backend` (до миграции)
Минимальная поверхность, которая понадобилась агенту, сводилась к runtime-слою:
- `system`: `SystemCoordinator`, `StartupInputs`, `startup_v2`, `StartupResultV2`, `fs_utils`.
- `data::loaders`: `ConfigurationDiscovery`, `progress::ProgressUpdate`.
- `application::type_system`: `web_api_service` и часть сервисов (completion / goto definition / types search).

### 2) Декомпозиция `bsl-shared` (этап 1)
1. Выделить `bsl-types` и перенести туда ключевые структуры (TypeResolution/ConcreteType/TypeId и т.п.).
2. Выделить `bsl-repository` для репозитория/индекса сигнатур, если он используется многими крейтами и не должен зависеть от application.
3. Выделить `bsl-api-dtos` и перенести туда DTO, которые сейчас живут в `bsl_shared::api::dtos` и используются в HTTP/MCP parity.
4. Обновлять импорты по одному слою за раз: сначала в `analysis-v2`/`backend`, затем в `bsl-agent`, затем во frontend/прочие.

## Риски
- Большой дифф и риск регрессий из-за перемещений модулей/путей импорта.
- Непредвиденные циклические зависимости при выносе `bsl-runtime`.
- Утечка application-зависимостей в library crates (например, `axum`, `tower-lsp`) — это нужно отсекать на уровне `Cargo.toml` и код-ревью.

## Наблюдаемая база (для верификации)
- Зависимость `bsl-agent` от `bsl-backend`: `bsl-agent/Cargo.toml`.
- `bsl-shared` содержит крупные доменные модули: `shared/src/domain/repository.rs`, `shared/src/domain/facet_utils.rs`.
- `backend/src/domain/flow_analyzer.rs` существует, но модуль `flow_analyzer` в `backend/src/domain/mod.rs` закомментирован; это не является активной частью runtime и не является целью этой миграции.
