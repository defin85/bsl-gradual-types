# План реализации M3: Workspace sync + caching

**Статус:** 🔴 ПЛАН  
**Цель:** обеспечить быстрый и честный UX на больших конфигурациях: синхронизировать только нужное и уметь догружать по demand.

---

## Область работ

- Manifest + fingerprint (Merkle) для дедупа и возобновления.
- `syncMode = hot_set|progressive|full`.
- `config.skeleton` как обязательный минимум метаданных для `hot_set/progressive`.
- Инкрементальные изменения (diff/patch), batching.
- Контракт “честности”: `completeness: full|partial` + `missingInputs[]`.
- Политики include/exclude (не тащить UI/ресурсы, догружать `*.xml` по demand).

---

## Пошаговый план

### Шаг 1: Manifest и дедуп
- Определить формат манифеста файлов (path/size/hash/mtime).
- Сервер отвечает “что нужно догрузить”.
- Уточнить, какие артефакты считаются “семантически релевантными” в IDE:
  - `*.bsl`,
  - `config.skeleton`,
  - `*.xml` только по `missingInputs[]`.

### Шаг 2: Progressive sync
- Ввести `ArtifactRef` и перечислить `kind`: `bsl|xml|config.skeleton|config.bundle|platform.bundle`.
- Сервер возвращает `missingInputs[]` для точного резолвинга и всегда помечает ответ `partial`, если входов не хватает.
- Клиент догружает недостающие артефакты через `workspace.applyChanges` (full-file blobs) и повторяет запрос.
- Режимы:
  - `hot_set`: без интерактивной догрузки (лучше как fast-start),
  - `progressive`: интерактивная догрузка + фоновый prefetch,
  - `full`: загрузка всего релевантного набора для CI.

### Шаг 3: Кэширование на сервере
- Ключи по fingerprint + settings/version.
- TTL/retention policy для SaaS.
- Включить `schemaVersion` сериализуемых артефактов в ключи кэша (в т.ч. `config.bundle`).

---

## Критерии завершения (DoD)

- `workspace.open` работает для `syncMode=hot_set` без заливки всего воркспейса.
- Запросы возвращают `partial` и список недостающих входов вместо “угадывания”.
- `config.skeleton` обязателен для `hot_set/progressive` и реально влияет на качество типов.
- Для `syncMode=progressive` возможен цикл: `context.pack(completeness=partial)` → `missingInputs` → `applyChanges` → `context.pack(completeness=full)`.
