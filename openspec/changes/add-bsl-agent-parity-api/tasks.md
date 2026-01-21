## 1. Контракт и спецификация
- [ ] 1.1 Зафиксировать в delta-spec новые parity эндпоинты `/api/mcp/types|search|metrics` и их DTO.
- [ ] 1.2 Зафиксировать правило выбора сессии: без `sessionId` работает только при ровно одной ready; иначе `INVALID_PARAMS`.
- [ ] 1.3 Зафиксировать поведение SPA в MCP-режиме: те же вкладки, read-only, отдельное сообщение при 0/>1 ready.

## 2. Реализация: bsl-agent parity HTTP API
- [ ] 2.1 Добавить роуты `/api/mcp/types`, `/api/mcp/search`, `/api/mcp/metrics` в HTTP UI `bsl-agent`.
- [ ] 2.2 Реализовать общий helper выбора сессии (sessionId или ровно одна ready), общий для всех parity эндпоинтов.
- [ ] 2.3 Переиспользовать существующую логику формирования `AnalysisResultDto` и `MetricsDto` без дублирования (предпочтительно).

## 3. Реализация: frontend (единый SPA)
- [ ] 3.1 В MCP-режиме переключить data-source для types/search/snapshot meta на `/api/mcp/*`.
- [ ] 3.2 При 0/>1 ready сессии: показывать инструкцию и не делать parity вызовов без `sessionId`.
- [ ] 3.3 Сохранить текущий MCP-дашборд (sessions/jobs) как “MCP особенности” в UI.

## 4. Тестирование и валидация
- [ ] 4.1 Интеграционный тест: parity эндпоинты возвращают 200 при ровно одной ready сессии.
- [ ] 4.2 Интеграционный тест: parity эндпоинты возвращают 400 при 0 ready и при >1 ready (без `sessionId`).
- [ ] 4.3 Интеграционный тест: parity эндпоинты возвращают 400 при `sessionId` несуществующей/не-ready сессии.

## 5. Документация
- [ ] 5.1 Обновить `docs/roadmap/mcp-bsl-agent/api.md`: добавить `/api/mcp/types|search|metrics` и правила выбора сессии.
- [ ] 5.2 Обновить `bsl-agent/README.md`: описать ограничения parity UI (работает только при ровно одной ready сессии).

