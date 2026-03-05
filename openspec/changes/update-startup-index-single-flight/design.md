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
  - Contract v1 (machine-readable):
    - `version: number`
    - `state: "idle" | "running" | "ready" | "failed"`
    - `ready: boolean` (`state == "ready"`)
    - `active_operation: "startup" | "buildIndex" | null`
    - `operation_id: string | null`
    - `message: string | null`
    - `updated_at_ms: number`

- Decision 3: full-index должен быть single-flight.
  - Если startup/build уже выполняется, повторный `buildIndex` не запускает второй тяжёлый проход.
  - Клиент получает детерминированный ответ "already running (attached)" с `operation_id`.

- Decision 4: extension на старте не использует filesystem sentinel как gate.
  - Решение о запуске full build принимается по серверному `index state`.

- Decision 5: совместимость с legacy LSP без `getIndexState` — fail-closed для startup auto-index.
  - extension MUST NOT запускать silent full build на старте, если сервер не поддерживает `getIndexState`;
  - extension MUST показывать явное предупреждение;
  - ручной `Build Index` сохраняется.

- Decision 6: fail-safe против "залипшего running".
  - Вводится watchdog timeout `hard_timeout_ms = 1200000` (20 минут) по умолчанию;
  - по timeout состояние переводится в `failed`, `active_operation` очищается.

- Decision 7: UX для ручного `Build Index` во время `running`.
  - повторный запрос возвращает attach-статус и не создаёт вторую тяжёлую операцию;
  - UI показывает информационное сообщение, а не ошибку.

## Implementation Outline
1. LSP:
   - добавить/расширить custom request `getIndexState`;
   - ввести единый state-holder для full-index операций;
   - внедрить single-flight guard в startup/build пути;
   - добавить watchdog timeout для terminal transition (`running` -> `failed` по timeout).
2. Extension:
   - заменить startup-gate на вызов `getIndexState`;
   - убрать зависимость от `unified_index.json` для startup решения;
   - сохранить текущие команды пользователя (`Build Index`, `Reindex`) с новым guard-поведением;
   - для legacy LSP (без `getIndexState`) применить fail-closed startup policy + явный warning.
3. Тесты:
   - backend: не запускается второй full build при уже идущем startup;
   - extension: при `ready` не дергается full build, при `running` не создаётся второй full build;
   - совместимость: при `Method not found` для `getIndexState` не выполняется silent full build;
   - timeout: `running` корректно переходит в `failed`.

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
  - Mitigation: fail-closed startup policy + явное предупреждение + доступный manual build.
- Риск “залипшего” running-state после аварии.
  - Mitigation: fail-safe reset состояния в terminal ветках + watchdog timeout (20 минут по умолчанию).
