# Tasks

## 1. Спека и versioning

- [x] Зафиксировать additive `v21` contract для flush-aware server egress split.
- [x] Зафиксировать client probe receive/resolve split и explicit degradation rules для legacy paths.

## 2. Server egress instrumentation

- [x] Записать bounded flush-complete boundary после реального transport write/flush completion для completion response.
- [x] Экспортировать `response_flush_completed_at_ms` и self-contained post-handler server egress wait в authoritative timeline payload.
- [x] Обновить contiguous contract baseline `contracts/lsp-completion-timeline/v18`.

## 3. Client probe milestones

- [x] Разделить в probe lifecycle client enter, LSP dispatch, raw transport response receive, promise resolve и client terminal.
- [x] Гарантировать bounded/redacted storage этих milestones без нового persistent telemetry pipeline.

## 4. Human-readable surfaces и incident bundle

- [x] Обновить Completion Timeline panel, clipboard export и incident bundle summary на новый `v21` gap split.
- [x] Явно деградировать на `v20` и на legacy probe paths, не выдумывая flush/receive boundaries.

## 5. Verification

- [x] Добавить focused backend contract tests на `v21` flush-aware payload.
- [x] Добавить extension tests на probe receive/resolve split и derived gap projection.
- [x] Прогнать минимальный релевантный verify set для backend + extension contracts.
