## ADDED Requirements

### Requirement: LLM-friendly форматы параметров (абсолютные пути, multi-root, deterministic)
Система SHALL поддерживать LLM-friendly формы параметров для ссылок на файлы/документы и scope, чтобы LLM мог работать с multi-root без ручного обращения к `root_id` и относительным путям.

Для всех tool-ов, где входные параметры содержат ссылку на документ/файл (`DocumentRef`/`FileRef`) и/или `WorkspaceScope::File`, система SHALL принимать **абсолютный путь** как альтернативу каноническому формату с `root_id` и относительным `path`.

Система SHALL разрешать абсолютный путь в `(root_id, relative_path)` детерминированно через **longest-prefix match** по каноническим roots текущей сессии. Если путь не принадлежит ни одному root — система SHALL возвращать `INVALID_PARAMS`. Если резолвинг неоднозначен — система SHALL возвращать `INVALID_PARAMS`.

#### Scenario: Multi-root — абсолютный путь однозначно резолвится в правильный root
- **GIVEN** сессия открыта с `roots=["/ws/config","/ws/ext1","/ws/ext2"]`
- **WHEN** клиент передаёт абсолютный путь `/ws/ext1/src/CommonModules/Foo/Module.bsl` в параметре, где ожидается файл/документ
- **THEN** сервер выбирает root `/ws/ext1` (longest-prefix) и использует `relative_path="src/CommonModules/Foo/Module.bsl"`

#### Scenario: Абсолютный путь вне roots отклоняется
- **GIVEN** сессия открыта с некоторыми `roots[]`
- **WHEN** клиент передаёт абсолютный путь, не принадлежащий ни одному root
- **THEN** сервер возвращает `INVALID_PARAMS` и не читает файл

### Requirement: Автоматизация `platform_version` при `configuration_path` (fail-fast при невозможности)
Если в `workspace_open` задан `configuration_path`, но не задан `platform_version`, система SHALL попытаться автоматически определить `platform_version` из дампа конфигурации.

Если определить `platform_version` невозможно, система SHALL возвращать `INVALID_PARAMS` (fail-fast) с понятным сообщением о необходимости указать `platform_version`.

#### Scenario: `platform_version` определяется из дампа конфигурации
- **GIVEN** клиент вызывает `workspace_open` с `configuration_path`, но без `platform_version`
- **AND** в дампе конфигурации присутствует информация, достаточная для определения версии платформы
- **WHEN** сервер обрабатывает `workspace_open`
- **THEN** сервер использует определённую `platform_version` для startup и не требует ручного подбора версии клиентом

#### Scenario: `platform_version` не удаётся определить — запрос отклоняется
- **GIVEN** клиент вызывает `workspace_open` с `configuration_path`, но без `platform_version`
- **AND** в дампе конфигурации отсутствует информация, достаточная для определения версии платформы
- **WHEN** сервер обрабатывает `workspace_open`
- **THEN** сервер возвращает `INVALID_PARAMS` и сообщение о необходимости указать `platform_version`

### Requirement: `mode="default"` не создаёт warning
Система SHALL трактовать `mode="default"` в `workspace_open` как режим по умолчанию (эквивалент отсутствию `mode`) и SHALL NOT добавлять warning `unknown mode: default`.

#### Scenario: `mode="default"` не создаёт warning
- **GIVEN** клиент вызывает `workspace_open` с `mode="default"`
- **WHEN** сервер обрабатывает запрос
- **THEN** ответ `workspace_open` содержит пустой `warnings[]` (если нет других причин для предупреждений)

### Requirement: `progress.percent=100` только для terminal job
Система SHALL обеспечивать, что `job_status.progress.percent` принимает значение `100` только в terminal-состоянии job (`succeeded|failed|canceled|aborted_by_restart`).

#### Scenario: Running job не может иметь `progress.percent=100`
- **GIVEN** job находится в состоянии `running`
- **WHEN** клиент опрашивает `job_status`
- **THEN** `progress.percent` находится в диапазоне `0..99`

