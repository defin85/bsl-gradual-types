## Контекст
В проекте уже существует Web UI (`bsl-web-server` + `frontend`), который работает через `/api/*` и раздаёт SPA из `target/site`.

Параллельно появился MCP‑сервер `bsl-agent` (stdio), который работает в модели **per-agent** и имеет собственные сущности:
- `workspace_session` (roots + overlays + readiness + missing_inputs)
- `jobs` (async, прогресс, результаты, persist/resume)
- локальный кэш (`BSL_CACHE_DIR`, strict fingerprint и т.п.)

Нужно дать разработчику browser‑дашборд, чтобы глазами оценивать состояние MCP и быстро диагностировать проблемы загрузки платформенной документации/конфигурации, не завися от LLM.

## Цели
- **Единый UI‑артефакт:** использовать существующий SPA (`frontend → target/site`) как единственную точку ответственности UI.
- **Read-only:** MCP‑дашборд не модифицирует состояние; `bsl-agent` не предоставляет write‑HTTP API.
- **Per-agent:** каждый MCP‑процесс может иметь свой UI порт, без конфликтов.
- **Безопасность по умолчанию:** UI доступен только локально (`127.0.0.1`), выключен по умолчанию.
- **Детект режима:** UI должен уметь отличать `bsl-web-server` от `bsl-agent` и корректно деградировать.
- **Диагностика “docs/config loaded”:** UI должен явно показывать platform/config fingerprints и счётчики типов, чтобы ловить случаи “platform docs не распарсились”.

## Не цели
- Реализация общего MCP daemon для нескольких клиентов.
- Полноценный MCP Inspector (tools/resources/prompts runner) — для этого есть внешний `@modelcontextprotocol/inspector`.

## Предлагаемое решение (высокий уровень)
### 1) HTTP UI в `bsl-agent`
`bsl-agent` (stdio MCP) получает опциональный HTTP listener:
- раздаёт статику SPA из директории (по умолчанию `target/site`);
- предоставляет read-only API `/api/mcp/*` для отображения MCP‑состояния.

HTTP UI запускается только при явном включении через env/CLI (конкретные имена фиксируются в спеках).

### 2) Унификация SPA
SPA модифицируется так, чтобы:
- перед стартовой загрузкой “web mode” данных выполнить capability detection через `/api/mcp/status`;
- если backend — `bsl-agent`, переключиться в режим “MCP Dashboard” и не дергать `/api/snapshot/*` и другие web-server операции;
- если backend — `bsl-web-server`, сохранить текущее поведение.

### 3) Совместимость `bsl-web-server`
Чтобы SPA мог всегда выполнять capability detection без “шумных” 404/ошибок, `bsl-web-server` добавляет read-only эндпоинт `/api/mcp/status` с ответом `supported=false` и `mode=web-server`.

## API: минимальный набор (read-only)
### `GET /api/mcp/status`
Назначение: capability detection и базовая диагностика.

Идея ответа:
- `backend`: `"bsl-agent"` | `"bsl-web-server"`
- `supported`: boolean (для `bsl-web-server` false)
- `read_only`: boolean (для MCP режима true)
- `instance_id`: строка (уникальный ID процесса/инстанса)
- `cache_dir`: строка (instance state dir)
- `ui_url`: строка (если порт auto)
- `version/build/git`: по аналогии с `/api/version`

### `GET /api/mcp/sessions`
Read-only сводка по активным/восстановленным сессиям: `session_id`, roots, `analysis_revision`, `ready`, `missing_inputs[]`, `startup_job_id`.

### `GET /api/mcp/jobs` и `GET /api/mcp/jobs/<job_id>`
Read-only доступ к состоянию job’ов: `state`, `phase`, `progress.percent`, `error`.

### `GET /api/mcp/deps/meta`
Read-only мета по загруженным deps (platform/config/index):
- platform_version, platform_fingerprint, config_fingerprint
- счётчики типов (platform/configuration/etc), чтобы явно видеть “platform docs loaded?”

Примечание: в web-server уже есть `GET /api/snapshot/meta` — для унификации можно сделать `/api/mcp/deps/meta` “тонкой обёрткой” в web-server режиме или возвращать `supported=false`.

## Конфигурация запуска UI (per-agent)
Требования:
- bind только `127.0.0.1` (запрет `0.0.0.0`).
- разрешить `:0` (автопорт) для исключения конфликтов между агентами.
- путь к статике должен быть конфигурируемым (по умолчанию `target/site`).
- при включённом UI сервер логирует `ui_url` в stderr (WSL: открыть с Windows через `http://localhost:<port>`).

## Безопасность и риски
### Риск: открытие UI наружу
Митигируем:
- bind только `127.0.0.1`;
- UI выключен по умолчанию;
- только `GET` эндпоинты под `/api/mcp/*`.

### Риск: “единый UI” вызывает write endpoints
Митигируем:
- capability detection + режим MCP;
- в MCP‑режиме UI скрывает кнопки “Reload deps” и любые write‑действия.

### Риск: конфликт кэша LSP ↔ MCP
Митигируем (на уровне реализации; в UI показываем диагностику):
- instance state (sessions/jobs/overlays) — per-agent;
- shared deps cache — content-addressed по fingerprint/deps_id + lockfiles + atomic publish;
- UI показывает fingerprints/идентификаторы, чтобы можно было понять, что именно загружено.

## Тестирование (на стадии реализации)
- Интеграционные HTTP тесты `bsl-agent`: `GET /` отдаёт `index.html`, `GET /api/mcp/status` → 200 и корректный JSON.
- Негативные тесты: `POST /api/mcp/status` и другие write методы возвращают 405/404.
- Smoke-test: UI корректно переключается в MCP режим при наличии `/api/mcp/status`.

