# Чек-лист: прямой переход на rust-analyzer/salsa-подход

Этот чек-лист нужен, если мы хотим пропустить промежуточную архитектуру из `README.md` и сразу
перейти к модели вычислений "как у rust-analyzer": `salsa`-база, ревизии, снапшоты, queries.

## 0) Решения до старта (фиксируем явно)

- [ ] Выбрана реализация salsa: `salsa` crate или `ra_ap_salsa` (совместимость, поддержка, риски обновления).
- [ ] Выбрана модель хостинга базы:
  - [ ] один writer thread + снапшоты для читателей (ra-style), или
  - [ ] `RwLock/Mutex` вокруг базы + `snapshot()` на запросы.
- [ ] Определена граница "deps": что именно считается зависимостями семантики (metadata/config/platform docs/index/settings).
- [ ] Зафиксирован контракт консистентности: результаты должны соответствовать паре `(file_version, deps_revision)`.

## 1) Инвентаризация текущего состояния (оценка объема рефакторинга)

- [ ] Найдены места, где LSP hot path инициирует тяжелую работу (parse/IR/type/index) и составлен список.
  - Примеры ориентиров: `backend/src/application/type_system/services/completion_service.rs`,
    `backend/src/application/type_system/services/hover_service.rs`.
- [ ] Выписаны текущие кэши/состояния и правила инвалидции (что, когда и почему сбрасывается):
  - `backend/src/system/ir_cache.rs` (IR по hash контента),
  - `backend/src/system/simple_cache.rs` (analysis cache),
  - `backend/src/system/parser_coordinator.rs` (TreeCache/AstCache/DiskCache и т.п.).
- [ ] Отмечены любые I/O и обращения к диску/сети на путях completion/hover/signatureHelp.
- [ ] Понято, где сейчас хранится "истина" по тексту/версии документа (и где возможен mixed state).

## 2) Модель salsa DB (ключи, inputs, queries)

- [ ] Введен стабильный `FileId` (interning) вместо `Url/String` как ключ большинства queries.
- [ ] Inputs (минимум):
  - [ ] `file_text(FileId) -> Arc<String>` или `Arc<str>`
  - [ ] `file_version(FileId) -> i32` (LSP version)
  - [ ] `deps_id() -> DepsSnapshotId` (стабильный fingerprint/идентификатор deps snapshot)
  - [ ] (опционально) `settings_id() -> SettingsId` (режимы strict/fast и прочие настройки, влияющие на семантику)
- [ ] Queries (минимум для LSP M2):
  - [ ] `line_index(FileId) -> LineIndex` (UTF-16 <-> byte)
  - [ ] `parse_result(FileId) -> ParseResult` (AST + errors)
  - [ ] `ir(FileId) -> SemanticProgram`
  - [ ] `completion_ctx(FileId, pos) -> ...`
  - [ ] `hover_ctx(FileId, pos) -> ...`
  - [ ] `signature_help_ctx(FileId, pos) -> ...`
- [ ] Все семантические queries зависят от `deps_id` (или inputs, из которых он вычисляется), чтобы исключить mixed deps.
- [ ] Все результаты детерминированы (стабильный порядок, стабильный `sortText`, отсутствие зависимости от `HashMap`-итерации).

## 3) Чистота queries (сайд-эффекты, скрытая мутабельность, снапшоты)

- [ ] Внутри queries нет I/O; любые данные с диска/из сети превращаются в inputs (через `DepsSnapshot`/fingerprint).
- [ ] Нет "скрытых" глобальных mutable кэшей, которые читаются/пишутся внутри queries.
- [ ] Любые кэши либо:
  - [ ] становятся salsa queries (и тогда инвалидация автоматически по deps), либо
  - [ ] живут внутри иммутабельного `DepsSnapshot` как `Arc<...>` и подаются в queries через inputs.
- [ ] Парсер/резолвер не нарушает semantics снапшота (например, не шарит общий mutable state между ревизиями).

## 4) Потоки, отмена, отсутствие тяжелой работы в hot path

- [ ] `didOpen/didChange` изменяют только inputs (текст/версию) и не делают тяжелого анализа синхронно.
- [ ] `deps_update` атомарно меняет deps inputs (и только после полной сборки нового снапшота).
- [ ] LSP handlers работают на `db.snapshot()` (или эквиваленте) и не держат write-lock на время вычислений.
- [ ] Тяжелые вычисления выполняются вне async hot path (например, через `spawn_blocking`/выделенный pool).
- [ ] Отмена `$/cancelRequest` реализована на уровне задач:
  - [ ] запрос не блокирует event loop,
  - [ ] результаты для отмененного/устаревшего запроса не публикуются.

## 5) DepsSnapshot: стабильный id и обновление зависимостей

- [ ] Определен состав `DepsSnapshot` (repo/resolver/index/platform docs/config/settings и т.п.).
- [ ] Есть стабильный `DepsSnapshotId` (fingerprint), который:
  - [ ] меняется только при реальном изменении deps,
  - [ ] логируется и может быть использован для disk cache/диагностики.
- [ ] Обновление deps делается через: build new snapshot -> atomic swap -> bump revision.
  - Ориентир по текущим fingerprints: `backend/src/system/system_coordinator/config_loader.rs`.
- [ ] (Желательно) Есть warmup для открытых документов после deps update, чтобы избежать latency всплесков на первом hover/completion.

## 6) Тесты и критерии готовности (без этого "прямой" переход рискован)

- [ ] Есть тестовый контур для LSP инкрементальности:
  - [ ] серия `didChange` не дает mixed state,
  - [ ] результаты соответствуют последнему `file_version`.
- [ ] Есть тесты на смену deps во время редактирования (no mixed deps).
- [ ] Есть тесты на детерминизм (стабильный порядок результатов/`sortText`).
- [ ] Есть тесты/проверки отмены (не зависаем, не публикуем устаревшее).
- [ ] Есть метрики/логи по latency и причинам fallback, чтобы ловить регрессии при большой миграции.

## 7) Красные флаги (если есть - лучше идти через промежуточную архитектуру)

- [ ] Много текущей логики завязано на mutable кэши с неявной инвалидцией (сложно сделать snapshot-safe).
- [ ] Нет быстрых регрессионных тестов для completion/hover/signatureHelp.
- [ ] Нужны быстрые улучшения M2 с минимальным риском массового рефакторинга.

