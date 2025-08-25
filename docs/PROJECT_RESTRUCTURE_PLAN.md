# План реорганизации проекта: переход на единую плоскую структуру (без дополнительной корневой папки)

## Цели
- Упростить структуру кода и навигацию за счёт плоской структуры верхнего уровня.
- Убрать смешение целевой и устаревшей архитектуры; исключить вложенный корневой модуль (`target`/`architecture`/`unified`).
- Свести всё под слои: Data → Parsing → Domain → Application → Presentation → System — как папки верхнего уровня в `src/`.
- Сохранить стабильные бинарные интерфейсы (LSP/Web/CLI), с временными реэкспортами для совместимости на период миграции.

## Итоговая структура директорий (плоская)
- `src/data/`
  - `repository.rs` — `TypeRepository`, `InMemoryTypeRepository`
  - `raw_models.rs`, `filters.rs`, `stats.rs`
  - `loaders/`
    - `syntax_helper.rs` — загрузка/парсинг синтакс‑помощника (с fallback на текущую логику)
    - `configuration_xml.rs` — guided/XML загрузчик конфигурации
- `src/parsing/`
  - `bsl/` — `ast.rs`, `parser.rs`, `tree_sitter_adapter.rs`
  - `query/` — `ast.rs`, `parser.rs`
- `src/domain/`
  - `types.rs` — модели типов, контекст, completion
  - `metrics.rs`
  - `resolution_service.rs`
  - `resolvers/` — `platform.rs`, `configuration.rs`, `builtin.rs`, `expression.rs`, `bsl_code.rs`
  - `analysis/` — `type_checker.rs`, `flow_sensitive.rs`, `interprocedural.rs`, `dependency_graph.rs`, `type_narrowing.rs`
- `src/application/`
  - `lsp_service.rs`, `web_service.rs`, `analysis_service.rs`
  - `query_service.rs`
  - `documentation_service.rs`
- `src/presentation/`
  - `lsp_interface.rs`, `web_interface.rs`, `cli_interface.rs`
- `src/system/`
  - `central_system.rs` — CentralTypeSystem, конфигурация, health/metrics
- `src/bin/` — бинарники используют только публичный плоский API (`crate::data`, `crate::domain`, ...)
- `src/lib.rs` — экспортирует: `pub mod data;`, `pub mod parsing;`, `pub mod domain;`, `pub mod application;`, `pub mod presentation;`, `pub mod system;` (+ временные алиасы для обратной совместимости)
- `tests/` — unit по слоям и интеграционные (lsp/web/repository/query)

## Маппинг из текущей структуры
- Удаляется вложенный корневой модуль целевой архитектуры:
  - `src/architecture/*` → распределить по `src/data`, `src/parsing`, `src/domain`, `src/application`, `src/presentation`, `src/system`.
  - `src/unified/*` → разложить по слоям (см. ниже); `src/unified/mod.rs` демонтировать.
- Адаптеры и загрузчики:
  - `src/adapters/*` → `src/data/loaders/*` (замена `#[path]` на обычные модули, обновление импортов).
- Централизация модели типов и анализа:
  - `src/core/types.rs` → `src/domain/types.rs` (единая модель типов).
  - `src/core/type_checker.rs`, `flow_sensitive.rs`, `interprocedural.rs`, `dependency_graph.rs`, `type_narrowing.rs` → `src/domain/analysis/*`.
  - `src/core/resolution.rs` → `src/domain/resolution_service.rs`.
  - `src/core/platform_resolver.rs` → либо `src/data/loaders/syntax_helper.rs` (как часть загрузки платформы), либо `src/domain/resolvers/platform.rs` (если семантически ближе к резолверу типов) — финальный выбор на этапе интеграции.
- Парсер/запросы/документация:
  - `src/parser/*` → `src/parsing/bsl/*`.
  - `src/query/*` → `src/parsing/query/*` + сервис `src/application/query_service.rs`.
  - `src/documentation/*` → `src/application/documentation_service.rs` (+ перенос текстов в `RawTypeData.documentation` при загрузке, где применимо).
- Очистка публичного API:
  - Удаляются алиасы и legacy‑пути (`crate::target::...`, `crate::architecture::...`, `crate::unified::...`) после периода совместимости; публичный API становится плоским: `crate::data::...`, `crate::domain::...`, и т.д.

## Правила ответственности
- `data` — загрузка и хранение сырых данных (без доменной логики).
- `parsing` — парсинг BSL/Query (без бизнес‑решений).
- `domain` — разрешение типов, резолверы, анализ/проверка типов.
- `application` — сценарные сервисы (LSP/Web/CLI/Query/Docs).
- `presentation` — адаптеры протоколов/форматов.
- `system` — инициализация слоёв, метрики, health, перезагрузка данных.

## Этапы миграции
1) Подготовка структуры (без изменения логики)
   - Создать `src/data`, `src/parsing`, `src/domain`, `src/application`, `src/presentation`, `src/system`.
   - Перенести `src/adapters/*` в `src/data/loaders/*`; обновить импорты.
   - В `src/lib.rs` добавить временные алиасы:
     - `pub mod unified { pub mod data { pub use crate::data::*; } /* при необходимости другие совместимости */ }`
     - При наличии импортов вида `crate::architecture::*`/`crate::target::*` — аналогичные реэкспорты на плоские модули.

2) Централизация модели типов
   - Переместить `core/types.rs` в `domain/types.rs`; обновить импорты в затронутых модулях.
   - Удалить/деактивировать конкурирующие интерфейсы в пользу слоёв `domain/*`.

3) Перенос парсеров, запросов и документации
   - Перенести `parser/*` → `parsing/bsl/*`.
   - Перенести `query/*` → `parsing/query/*`; создать `application/query_service.rs`.
   - Создать `application/documentation_service.rs`; интегрировать в веб‑сервисы (например, `WebTypeService`).

4) Data‑инициализация и резолверы
   - В `CentralTypeSystem`: заменить загрузку платформы на `data/loaders/syntax_helper.rs` (с fallback на текущую логику внутри лоадера).
   - В инициализации домена вызывать кэш/подготовку: `PlatformTypeResolver::initialize_cache(&repository)`, `BslCodeResolver::initialize_parser()`.
   - В высокоуровневых сценариях (`WebTypeService::advanced_search`) задействовать фильтры через `TypeRepository`.

5) Очистка и фиксация публичного API
   - Обновить `lib.rs` — оставить только плоские `pub mod ...;` без вложенных корневых модулей.
   - Обновить импорты в `src/bin/*` и по коду на плоские пути.
   - Удалить остатки `architecture/`, `unified/`, `core/` (после переноса), `documentation/`, `parser/`, `query/` вне соответствующих плоских слоёв.
   - Обновить документацию в `docs/` и reference.

## Контроль качества
- Формат: `cargo fmt` (прицельно по изменённым файлам).
- Статический анализ: `cargo clippy -- -D warnings`.
- Тесты: unit для `data` и `domain`, интеграционные для LSP/Web/Repository/Query.
- Ручная проверка бинарей: `lsp-server`, `bsl-web-server`, `bsl-analyzer`, `type-check`.

## Риски и митигация
- Разрыв импортов — поэтапные `pub use` в `lib.rs`, сохранение интерфейсов до финальной очистки.
- Время инициализации (парсинг синтакс‑помощника) — кеширование в `TypeRepository`, ленивые загрузчики.
- Регрессии в сервисах — сохранение поведения на этапах 1–3; изменения — только в 4–5 с покрытием тестами.

## Критерии готовности (DoD)
- Бинарники собираются и работают на плоском публичном API (`crate::data`, `crate::domain`, ...), без legacy‑модулей.
- Тесты проходят; метрики/health доступны.
- Документация и оглавление `docs/README.md` обновлены.
