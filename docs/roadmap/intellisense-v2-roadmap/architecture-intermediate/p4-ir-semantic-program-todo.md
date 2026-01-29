# P4: TODO list — IR/SemanticProgram как query (зависит от deps_id)

**Дата:** 2026-01-09  
**Статус:** 🟢 DONE  
**Основание:** Фаза P4 из `docs/roadmap/intellisense-v2-roadmap/architecture-intermediate/salsa-migration-plan.md`

## Цель P4

- Ввести salsa query `ir(FileId) -> SemanticProgram`, которая является **чистой функцией** от:
  - `parse_result(FileId)` (AST из `bsl-syntax`),
  - `deps_snapshot` (актуальные `TypeRepository/Resolver/SignatureIndex`) и **обязательной** зависимости от `deps_id()`,
  - `settings` (если настройки влияют на построение IR, минимум — dependency на `settings_id`).
- Устранить класс ошибок **mixed deps**: IR, рассчитанный на старых зависимостях, не должен “просачиваться” в новые запросы.
- Подготовить v2 hot path (completion/hover/signatureHelp): IR берётся из снапшота и не строится “на лету” через legacy `parse_to_ir`.

## Контракт (инварианты)

- **Determinism:** одинаковые `(text, deps_id, settings)` → одинаковый `SemanticProgram` (включая порядок узлов/символов; избегаем зависимости от порядка итерации `HashMap`).
- **No I/O:** внутри query нельзя читать файлы/метаданные/конфиги; всё внешнее — inputs/снапшоты.
- **Deps correctness:** query **обязана** зависеть от `deps_id` и использовать зависимости только из соответствующего deps snapshot (без чтения глобального mutable state).
- **No in-place deps mutation:** зависимости внутри deps snapshot (repo/resolver/index) считаются **замороженными**; любые изменения, влияющие на семантику, происходят только через построение нового снапшота и атомарную замену вместе с bump `deps_id` (не через “тихие” мутации существующего repo).
- **UTF-16 корректность:** `Span` внутри IR хранится как **UTF-8 byte offsets**; на границе (LSP/web) конвертируется в UTF‑16 позиции через единый `LineIndex`.
- **Snapshot safety:** никаких “скрытых” глобальных кэшей/синглтонов, которые читаются/пишутся внутри query.
- **Cancellation-friendly:** тяжёлые вычисления находятся внутри salsa-queries, чтобы отмена могла прервать пересчёт.

## Внешние референсы (prior art)

- Salsa `Update` (почему нельзя просто вернуть `Arc<T>` для `T` из другого crate без `Update`):
  - https://docs.rs/salsa/latest/salsa/trait.Update.html
- Query-based подход (как модель вычислений и инвалидации):
  - https://rustc-dev-guide.rust-lang.org/query.html
- (Опционально) rust-analyzer стиль (экосистема `ra_ap_salsa`):
  - https://docs.rs/ra_ap_salsa/

## Локальные референсы (в репо)

- IR тип: `shared/src/ir/program.rs`
- AST (syntax layer): `syntax/src/ast.rs`
- Legacy AST→IR конвертация: `backend/src/application/ast_to_ir/*`
- Legacy `parse_to_ir` (удалено): ранее было в `backend/src/system/parser_coordinator.rs`
- Legacy IR cache: `backend/src/system/ir_cache.rs`
- v2 queries: `analysis-v2/src/lib.rs` (`parse_result`, `line_index`)
- deps_id wiring (LSP → v2 host): `backend/src/bin/lsp_server/server/core.rs`

## Решения P4 (фиксируем перед кодом)

### 1) Где живёт AST→IR конвертация (layering)

Чтобы `bsl-analysis-v2` мог считать IR как query, ему нельзя зависеть от `bsl-backend`.
Значит, AST→IR код нужно вынести из backend.

Варианты:

- **A (текущее решение):** AST→IR живёт внутри `bsl-analysis-v2` (модуль `ast_to_ir`).
  - Зависимости: `bsl-syntax` (AST), `bsl-shared` (IR + domain types).
  - `bsl-backend` использует тот же конвертер через зависимость от `bsl-analysis-v2`.
- **B:** перенести AST→IR в `bsl-shared`.
  - Меньше крейтов, но смешиваем “shared domain” и “semantic engine” в одном слое.
- **C:** оставить AST→IR в backend, а в v2 прокидывать “строитель IR” через trait-object внутри deps snapshot.
  - Минимальный перенос файлов, но больше сложностей с границами, и выше риск протечек backend-деталей в v2.

Рекомендация: **A**.

### 2) Как выглядит deps snapshot для IR (salsa input)

IR зависит не только от `deps_id`, но и от “тела” deps: репозиторий типов, резолвер, индекс сигнатур.
Если query читает их из глобального состояния (например, через `SystemCoordinator`), мы теряем гарантию “no mixed deps”.

Варианты:

- **A (рекомендация):** расширить `DepsSnapshot` salsa input до:
  - `id: DepsSnapshotId` (fingerprint для инвалидации),
  - `semantic: Arc<SemanticDeps>` (иммутабельный снапшот зависимостей).
- **B:** оставить только `deps_id` и читать `TypeRepository/Resolver` из внешнего мира.
  - Допустимо только как временный “скелет”, но нарушает snapshot-safety, если deps меняются конкурентно.

Рекомендация: **A**.

Минимальный состав `SemanticDeps` для P4:
- `repository: Arc<dyn bsl_shared::domain::repository::TypeRepository>`
- `signature_index: bsl_shared::domain::signature_index::SignatureIndex` (клон)
- `resolver: Option<Arc<bsl_shared::domain::resolver::TypeResolver>>`

Важно: текущая реализация `TypeRepository` может иметь внутреннюю мутабельность (locks). Для корректности salsa это допустимо
только при строгом правиле “нет in-place изменений”: любые обновления репозитория, которые могут повлиять на IR, должны идти через
замену deps snapshot + bump `deps_id`.

### 3) Контекст файла (path / module kind)

Построение IR в проекте зависит от контекста модуля (например, seed контекста формы по пути файла).
Значит, “путь” или эквивалентный `CodeLocation` должен стать input.

Варианты:
- **A (проще):** добавить `file_path(FileId) -> Arc<str>` (или поле `path: Arc<str>` в `SourceFile`).
- **B (чище):** хранить уже нормализованный `CodeLocation/ModuleType` как input (без зависимости от платформы путей).

Рекомендация для P4: начать с **A**, и при необходимости перейти на **B** в P5/P8.

### 4) Тип результата query и `salsa::Update`

`Arc<T>` реализует `Update`, только если `T: Update`. Для `SemanticProgram` (тип из `bsl-shared`) нельзя реализовать
`salsa::Update` из `bsl-analysis-v2` из-за orphan rules.

Поэтому возвращаем **обёртку**, как в P3:
- `SemanticProgramSnapshot(Arc<SemanticProgram>)` с `PartialEq` через `Arc::ptr_eq`,
- `unsafe impl salsa::Update` (консервативно “always update”).

### 5) Ошибки построения IR

AST→IR может вернуть ошибку (например, несовместимость инвариантов конвертера).
Так как цель P4 — *структура* query-графа и инвалидация, минимальный вариант:
- на ошибке возвращать `SemanticProgram::new()` (с заполненным `source_info.path` если есть),
- (опционально) завести отдельную query `ir_build_errors(FileId) -> Vec<IrBuildError>` позже (P6).

## TODO (шаги)

### 1) Разместить AST→IR на уровне v2 (внутри `bsl-analysis-v2`)

- [x] Вынести AST→IR из backend в `bsl-analysis-v2::ast_to_ir`.
- [x] Удалить отдельный semantic workspace-crate, чтобы избежать двух «истин» и лишних зависимостей.
  - [x] заменить импорты AST: `crate::parsing::bsl::ast::*` → `bsl_syntax::ast::*` (или `bsl_syntax::ast::{Program, Statement, Expression}`).
  - [x] сохранить публичный API `AstToIrConverter::convert_with_resolver(...)`.
- [x] Обновить импорты в backend/тестах на `bsl_analysis_v2::AstToIrConverter`.
- [x] Перенести/адаптировать тесты конвертера на использование v2 API.

### 2) Расширить v2 inputs: deps snapshot + file context

- [x] Добавить file context input:
  - [x] Вариант A: поле `path: Arc<str>` в `analysis-v2::SourceFile`.
  - [x] В LSP writer path: задавать `path` (минимум: `file_path` или fallback на `uri.to_string()`).
- [x] Расширить `analysis-v2::DepsSnapshot`:
  - [x] добавить поле/запрос на “тело” deps (например, `semantic: DepsDataSnapshot`),
  - [x] `DepsDataSnapshot` = newtype над `Arc<SemanticDeps>` с `Update`/`PartialEq` (по аналогии с P3).
- [x] Расширить `analysis-v2::Change`:
  - [x] заменить `SetDepsId` на атомарное обновление `(deps_id + deps_data)` (или добавить отдельный вариант `SetDepsSnapshot`).
- [x] В backend собрать `SemanticDeps` из актуального состояния после reload (минимум: repo + resolver + signature_index clone)
  и подать в v2 host.
- [x] Зафиксировать правило обновления deps: нельзя менять repo “тихо” без обновления `deps_id`/deps snapshot (иначе IR может
  измениться без пересчёта salsa queries).

### 3) Добавить `ir` salsa query в `bsl-analysis-v2`

- [x] Добавить зависимость `bsl-shared` (для `SemanticProgram`) и модуль AST→IR внутри `bsl-analysis-v2`.
- [x] Добавить newtype `SemanticProgramSnapshot(Arc<SemanticProgram>)` + `Update`.
- [x] Добавить tracked query `ir(db, file: SourceFile, deps: DepsSnapshot, settings: SettingsSnapshot) -> SemanticProgramSnapshot`:
  - [x] зависит от `parse_result(file)` и `deps.id(db)` (и при необходимости `settings.id(db)`),
  - [x] использует `AstToIrConverter::convert_with_resolver` и deps-объекты из snapshot,
  - [x] не делает I/O и не использует global caches.
- [x] Добавить публичный метод `AnalysisV2::ir(FileId) -> Cancellable<Option<Arc<SemanticProgram>>>`.

### 4) Минимальная интеграция / smoke usage

- [x] В v2 ветке (под флагом) добавить “smoke” использование `ir(FileId)`:
  - [x] логировать `nodes.len()` (под `BSL_INTELLISENSE_V2_P4_SMOKE`).
  - [x] (опционально) логировать observed `deps_id`.
- [x] Верифицировать через `rg`, что на пути v2 completion/hover/signatureHelp нет прямых вызовов `parse_to_ir`.

### 5) Тесты (обязательная часть P4)

- [x] Юнит-тесты AST→IR (smoke):
  - [x] простая программа → `SemanticProgram` строится (узлы/символы непустые или ожидаемые по фикстуре).
- [x] Интеграционные тесты в `bsl-analysis-v2`:
  - [x] изменение `file_text` инвалидирует `ir` (включая переход valid → syntax error без panic/`None`).
  - [x] смена `deps_id` инвалидирует `ir`.
  - [x] `RemoveFile` → `ir` возвращает `None`.
  - [x] (если IR зависит от настроек) смена `settings_id` инвалидирует `ir`.
  - [x] (Опционально) тест на детерминизм: два вызова `ir` на одинаковом входе дают одинаковую “стабильную сводку” (hash/JSON с сортировкой).

### 6) Роль legacy IR cache

- [x] Зафиксировать правило: v2 queries (`bsl-analysis-v2`) **не используют** `backend/src/system/ir_cache.rs` (legacy-only).
- [x] Решение: legacy IR cache остаётся в legacy/CLI/Web путях до полной миграции LSP hot path на v2 (P5+); v2 получает IR через salsa query `ir`.

## DoD (P4 считается закрытым, если)

- [x] В `bsl-analysis-v2` есть query `ir` и публичный read API.
- [x] Query зависит от `deps_id` и использует зависимости из deps snapshot (без чтения глобального mutable state).
- [x] Query не делает I/O (проверено по коду + `rg` на `parse_to_ir` в LSP v2 пути).
- [x] Добавлены тесты на инвалидацию по `file_text` и `deps_id`.
- [x] `cargo test -p bsl-analysis-v2` проходит.
- [x] `cargo test --workspace` проходит.

## Реализация (где смотреть в коде)

- `analysis-v2/src/ast_to_ir/mod.rs`: AST→IR модуль (публичный API `AstToIrConverter`).
- `analysis-v2/src/ast_to_ir/converter.rs`: `AstToIrConverter::convert_with_resolver(...)`.
- `analysis-v2/src/lib.rs`: `SemanticDeps`, `DepsDataSnapshot`, `SemanticProgramSnapshot`, tracked query `ir`, `AnalysisV2::ir`.
- `backend/src/bin/lsp_server/server/core.rs`: сборка deps snapshot для v2 + `Change::SetDepsSnapshot`.
- `backend/src/bin/lsp_server/server/language_server.rs`: smoke-лог под `BSL_INTELLISENSE_V2_P4_SMOKE`.
