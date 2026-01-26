# Change: add-mcp-bsl-agent-type-tools

## Why
Сейчас LLM, работающая только через MCP (stdio), не может удобно ориентироваться в типах платформы/конфигурации:
в `bsl-agent` отсутствуют MCP tools для получения списка типов и детальных метаданных типа (реквизиты, табличные части).

При этом в `bsl-agent` уже есть parity HTTP API (`/api/mcp/types|search|metrics`), который возвращает `AnalysisResultDto/TypeDto`.
Это полезно для embedded UI, но не решает задачу для MCP-клиентов без доступа к HTTP.

## What Changes
- Добавить MCP tools (stdio) для discovery типов:
  - список типов (пагинация + фильтры),
  - поиск типов по строке,
  - получение деталей конкретного типа (включая реквизиты и табличные части).
- Зафиксировать требования к предсказуемости:
  - детерминированный порядок результатов,
  - лимиты на размер выдачи,
  - понятные ошибки при неготовой сессии/неизвестном типе/некорректных параметрах.
- Обновить on-demand справку `mcp_help` примерами вызовов новых tools.

## Impact
- Спецификации:
  - delta к `openspec/specs/mcp-bsl-agent/spec.md` (новые MCP tools для types discovery).
- Код (в apply-стадии):
  - `bsl-agent/src/server/mod.rs` (новые tools + интеграция с JobManager),
  - `bsl-agent/src/server/types.rs` (params),
  - `bsl-agent/src/session/*` (вызовы parity/lookup для типов),
  - тесты `bsl-agent` (см. `tasks.md`).

## Assumptions (to be confirmed)
- Параметр `session_id` в MCP tools обязательный (как и в остальных semantic tools); server остаётся single-session.
- Для списка/поиска по умолчанию НЕ включаем тяжёлые поля (например, полный список методов), но даём режимы `view` (в т.ч. `names_only`) и отдельный tool для деталей.
- “Детали типа” должны включать как минимум:
  - `TypeDto.properties` (реквизиты/свойства),
  - `TypeDto.tabularSections` (табличные части с атрибутами).
- Для списка/поиска нужен фильтр по источнику (`source=platform|configuration`).

## Non-goals
- Изменение существующего parity HTTP API `/api/mcp/*`.
- Write-операции над workspace/конфигурацией (только read-only discovery).
- Попытка сделать “универсальный” браузер метаданных: только то, что доступно через типовую модель (`TypeDto`).
