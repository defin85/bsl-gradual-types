# Design: оптимизация описаний MCP tool-ов `bsl-agent`

## Проблема
Есть конфликт целей:
- **Однозначность** для клиента/LLM требует конкретики (форматы, правила, обязательные поля).
- **Экономия токенов** требует не раздувать `tools/list`.

Если вшивать JSON-примеры в каждый tool.description, стоимость “постоянного контекста” растёт линейно с количеством tool-ов и может заметно съедать бюджет.

## Принцип решения
1) `tool.description` — короткий, но “контрактный” текст:
   - фиксирует *только* то, что иначе приходится угадывать;
   - без многострочных примеров;
   - одинаковый стиль и терминология.

2) Примеры — по запросу:
   - отдельный read-only tool `mcp_help`/`tool_examples`, который возвращает:
     - quickstart flow;
     - примеры payload’ов;
     - правила multi-root/абсолютных путей;
     - частые причины `INVALID_PARAMS`.

Так `tools/list` остаётся компактным, а клиент может “догрузить” примеры точечно.

## Стиль описаний (черновик)
Шаблон (1 строка):
`<Action>. <Key format/constraint>. <Async guidance if relevant>.`

Примеры формулировок (без JSON):
- `workspace_open`: “Open workspace (single-session). If configuration_path set, platform_version required (auto-infer if possible). mode=default allowed.”
- `workspace_documents_set`: “Set overlays / hot-set. files: DocumentRef/FileRef; accepts absolute paths; version required with text.”
- `bsl_diagnostics_start`: “Start diagnostics job. scope: project|hot|file. Use job_wait/job_result.”

## Риски/компромиссы
- Добавление help tool увеличивает число tool-ов на 1, но экономит токены по сравнению с вшиванием примеров в каждый description.
- Описания должны быть стабильными: менять их часто — риск для тестов и документации.

