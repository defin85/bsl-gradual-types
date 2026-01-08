# P1: TODO list — идентификаторы и inputs (FileId, deps_id, settings_id)

**Дата:** 2026-01-08  
**Статус:** 🔴 TODO  
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

- [ ] Уточнить ключ интернинга: canonical `path` (предпочтительно) vs raw `Url`.
- [ ] Сделать маппинг `Url/path -> FileId` сессионно-стабильным:
  - [ ] на `didClose` удалять только `file_text/file_version` (RemoveFile), но не удалять mapping.
- [ ] Добавить короткий комментарий/инвариант в LSP слое: что такое “session stable”.

### 2) Inputs: `file_text(FileId)` и `file_version(FileId)`

- [ ] В `bsl-analysis-v2` привести representation текста к `Arc<str>` или `Arc<String>`.
- [ ] Сделать явные методы/queries уровня `AnalysisV2`:
  - [ ] `file_text(FileId) -> Arc<str>` (или `Option<Arc<str>>`)
  - [ ] `file_version(FileId) -> i32` (или `Option<i32>`)
- [ ] Добавить тесты на корректность обновления `text` и `version` (не только len).

### 3) Input: `deps_id() -> DepsSnapshotId`

- [ ] Определить тип `DepsSnapshotId` в `bsl-analysis-v2` (opaque, логируемый).
- [ ] Добавить singleton salsa input `deps_id`.
- [ ] Определить источник fingerprint в backend:
  - [ ] стартовая версия: использовать уже существующий hash
    (например, `IndexSnapshotId` из `backend/src/system/intellisense_index.rs`),
  - [ ] расширение: добавить `DEPS_SCHEMA_VERSION` (отдельно от индекса), если нужно.
- [ ] Протянуть обновление `deps_id` в LSP:
  - [ ] начальная установка при старте,
  - [ ] обновление при смене platform/config deps (после reload/индексации).

### 4) Input: `settings_id() -> SettingsId`

- [ ] Определить тип `SettingsId` в `bsl-analysis-v2` (opaque, логируемый).
- [ ] Добавить singleton salsa input `settings_id`.
- [ ] Определить стабильный способ вычисления fingerprint настроек в backend/LSP:
  - [ ] минимальный набор полей, влияющих на семантику/кэш (P1),
  - [ ] `SETTINGS_SCHEMA_VERSION` для контролируемого bump.
- [ ] Протянуть обновление `settings_id`:
  - [ ] `did_change_configuration`,
  - [ ] любые другие точки смены `BslSettings`, которые могут влиять на v2.

### 5) Observability и верификация

- [ ] В v2 ветке completion/hover логировать observed: `(file_version, deps_id, settings_id)`.
- [ ] Добавить unit test(ы) в `bsl-analysis-v2`, что `deps_id/settings_id` читаются из снапшота.
- [ ] Прогон:
  - [ ] `cargo test -p bsl-analysis-v2`
  - [ ] `cargo test --workspace`

## DoD (P1 считается закрытым, если)

- [ ] `file_text/file_version/deps_id/settings_id` существуют как inputs и обновляются только через v2 путь.
- [ ] `FileId` сессионно-стабилен, и v2 queries не завязаны на `Url/String`.
- [ ] Логи/трассировка показывают `(file_version, deps_id, settings_id)` для каждого completion/hover.
