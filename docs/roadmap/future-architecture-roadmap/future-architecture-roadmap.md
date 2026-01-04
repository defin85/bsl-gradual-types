# Roadmap: Future Architecture (vNext) — Semantic Server + `bsl-agent`

**Статус:** 🔴 ПЛАН  
**Приоритет:** HIGH  
**Цель:** перейти к server-first семантической платформе (SaaS + on-prem/offline) для IDE/LLM‑агентов: типы/диагностика/навигация/контекст‑пакеты, без поставки inference‑ядра на клиент.

Связанные документы:
- `docs/roadmap/ADR-001-server-vs-client-utf8.md`
- `docs/roadmap/server-first-offline-roadmap.md`
- `docs/roadmap/future-architecture/architecture-vnext.md`
- `.claude/rules/architecture.md` (текущее состояние и слои)

---

## Контекст (почему нужен этот roadmap)

- Сейчас LSP/Web/CLI живут локально и содержат inference‑ядро, что конфликтует с ADR-001 (server-first, защита IP, usage-based).
- Большие 1С‑конфигурации требуют stateful workspace‑сессий, кэширования и честной “частичной семантики”.
- Нужно поддержать две поставки:
  - **SaaS:** код/метаданные могут уходить на сервер.
  - **On-prem/offline:** код не покидает периметр (тот же серверный API, но локальный endpoint).

---

## Definition (что считаем “готово”)

**Обязательно:**
- Серверный компонент (Semantic Server) — единственное место inference/semantic graph.
- Клиентский компонент `bsl-agent` — thin read-only MCP server (stdio) + sync client.
- Workspace‑сессии: `open/applyChanges/status/close`.
- Запросы: `diagnostics`, `typeAtPosition`, `members`, `definition`, `references`, `context.pack`.
- Версионирование контрактов и протоколов (без ломания клиентов).

**Опционально (как ускоритель):**
- “Data-layer bundle” (нормализованные `RawTypeData`/индексы) вместо загрузки “сырья” XML/HTML, особенно для больших конфигураций.

---

## Принятые решения v1 (для соответствия `architecture-vnext.md`)

1) **IDE по умолчанию = `syncMode=progressive`** (а не `full`).
2) **`config.skeleton` обязателен** для `hot_set/progressive`:
   - минимальный индекс метаданных + отображение “объект → пути модулей/semantic xml paths”.
3) **LLM‑ориентированный API не должен быть “chatty”**:
   - основной endpoint — `context.pack` (крупнозернистый),
   - batch запросы предпочтительнее одиночных (`*.batch`),
   - ответы всегда “честные”: `completeness=full|partial` + `missingInputs[]`.
4) **`workspace.applyChanges` — единый транспорт**:
   - инкрементальные текстовые изменения,
   - догрузка недостающих артефактов (full-file blobs) по `missingInputs[]`.
5) **Data-layer bundle (`config.bundle`)** — optional:
   - SaaS: включается политикой/фичефлагом (валидация/квоты/изоляция),
   - on-prem/offline: можно включать по умолчанию ради скорости.
6) **Platform Types DB в SaaS**:
   - curated prebuilt по умолчанию,
   - `platform.upload` только под политикой (admin/tenant scoped), сборка асинхронно, хранение tenant-scoped.

---

## Milestones

### M1: Контракты и границы (server-only ядро)
**Цель:** клиенты перестают линковать inference‑ядро; выделяем контракты/DTO.

### M2: Semantic Server API MVP
**Цель:** минимальный серверный API и `RemoteGateway` для клиентов.

### M3: Workspace sync + caching
**Цель:** манифест/фингерпринт (Merkle), дедуп, `config.skeleton`, инкрементальные изменения, честная “partial semantics” + `missingInputs[]`.

### M4: `bsl-agent` (MCP/CLI/IDE)
**Цель:** единая точка интеграции (stdio MCP) для Codex/Claude/VSCode/Cursor; read-only.

### M5: Platform Types DB (prebuilt + пополнение)
**Цель:** сервер хранит платформенные типы по версиям; клиент может пополнять (upload/ensure) при необходимости.

---

## Риски

- Рост latency для IDE при удалённом inference без кэширования/батчинга.
- Сложность синхронизации воркспейса (особенно конфигурации >1GB).
- Trust model для “data-layer bundle” в SaaS (валидация/квоты/изоляция).
- UX деградирует, если `context.pack` не будет укладываться в “несколько секунд” и/или потребуется много round-trip на один запрос агента.
