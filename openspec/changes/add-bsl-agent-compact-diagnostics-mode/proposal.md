# Change: add-bsl-agent-compact-diagnostics-mode

## Why
`bsl-agent` уже отдаёт рабочий diagnostics payload для MCP: transport чистый, job-модель удобная, а каждая диагностика содержит `range`, `severity` и `message`.

Проблема в том, что payload остаётся шумным для больших ответов и особенно для `project` scope:
- в single-file ответах у каждой записи повторяется один и тот же `file/root_id`;
- `code` часто равен `null`, но всё равно занимает место в JSON;
- отсутствует компактная сводка (`errors/warnings/infos/unique_messages`);
- повторяющиеся сообщения на разных строках не сворачиваются в более компактное представление;
- у клиента нет встроенного способа быстро отфильтровать только `error|warning|info`.

Нужен opt-in compact режим, который уменьшает контекстный шум без ломки текущего контракта `bsl_diagnostics_start`.

## What Changes
- Добавить opt-in параметры shaping для `bsl_diagnostics_start`:
  - `compact`
  - `group_by`
  - `omit_null_fields`
  - `omit_repeated_file`
  - `severity_filter`
- Добавить compact response contract:
  - `summary: { errors, warnings, infos, unique_messages }`
  - grouped output для `group_by=message`
  - top-level `common_file` для случая, когда все возвращённые diagnostics относятся к одному документу
- Сохранить backward compatibility:
  - по умолчанию `compact=false`
  - legacy flat payload остаётся без breaking changes
- Явно задокументировать, что `include_impact` и `include_coverage` не переопределяются этим change и остаются вне scope compact режима.

## Impact
- Спецификация: `openspec/specs/mcp-bsl-agent/spec.md`
- Код:
  - `bsl-agent` MCP server types / tool router / diagnostics DTO serialization
  - `bsl-agent` stdio integration tests и help/README
- Тесты:
  - backward compatibility для default payload
  - compact single-file payload
  - grouping/filtering/summary regressions
