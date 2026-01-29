# IntelliSense v2: промежуточная архитектура (Salsa/rust-analyzer style)

Эта папка фиксирует промежуточный вариант архитектуры для IntelliSense v2, который:
- устраняет "mixed state" (текст уже обновлён, а семантика/IR ещё от старой версии);
- убирает тяжёлую семантику (IR/типизацию) из hot path `completion/hover/signatureHelp`;
- корректно переживает изменения зависимостей во время редактирования (обновление метаданных/платформы).

Цель: получить ключевые свойства "как у rust-analyzer" без необходимости сразу полностью мигрировать код на `salsa` crate.

---

## 1. Контекст и проблема

Исторически (legacy/v1) часть семантики могла строиться "на лету" внутри LSP запросов:
- `completion`: при cache miss строил IR из текста (`parse_to_ir`) и клал в legacy IR cache.
- `hover`: аналогично мог строить IR в запросе при cache miss.

В v2 пути (через `bsl-analysis-v2`) `completion/hover/signatureHelp` читают IR из снапшота
(query `ir(file)`), без вызовов `parse_to_ir` в hot path
(см. `backend/src/application/type_system/services/completion_service.rs`,
`backend/src/application/type_system/services/hover_service.rs`,
`backend/src/application/type_system/services/signature_help_service.rs`).

При этом `didChange` обновляет только текст (и запускает инкрементальный парсинг отдельно),
поэтому легко получить состояние вида:
- текст новый,
- IR/типизация/индексы старые,
- запрос completion/hover видит смесь данных.

Дополнительная сложность: зависимости могут меняться во время редактирования (например, обновились metadata).

---

## 2. Архитектурные драйверы (что важно)

- **Incremental correctness:** результаты соответствуют последнему `didOpen/didChange` (LSP версионность).
- **No heavy work in hot path:** completion/hover не должны инициировать полный парсинг/IR.
- **Cancelability:** отмена запросов (LSP `$/cancelRequest`) не должна приводить к "повисшим" задачам.
- **Deps reload safety:** смена метаданных/репозитория типов не должна давать mixed deps в одном запросе.
- **Determinism:** одинаковый контекст -> одинаковый результат (включая порядок и `sortText`).

---

## 3. Модель "Salsa-style" без обязательного `salsa` crate

Идея: строим единый вычислительный граф "inputs -> queries", а LSP обработчики читают
консистентный снапшот и больше не делают тяжёлых вычислений напрямую.

### 3.1 Inputs (входы)

Минимальный набор входов для консистентности:

- `file_text(uri) -> Arc<String>`
- `file_version(uri) -> i32` (LSP version после применения всех `contentChanges`)
- `deps_epoch() -> u64`
  - монотонно растёт при любом изменении зависимостей (platform docs, metadata, signature index и т.п.).

Если `deps_epoch` поменялся, все семантические результаты считаем устаревшими и пересчитываем.

### 3.2 Queries (запросы)

Минимальные queries, которые закрывают M2:

- `line_index(uri, file_text) -> LineIndex` (UTF-16 <-> byte)
- `parse_result(uri, file_text, deps_epoch) -> ParseResult` (AST + syntax errors)
- `ir(uri, parse_result, deps_epoch) -> SemanticProgram`
- `completion_ctx(uri, pos, ir, deps_epoch) -> ...`
- `hover_ctx(uri, pos, ir, deps_epoch) -> ...`
- `signature_help_ctx(uri, pos, ir, deps_epoch) -> ...`

Важно: все семантические queries обязаны зависеть от `deps_epoch`, иначе возможен "mixed deps".

---

### 3.3 DepsSnapshotId: Merkle fingerprints (как связать с `deps_epoch`)

`deps_epoch` удобен как "монотонная ревизия", но для кэшей и диагностики нужен **стабильный идентификатор**
зависимостей, который можно сравнить/залогировать и которым удобно ключевать disk cache.

В проекте уже реализованы Merkle fingerprints для конфигурации (BLAKE3):
- `backend/src/system/system_coordinator/config_loader.rs`: `config_fingerprint` (XML-only, layer A)
- `backend/src/system/system_coordinator/config_loader.rs`: `config_layer_b_fingerprint` (XML + BSL modules, layer B)
- режимы:
  - fast (по умолчанию): учитывает `(size, mtime_ns)`
  - strict: учитывает `content_hash`, включается `BSL_CACHE_STRICT_FINGERPRINT=1`

Предлагаемая связь:
- `DepsSnapshot` хранит `deps_id: String` (или `DepsSnapshotId`), например:
  - `platform_fp` (версия/пакет платформенных типов; если появится fingerprint для HBK/PlatformBundle)
  - `config_layer_b_fp` (для семантики и индекса экспортов)
  - `settings_fp` (версия алгоритмов парсинга/индексации, strict/fast режим)
- `AnalysisHost` хранит:
  - `deps_id_current`
  - `deps_epoch: u64`
- при `deps_update` пересчитываем новый `deps_id`; если он **отличается**, тогда:
  1) атомарно заменяем `DepsSnapshot` целиком;
  2) увеличиваем `deps_epoch` на 1;
  3) (опционально) планируем warmup для всех открытых документов.

Правило консистентности:
- `didOpen/didChange` меняют только `file_text/file_version`, **не влияют** на `deps_id`.
- изменения на диске (metadata/config/modules/platform docs) влияют только через `deps_update` и смену `deps_id`.

Future (не обязательно для M2):
- хранить "листья" Merkle (path -> leaf-hash) для частичной инвалидации / лучшей диагностики "что именно поменялось".

---

## 4. AnalysisHost: где живёт DB и как её читают LSP фичи

### 4.1 Компоненты

- `DepsSnapshot` (иммутабельный снапшот зависимостей):
  - `AnalysisEngine`
  - `ParserCoordinator` (с нужным resolver/repository)
  - `IntellisenseIndexStore` snapshot/id
  - прочие необходимые ссылки (formatter/lookup и т.п.)

- `AnalysisHost`:
  - хранит текущий `DepsSnapshot` (атомарно заменяемый) и счётчик `deps_epoch`.
  - хранит per-document inputs (`file_text`, `file_version`).
  - предоставляет `snapshot()` для обработки запросов.

### 4.2 Два способа "хостить" AnalysisHost

**Вариант A: `RwLock + snapshot` (проще, как первый шаг)**

- Внутри `AnalysisHost` лежит база под `RwLock`.
- `didOpen/didChange` берут `write`-lock и обновляют inputs.
- `completion/hover/signatureHelp` берут `read`-lock, делают лёгкий `snapshot` (Arc-клоны), отпускают lock
  и дальше считают на snapshot без блокировок.

Плюсы: проще внедрить в текущий код.
Минусы: при ошибках можно получить contention; тяжёлые вычисления нужно выносить из async runtime.

**Вариант B: отдельный analysis thread (ближе к rust-analyzer)**

- Один поток владеет mut-DB и применяет события (`didChange`, `deps_update`) последовательно.
- LSP потоки отправляют события в очередь.
- Запросы работают на snapshot DB (обычно через clone легковесных Arc-структур).

Плюсы: проще гарантировать отсутствие mixed state/dep; естественный debouncing/coalescing изменений.
Минусы: добавляется инфраструктура очередей/протокола взаимодействия.

---

## 5. Обработка изменения зависимостей во время редактирования

Сценарий: "добавили новые метаданные" (или обновили platform docs).

Правило: зависимости меняются только через атомарную замену `DepsSnapshot`.

Процедура:
1) Сформировать новый `DepsSnapshot` целиком (в blocking потоке, если нужно).
2) В `AnalysisHost` атомарно заменить snapshot и увеличить `deps_epoch`.
3) Все последующие queries автоматически будут пересчитаны из-за зависимости от `deps_epoch`.
4) Опционально: запланировать фоновый warmup для открытых документов.

---

## 6. Инварианты консистентности

- `didChange` применяет `contentChanges` в порядке получения (LSP spec) и обновляет `file_version` на версию "после".
- `completion/hover/signatureHelp` читают одну согласованную пару:
  - `(file_text, file_version)` и соответствующий им результат queries на том же `deps_epoch`.
- Семантические результаты, рассчитанные на старом `deps_epoch` или для старого `file_version`, не публикуются.
- Нет I/O в hot path (диск/сеть) для completion/hover/resolve.

---

## 7. План миграции от текущего состояния

1) Ввести `AnalysisHost` и хранить документы не как `HashMap<Url, String>`, а как inputs в хосте.
2) Убрать построение IR из запросов:
   - `completion_service`/`hover_service` должны получать IR из хоста (query `ir(uri)`), а не вызывать `parse_to_ir`.
3) Добавить `deps_epoch` и подключить его к семантическим вычислениям.
4) На `deps_update` (metadata/platform reload) делать replace `DepsSnapshot` + bump `deps_epoch`.
5) (Опционально) Перейти с `RwLock + snapshot` на отдельный analysis thread, если появится contention/латентность.

---

## 8. Что НЕ делаем на этом этапе

- Не вводим тонкую инвалидацию "по одному объекту метаданных" (пока достаточно `deps_epoch`).
- Не переносим весь проект на `salsa` прямо сейчас: сначала фиксируем API и инварианты.
