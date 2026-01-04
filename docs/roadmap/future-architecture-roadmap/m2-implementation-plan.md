# План реализации M2: Semantic Server API MVP

**Статус:** 🔴 ПЛАН  
**Цель:** поднять минимальный серверный API и подключить к нему клиентов через `RemoteGateway`.

---

## Область работ

- Серверный транспорт (HTTP или gRPC) + versioned API.
- Минимальные endpoints:
  - `health`
  - `workspace.open`
  - `workspace.applyChanges`
  - `workspace.status`
  - `workspace.close`
  - `context.pack`
  - `typeAtPosition`
  - `members`
  - `diagnostics`

---

## Пошаговый план

### Шаг 1: Протокол и версии
- Определить `api_version` и правила backward-compat.
- Определить идемпотентность и cancelability.
- Добавить handshake в `workspace.open`:
  - клиент → `clientVersion`, `supportedProtocolVersions[]`,
  - сервер → `protocolVersion`, `capabilities`.

### Шаг 2: Workspace lifecycle
- Ввести server-side `workspace_id`.
- Сделать in-memory state в MVP (без персистентного storage), но с TTL.
- Зафиксировать контракт `workspace.applyChanges`:
  - инкрементальные изменения текста (диапазоны),
  - full-file blobs для догрузки артефактов (используется вместе с `missingInputs[]` в M3).

### Шаг 3: Query MVP
- Пробросить `typeAtPosition/members/diagnostics` через gateway.
- Добавить `context.pack` (MVP):
  - собрать “минимальный текстовый пакет” для LLM вокруг `focus` (файл+позиция),
  - возвращать `completeness` и `missingInputs[]` (даже если пока всегда empty; наполнение в M3).

---

## Критерии завершения (DoD)

- Локальный сервер отвечает на `health`.
- `RemoteGateway` может открыть workspace и получить `typeAtPosition` на простом файле.
- `RemoteGateway` может вызвать `context.pack` и получить ответ в текстовом формате.
