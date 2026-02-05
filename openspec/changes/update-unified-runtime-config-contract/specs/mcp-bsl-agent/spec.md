## MODIFIED Requirements

### Requirement: Runtime update settings для активной сессии `bsl-agent`
Система SHALL предоставлять MCP tool (stdio) для обновления runtime-config активной workspace-сессии без её перезапуска.
Tool MUST принимать `session_id` и **канонический payload overrides (camelCase)**, совместимый с LSP settings и схемой `bsl-runtime-config`.

Tool MUST также принимать legacy snake_case поля как alias для обратной совместимости.

#### Scenario: Изменение `BSL_CACHE_DISABLE` через tool немедленно влияет на поведение
- **GIVEN** открыта workspace-сессия `bsl-agent` и кэш включён
- **WHEN** клиент вызывает tool обновления settings с `envOverrides.BSL_CACHE_DISABLE=true`
- **THEN** последующие операции используют отключённый кэш без перезапуска сессии

#### Scenario: legacy snake_case payload принимается
- **GIVEN** активная сессия
- **WHEN** клиент вызывает tool с `env_overrides.BSL_CACHE_DISABLE=true`
- **THEN** поведение совпадает с `envOverrides.BSL_CACHE_DISABLE=true`

### Requirement: MCP tool возвращает effective runtime-config и канонические поля
Система SHALL возвращать в ответах `workspace_get_settings` и `workspace_update_settings`:
- effective runtime-config snapshot,
- канонические поля (`envOverrides`, `devEnvOverrides`, `allowDevOverrides`).

Legacy snake_case поля MAY присутствовать временно, но считаются deprecated.

#### Scenario: Ответ содержит canonical поля
- **GIVEN** активная сессия
- **WHEN** клиент вызывает `workspace_get_settings`
- **THEN** ответ содержит `envOverrides/devEnvOverrides/allowDevOverrides`

## ADDED Requirements

### Requirement: bsl-agent предоставляет observability metrics tool
Система SHALL предоставлять MCP tool `workspace_get_observability_metrics(session_id)`, который возвращает snapshot observability-метрик для указанной workspace-сессии.

Tool MUST требовать `ready=true` и SHALL отклонять не-ready сессию как `INVALID_PARAMS`.

#### Scenario: Observability tool работает для ready-сессии
- **GIVEN** сессия `ready=true`
- **WHEN** клиент вызывает `workspace_get_observability_metrics(session_id)`
- **THEN** сервер возвращает JSON snapshot метрик (совместимый по форме с LSP `bsl.getObservabilityMetrics`)

#### Scenario: Observability tool отклоняет не-ready сессию
- **GIVEN** сессия `ready=false`
- **WHEN** клиент вызывает `workspace_get_observability_metrics(session_id)`
- **THEN** сервер возвращает `INVALID_PARAMS` с сообщением о том, что workspace не ready

