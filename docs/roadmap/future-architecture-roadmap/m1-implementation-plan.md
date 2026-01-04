# План реализации M1: Контракты и границы (server-only ядро)

**Статус:** 🔴 ПЛАН  
**Цель:** отделить контрактные типы/DTO от inference‑ядра так, чтобы клиентские бинарники не содержали `AnalysisEngine/TypeResolver`.

---

## Область работ

- Выделить “контракты” (DTO/ошибки/протокол) в отдельный crate (например, `shared-contracts`).
- Перенести inference‑ядро в server-only crate (например, `bsl-inference-core`).
- Добавить gateway абстракцию: `TypeSystemGateway` → `LocalGateway` (server) / `RemoteGateway` (клиенты).
- Зафиксировать контракты для sync и LLM‑ориентированных запросов (`config.skeleton`, `ArtifactRef`, `context.pack`).

---

## Пошаговый план

### Шаг 1: Инвентаризация зависимостей
- Найти публичные типы, которые нужны клиентам: DTO, IR/позиции, ошибки, идентификаторы workspace.
- Зафиксировать boundary: что точно остаётся server-only (inference, кэши, индексация, storage).
- Зафиксировать минимальные контракты v1 (то, что нужно агентам):
  - `SyncMode = hot_set|progressive|full`
  - `WorkspaceId`, `WorkspaceOpenRequest/Response`, `WorkspaceApplyChangesRequest/Response`
  - `ArtifactRef` + `missingInputs[]` (kind/path/reason/priority)
  - `ContextPackRequest/Response` (крупнозернистый формат под LLM)
  - `schemaVersion` для сериализуемых payload’ов (bundle/снимки/индексы)

### Шаг 2: Вынести контракты
- Создать crate для контрактов.
- Перенести DTO/ошибки/версии схем.
- Добавить versioning/handshake:
  - `api_version` (`/v1`),
  - `supportedProtocolVersions[]` → выбранный `protocolVersion` + `capabilities`.

### Шаг 3: Вынести inference‑ядро
- Создать server-only crate.
- Переместить `AnalysisEngine`/`TypeResolver`/репозитории/индексаторы (или обвязку вокруг них).

### Шаг 4: Подключить gateway
- Определить `TypeSystemGateway` интерфейс на стороне клиентов.
- Реализовать `LocalGateway` внутри сервера как thin wrapper.
- Подготовить `RemoteGateway` интерфейс (без реализации транспорта, это в M2):
  - методы под `context.pack` и batch запросы.

---

## Критерии завершения (DoD)

- Клиентские targets (`cli`, `lsp_server`/IDE-адаптеры) не содержат inference‑ядра.
- `cargo build --workspace` проходит.
- Контракты для `context.pack`/`missingInputs`/`SyncMode` определены и версионируются.
