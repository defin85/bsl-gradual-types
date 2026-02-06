## Context
Unified runtime-config уже внедрён, но сейчас “унификация” разорвана на границе LSP ↔ MCP:
- LSP settings: camelCase поля (`envOverrides`, `devEnvOverrides`, `bsl.dev.enableDevEnvOverrides`).
- `bsl-agent` MCP tools: snake_case поля (`env_overrides`, `dev_env_overrides`, `allow_dev_overrides`).

Это мешает:
- использовать один и тот же payload в UI/скриптах,
- писать повторно используемые клиенты и тесты,
- понимать гарантии “без рестарта” (часть значений startup-only).

## Goals
- Единый канонический JSON контракт overrides (camelCase) для LSP + `bsl-agent`.
- Явная модель mutability:
  - `runtime`: изменения обязаны влиять без рестарта,
  - `startup_only`: изменения отражаются в snapshot сразу, но могут требовать рестарта coordinator/session для эффекта.
- `bsl-agent` предоставляет tool для observability-метрик, аналогичный LSP.

## Non-Goals
- Не менять смысл/приоритет слоёв overrides (stable/dev-only/env bootstrap).
- Не делать все ключи “runtime” любой ценой; вместо этого — явное описание startup-only.
- Не добавлять удалённую синхронизацию настроек между процессами.

## Decisions
### Canonical payload
Канонический payload (camelCase):
```json
{
  "envOverrides": { "BSL_CACHE_DISABLE": true },
  "devEnvOverrides": { "BSL_COMPLETION_TRACE": true },
  "allowDevOverrides": true
}
```

Совместимость:
- `bsl-agent` MUST принимать legacy snake_case для входа как alias (на ограниченный период).
- `bsl-agent` SHOULD возвращать camelCase в ответах; legacy snake_case MAY оставаться временно, но считается deprecated.

### Mutability model
Добавить в registry метаданные `mutability`:
- `runtime`: hot-path consumers читают из runtime-config и применяют сразу.
- `startup_only`: применяется только при инициализации (startup) — например, пути/директории или значения, влияющие на построение coordinator.

В snapshot добавить карту `mutability` по ключам.
В `ApplyOverridesReport` добавить `requires_restart_keys[]`.

### Observability tool
Добавить MCP tool:
- `workspace_get_observability_metrics(session_id)`:
  - требует `ready=true`,
  - возвращает JSON snapshot метрик (совместимый по форме с LSP `bsl.getObservabilityMetrics`).

## Risks / Mitigations
- **Breaking change** для клиентов, которые парсят только snake_case ответы `bsl-agent`.
  - Смягчение: временно возвращать оба набора полей или поддерживать alias в десериализации у клиентов.
- Мутируемость может потребовать пересмотра некоторых consumers.
  - Смягчение: начать с небольшого списка startup-only ключей и расширять постепенно; snapshot делает поведение прозрачным.

