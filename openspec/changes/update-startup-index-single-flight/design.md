## Context
На текущем старте extension присутствуют два источника запуска тяжёлого full-index:
- LSP startup (`initialized` -> `startup_v2` -> загрузка platform/config типов + индексация модулей),
- extension startup hook (`initializeIndexIfNeeded` -> `bsl/buildIndex`).

В результате cold start может выполнять дублирующую работу, хотя пользователь не инициировал повторную индексацию.

Дополнительно проверка готовности индекса в extension опирается на локальный файловый sentinel (`unified_index.json`), что не является надёжным источником истины относительно runtime-состояния LSP.

## Goals / Non-Goals
- Goals:
  - Убрать повторный full-index на старте extension.
  - Ввести единый источник истины о состоянии индекса на стороне LSP.
  - Сохранить предсказуемое поведение ручного `Build Index` и auto-reindex.
  - Снизить startup latency и фоновую нагрузку на больших конфигурациях.
- Non-Goals:
  - Полная переработка хранения индекса.
  - Унификация всех legacy cache артефактов в один формат.
  - Изменение inference/diagnostics контрактов.

## Architecture Drivers
- Производительность cold start (latency + CPU/IO).
- Предсказуемость UX (одна тяжёлая операция вместо двух).
- Операционная наблюдаемость: machine-readable state для клиента.
- Простота внедрения без широкого refactor runtime.

## Options Considered

### Option A: Просто отключить auto build на старте extension
- Плюсы: минимальные правки.
- Минусы: нет корректного recovery path при `failed/idle`, логика решения разносится по условностям.

### Option B (Recommended): Server-driven index state + single-flight full-index
- Идея:
  - LSP публикует текущий `index state` через custom request.
  - `buildIndex` и startup full-index защищены single-flight guard.
  - extension принимает решение на старте по `index state`.
- Плюсы:
  - один источник истины;
  - убирает дублирование системно;
  - прозрачно для ручных и автосценариев.
- Минусы:
  - нужен новый/расширенный контракт custom request.

### Option C: Оставить текущий подход и синхронизировать filesystem sentinel
- Плюсы: не меняет протокол.
- Минусы: хрупко, platform-specific, остаётся риск рассинхронизации между файловым артефактом и реальным runtime state.

## Decisions
- Decision 1: выбрать Option B.
  - Why: надёжный контроль дедупликации на границе extension/LSP с минимальным архитектурным долгом.

- Decision 2: зафиксировать server-driven `index state` контракт.
  - Минимально: `state`, `ready`, `message`, `active_operation`.
  - Дополнительно (опционально): timestamps/fingerprint/version для observability.

- Decision 3: full-index должен быть single-flight.
  - Если startup/build уже выполняется, повторный `buildIndex` не запускает второй тяжёлый проход.
  - Клиент получает детерминированный ответ “already running / attached to current operation”.

- Decision 4: extension на старте не использует filesystem sentinel как gate.
  - Решение о запуске full build принимается по серверному `index state`.

## Implementation Outline
1. LSP:
   - добавить/расширить custom request `getIndexState`;
   - ввести единый state-holder для full-index операций;
   - внедрить single-flight guard в startup/build пути.
2. Extension:
   - заменить startup-gate на вызов `getIndexState`;
   - убрать зависимость от `unified_index.json` для startup решения;
   - сохранить текущие команды пользователя (`Build Index`, `Reindex`) с новым guard-поведением.
3. Тесты:
   - backend: не запускается второй full build при уже идущем startup;
   - extension: при `ready` не дергается full build, при `running` не создаётся второй full build.

## Test Strategy
- Backend:
  - contract tests для `index state`;
  - concurrency/single-flight tests для startup + `buildIndex`.
- Extension:
  - activation orchestration tests на `running|ready|failed|idle`.
- End-to-end (smoke):
  - startup на большой конфигурации: один full-index проход.

## Risks / Trade-offs
- Риск race-condition при одновременном startup и manual build.
  - Mitigation: atomic state transition + single-flight lock.
- Риск несовместимости extension со старым LSP без `getIndexState`.
  - Mitigation: явный capability/fallback режим с безопасным поведением и логированием.
- Риск “залипшего” running-state после аварии.
  - Mitigation: fail-safe reset состояния в terminal ветках + watchdog timeout.
