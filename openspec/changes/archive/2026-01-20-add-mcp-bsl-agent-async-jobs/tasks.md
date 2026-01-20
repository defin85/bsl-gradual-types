# Tasks: `add-mcp-bsl-agent-async-jobs`

## 1. Спецификация и контракт
- [x] Уточнить контракт MCP tools: перечень `workspace_*`, `bsl_*_start`, `job_*`, форматы ответов и ошибок.
- [x] Зафиксировать модель прогресса: `phase` + `progress.percent` (0..100), правила монотонности и финальные состояния.
- [x] Зафиксировать поведение `persist/resume`: где хранится состояние, как выбирается `session_id`, что происходит с незавершёнными job при рестарте.

## 2. Серверная модель выполнения (job runner)
- [x] Добавить сущности `JobId`, `JobState`, `JobPhase`, `JobProgress` и единый реестр job’ов (на процесс `bsl-agent`).
- [x] Реализовать хранение состояния job’ов и результатов на диск (TTL/GC/лимиты; не блокировать критические пути).
- [x] Реализовать `job_status`, `job_wait`, `job_result`, `job_cancel` (best-effort cancel).

## 3. Асинхронный startup
- [x] Переделать `workspace_open`: возвращать быстро, запускать startup в фоне как `startup_job_id`.
- [x] Интегрировать прогресс startup из backend (`ProgressUpdate`) в job progress (`phase/percent`).
- [x] Обновить `workspace_status` так, чтобы он отражал фактическую готовность (`ready`) и прогресс текущего startup/job.

## 4. Асинхронные семантические tools
- [x] Заменить sync tools `bsl_diagnostics|bsl_symbol_search|bsl_type_at_position|bsl_members|bsl_definition|bsl_references|context_pack|context_expand` на async `*_start`.
- [x] Каждая операция должна сохранять результат в `job_result` и поддерживать `job_wait`.
- [x] Определить и реализовать ошибки “not_ready / missing_inputs” в job-статусе и/или результате.

## 5. Persist/Resume API
- [x] Добавить `workspace_resume` (восстановить сессию по `session_id`) и `workspace_list` (список доступных сессий).
- [x] Определить правило “aborted_by_restart” для job’ов в `running|queued` при рестарте процесса.
- [x] Дедупликацию/повторное использование сессии по fingerprint входов не реализуем (опционально; оставляем на будущее).

## 6. Тесты и валидация
- [x] Обновить/добавить интеграционные stdio‑тесты MCP: сценарий `workspace_open` → polling `workspace_status/job_status` → `job_result`.
- [x] Тест на persist/resume: старт → получить `session_id` → завершить процесс → поднять заново → `workspace_resume` → `job_status/result`.
- [x] Прогнать `cargo test --workspace` и `cargo clippy --all-targets -- -D warnings`.

## 7. Документация
- [x] Обновить `docs/roadmap/mcp-bsl-agent/api.md` (или эквивалент) под job‑модель (breaking change).
- [x] Добавить короткие инструкции для LLM‑клиента: “start → poll → result”.
