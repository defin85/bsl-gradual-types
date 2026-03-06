# Design: add-bsl-agent-compact-diagnostics-mode

## Context
Текущий `bsl_diagnostics_start` уже пригоден для MCP/LLM:
- transport/job model корректны;
- flat payload стабилен;
- каждая запись содержит `diagnostic_id`, `file`, `range`, `severity`, `code?`, `message`.

Проблема не в корректности, а в signal-to-noise:
- repeated `file/root_id` на single-file ответах;
- `code: null` в большинстве записей;
- отсутствие встроенной summary;
- повторяющиеся сообщения на разных строках плохо подходят для больших LLM ответов;
- нет простого server-side shaping по severity.

## Goals
- Добавить opt-in compact режим без breaking changes для существующих MCP-клиентов.
- Уменьшить повторяемость и размер diagnostics payload в типичных single-file и больших project-scope ответах.
- Сохранить детерминированность и пригодность для машинной обработки.

## Non-Goals
- Не менять async contract `start -> wait -> result`.
- Не переопределять semantics `include_impact` и `include_coverage` в рамках этого change.
- Не менять формат `diagnostic_id`: это остаётся стабильным machine identifier для drilldown (`context_pack`, `context_expand`, follow-up tooling).

## Decisions

### 1. Новые opt-in request параметры
`bsl_diagnostics_start` получает дополнительные shaping-параметры:
- `compact: bool` (`false` по умолчанию)
- `group_by: "none" | "message"` (`"none"` по умолчанию)
- `omit_null_fields: bool` (`false` по умолчанию)
- `omit_repeated_file: bool` (`false` по умолчанию)
- `severity_filter: "error" | "warning" | "info" | null`

Это read-only shaping уже вычисленного diagnostics result. Анализ и job execution semantics не меняются.

### 2. Backward compatibility by default
Если `compact=false` и новые shaping-параметры не заданы, `bsl_diagnostics_start` обязан возвращать текущий flat payload:
- `analysis_revision`
- `flow_sensitive_enabled`
- `diagnostics[]`
- `truncated`

Это важно для существующих MCP-клиентов и уже написанных stdio tests.

### 3. Compact response shape
Если `compact=true`, ответ должен содержать top-level `summary`:

```json
{
  "analysis_revision": 42,
  "flow_sensitive_enabled": false,
  "truncated": false,
  "summary": {
    "errors": 3,
    "warnings": 5,
    "infos": 0,
    "unique_messages": 2
  }
}
```

`summary` считается по diagnostics после применения `severity_filter` и после фактического ограничения выдачи (`limit` / `truncated`), чтобы summary всегда описывала именно возвращённый payload.

### 4. `omit_repeated_file` через `common_file`
Если `omit_repeated_file=true` и все diagnostics в результирующей выборке относятся к одному и тому же документу, сервер выносит документ в top-level `common_file` и может не повторять `file` внутри каждой записи/occurrence.

Если результирующая выборка содержит несколько файлов, сервер не делает fail-fast: `common_file` отсутствует, а per-item `file` остаётся.

Это покрывает:
- tagged single-file scope;
- `hot`, если effective result set реально оказался однофайловым.

### 5. `group_by=message`
Для `group_by="message"` compact response должен переключаться с flat diagnostics на grouped payload:

```json
{
  "summary": { "...": "..." },
  "groups": [
    {
      "message": "Переменная не объявлена",
      "severity": "error",
      "count": 4,
      "occurrences": [
        { "diagnostic_id": "...", "range": { "...": "..." } }
      ]
    }
  ]
}
```

Требования:
- grouping key = как минимум `message` + `severity`;
- порядок групп детерминированный;
- `groups[]` является primary payload для `group_by=message`;
- flat `diagnostics[]` в этом режиме не дублируется.

`occurrences[]` должны сохранять достаточно данных для drilldown: минимум `diagnostic_id` и `range`, а `file`/`code` могут зависеть от `omit_repeated_file` и `omit_null_fields`.

### 6. `omit_null_fields`
Если `omit_null_fields=true`, nullable поля, у которых значение `null`, не сериализуются в JSON вообще.

Первый target этого требования: `code`, который в реальных diagnostics часто отсутствует и не должен засорять payload.

### 7. `severity_filter`
`severity_filter` применяется server-side к уже вычисленному списку diagnostics до compact/group shaping.

Это must-have для LLM/операторского сценария, где часто нужно быстро получить только `error` или только `warning`, не вытаскивая полный список.

### 8. Ownership boundary для docs/examples
Этот change владеет diagnostics-specific user-facing material:
- описание shaping параметров;
- compact payload examples;
- `mcp_help/README` examples для `compact`, `group_by`, `omit_null_fields`, `omit_repeated_file`, `severity_filter`.

Change `update-bsl-agent-mcp-ergonomics` владеет workflow recipes и общими operator-facing примерами.

Это разграничение нужно, чтобы `mcp_help` и README не дублировали одну и ту же diagnostics матрицу в двух местах.

### 9. Convenience wrappers reuse the same compact shaping path
Любой convenience entry point для single-file diagnostics, включая `bsl_diagnostics_file_start`, обязан:
- принимать тот же single-file-compatible набор shaping параметров;
- использовать тот же diagnostics result serializer/shaper;
- не вводить отдельный compact-only payload contract;
- оставлять `diagnostic_id` стабильным drilldown identifier в flat и grouped режимах.

## Risks / Trade-offs
- Появляется альтернативная shape ответа для `compact=true`; это допустимо, потому что режим opt-in.
- `group_by=message` не должен ломать downstream сценарии с `diagnostic_id`, поэтому `occurrences[]` обязаны сохранять stable ids.
- `summary` по возвращённой выборке, а не по “всем найденным diagnostics”, лучше для MCP клиента, но хуже для глобальной аналитики. Для аналитики остаётся legacy/full mode.

## Migration Plan
1. Сначала расширить spec и DTO request/response.
2. Затем реализовать compact serialization без изменения default path.
3. После этого добавить grouped path и summary regressions.
4. Обновить `mcp_help` и README diagnostics-specific примерами compact режима и не дублировать workflow recipes, принадлежащие ergonomics change.

## Open Questions
- Нет блокирующих. Если позже понадобится более агрессивное сворачивание, можно отдельно обсудить диапазонный `severity_filter` (`>=warning`) и человеко-читаемый short id, но это не входит в данный change.
