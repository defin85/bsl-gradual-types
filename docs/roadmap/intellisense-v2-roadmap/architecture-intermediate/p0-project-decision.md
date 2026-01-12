# P0: план работ (salsa + отдельный workspace-crate)

**Дата:** 2026-01-08  
**Статус:** 🟢 РЕАЛИЗОВАНО  
**Проверка:** `cargo test -p bsl-analysis-v2`, `cargo test --workspace`  
**Область:** Фаза P0 из `docs/roadmap/intellisense-v2-roadmap/architecture-intermediate/salsa-migration-plan.md`

**Примечание (P9):** runtime feature-flag `BSL_INTELLISENSE_V2_SALSA` удалён; LSP всегда использует v2 путь.

## Решение (зафиксировано)

- [x] Используем upstream `salsa` (не `ra_ap_salsa`).
- [x] Делаем отдельный workspace-crate для v2 анализа.
- [x] `salsa` версия: `0.25.2` (workspace dependency).
- [x] Feature-flag для P0: env `BSL_INTELLISENSE_V2_SALSA=1` (использовался на этапе миграции; удалён в P9).
- [x] `Url -> FileId` маппинг живёт в LSP слое, `FileId` выдаётся монотонно.

## Цель P0 (что должно получиться)

- [x] В workspace есть новый crate `analysis-v2/` с `package.name = "bsl-analysis-v2"` и `lib.name = "bsl_analysis_v2"`.
- [x] В `bsl-analysis-v2` есть минимальная salsa DB + `AnalysisHostV2/AnalysisV2` (snapshot API).
- [x] В `bsl-lsp-server` есть минимальная проводка: `didOpen/didChange -> apply_change` (v2 inputs).
- [x] В LSP `completion` есть v2 ветка под env-флаг (в P0 допустима заглушка результата).
- [x] `cargo test --workspace` проходит.

## Не-цели P0 (важно не расползтись)

- [ ] Не переносим реальный парсинг/IR/type-inference в salsa queries (это P2–P5).
- [ ] Не реализуем полноценный ra-style cancel/writer-thread протокол (интерфейс можно заложить, гарантий не обещаем).
- [ ] Не удаляем и не переписываем существующие кэши (`IrCache`, `AnalysisCache`) в legacy пути.

## План по шагам

### Шаг 1: Создать workspace-crate `bsl-analysis-v2`

- [x] Добавить workspace member: `analysis-v2` в корневой `Cargo.toml`.
- [x] Добавить workspace dependency: `salsa = "0.25.2"` в `[workspace.dependencies]`.
- [x] Создать `analysis-v2/Cargo.toml` (crate `bsl-analysis-v2`) и `analysis-v2/src/lib.rs`.
- [x] Убедиться, что crate не зависит от LSP (`tower-lsp`) и не делает I/O.

**Выход:** новый crate собирается и тестируется отдельно.

### Шаг 2: Определить публичный API (минимум для P0)

- [x] `FileId(u32)` как внешний ключ (client-picked, выдаётся LSP слоем).
- [x] `Change` (включая `RemoveFile`).
- [x] `AnalysisHostV2`:
  - [x] `apply_change(Change)`
  - [x] `analysis()` -> `AnalysisV2`
- [x] `AnalysisV2`:
  - [x] держит read-only снапшот DB (через `AnalysisDatabase::clone()`)
  - [x] имеет методы-заглушки `completion/hover/signature_help` (P0: пустой результат)
- [x] `type Cancellable<T> = Result<T, salsa::Cancelled>` (catch `Cancelled` через `catch_unwind`).

**Выход:** `bsl-backend` может использовать API без knowledge о внутренностях DB.

### Шаг 3: Минимальная salsa DB и queries (доказать инкрементальность)

- [x] Ввести inputs:
  - [x] `SourceFile { id, text, version }` (salsa `#[input]`)
  - [x] маппинг `FileId -> SourceFile` живёт в `AnalysisHostV2`
- [x] Ввести derived query для демонстрации:
  - [x] `file_text_len(db, SourceFile) -> usize`
- [x] `AnalysisV2` использует только данные из DB снапшота, не читает внешние источники текста.

**Выход:** есть минимальный граф inputs->queries, который можно тестировать.

### Шаг 4: Тесты P0

- [x] Юнит-тесты в `bsl-analysis-v2`:
  - [x] `apply_change` меняет результат derived query
  - [x] `RemoveFile` делает query `None`

**Выход:** воспроизводимый тестовый контур P0.

### Шаг 5: Интеграция в LSP слой (минимальная проводка)

- [x] Добавить хранение `AnalysisHostV2` в `BslLanguageServer` (инициализация при старте).
- [x] Добавить `Url -> FileId` маппинг (внутри LSP сервера), выдача через `AtomicU32`.
- [x] На `didOpen/didChange`:
  - [x] обновлять v2 inputs через `apply_change`
  - [x] не менять legacy путь (P0 только параллельная ветка)
- [x] В `completion`:
  - [x] в P0 было ветвление: `BSL_INTELLISENSE_V2_SALSA=1` -> v2 completion (P0 заглушка), иначе -> legacy.
  - [x] в P9 ветвление удалено, LSP всегда v2.

**Выход:** v2 путь включается env-флагом и не ломает legacy.

### Шаг 6: Наблюдаемость (минимум)

- [x] Логировать при старте сервера, включён ли v2 путь.
- [x] Логировать в v2 completion: `uri`, `FileId`, `text_len`/`cancelled`.

**Выход:** можно диагностировать, что реально используется в рантайме.

## Верификация (перед закрытием P0)

- [x] `cargo test -p bsl-analysis-v2`
- [x] `cargo test --workspace`
