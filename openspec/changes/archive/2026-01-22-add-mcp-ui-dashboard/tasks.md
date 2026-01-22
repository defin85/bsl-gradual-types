## 1. Проектирование и контракт
- [x] 1.1 Уточнить минимальный набор read-only данных для UI (sessions/jobs/deps/meta/missing_inputs) и формат `/api/mcp/*`.
- [x] 1.2 Описать режимы UI (web-server vs mcp-agent) и правила capability detection.
- [x] 1.3 Зафиксировать конфигурацию запуска HTTP UI (env/CLI) и требования безопасности (localhost-only, без write endpoints).

## 2. Реализация: `bsl-agent` HTTP UI (read-only)
- [x] 2.1 Добавить опциональный HTTP сервер в `bsl-agent` (включается флагом/env), не влияющий на stdio MCP.
- [x] 2.2 Раздавать существующую статику SPA из `target/site` (или конфигurable path), с fallback на `index.html`.
- [x] 2.3 Реализовать `/api/mcp/status` и остальные `/api/mcp/*` эндпоинты (только `GET`, без POST).
- [x] 2.4 Гарантировать, что bind только `127.0.0.1` (запрет `0.0.0.0`), и корректная ошибка/лог при нарушении.

## 3. Реализация: унификация с `bsl-web-server`
- [x] 3.1 Добавить в `bsl-web-server` read-only эндпоинт `/api/mcp/status` (возвращает `supported=false`/`mode=web-server`) для унификации UI.
- [x] 3.2 Проверить, что Web UI продолжает работать без изменений существующих API контрактов.

## 4. Реализация: `frontend` (единая точка ответственности UI)
- [x] 4.1 Добавить capability detection на старте: определить backend mode по `/api/mcp/status` (или эквиваленту).
- [x] 4.2 В MCP-режиме: показывать read-only “MCP Dashboard”, не вызывать `snapshot/reload` и не отображать “reload” кнопки/действия.
- [x] 4.3 В web-server режиме: сохранить текущее поведение (snapshot/meta, metrics, types, reload).

## 5. Тестирование и валидация
- [x] 5.1 Добавить интеграционные тесты (HTTP): `bsl-agent` отдаёт `/api/mcp/status` и корректно раздаёт `index.html`.
- [x] 5.2 Добавить тест “read-only”: все `/api/mcp/*` не принимают write методы (POST/PUT/PATCH/DELETE → 405/404).
- [x] 5.3 Проверить совместимость в WSL: ссылка `http://localhost:<port>` открывается с Windows-хоста.

## 6. Документация
- [x] 6.1 Обновить `docs/roadmap/mcp-bsl-agent/api.md` (описать `/api/mcp/*` и режимы UI).
- [x] 6.2 Добавить краткий гайд запуска UI для MCP (env vars, примеры) в релевантный раздел документации.
