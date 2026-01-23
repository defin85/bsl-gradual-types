# Design: add-bsl-agent-parity-api

## Цели дизайна
- Дать UI в MCP режиме “тот же UX”, что и `bsl-web-server`, но на данных `bsl-agent`.
- Оставаться строго read-only и localhost-only (как уже зафиксировано предыдущими change).
- Избежать неоднозначности при нескольких сессиях: “работает только при ровно одной ready”.

## Single-session политика MCP (одна workspace-сессия на процесс)
Чтобы parity UI (и в целом MCP-клиенты) не сталкивались с неоднозначностью выбора сессии, `bsl-agent` SHALL ограничивать количество одновременно существующих workspace-сессий в одном процессе до 1.

Это означает:
- Если в процессе уже существует активная сессия, повторный `workspace_open`:
  - SHALL быть идемпотентным и возвращать уже открытую сессию, если параметры вызова совпадают (roots + входы platform/config).
  - SHALL отклоняться как `INVALID_PARAMS` (HTTP 400 или эквивалент), если параметры отличаются, и предлагать закрыть текущую сессию (`workspace_close`) перед открытием новой.

Примечание: parity API всё равно сохраняет проверку “ровно одна ready” как защиту от неконсистентного состояния, но при корректной реализации single-session эта ситуация не должна возникать.

## Parity HTTP API (bsl-agent)
### Роуты
Эндпоинты располагаются в namespace `/api/mcp/*` и возвращают те же DTO, что и web-server:
- `GET /api/mcp/types[?page=&limit=&category=&certainty_level=&flow_sensitive_only=&sessionId=...]` → `AnalysisResultDto`
- `GET /api/mcp/search?q=...[&sessionId=...]` → `AnalysisResultDto`
- `GET /api/mcp/metrics[?sessionId=...]` → `MetricsDto` (опционально для UI; возможно “прокладка” поверх метрик из `AnalysisResultDto`)

Примечание: для snapshot/meta в MCP уже существует `GET /api/mcp/deps/meta[?sessionId=...]` → `SnapshotMetaDto`.

### Выбор сессии
Все parity эндпоинты используют одинаковую логику выбора workspace-сессии:

1) Если передан `sessionId`:
   - сервер MUST валидировать, что сессия существует;
   - сервер MUST требовать `ready=true` (иначе `INVALID_PARAMS` / HTTP 400: “workspace not ready”).

2) Если `sessionId` не передан:
   - сервер MUST выбрать ready-сессии (`ready=true`);
   - если ready-сессий ровно 1 → использовать её;
   - если ready-сессий 0 или >1 → вернуть `INVALID_PARAMS` / HTTP 400 с понятным текстом:
     - `no ready sessions` или `exactly one ready session is required`.

Это соответствует UX-требованию: “UI работает только когда ровно одна ready”.

## Изменения в SPA (frontend)
### Поведение режима MCP
- При `GET /api/mcp/status` с `mode=mcp-agent` SPA переключается в MCP-режим.
- В MCP-режиме SPA:
  - отображает те же вкладки (Dashboard/Карточки/Таблица/Граф), что и в web-server;
  - использует parity API (`/api/mcp/types`, `/api/mcp/search`, `/api/mcp/deps/meta`, при необходимости `/api/mcp/metrics`);
  - не вызывает mutating web-server API (`POST /api/snapshot/reload` и т.п.).

### Ошибка “нет ровно одной ready”
Если `GET /api/mcp/sessions` показывает, что ready-сессий 0 или >1:
- SPA показывает понятный экран/баннер с инструкцией:
  - “Оставьте ровно одну ready сессию” (и подсказка: закрыть лишние сессии / дождаться startup);
- SPA не делает запросы parity API без `sessionId`;
- MCP-дашборд (sessions/jobs) остаётся доступным для диагностики.

## Реализация (на стадии apply)
Предпочтение: переиспользовать существующую логику формирования `AnalysisResultDto` (как в web-server) без дублирования.
Варианты (выбрать минимально инвазивный):
1) Сделать нужные функции/типы доступными для `bsl-agent` через `bsl-backend` crate (если цепочка модулей допускает).
2) Вынести “web api → dto” логику в общий модуль/крейта workspace (если (1) приводит к нежелательной связности).

## Наблюдаемость и диагностика
- Сообщения об ошибках для `INVALID_PARAMS` должны быть стабильны и понятны (для UI и для логов).
- В логах `bsl-agent` фиксировать, когда parity эндпоинты отклоняются из-за отсутствия ровно одной ready сессии (на уровне debug/trace).
