# Design: Асинхронный `bsl-agent` (jobs + progress + persist/resume)

## Цели
- Убрать “непонятные зависания” у LLM‑клиентов: любой `tools/call` должен быть быстрым.
- Дать обратную связь о ходе работы: фаза + процент.
- Обеспечить восстановление после рестарта MCP‑процесса (persist/resume) без потери контекста.
- Сохранить local‑first и read‑only ограничения для workspace.

## Ключевые принципы
1. **Job‑модель для всего тяжёлого**
   - Startup и все семантические операции выполняются как jobs.
   - Синхронные семантические tools удаляются/заменяются на `*_start` (breaking change).
2. **Progress через polling**
   - Без server‑initiated push: клиент вызывает `workspace_status`/`job_status`/`job_wait`.
3. **Persist на диск**
   - Сессии и job‑статусы/результаты сохраняются на диск (в пределах каталога кэша), чтобы пережить рестарт Codex.
4. **Best‑effort cancel**
   - `job_cancel` помечает job как отменённый; реальная остановка зависит от границ операций и реализуется кооперативно.

## API дизайн (высокоуровнево)

### Workspace/session tools
- `workspace_open`: создаёт сессию и стартует `startup` job (если нужны platform/config входы).
- `workspace_status`: возвращает `ready`, `phase`, `progress.percent` и сводку предупреждений/недостающих входов.
- `workspace_close`: закрывает сессию.
- `workspace_resume`: восстановить сессию по `session_id`.
- `workspace_list`: перечислить сохранённые сессии (для resume UX).

### Job tools
- `job_status(job_id)`
- `job_wait(job_id, timeout_ms)` (long‑poll)
- `job_result(job_id)`
- `job_cancel(job_id)`

### Семантические tools (async)
Для каждого текущего sync tool создаётся `*_start`:
- `bsl_diagnostics_start`
- `bsl_symbol_search_start`
- `bsl_type_at_position_start`
- `bsl_members_start`
- `bsl_definition_start`
- `bsl_references_start`
- `context_pack_start`
- `context_expand_start`

Все `*_start` возвращают `job_id` и (опционально) `recommended_poll_ms`.

## Модель прогресса
- `phase`: строка из ограниченного набора (startup фазы + “running/finished” для обычных job).
- `progress.percent`: `0..100`.
- Правило: значение монотонно не убывает в рамках одного job.

## Persist/resume модель

### Хранилище состояния
Хранить в отдельном namespace внутри каталога кэша:
`$BSL_CACHE_DIR/bsl-agent-state/v1/`

Минимальные сущности:
- `session.json` (id, roots, inputs, created_at, last_used_at, ready flags)
- `jobs/<job_id>.json` (state, phase, percent, started_at, updated_at, error)
- `jobs/<job_id>.result.json` (результат; формат зависит от tool)

### Поведение при рестарте
- При старте MCP‑процесса загружается список сохранённых сессий.
- Любые job в состояниях `queued|running` помечаются как `aborted_by_restart` (с причиной).
- Клиент может вызвать `workspace_resume`, затем заново стартовать нужные `*_start` jobs.

## Конкурентность и кэш
- DiskCache уже поддерживает межпроцессные locks per‑key; jobs должны использовать его как единственный механизм синхронизации для тяжёлых артефактов.
- Persist‑файлы `bsl-agent-state` должны иметь собственную сериализацию доступа (atomic write + маленькие locks), чтобы параллельные jobs не ломали состояние.

## Обратная совместимость
Не требуется: это intentional breaking change. Старые tool имена могут быть удалены или оставлены только как thin wrappers (по решению реализации), но контракт в спеках фиксируется для нового набора tools.

