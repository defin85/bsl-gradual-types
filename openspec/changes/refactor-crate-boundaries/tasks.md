# Tasks: refactor-crate-boundaries

## 1. Спецификация и критерии приемки
- [x] Добавить delta к `openspec/changes/refactor-crate-boundaries/specs/dev-workflow/spec.md` (границы зависимостей и этапы миграции).
- [x] `openspec validate refactor-crate-boundaries --strict --no-interactive`

## 2. Развязка `bsl-agent` ↔ `bsl-backend`
- [x] Инвентаризировать фактическое использование `bsl-backend` внутри `bsl-agent` (модули/типы/функции) и зафиксировать минимальную поверхность API для вынесения.
- [x] Выделить библиотечный крейт `bsl-runtime` (или согласованное имя) и перенести в него:
  - [x] общие структуры startup результата (например, `StartupResultV2`) или эквивалент,
  - [x] сборку deps snapshot (`DepsBundleV2` / `SemanticDeps` wiring) и общие настройки кэша,
  - [x] системную координацию, которую сейчас импортирует агент (если применимо).
- [x] Обновить зависимости:
  - [x] `bsl-agent` зависит от `bsl-runtime` (и других lib crates), но не от `bsl-backend`,
  - [x] `bsl-backend` использует `bsl-runtime` как библиотеку.
- [x] Обновить места вызова в `bsl-agent` (session/startup) и в `bsl-backend` (web/LSP адаптеры) после переноса.
- [x] Обновить документацию (минимум: `bsl-agent/README.md` и/или `backend/README.md`) о новой границе зависимостей.

## 3. Декомпозиция `bsl-shared` (этап 1: базовые типы и репозиторий)
- [x] Создать крейт `bsl-types` и перенести в него базовые доменные типы типовой системы:
  - [x] `TypeId`/идентификаторы,
  - [x] `TypeResolution`/`ConcreteType` и близкие структуры, которые широко используются.
- [x] Создать крейт `bsl-repository` и перенести в него API репозитория типов/индексов:
  - [x] trait-ы репозитория и ключевые структуры, которые не должны жить в `bsl-shared`.
- [x] Создать крейт `bsl-api-dtos` и перенести в него API DTO (используемые в HTTP/MCP/LSP/CLI):
  - [x] `bsl_shared::api::dtos::*` (например, `AnalysisResultDto`, `TypeDto`, `PaginationDto` и др.).
- [x] Обновить зависимости крейтов workspace на новые крейты, минимизируя churn (поэтапно).
- [x] Проверить WASM-совместимость (если какие-то типы используются во frontend/WASM контексте).

## 4. Регрессии и quality gates
- [x] Добавить/обновить тест(ы), проверяющий(е) отсутствие зависимости `bsl-agent` → `bsl-backend` (например, через `cargo tree` в CI или unit/integration check).
- [x] `cargo fmt`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --workspace`
