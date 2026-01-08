# P2: TODO list — LineIndex и позиционирование как query

**Дата:** 2026-01-08  
**Статус:** 🟢 DONE (clamp, пункт 6: `bsl-line-index`)  
**Основание:** Фаза P2 из `docs/roadmap/intellisense-v2-roadmap/architecture-intermediate/salsa-migration-plan.md`

## Цель P2

- Сделать конвертацию LSP позиций (UTF-16) в byte offset/line+byte column (Point) **функцией от `file_text` снапшота**.
- Устранить класс ошибок “mixed text”: нельзя брать текст из одного источника, а `LineIndex` — из другого.

## Внешние референсы (prior art)

- Rust Analyzer `line-index` (battle-tested UTF-16/UTF-8 mapping):
  - https://github.com/rust-lang/rust-analyzer/tree/master/lib/line-index
  - https://docs.rs/line-index/0.1.2
- LSP 3.17 Position/PositionEncoding (UTF-16 обязателен, character clamp к длине строки):
  - https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#position

## Локальные референсы (в репо)

- Shared реализация и тесты позиционирования (v2 + legacy): `line-index/src/lib.rs`
- Legacy adapter (`tree_sitter::Point` API): `backend/src/system/positioning.rs`
- v2 API (salsa snapshot + helpers): `analysis-v2/src/lib.rs`
- Чек-лист минимальных queries (включая `line_index`): `docs/roadmap/intellisense-v2-roadmap/architecture-intermediate/direct-salsa-checklist.md`

## Решение по реализации (фиксируем перед кодом)

Рекомендация для P2: **портировать текущую логику из `backend/src/system/positioning.rs` в `bsl-analysis-v2`**,
без зависимости от `bsl-backend` и без подключения тяжёлых shared-крейтов.

После стабилизации: вынесли `LineIndex` в отдельный лёгкий crate `bsl-line-index` и переиспользуем в `backend` и `analysis-v2`.
При необходимости можно заменить внутреннюю реализацию на upstream `line-index` (RA).

Производительность: наивная конвертация через сканирование строки O(n) на запрос; оптимизировать только по профилированию.

## TODO (шаги)

### 1) Ввести `LineIndex` в `bsl-analysis-v2`

- [x] Добавить API позиционирования в `bsl-analysis-v2` (реэкспорт из `bsl-line-index`; реализация вынесена в пункте 6):
  - [x] `utf16_to_byte_offset(text: &str, utf16_offset: u32) -> usize`
  - [x] `byte_offset_to_utf16(text: &str, byte_offset: usize) -> u32`
  - [x] `LineIndex::new(source: &str)` + хранение line starts (реализация в `bsl-line-index`)
  - [x] методы конвертации:
    - [x] `utf16_position_to_byte_offset(source: &str, line: u32, utf16_col: u32) -> usize`
    - [x] `byte_offset_to_point(source: &str, byte_offset: usize) -> (line: usize, byte_col: usize)`
    - [x] (если нужно для diagnostics) `byte_offset_to_utf16_position(source: &str, byte_offset: usize) -> (line: u32, utf16_col: u32)`

### 2) Сделать `line_index(FileId)` salsa query

- [x] Добавить query `line_index(SourceFile) -> Arc<LineIndex>`:
  - [x] зависит только от `SourceFile.text`
  - [x] не делает I/O, не читает глобальные mutable кэши
- [x] Добавить публичный метод `AnalysisV2::line_index(FileId) -> Cancellable<Option<Arc<LineIndex>>>`.

### 3) Сделать позиционирование “через снапшот” в v2 API

- [x] Добавить helper-и уровня `AnalysisV2`, которые берут `file_text + line_index` из *одного* снапшота:
  - [x] `utf16_position_to_byte_offset(FileId, line, character) -> Cancellable<Option<usize>>`
  - [x] `utf16_position_to_point(FileId, line, character) -> Cancellable<Option<(usize, usize)>>`
- [x] Договориться о семантике out-of-range:
  - [x] line/character clamp к допустимому диапазону (в духе LSP spec) + закрепить тестами.

### 4) Подключить в LSP v2 ветку (без “mixed text”)

- [x] В `backend/src/bin/lsp_server/server/language_server.rs` (v2 ветки completion/hover/signatureHelp):
  - [x] брать `AnalysisV2` через `self.analysis_host_v2.lock().await.analysis()`
  - [x] конвертировать позицию **только** через `analysis.*positioning*` методы
  - [x] (временно) логировать полученный byte offset/point для верификации

### 5) Тесты (перенос/добавление)

- [x] Сконцентрировать тесты крайних случаев UTF-16 на уровне `bsl-line-index` (общая реализация для v2 и legacy):
  - [x] ASCII
  - [x] кириллица (2-byte UTF-8)
  - [x] emoji / суррогатные пары (4-byte UTF-8, 2 code units UTF-16)
  - [x] clamp поведения (character > line len)
- [x] (Опционально) property-like тест “roundtrip” в `bsl-line-index`:
  - [x] `pos -> offset -> pos` для набора строк.
- [x] В `bsl-analysis-v2` оставить интеграционный тест “позиционирование читается из снапшота”.

### 6) Устранить дублирование `LineIndex` / перейти на upstream

- [x] Убрать расхождение поведения между legacy (`backend`) и v2 (`bsl-analysis-v2`) позиционированием одним из путей:
  - [x] Вариант A: вынести реализацию в отдельный лёгкий crate (например, `bsl-line-index`) и использовать в обоих местах (`line-index/src/lib.rs`)
  - [ ] Вариант B: перейти на upstream `line-index` (rust-analyzer) и адаптировать под нужные операции (включая Point/byte column)

**Критерии готовности (пункт 6):**
- [x] В кодовой базе осталась **одна** реализация конвертаций (или единый upstream), используемая и v2, и legacy путём.
- [x] Полный набор edge-case тестов (ASCII/кириллица/emoji/clamp) живёт в `bsl-line-index` и покрывает обе интеграции; в других местах допустимы smoke-тесты.
- [x] Поведение out-of-range зафиксировано (clamp) и не расходится между путями.
- [x] `cargo test --workspace` проходит.

## DoD (P2 считается закрытым, если)

- [x] В `bsl-analysis-v2` есть query `line_index` и публичный read API для позиционирования.
- [x] В v2 LSP ветке позиция переводится через снапшот (без обращения к legacy `LineIndex`/тексту).
- [x] Добавлены тесты на UTF-16 edge cases в `bsl-line-index` (и интеграционный тест снапшота в `bsl-analysis-v2`).
- [x] `cargo test -p bsl-analysis-v2` проходит.
- [x] (опционально) Если выбран пункт 6: выполнены критерии “Устранить дублирование `LineIndex` / перейти на upstream”.
