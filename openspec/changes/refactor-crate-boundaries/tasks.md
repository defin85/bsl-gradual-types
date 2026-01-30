# Tasks: refactor-crate-boundaries

## 1. Спецификация и критерии приемки
- [ ] Добавить delta к `openspec/changes/refactor-crate-boundaries/specs/dev-workflow/spec.md` (границы зависимостей и этапы миграции).
- [ ] `openspec validate refactor-crate-boundaries --strict --no-interactive`

## 2. Развязка `bsl-agent` ↔ `bsl-backend`
- [ ] Инвентаризировать фактическое использование `bsl-backend` внутри `bsl-agent` (модули/типы/функции) и зафиксировать минимальную поверхность API для вынесения.
- [ ] Выделить библиотечный крейт `bsl-runtime` (или согласованное имя) и перенести в него:
  - [ ] общие структуры startup результата (например, `StartupResultV2`) или эквивалент,
  - [ ] сборку deps snapshot (`DepsBundleV2` / `SemanticDeps` wiring) и общие настройки кэша,
  - [ ] системную координацию, которую сейчас импортирует агент (если применимо).
- [ ] Обновить зависимости:
  - [ ] `bsl-agent` зависит от `bsl-runtime` (и других lib crates), но не от `bsl-backend`,
  - [ ] `bsl-backend` использует `bsl-runtime` как библиотеку.
- [ ] Обновить места вызова в `bsl-agent` (session/startup) и в `bsl-backend` (web/LSP адаптеры) после переноса.
- [ ] Обновить документацию (минимум: `bsl-agent/README.md` и/или `backend/README.md`) о новой границе зависимостей.

## 3. Декомпозиция `bsl-shared` (этап 1: базовые типы и репозиторий)
- [ ] Создать крейт `bsl-types` и перенести в него базовые доменные типы типовой системы:
  - [ ] `TypeId`/идентификаторы,
  - [ ] `TypeResolution`/`ConcreteType` и близкие структуры, которые широко используются.
- [ ] Создать крейт `bsl-repository` и перенести в него API репозитория типов/индексов:
  - [ ] trait-ы репозитория и ключевые структуры, которые не должны жить в `bsl-shared`.
- [ ] Создать крейт `bsl-api-dtos` и перенести в него API DTO (используемые в HTTP/MCP/LSP/CLI):
  - [ ] `bsl_shared::api::dtos::*` (например, `AnalysisResultDto`, `TypeDto`, `PaginationDto` и др.).
- [ ] Обновить зависимости крейтов workspace на новые крейты, минимизируя churn (поэтапно).
- [ ] Проверить WASM-совместимость (если какие-то типы используются во frontend/WASM контексте).

## 4. Регрессии и quality gates
- [ ] Добавить/обновить тест(ы), проверяющий(е) отсутствие зависимости `bsl-agent` → `bsl-backend` (например, через `cargo tree` в CI или unit/integration check).
- [ ] `cargo fmt`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
