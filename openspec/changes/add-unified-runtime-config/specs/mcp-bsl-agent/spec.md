## MODIFIED Requirements

### Requirement: Runtime update settings для активной сессии `bsl-agent`
Система SHALL предоставлять MCP tool (stdio) для обновления runtime-config активной workspace-сессии без её перезапуска.
Tool MUST принимать `session_id` и payload overrides, совместимый со схемой `bsl-runtime-config`.

#### Scenario: Изменение `BSL_CACHE_DISABLE` через tool немедленно влияет на поведение
- **GIVEN** открыта workspace-сессия `bsl-agent` и кэш включён
- **WHEN** клиент вызывает tool обновления settings с `envOverrides.BSL_CACHE_DISABLE=true`
- **THEN** последующие операции используют отключённый кэш без перезапуска сессии

### Requirement: bsl-agent принимает stable и dev-only overrides
Система SHALL принимать `envOverrides` и `devEnvOverrides` в tool runtime update, и применять их к effective runtime-config.

#### Scenario: Dev-only override включается и отражается в метриках/логах
- **GIVEN** активная сессия
- **WHEN** клиент включает `devEnvOverrides.BSL_COMPLETION_TRACE=true`
- **THEN** последующие операции логируют/экспортируют dev-only trace поведение согласно ключу

