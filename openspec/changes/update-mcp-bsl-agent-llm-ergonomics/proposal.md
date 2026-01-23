# Change: update-mcp-bsl-agent-llm-ergonomics

## Why
`bsl-agent` в первую очередь используется LLM как инструмент получения семантического контекста. Сейчас LLM-использование сталкивается с несколькими “шероховатостями”, которые делают поведение менее повторяемым и усложняют tool-calls:

- предупреждение `unknown mode: default` при `workspace_open(mode="default")`;
- вводящее в заблуждение поле `missing_inputs` (в т.ч. ощущение, что `configuration_path` обязателен всегда);
- строго типизированные форматы параметров (enum-объекты со `kind`, вложенные `doc {root_id,path}`), которые неочевидны для LLM;
- прогресс, который визуально может “зависнуть” на 100% до фактического завершения job;
- необходимость ручного подбора `platform_version` при наличии `configuration_path`.

Нужно сделать поведение максимально простым и детерминированным для LLM, при этом сохранив read-only и безопасность (sandbox по roots) и не ломая существующий канонический формат параметров.

## What Changes
- `workspace_open` принимает `mode="default"` как алиас режима по умолчанию (без warning).
- `workspace_open` перестаёт считать `configuration_path` обязательным и выравнивает `missing_inputs` с реальными требованиями startup.
- Если задан `configuration_path` и не задан `platform_version`, `bsl-agent` пытается автоматически определить версию платформы из дампа конфигурации (например, через `Configuration.xml`/режим совместимости). Если определить нельзя — возвращает `INVALID_PARAMS` (fail-fast).
- Инпуты tool-ов, где используются ссылки на файлы/документы и scope, получают совместимые “LLM-friendly” формы на базе абсолютных путей, работающие и для multi-root:
  - абсолютный путь однозначно маппится на root через longest-prefix match;
  - при неоднозначности/вне roots — `INVALID_PARAMS`.
- Прогресс job’ов не может быть `100`, пока job не в terminal-состоянии; `100` выставляется только при `succeeded|failed|canceled|aborted_by_restart`.

## Impact
- Спецификация: `openspec/specs/mcp-bsl-agent/spec.md`
- Код: `bsl-agent` (парсинг параметров tools, нормализация путей, startup inputs, прогресс)
- Документация: `docs/roadmap/mcp-bsl-agent/api.md`, `bsl-agent/README.md`
- Тесты: интеграционные тесты MCP (stdio) и/или unit-тесты нормализации путей/инференса версии

