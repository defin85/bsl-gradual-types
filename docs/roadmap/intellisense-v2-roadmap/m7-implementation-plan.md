# План реализации M7: Индексы/снапшоты без клонирования (под рост полноты)

**Статус:** ✅ РЕАЛИЗОВАНО  
**Цель:** убрать архитектурные узкие места hot path (clone больших структур + lock contention) при росте индексов и данных для полноты.

---

## Область работ

- Снапшоты индексов: чтение без `clone()` больших HashMap/Vec
- Конкурентный доступ (много читателей, редкие writer‑ы)
- Согласованность snapshot id и deps/settings снапшотов (без mixed state для completion v2)
- Минимизация выделений памяти в completion path

---

## Пошаговый план

### Шаг 1: Перевод snapshot на `Arc<...>`
- Хранить внутри `IntellisenseIndexStore` `ArcSwap<IndexSnapshot>` и атомарно подменять указатель.
- Чтение в completion: lock-free (`Arc` clone), без копирования содержимого.

**Выход:** `snapshot()` O(1) по времени/памяти.

---

### Шаг 2: Согласование ids снапшотов (deps/index/settings)
- Completion v2 должен использовать данные из одного “мира” (без смешивания deps/index/settings).
- Смена конфигурации/платформы должна приводить к смене ids (deps + index).

**Выход:** отсутствие mixed state для completion.

---

### Шаг 3: Метрики и профилирование
- Добавить метрики времени на стадии:
  - snapshot read
  - collect
  - rank
  - format
- В perf‑режиме фиксировать regressions.

**Выход:** измеримость узких мест.

---

## Критерии завершения

- `snapshot()` не клонирует большие структуры.
- Completion не деградирует по памяти при росте индекса.
- Есть метрики и perf-бенч, которые фиксируют regression.

---

## Задачи (тикеты) по M7

### T1: Arc‑based snapshots ✅
**DoD:**
- snapshot read O(1);
- тесты на потокобезопасность/согласованность.

### T2: Snapshot id согласован с deps/settings (без mixed state) ✅
**DoD:**
- нет mixed state;
- тесты на “смена конфигурации/платформы”.

### T3: Метрики стадий completion ✅
**DoD:**
- метрики/trace по стадиям;
- отчет для локального запуска.

---

## Прогресс (факты по коду)

- `IndexSnapshot` переведён на `Arc<...>` (Copy-on-write через `Arc::make_mut`), поэтому `IntellisenseIndexStore::snapshot()` больше не клонирует большие `HashMap/Vec` (O(1) по времени/памяти на hot path).
  - `backend/src/system/intellisense_index.rs`
  - тест: `snapshot_is_copy_on_write`
- `IntellisenseIndexStore` хранит снапшот в `ArcSwap` и подменяет указатель атомарно (убирает lock contention на чтении).
  - `backend/src/system/intellisense_index.rs`
- Дисковое хранилище индекса обновлено под новые типы на `Arc` (включая symbol payload); включён `serde` feature `rc`.
  - `backend/src/system/intellisense_index_store.rs`
  - `Cargo.toml`
- Согласованность deps+index для completion обеспечена атомарным `snapshot_with_deps()` и покрыта тестами.
  - `backend/src/bin/lsp_server/server/analysis_v2_runtime.rs` (`p8_snapshot_with_deps_is_atomic`)
  - `backend/src/bin/lsp_server/server/core.rs` (`p8_deps_update_is_atomic_and_completion_uses_runtime_index_snapshot`)
- Смена платформы/конфигурации приводит к смене deps/index ids (тесты на стабильность/изменяемость fingerprint/id).
  - `backend/src/system/startup_v2.rs` (`changing_platform_version_changes_deps_and_index_ids`, `changing_configuration_fingerprint_changes_deps_and_index_ids`)
- Добавлены метрики длительностей стадий completion: snapshot read / collect / rank / format.
  - измерения: `backend/src/application/type_system/services/completion_service.rs` (`CompletionStats.stage_*`)
  - экспорт: `backend/src/system/basic_observability.rs` (`record_completion_stage_latency` → `completion_stage_*_ms`)
  - запись метрик из LSP: `backend/src/bin/lsp_server/server/language_server.rs`
- Perf-бенч completion использует сценарии из `backend/tests/perf/scenarios/*.json` (включая `platform_version` при наличии конфигурации).
  - `backend/benches/intellisense_completion_benchmark.rs`

**Как смотреть локально (Web API):**

```bash
./scripts/start-web-api.sh --build
curl -s http://localhost:3002/api/metrics | rg \"completion_stage_\"
```

**Как прогнать perf-бенч:**

```bash
cargo bench -p bsl-backend --bench intellisense_completion_benchmark
```

**Проверка:**
- `cargo test --workspace`
