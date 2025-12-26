# Roadmap: IntelliSense для BSL (Completion/Signature Help/Snippets)

**Статус:** 🔴 ПЛАН  
**Приоритет:** HIGH  
**Цель:** реализовать полнофункциональный IntelliSense на базе LSP: контекстное автодополнение, подписи методов, релевантное ранжирование и вставку импортов/using.

---

## Контекст

Сейчас автодополнение есть фрагментарно и зависит от базовых индексов. Требуется единая архитектура, чтобы:
- повысить качество подсказок (контекст, фасеты, типы),
- обеспечить низкую задержку,
- поддерживать инкрементальное обновление при изменениях файлов/метаданных.

---

## Объем (definition of IntelliSense)

**Обязательно:**
- LSP `textDocument/completion`
- LSP `completionItem/resolve`
- LSP `textDocument/signatureHelp`
- контекстные подсказки (по типам/фасетам/контексту выполнения)
- импорт/using insertion (где это применимо)
- поддержка snippets (placeholders/optional params)

**Желательно:**
- умное ранжирование и фильтрация
- dedupe подсказок из разных источников
- telemetry/метрики качества (локально)

---

## Нефункциональные требования

- **Latency:** P95 < 50ms для обычных файлов; P99 < 150ms для больших.
- **Determinism:** одинаковые входные данные → одинаковые результаты.
- **Cancelability:** поддержка отмены запросов LSP.
- **Incremental:** корректная инвалидация по изменению файлов/метаданных.
- **Memory:** не более +15% к базовому потреблению LSP.

---

## Архитектурные принципы

- Единый источник типов (TypeSystem/TypeResolution).
- Разделение этапов: **сбор данных → фильтрация → ранжирование → форматирование**.
- Кэширование на уровне workspace и файла.
- Никаких блокирующих I/O в hot path completion.

---

## Milestones

### M1: Спецификация IntelliSense API и формата данных
**Цель:** зафиксировать входы/выходы completion pipeline.

**Критерии успеха:**
- описан список источников подсказок (keywords, symbols, metadata, stdlib);
- определен формат CompletionCandidate;
- определен контракт для SignatureHelp.

#### Спецификация M1

**Источники подсказок (CandidateSource):**
1) keywords (ключевые слова BSL)
2) stdlib (платформенные типы/методы)
3) metadata (объекты конфигурации, фасеты)
4) symbols (локальные/глобальные переменные, параметры, поля)
5) modules (экспортируемые функции/процедуры модулей)

**Контекст запроса (CompletionContext):**
- file_uri
- position (line, col)
- trigger_char (nullable)
- scope (local/global/module)
- type_context (resolved TypeResolution или Unknown)
- facet_context (Manager/Object/Reference/Selection)
- execution_context (Server/Client/Universal)

**Формат кандидата (CompletionCandidate):**
```
{
  label: String,
  kind: CompletionItemKind,
  source: CandidateSource,
  detail: String?,
  sort_key: String,
  score: f32,
  insert_text: String?,
  insert_text_format: PlainText|Snippet,
  text_edit: TextEdit?,
  additional_text_edits: [TextEdit]?,
  data: { candidate_id, source, payload_version }
}
```

**Политика сортировки:**
1) score desc
2) source priority (metadata > symbols > stdlib > modules > keywords)
3) label lexicographic

**LSP контракт:**
- `initialize.capabilities`:
  - `completionProvider.resolveProvider = true`
  - `completionProvider.triggerCharacters = [".", "("]`
  - `signatureHelpProvider.triggerCharacters = ["("]`
- `textDocument/completion` возвращает список `CompletionItem` с минимумом полей.
- `completionItem/resolve` добавляет тяжелые данные (documentation, detail, edits).
- `textDocument/signatureHelp` возвращает `SignatureHelp` с activeSignature/activeParameter.

**SignatureHelp формат (внутренний):**
```
{
  signatures: [{
    label: String,
    documentation: String?,
    parameters: [{ label: String, documentation: String? }]
  }],
  active_signature: usize,
  active_parameter: usize
}
```

**Ограничения ответа:**
- лимит по количеству candidates (по умолчанию 200)
- `isIncomplete=true` при ограничении списка
- все тяжелые данные переносятся в resolve

---

### M2: Индексы и источники данных
**Цель:** подготовить стабильные данные для подсказок.

**Задачи:**
- индекс типов и методов платформы (с фасетами);
- индекс модулей конфигурации (BSL + metadata);
- синхронизация с disk cache/IR cache.

**Критерии успеха:**
- стабильный индекс после изменения одного файла;
- инвалидация работает корректно.

**Фактический статус (по коду):**
- реализованы `IndexSnapshotId`, `IndexItem`, `IndexKind` и in‑memory store (`backend/src/system/intellisense_index.rs`);
- `TypeIndex`/`MetadataIndex` заполняются из metadata loader, `ModuleIndex` — из `module_signatures` (`backend/src/system/system_coordinator/config_loader.rs`);
- `SymbolIndex` и `KeywordIndex` не наполняются;
- нет persistence/warmup индексов; `IndexStoreVersion` отсутствует;
- инвалидация есть, но `invalidate_file` вызывается без `module_key`, а `invalidate_platform_types` нигде не вызывается.

---

#### Спецификация M2

**Индексы (IndexKind):**
1) TypeIndex (platform + config): типы, фасеты, сигнатуры
2) SymbolIndex: локальные/глобальные символы (vars, params, consts)
3) ModuleIndex: экспортируемые процедуры/функции модулей
4) MetadataIndex: объекты конфигурации (documents, catalogs, registers)
5) KeywordIndex: ключевые слова/директивы BSL

**Единый формат записи (IndexItem):**
```
{
  name: String,
  kind: SymbolKind|TypeKind|MetadataKind|Keyword,
  source: IndexKind,
  uri: String?,
  span: (start_line, start_col, end_line, end_col)?,
  signature: String?,
  facets: [FacetKind]?,
  visibility: Public|Private?,
  scope: Local|Module|Global?,
  payload_version: u32
}
```

**Хранение и доступ:**
- In-memory карты:
  - `TypeIndex: HashMap<TypeKey, TypeInfo>`
  - `SymbolIndex: HashMap<Uri, Vec<SymbolInfo>>`
  - `ModuleIndex: HashMap<ModuleKey, Vec<ExportedSymbol>>`
  - `MetadataIndex: HashMap<MetadataKind, Vec<MetadataObject>>`
- Persistent слой (по возможности) для крупных индексированных данных.

**Инкрементальная инвалидация:**
- file change → invalidate SymbolIndex[uri] + ModuleIndex[module]
- metadata change → invalidate MetadataIndex + TypeIndex[config facets]
- platform docs change → invalidate TypeIndex[platform]
- version bump (payload_version) → invalidate all

**Снимки и консистентность:**
- единый `IndexSnapshotId` для всех индексов
- запрос completion использует атомарный snapshot (no mixed state)

**Сборка индекса:**
- initial build при открытии workspace
- background rebuild на изменения (debounced)
- отсутствие блокирующих операций в completion path

**Совмещение с существующими кэшами/индексами:**
- `backend/src/system/ast_cache.rs` → источник `SymbolIndex` (локальные символы/AST).
- `backend/src/system/ir_cache.rs` → источник `ModuleIndex` (экспортируемые процедуры/функции) и часть `TypeIndex`.
- `backend/src/system/disk_cache.rs` → persistence для тяжёлых индексов; не использовать в hot path completion.
- `TypeRepository`/metadata загрузчики → источник `TypeIndex`/`MetadataIndex`.

**Единые правила инвалидации:**
- изменение BSL файла → сброс `SymbolIndex[uri]`, `ModuleIndex[module]`, AST/IR cache для файла;
- изменение metadata/config → сброс `MetadataIndex`, `TypeIndex` (включая фасеты), IR‑derived facets;
- смена версии платформы → сброс `TypeIndex(platform)`;
- смена версии формата индексов → глобальная инвалидация.

**Snapshot‑консистентность:**
- все индексы должны ссылаться на общий `IndexSnapshotId`;
- completion использует только атомарный snapshot (без смешивания).

**IndexSnapshotId (формирование):**
- `IndexSnapshotId = H(config_fingerprint || platform_version || index_schema_version)`
- `index_schema_version` хранить как строку (например, `"intellisense-index-v1"`).
- при смене `index_schema_version` сбрасывать persistent индексы полностью.

**Версионирование форматов:**
- `payload_version` в `IndexItem` должен меняться при любом изменении формата или семантики полей.
- отдельный `IndexStoreVersion` для формата хранения на диске.

**Хранилище индексов (пример layout):**
- `.bsl_cache/index/`
  - `types/<IndexSnapshotId>.bin`
  - `symbols/<IndexSnapshotId>/<uri_hash>.bin`
  - `modules/<IndexSnapshotId>.bin`
  - `metadata/<IndexSnapshotId>.bin`
- хранение в бинарном формате (bincode/zstd), чтение только при warmup.

**Гарантии консистентности:**
- `SymbolIndex` и `ModuleIndex` должны быть рассчитаны на одном AST/IR snapshot.
- `TypeIndex` и `MetadataIndex` должны быть согласованы с конфигурацией по `config_fingerprint`.

### M3: LSP Completion MVP
**Цель:** базовое автодополнение через LSP.

**Задачи:**
- LSP `textDocument/completion` в hot path;
- базовая фильтрация по контексту выражения;
- минимальные `CompletionItem` с label/kind/detail.

**Критерии успеха:**
- корректные подсказки для ключевых слов и платформенных типов;
- latency P95 < 50ms на типовых файлах.

---

#### Спецификация M3

**LSP capabilities:**
- `completionProvider.resolveProvider = true`
- `completionProvider.triggerCharacters = [".", "("]`
- `signatureHelpProvider.triggerCharacters = ["("]`

**Pipeline MVP:**
1) Parse request → `CompletionContext`
2) Fetch candidates from `KeywordIndex` + `TypeIndex`
3) Filter by prefix + scope
4) Rank by source priority
5) Map to `CompletionItem`

**Ограничения ответа:**
- max_items = 200
- `isIncomplete=true` при превышении лимита
- тяжелые поля (documentation, detail) только через resolve

**Минимальный `CompletionItem`:**
```
{
  label,
  kind,
  sortText,
  filterText,
  insertText,
  insertTextFormat
}
```

**MVP источники:**
- keywords (ключевые слова)
- platform types (основные типы и их методы)

**Метрики MVP:**
- P95 < 50ms (без resolve)
- P99 < 150ms (resolve не более 1 item)

### M4: Контекстное ранжирование и качество
**Цель:** повысить релевантность результатов.

**Задачи:**
- ранжирование по типам/фасетам/контексту;
- дедуп по источникам;
- политика сортировки и стабильности.

**Критерии успеха:**
- снижение «мусорных» подсказок;
- стабильный порядок для одинакового контекста.

---

### M5: Snippets и Signature Help
**Цель:** поддержка placeholders и подписей.

**Задачи:**
- `completionItem/resolve` для тяжелых деталей;
- snippets для методов с параметрами;
- `textDocument/signatureHelp` с активным параметром.

**Критерии успеха:**
- корректные placeholders и optional params;
- signature help на вызовах методов и функций.

---

### M6: Импорты/Using и auto-insert
**Цель:** автоматическая вставка импортов/using.

**Задачи:**
- определение missing import;
- textEdit для вставки;
- конфиг переключения авто‑импорта.

**Критерии успеха:**
- корректная вставка в типовой структуре модулей;
- без side effects при отмене.

---

### M7: Производительность и телеметрия
**Цель:** измеримость и стабильность.

**Задачи:**
- метрики latency/coverage;
- трассировка completion pipeline (debug mode);
- нагрузочные тесты.

**Критерии успеха:**
- P95/P99 метрики соответствуют NFR;
- есть отчеты для CI/локально.

---

### M8: Тесты и регрессии
**Цель:** надежность изменения подсказок.

**Задачи:**
- unit tests для фильтрации/ранжирования;
- golden tests для completion output;
- интеграционные тесты с LSP.

**Критерии успеха:**
- >= 30 unit tests + >= 10 golden tests;
- стабильные результаты на CI.

---

## Зависимости

- Типовая система и фасеты (TypeSystem, TypeResolution).
- Парсер конфигурации и индекс модулей.
- Disk cache/IR cache для ускорения.

---

## Definition of Done

- Completion, SignatureHelp, Snippets работают через LSP.
- Контекстные подсказки по типам/фасетам.
- Низкая задержка на типовых проектах.
- Набор тестов и метрик качества.
