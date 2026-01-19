## ADDED Requirements

### Requirement: MCP server `bsl-agent` (stdio) для семантики проекта
Система SHALL предоставлять локальный MCP‑сервер `bsl-agent` по stdio, доступный для MCP‑клиентов (IDE/CLI), и реализующий базовый lifecycle сессии.

#### Scenario: Открытие и закрытие сессии
- **GIVEN** локальный workspace путь добавлен в roots
- **WHEN** клиент вызывает `workspace_open`, затем `workspace_status`, затем `workspace_close`
- **THEN** сервер возвращает `session_id` и корректный статус готовности, и освобождает ресурсы при закрытии

### Requirement: Read-only для workspace и sandbox чтения файлов
Система SHALL не модифицировать файлы проекта в roots (никаких write/patch) и SHALL ограничивать доступ к FS набором roots (sandbox), предотвращая path traversal и чтение вне roots.

Примечание: запись допускается только в директорию локального кэша (вне roots) и только для производных артефактов.

#### Scenario: Запрос к файлу вне roots запрещён
- **GIVEN** сессия открыта с roots
- **WHEN** клиент пытается запросить документ по пути вне roots
- **THEN** сервер возвращает ошибку `INVALID_PARAMS` (или эквивалентную) и не читает файл

### Requirement: Локальный кэш для платформы/конфигурации/AST (DiskCache)
Система SHALL поддерживать локальный дисковый кэш для тяжёлых артефактов (platform docs/config metadata/AST) и SHALL управляться переменными окружения, совместимыми с существующим кэшем проекта (`BSL_CACHE_DIR`, `BSL_CACHE_DISABLE`, `BSL_CACHE_STRICT_FINGERPRINT` и др.).

`platform_docs_archive` SHALL принимать как путь к файлу документации (например, `.hbk`/архив синтаксис‑помощника), так и путь к директории с распакованной документацией.

#### Scenario: Включённый кэш создаёт артефакты в `BSL_CACHE_DIR`
- **GIVEN** `BSL_CACHE_DIR` указывает на пустую временную директорию и `BSL_CACHE_DISABLE` не задан
- **WHEN** клиент открывает сессию с `platform_docs_archive` и/или `configuration_path` и дожидается готовности через `workspace_status`
- **THEN** в `BSL_CACHE_DIR` появляются артефакты кэша (manifest + payload) для соответствующих входных данных

#### Scenario: Отключённый кэш не читает и не пишет артефакты
- **GIVEN** `BSL_CACHE_DIR` указывает на пустую временную директорию и `BSL_CACHE_DISABLE=1`
- **WHEN** клиент открывает сессию с `platform_docs_archive` и/или `configuration_path`
- **THEN** система не создаёт и не использует артефакты дискового кэша (каталог остаётся пустым, либо без новых записей)

### Requirement: Совместное использование кэша несколькими процессами (LSP + MCP)
Система SHALL безопасно разделять один `DiskCache` между несколькими процессами (например, LSP сервером и `bsl-agent`), предотвращая повреждение артефактов при одновременной сборке/записи.

#### Scenario: Одновременная сборка одного ключа не повреждает кэш
- **GIVEN** два процесса используют один и тот же `BSL_CACHE_DIR` и один и тот же ключ кэша
- **WHEN** оба процесса одновременно вызывают операцию “получить или построить” (get-or-build) для этого ключа
- **THEN** артефакт в кэше остаётся валидным, и второй процесс получает либо готовый результат из кэша, либо корректно ждёт завершения записи

#### Scenario: Cleanup/eviction не удаляет entry под активным lock
- **GIVEN** один процесс удерживает per‑key `.lock` на cache entry
- **WHEN** другой процесс запускает cleanup/eviction (TTL/size) для дискового кэша
- **THEN** entry не удаляется, пока lock удерживается (entry пропускается), и взаимное исключение не нарушается

### Requirement: Unsaved buffers через ad-hoc snapshot и session overlay
Система SHALL поддерживать unsaved тексты как (1) ad-hoc snapshot для одного вызова и (2) session overlay для `scope=hot`.

#### Scenario: Overlay меняет ревизию анализа
- **GIVEN** сессия открыта и `analysis_revision = N`
- **WHEN** клиент вызывает `workspace_documents_set` с `FileRef.text`
- **THEN** сервер возвращает `analysis_revision = N+1`, и последующие семантические ответы ссылаются на новую ревизию

### Requirement: Семантические tools (MVP)
Система SHALL предоставлять MCP tools (MVP): `bsl_diagnostics`, `bsl_symbol_search`, `bsl_type_at_position`, `bsl_members`, `bsl_definition`, `bsl_references`.

#### Scenario: Получение диагностики по проекту
- **GIVEN** сессия открыта на workspace с файлами `*.bsl`
- **WHEN** клиент вызывает `bsl_diagnostics` со `scope=project`
- **THEN** сервер возвращает список diagnostics с `analysis_revision` и стабильными `diagnostic_id` в рамках ревизии

### Requirement: Детерминизм выдачи и стабильные идентификаторы
Система SHALL обеспечивать детерминизм: одинаковый snapshot документов → одинаковые ответы (порядок и ID), а идентификаторы SHALL быть стабильны внутри одного `analysis_revision`.

#### Scenario: Повторный вызов возвращает те же результаты
- **GIVEN** snapshot документов не изменился
- **WHEN** клиент дважды вызывает один и тот же tool с одинаковыми параметрами
- **THEN** сервер возвращает одинаковые результаты и порядок, и те же ID

### Requirement: `context_pack` с жёстким бюджетом и дозапросом
Система SHALL предоставлять tool `context_pack`, который возвращает LLM‑готовый текстовый пакет и `items[]` в рамках `budget_chars`, и tool `context_expand` для расширения конкретного item.

#### Scenario: Превышение бюджета приводит к явной обрезке
- **GIVEN** `budget_chars` задан как жёсткий лимит
- **WHEN** данных для пакета больше, чем бюджет
- **THEN** сервер возвращает `truncated=true` и текст строго в рамках `budget_chars`

### Requirement: Интеграционные тесты MCP (stdio)
Система SHALL иметь интеграционные тесты, проверяющие MCP контракт по stdio (initialize/tools/list/tools/call) и стабильность `context_pack`.

#### Scenario: Интеграционный тест поднимает процесс и вызывает tools
- **GIVEN** тест запускает `bsl-agent` как процесс
- **WHEN** тест выполняет `initialize`, `tools/list` и несколько `tools/call`
- **THEN** ответы валидны, а результаты `context_pack` воспроизводимы (golden/snapshot)
