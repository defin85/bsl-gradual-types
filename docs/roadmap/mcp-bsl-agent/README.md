# BSL Agent (MCP): семантический контекст проекта для LLM

**Статус:** 🔴 ПЛАН  
**Цель:** локальный MCP-сервер (stdio), который читает workspace и отдаёт LLM “IDE‑grade” семантический контекст по проекту: диагностики, типы, переходы по символам и компактный `context_pack` в рамках бюджета.

Этот документ фиксирует решения по архитектуре и API для `bsl-agent` (MCP) и отвечает на открытые вопросы (локально vs remote, reuse LSP и т.д.).

## Документы

- Архитектура: `docs/roadmap/mcp-bsl-agent/architecture.md`
- MCP API (tools/resources/prompts): `docs/roadmap/mcp-bsl-agent/api.md`
- План реализации (phased): `docs/roadmap/mcp-bsl-agent/implementation-plan.md`
