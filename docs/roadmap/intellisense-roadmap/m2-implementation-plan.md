# План реализации M2: Индексы и источники данных

**Статус:** 🟡 ЧАСТИЧНО РЕАЛИЗОВАНО  
**Цель:** реализовать индексы и инвалидацию так, чтобы completion использовал атомарный snapshot без смешивания состояний.

---

## Область работ

- TypeIndex / SymbolIndex / ModuleIndex / MetadataIndex / KeywordIndex
- Snapshot‑консистентность и правила инвалидации
- Прогрев индексов из disk cache (без hot‑path I/O)

---

## Пошаговый план

### Шаг 1: Определить структуры данных и версии 🟡
- Добавить `IndexSnapshotId` и `index_schema_version`.
- Ввести `IndexItem`/`IndexKind`/`IndexStoreVersion`.
- Зафиксировать формат ключей и лимиты (max items, payload_version).

**Выход:** типы/структуры в shared/backend модуле.

---

### Шаг 2: Реализация in‑memory индексов 🟡
- TypeIndex: карта типов + фасеты + сигнатуры.
- SymbolIndex: per‑URI индекс локальных символов.
- ModuleIndex: экспортируемые функции/процедуры.
- MetadataIndex: объекты конфигурации.
- KeywordIndex: ключевые слова и директивы.

**Выход:** модуль индексов с CRUD API и snapshot‑чтением.

---

### Шаг 3: Инвалидация и snapshot 🟡
- Реализовать правила инвалидации для:
  - изменения BSL файла;
  - изменения метаданных;
  - смены платформы;
  - смены версии индекса.
- Ввести атомарное обновление `IndexSnapshotId`.

**Выход:** корректная перестройка без смешивания состояний.

---

### Шаг 4: Интеграция с существующими кэшами 🟡
- AST cache → SymbolIndex. (ещё не подключено)
- IR cache → ModuleIndex/TypeIndex. (частично: ModuleIndex от module_signatures)
- Metadata loader/TypeRepository → TypeIndex/MetadataIndex. (подключено)
- Disk cache → warmup (без блокировки hot path). (ещё не подключено)

**Выход:** единый pipeline индексации от текущих источников.

---

### Шаг 5: Persistence (warmup) ⏳
- Формат хранения `IndexStoreVersion`.
- Layout `.bsl_cache/index/...` по snapshot id.
- Загрузка индексов при старте (best‑effort).

**Выход:** ускоренный cold start без влияния на completion latency.

---

### Шаг 6: Тесты ⏳
- Unit: CRUD индексов, инвалидация, snapshot.
- Integration: изменение файла → обновление только нужных индексов.
- Regression: стабильность snapshot при параллельных изменениях.

**Выход:** базовый набор тестов M2.

---

## Критерии завершения

- Индексы работают в памяти, имеют snapshot‑консистентность.
- Инвалидация корректно обновляет только затронутые части.
- Есть warmup из disk cache без I/O в hot‑path completion.
- Тесты покрывают базовые сценарии.

---

## Фактический статус (по коду)

- `IndexSnapshotId`, `INDEX_SCHEMA_VERSION`, `IndexItem`, `IndexKind` реализованы в `backend/src/system/intellisense_index.rs`.
- `IndexStoreVersion` не найден в коде (упоминается только в документации).
- In‑memory индексы есть, но заполняются только `TypeIndex`/`MetadataIndex` (из metadata loader) и `ModuleIndex` (из module_signatures).
- `SymbolIndex` наполняется из AST cache (ParserCoordinator), `KeywordIndex` наполняется из syntax_helper (shlang) с fallback на встроенный список.
- Инвалидация реализована, но `invalidate_platform_types` нигде не вызывается, а `invalidate_file` вызывается без `module_key` (ModuleIndex не чистится на изменение файла).
- Persistence/warmup для индексов реализованы (disk‑store + фоновые best‑effort загрузки + метрики warmup).
- Тесты: есть unit‑тесты CRUD/инвалидации в `backend/src/system/intellisense_index.rs`, нет integration/regression.

## Чек-лист задач для завершения M2

- Добавить `IndexStoreVersion` и формат persistence для индексов; определить layout и сериализацию/десериализацию в `backend/src/system/disk_cache.rs` и/или новом модуле хранения индексов.
- Реализовать warmup индексов при старте (best-effort), без I/O в hot‑path completion.
- Подключить `SymbolIndex` к `backend/src/system/ast_cache.rs` и обновлять его при пересборке AST.
- Заполнять `KeywordIndex` (единый источник ключевых слов/директив, например через константный список или отдельный loader).
- При изменении файла передавать `module_key` в `IntellisenseIndexStore::invalidate_file`, чтобы инвалидировать `ModuleIndex` (см. `backend/src/application/type_system/service.rs`).
- Вызвать `invalidate_platform_types` при смене платформы/доков платформы.
- Добавить интеграционные тесты инвалидации/снапшота и регрессионные тесты на параллельные изменения.

## Задачи (тикеты) по M2

### T1: Persistence формата индексов ✅
**Статус:** выполнено
**Цель:** ввести стабильный формат хранения и версионирования индексов.
**Где:** `backend/src/system/disk_cache.rs` или новый модуль storage.
**DoD:**
- добавлен `IndexStoreVersion` и фиксированный layout `.bsl_cache/index/...`;
- сериализация/десериализация индексов покрыта unit‑тестами;
- поведение при смене версии — полная инвалидация persistent данных.

### T2: Warmup индексов при старте ✅
**Статус:** выполнено
**Цель:** best‑effort прогрев индексов без I/O в hot path.
**Где:** `backend/src/system/system_coordinator/coordinator.rs` (инициализация) + storage слой.
**DoD:**
- индексы загружаются на старте в фоне;
- completion/hover не делают I/O на disk cache;
- метрики/лог о попадании/промахе при warmup.

### T3: Заполнение SymbolIndex из AST cache ✅
**Статус:** выполнено
**Цель:** связывание локальных символов с файлами.
**Где:** `backend/src/system/parser_coordinator.rs`, `backend/src/system/intellisense_index.rs`.
**DoD:**
- на пересборке AST обновляется `SymbolIndex[uri]`;
- инвалидация файла очищает `SymbolIndex[uri]`;
- unit‑тест на обновление/инвалидацию `SymbolIndex`.

### T4: Источник KeywordIndex ✅
**Статус:** выполнено
**Цель:** единый источник ключевых слов/директив BSL.
**Где:** новый модуль loader или константный список в `backend/src/system`.
**DoD:**
- `KeywordIndex` заполняется при старте/инициализации;
- есть unit‑тест, что список не пустой и стабилен.

### T5: Корректная инвалидация ModuleIndex при изменении файла ✅
**Статус:** выполнено
**Цель:** не смешивать состояния при обновлении файла.
**Где:** `backend/src/application/type_system/service.rs`.
**DoD:**
- `invalidate_file` вызывается с корректным `module_key`;
- есть тест или проверяемый кейс, что `ModuleIndex[module]` очищается на изменение.

### T6: Инвалидация платформенных типов ✅
**Статус:** выполнено
**Цель:** корректная инвалидация при смене платформы/доков.
**Где:** точка смены платформы (координатор/loader) + `IntellisenseIndexStore`.
**DoD:**
- при смене платформы вызывается `invalidate_platform_types`;
- unit‑тест покрывает сценарий.

### T7: Интеграционные и регрессионные тесты ✅
**Статус:** выполнено
**Цель:** зафиксировать поведение snapshot/инвалидации.
**Где:** `backend/tests/...`.
**DoD:**
- интеграционный тест: изменение файла → обновление только нужных индексов;
- регрессионный тест: стабильность snapshot при параллельных изменениях.
