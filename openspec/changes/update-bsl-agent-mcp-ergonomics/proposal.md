# Change: update-bsl-agent-mcp-ergonomics

## Why
Реальный feedback по `bsl-agent` на 1С-проекте показывает, что базовый toolset уже удачный: async job-model предсказуем, `mcp_help` полезен, lifecycle и семантические tools покрывают практические сценарии.

Основные проблемы теперь лежат не в core functionality, а в discoverability и operator ergonomics:
- из описания `workspace_open` недостаточно ясно, что `platform_docs_archive` загружает platform types и method signatures, а `configuration_path` добавляет только configuration metadata;
- для common сценариев не хватает recipe-oriented help;
- `build_info` не возвращает runtime context, который оператору реально нужен (`log_file_path`, UI context);
- для простого file diagnostics не хватает thin convenience entry point поверх существующего file-scope;
- error messages в нескольких частых lifecycle кейсах полезны, но ещё не стандартизованы как operator-facing contract.

Отдельно, compact diagnostics payload уже вынесен в активный change `add-bsl-agent-compact-diagnostics-mode`. Новый change должен быть совместим с ним и не дублировать его scope.

## What Changes
- Уточнить user-facing контракт `workspace_open`:
  - `platform_docs_archive` загружает platform types и method signatures;
  - без него full platform type lookup может быть недоступен;
  - `configuration_path` добавляет только configuration metadata types и не заменяет platform docs.
- Расширить `mcp_help` recipe-oriented примерами:
  - diagnostics по файлу,
  - hot diagnostics с overlay,
  - type at position,
  - definition + references,
  - resume после рестарта,
  - а также явное пояснение, что `job_wait` возвращает status only, а payload приходит через `job_result`.
- Расширить `build_info` operator-visible runtime context:
  - `log_file_path`
  - `ui_url` / UI availability context
- Добавить convenience tool `bsl_diagnostics_file_start(...)` как thin wrapper поверх существующего file-scope diagnostics path.
- Стандартизовать operator-facing error wording для частых случаев:
  - workspace not ready
  - path outside roots
  - job_result before succeeded

## Impact
- Спецификация: `openspec/specs/mcp-bsl-agent/spec.md`
- Код:
  - `bsl-agent` help/tool descriptions/README
  - `build_info` response DTO
  - diagnostics tool surface (`bsl_diagnostics_file_start`)
  - error-message normalization для common lifecycle mistakes
- Тесты:
  - stdio help/build_info regressions
  - stdio file-diagnostics convenience tool
  - regressions на canonical error wording

## Non-Goals
- Этот change НЕ переопределяет compact diagnostics payload и НЕ дублирует `add-bsl-agent-compact-diagnostics-mode`.
- Этот change НЕ меняет semantics `include_impact` / `include_coverage`; их cleanup требует отдельного решения.
