# P1: TODO list — идентификаторы и inputs (FileId, deps_id, settings_id)

**Дата:** 2026-01-08  
**Статус:** 🟢 РЕАЛИЗОВАНО  
**Проверка:** `cargo test -p bsl-analysis-v2`, `cargo test --workspace`  
**Основание:** Фаза P1 из `docs/roadmap/intellisense-v2-roadmap/architecture-intermediate/salsa-migration-plan.md`

## Цель P1

- Перевести идентичности и глобальные зависимости в явные salsa inputs:
  `file_text/file_version/deps_id/settings_id`.
- Убрать shared mutable state как источник correctness для hot path.
- Для каждого запроса completion/hover логировать observed: `(file_version, deps_id, settings_id)`.

## Решения P1 (фиксируем заранее)

- `FileId` остаётся LSP-owned, выдаётся монотонно и используется во всех v2 queries.
- Маппинг `Url/path -> FileId` должен быть **сессионно-стабилен** (не “перевыдавать” ID при close/open).
- `deps_id` и `settings_id` — **singleton inputs** (глобальные) и меняются только через v2 writer path.
- `DepsSnapshotId` — **стабильный fingerprint**, собранный из уже существующих fingerprints:
  ориентир — `backend/src/system/system_coordinator/config_loader.rs` и текущая логика `IndexSnapshotId`
  (`backend/src/system/intellisense_index.rs`).
- `SettingsId` — стабильный fingerprint от релевантных настроек + `SETTINGS_SCHEMA_VERSION`
  (временный fallback допустим: монотонный epoch).

## TODO (шаги)

### 1) `FileId` (interning) на границе LSP

- [x] Уточнить ключ интернинга: canonical `path` (предпочтительно) vs raw `Url`.
- [x] Сделать маппинг `Url/path -> FileId` сессионно-стабильным:
  - [x] на `didClose` удалять только `file_text/file_version` (RemoveFile), но не удалять mapping.
- [x] Добавить короткий комментарий/инвариант в LSP слое: что такое “session stable”.

### 2) Inputs: `file_text(FileId)` и `file_version(FileId)`

- [x] В `bsl-analysis-v2` привести representation текста к `Arc<str>` или `Arc<String>`.
- [x] Сделать явные методы/queries уровня `AnalysisV2`:
  - [x] `file_text(FileId) -> Arc<str>` (или `Option<Arc<str>>`)
  - [x] `file_version(FileId) -> i32` (или `Option<i32>`)
- [x] Добавить тесты на корректность обновления `text` и `version` (не только len).

### 3) Input: `deps_id() -> DepsSnapshotId`

- [x] Определить тип `DepsSnapshotId` в `bsl-analysis-v2` (opaque, логируемый).
- [x] Добавить singleton salsa input `deps_id`.
- [x] Определить источник fingerprint в backend:
  - [x] стартовая версия: использовать уже существующий hash
    (например, `IndexSnapshotId` из `backend/src/system/intellisense_index.rs`),
  - [x] расширение: добавить `DEPS_SCHEMA_VERSION` (отдельно от индекса), если нужно.
- [x] Протянуть обновление `deps_id` в LSP:
  - [x] начальная установка при старте,
  - [x] обновление при смене platform/config deps (после reload/индексации).

### 4) Input: `settings_id() -> SettingsId`

- [x] Определить тип `SettingsId` в `bsl-analysis-v2` (opaque, логируемый).
- [x] Добавить singleton salsa input `settings_id`.
- [x] Определить стабильный способ вычисления fingerprint настроек в backend/LSP:
  - [x] минимальный набор полей, влияющих на семантику/кэш (P1),
  - [x] `SETTINGS_SCHEMA_VERSION` для контролируемого bump.
- [x] Протянуть обновление `settings_id`:
  - [x] `did_change_configuration`,
  - [x] любые другие точки смены `BslSettings`, которые могут влиять на v2.

### 5) Observability и верификация

- [x] В v2 ветке completion/hover логировать observed: `(file_version, deps_id, settings_id)`.
- [x] Добавить unit test(ы) в `bsl-analysis-v2`, что `deps_id/settings_id` читаются из снапшота.
- [x] Прогон:
  - [x] `cargo test -p bsl-analysis-v2`
  - [x] `cargo test --workspace`

## DoD (P1 считается закрытым, если)

- [x] `file_text/file_version/deps_id/settings_id` существуют как inputs и обновляются только через v2 путь.
- [x] `FileId` сессионно-стабилен, и v2 queries не завязаны на `Url/String`.
- [x] Логи/трассировка показывают `(file_version, deps_id, settings_id)` для каждого completion/hover.
