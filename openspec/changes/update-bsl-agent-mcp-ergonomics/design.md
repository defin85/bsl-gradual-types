# Design: update-bsl-agent-mcp-ergonomics

## Context
`bsl-agent` уже достаточно хорош как semantically-capable MCP server:
- async job contract согласован;
- lifecycle session/jobs предсказуем;
- `mcp_help` существует и реально используется;
- core semantic tools покрывают production сценарии.

Реальный feedback показывает следующий слой проблем:
- discoverability входов `workspace_open`;
- недостаток cookbook-style recipes;
- отсутствие operator-visible runtime context в `build_info`;
- избыточная церемония для простого file diagnostics;
- error wording ещё не оформлен как жёсткий UX contract.

Одновременно в активной разработке уже есть `add-bsl-agent-compact-diagnostics-mode`. Этот change должен быть complementary:
- не дублировать compact payload API;
- не владеть shaping semantics diagnostics;
- использовать тот же underlying diagnostics path.

## Goals
- Сделать user-facing MCP surface понятнее без радикального расширения API.
- Снизить ceremony для common single-file diagnostics path.
- Улучшить operator visibility через `build_info`.
- Зафиксировать canonical help/error wording для частых сценариев.

## Non-Goals
- Не менять core async model `*_start -> job_wait -> job_result`.
- Не переопределять compact diagnostics response shape.
- Не удалять и не repurpose-ить `include_impact` / `include_coverage`.

## Decisions

### 1. `workspace_open` должен явно различать platform docs и configuration metadata
В user-facing описании, `mcp_help(workspace_open)` и README должно быть явно сказано:
- `platform_docs_archive` загружает platform types и method signatures;
- без него доступны только fallback/basic platform capabilities, а full platform lookup может не сработать;
- `configuration_path` добавляет configuration metadata (`Документы.*`, `Справочники.*`, и т.д.);
- `configuration_path` не заменяет `platform_docs_archive`.

Это не новый runtime behavior, а явная фиксация того, как система уже фактически используется.

### 2. `mcp_help` должен быть recipe-oriented, а не только per-tool
Текущий `mcp_help` уже содержит примеры, но ему не хватает high-signal workflows.

Минимальный набор recipes:
1. diagnostics по файлу;
2. hot diagnostics с overlay;
3. type at position;
4. definition + references;
5. resume после рестарта.

Для async инструментов help обязан явно говорить:
- `job_wait` возвращает только status/progress;
- `job_result` возвращает payload после `succeeded`.

### 3. `build_info` должен возвращать operator runtime context
`build_info` уже полезен для version/build identity, но оператору обычно нужен ещё runtime context.

Минимальный additive contract:
- `log_file_path: string | null`
- `ui_url: string | null`

Это additive расширение без breaking changes. Отдельный tool `ui_url` остаётся каноническим read-only способом получения HTTP UI URL; `build_info` лишь даёт быстрый snapshot того же контекста.

### 4. `bsl_diagnostics_file_start` как thin convenience wrapper
Новый tool не должен создавать второй diagnostics pipeline.

Он обязан быть тонкой обёрткой над существующим tagged file scope path:
- принимает `session_id` и `path`;
- internally вызывает тот же diagnostics path, что и `bsl_diagnostics_start(scope={kind:file,...})`;
- возвращает обычный `job_id` и живёт в той же async job model.

Для совместимости с `add-bsl-agent-compact-diagnostics-mode` tool должен переиспользовать тот же diagnostics request/result contract, который существует у базового file-scope path на момент реализации. Никакого отдельного “compact-only” формата для convenience tool вводить нельзя.

### 5. Canonical operator-facing error wording
Важны не все ошибки, а только самые частые operational cases.

Нужно стабилизировать wording для:
- workspace not ready;
- path outside roots;
- `job_result` до terminal `succeeded`.

Требование не в полном переводе всех ошибок на новый taxonomy, а в том, чтобы эти частые случаи возвращали predictable фразы, на которые можно опираться и человеку, и LLM.

## Risks / Trade-offs
- Добавление convenience tool слегка расширяет surface area, но это оправдано, потому что он thin wrapper и не вводит новый execution path.
- Дублирование `ui_url` в `build_info` создаёт частичное пересечение с отдельным tool, но выигрывает в operator ergonomics.
- Recipe-heavy `mcp_help` нельзя превращать в длинный reference manual; recipes должны оставаться компактными.

## Migration Plan
1. Сначала зафиксировать contract и docs/help wording.
2. Затем расширить `build_info`.
3. Потом добавить `bsl_diagnostics_file_start`.
4. После этого стабилизировать canonical error wording и закрыть regressions.

## Alignment With `add-bsl-agent-compact-diagnostics-mode`
- Compact diagnostics payload остаётся responsibility change `add-bsl-agent-compact-diagnostics-mode`.
- Recipes и convenience tool, которые касаются diagnostics, должны быть совместимы с compact mode, но не переопределять его.
- Если оба change реализуются параллельно, diagnostics request/response code path должен быть общим.
