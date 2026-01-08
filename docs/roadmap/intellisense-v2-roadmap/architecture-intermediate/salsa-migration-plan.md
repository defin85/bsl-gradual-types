# План миграции: IntelliSense v2 → rust-analyzer/salsa-подход

**Статус:** 🔴 ПЛАН  
**Зачем:** получить "как у rust-analyzer" свойства (консистентные снапшоты, инкрементальные queries, дешёвый hot path) без mixed state и mixed deps.

Этот план предполагает, что мы мигрируем **архитектурно** на salsa-подход (DB + queries + snapshots), но **не обязаны** повторять стек rust-analyzer 1:1 (у нас tree-sitter, свой IR и типизация).

См. также: `docs/roadmap/intellisense-v2-roadmap/architecture-intermediate/README.md` и `docs/roadmap/intellisense-v2-roadmap/architecture-intermediate/direct-salsa-checklist.md`.

---

## 0. Предпосылки и границы

### Цели миграции

- Hot path `completion/hover/signatureHelp` читает **консистентный снапшот** и не инициирует тяжелую работу напрямую.
- Инвалидация и пересчёт семантики управляются ревизиями/inputs (`file_text`, `file_version`, `deps_id`).
- Обновления зависимостей (metadata/platform/index) не приводят к mixed deps в одном запросе.
- Отмена и коалесинг LSP запросов/изменений не приводят к "повисшим" задачам и публикации устаревших результатов.
- Детерминизм: одинаковый контекст → одинаковый результат (порядок, `sortText`, resolve identity).

### Не-цели (на первом этапе)

- Полная тонкая инвалидация по "листьям" deps (можно начать с coarse `deps_id`/epoch).
- Перенос всех подсистем проекта на salsa (фокус только на IntelliSense v2 core).
- Оптимизация "до идеала" без профилирования (производительность тюним по метрикам).

### Ограничения текущего кода (ориентиры)

- LSP `didChange` сейчас делает и инвалидацию, и инкрементальный парсинг, и синтакс/семантик‑валидацию:
  `backend/src/bin/lsp_server/handlers/text_document.rs`.
- `completion/hover` сейчас могут строить IR "на лету" (см. описание в промежуточной архитектуре).
- Есть отдельные mutable кэши: `backend/src/system/ir_cache.rs`, `backend/src/system/simple_cache.rs`.
- Парсинг/кэши и позиционирование завязаны на `ParserCoordinator`/tree-sitter и `LineIndex`:
  `backend/src/system/parser_coordinator.rs`, `backend/src/system/positioning.rs`.

---

## 1. Стратегия миграции (strangler + feature flag)

Прямая миграция "всё и сразу" опасна: слишком много точек отказа, сложно отладить инкрементальность/отмену.
Поэтому стратегия:

- Вводим **новый движок анализа v2** (salsa DB) параллельно существующему коду.
- Держим **feature flag** (например, настройка LSP/ENV) и возможность fallback на legacy поведение.
- Переводим фичи по одной (completion → hover → signatureHelp → diagnostics), фиксируя DoD/тесты на каждом шаге.

---

## 2. Фаза P0: Подготовка и прототип

**Результат:** минимальная компилируемая salsa-инфраструктура и "сквозной" сценарий на одном файле.

- [ ] Выбрать salsa реализацию:
  - [ ] `salsa` crate или `ra_ap_salsa` (обсудить совместимость и долгосрочную поддержку).
- [ ] Добавить новый модуль/крейt `analysis_v2` (или аналог) с DB и базовыми типами.
- [ ] Ввести feature flag для переключения completion/hover на v2 (по умолчанию выключен).
- [ ] Скелет API уровня LSP: `AnalysisHostV2::snapshot()` + методы `completion/hover/signature_help`.

**Верификация:**
- [ ] `cargo test --workspace` проходит.
- [ ] Есть простая тест‑фикстура, которая создаёт DB, задаёт `file_text`, читает `line_index`/`parse_result`.

---

## 3. Фаза P1: Идентификаторы и inputs (FileId, deps_id, settings_id)

**Результат:** DB принимает изменения текста/версии и зависимостей через inputs, без shared mutable state.

- [ ] `FileId` (interning):
  - [ ] маппинг `Url/path -> FileId` живёт рядом с LSP (сессионно‑стабилен),
  - [ ] queries работают через `FileId`, а не `Url/String`.
- [ ] Inputs:
  - [ ] `file_text(FileId) -> Arc<str>/Arc<String>`
  - [ ] `file_version(FileId) -> i32`
  - [ ] `deps_id() -> DepsSnapshotId` (стабильный fingerprint)
  - [ ] `settings_id() -> SettingsId` (если настройки влияют на семантику/кэш).
- [ ] `DepsSnapshotId`:
  - [ ] построение из уже существующих fingerprints (ориентир: `backend/src/system/system_coordinator/config_loader.rs`),
  - [ ] смена deps идёт через атомарный swap "снапшот deps целиком".

**Верификация:**
- [ ] Логи/трассировка показывают `(file_version, deps_id)` для каждого запроса completion/hover.

---

## 4. Фаза P2: LineIndex и позиционирование как query

**Результат:** конвертация UTF‑16↔UTF‑8/Point делается на консистентном тексте снапшота.

- [ ] Query `line_index(FileId) -> LineIndex` на основе `backend/src/system/positioning.rs`.
- [ ] Все LSP позиции (completion/hover/signatureHelp) конвертируются только через `line_index + file_text` из снапшота.
- [ ] Запрет смешивания: нельзя брать текст из одного источника, а `LineIndex` из другого.

**Верификация:**
- [ ] Добавить/перенести тесты на крайние случаи UTF‑16 (суррогаты, кириллица, emoji) на уровне v2.

---

## 5. Фаза P3: ParseResult как query (tree-sitter)

**Результат:** синтаксический разбор становится функцией от `file_text` (и настроек), кэшируется salsa.

- [ ] Query `parse_result(FileId) -> ParseResult`:
  - [ ] без I/O,
  - [ ] без глобальных mutable кэшей, влияющих на результат.
- [ ] На первом шаге допускается **полный парсинг из текста** (простая корректность).
- [ ] Опционально позже: ускорение инкрементальным tree-sitter (только если профилирование покажет необходимость).

**Верификация:**
- [ ] `didChange` перестаёт быть источником парсинга "как побочного эффекта" для correctness; correctness обеспечивается queries.

---

## 6. Фаза P4: IR/SemanticProgram как query (зависит от deps_id)

**Результат:** IR становится salsa query и автоматически инвалидируется по deps.

- [ ] Query `ir(FileId) -> Arc<SemanticProgram>`:
  - [ ] зависит от `parse_result(FileId)` и `deps_id()`,
  - [ ] использует актуальный `TypeRepository/Resolver` из deps snapshot.
- [ ] Убрать прямой `parse_to_ir` из hot path (completion/hover/signatureHelp), заменив на чтение `ir(FileId)` из снапшота.
- [ ] Переоценить роль `backend/src/system/ir_cache.rs`:
  - [ ] либо убрать из LSP фич,
  - [ ] либо оставить только как временную оптимизацию вне v2 пути (но не смешивать результаты).

**Верификация:**
- [ ] По трассировке видно, что completion/hover больше не запускают построение IR напрямую.

---

## 7. Фаза P5: Перевод completion/hover/signatureHelp на v2

**Результат:** основные LSP фичи читают только снапшоты v2.

- [ ] Completion:
  - [ ] `backend/src/bin/lsp_server/handlers/completion.rs` вызывает v2 API (под feature flag),
  - [ ] внутри v2 completion использует `completion_service`-логику, но получает данные через queries (`line_index/ir/...`).
- [ ] Hover и SignatureHelp аналогично.
- [ ] Детерминизм:
  - [ ] сортировка стабилизирована (не зависит от порядка в `HashMap`),
  - [ ] `sortText`/`candidate_id` стабильны.

**Верификация:**
- [ ] Golden/fixture тесты для completion/hover/signatureHelp выполняются в двух режимах (legacy и v2) и сравниваются (минимум: без падений и с ожидаемым покрытием).

---

## 8. Фаза P6: Diagnostics pipeline (syntax + semantic) как фоновые задачи

**Результат:** диагностики не блокируют hot path и соответствуют последней версии документа.

- [ ] Queries для диагностик:
  - [ ] `syntax_diagnostics(FileId) -> ...`
  - [ ] `semantic_diagnostics(FileId) -> ...` (зависит от `deps_id`).
- [ ] На `didChange`:
  - [ ] обновляем inputs (`file_text/file_version`),
  - [ ] планируем фоновые вычисления диагностик,
  - [ ] отменяем устаревшие задачи.
- [ ] Публикация диагностик:
  - [ ] результаты публикуются только если `file_version` совпадает с актуальной.

**Верификация:**
- [ ] Тест на сценарий: быстрые серии `didChange` → диагностики соответствуют последнему тексту, без "прыжков назад".

---

## 9. Фаза P7: AnalysisHost v2 как отдельный writer thread (ra-style)

**Результат:** исчезает риск mixed state на уровне синхронизации, снижается contention.

- [ ] Один поток владеет mutable DB (и deps snapshot), применяет события последовательно:
  - `didOpen/didChange/didClose`, `deps_update`, изменения настроек.
- [ ] LSP потоки/таски получают `snapshot` и выполняют queries без удержания write lock.
- [ ] Протокол запросов:
  - [ ] request включает `FileId + file_version` (и при необходимости позицию),
  - [ ] ответ включает `observed (file_version, deps_id)` для диагностики/логов.
- [ ] Отмена:
  - [ ] отмененные запросы не публикуют результат,
  - [ ] устаревшие запросы (старый version) игнорируются.

**Верификация:**
- [ ] Нагрузочный сценарий (ручной): ввод текста + частые completion → нет "подвисаний" и нет mixed state.

---

## 10. Фаза P8: Интеграция deps_update (metadata/platform/index) как атомарный снапшот

**Результат:** смена зависимостей безопасна и наблюдаема.

- [ ] `DepsSnapshot` содержит всё, что нужно queries (repo/resolver/index store и т.п.).
- [ ] `deps_update`:
  - [ ] строит новый `DepsSnapshot` целиком (в blocking/pool),
  - [ ] атомарно заменяет текущий snapshot,
  - [ ] обновляет `deps_id` input (или bump deps revision).
- [ ] (Опционально) warmup открытых документов после deps_update.

**Верификация:**
- [ ] Тест/скрипт: deps_update во время редактирования не приводит к смешанным результатам.

---

## 11. Фаза P9: Удаление legacy путей и кэшей (после стабилизации)

**Результат:** единый путь вычислений, меньше дублирования и меньше риск mixed state.

- [ ] Удалить/задепрекейтить старые пути построения IR в LSP сервисах.
- [ ] Пересмотреть `IrCache`/`AnalysisCache`:
  - [ ] либо удалить,
  - [ ] либо оставить только для CLI/Web сценариев, но без участия в LSP v2.
- [ ] Упростить `TypeSystemService` до фасада над v2 host/snapshots (или разделить API по клиентам).

**Верификация:**
- [ ] Отсутствуют обращения `parse_to_ir` из hot path LSP (по `rg`/CI).

---

## 12. Минимальные "стоп‑критерии" (когда переход можно включать по умолчанию)

- [ ] Completion/hover/signatureHelp используют только v2 снапшоты.
- [ ] Нет mixed state при быстрых `didChange` (интеграционные тесты).
- [ ] Нет mixed deps при `deps_update` (интеграционные тесты).
- [ ] Latency не хуже baseline на типовых проектах (метрики/ручные измерения).
- [ ] Есть трассировка/метрики причин деградации (почему fallback/почему incomplete).

---

## 13. Открытые вопросы (фиксируем до P4/P5)

- Какой salsa crate выбираем и почему (совместимость, активность, примеры).
- Представление текста в DB: `String/Arc<str>` vs `Rope` (память/скорость line index/срезы).
- Граница "deps": что обязано быть внутри `DepsSnapshot` vs что можно держать вне DB (и как обеспечить snapshot‑safety).
- Как интегрируем существующую инкрементальность tree-sitter: оставить как оптимизацию вне DB или делать parse query чистой функцией от текста.

