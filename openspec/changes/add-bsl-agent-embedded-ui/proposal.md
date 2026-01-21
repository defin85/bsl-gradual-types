# Change: add-bsl-agent-embedded-ui

## Зачем
Сейчас HTTP UI `bsl-agent` может раздавать SPA только из внешней директории (`BSL_AGENT_HTTP_STATIC_DIR`, по умолчанию `target/site`). Если передать `bsl-agent` как MCP сервер другому человеку без артефакта фронтенда, UI поднять нельзя: API `/api/mcp/*` работает, но браузерный SPA отсутствует.

Нужно, чтобы `bsl-agent` мог поднимать UI “из коробки” без внешних файлов — за счёт встраивания (embed) собранного SPA внутрь бинарника.

## Что меняется
- `bsl-agent` начинает встраивать собранный SPA (артефакт `frontend → target/site`) внутрь бинарника и раздавать его из памяти по HTTP UI.
- `BSL_AGENT_HTTP_STATIC_DIR` сохраняется и имеет приоритет над embedded: если переменная задана и директория доступна, раздаётся статика с диска (удобно для разработки).
- Сборка `bsl-agent` должна падать с понятной ошибкой, если SPA не собран (нет `target/site`), чтобы избежать “пустого” UI.

## Влияние
- Затронутая спецификация: `openspec/specs/mcp-bsl-agent/spec.md` (добавляются требования про embedded SPA и приоритет `BSL_AGENT_HTTP_STATIC_DIR`).
- Затронутый код (на стадии реализации): `bsl-agent` (HTTP UI static serving + build-time check).

## Не цели (Non-goals)
- Не встраиваем SPA в `bsl-web-server`.
- Не меняем контракт `/api/mcp/*` и read-only модель.

