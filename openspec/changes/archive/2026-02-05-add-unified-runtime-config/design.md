## Context
Проект использует множество параметров `BSL_*` для кэша, порогов производительности, отладочных флагов и поведения подсистем.
Исторически они читались напрямую из переменных окружения и зачастую кэшировались в `LazyLock/OnceLock`, что делает изменения без рестарта невозможными.

Одновременно, у проекта есть две “публичные” поверхности управления:
- VS Code settings → LSP (через `workspace/didChangeConfiguration`),
- MCP `bsl-agent` tools → рабочая сессия агента.

Цель этого change — обеспечить единый слой runtime-config, который:
1) покрывает все runtime `BSL_*` ключи,
2) принимает runtime overrides,
3) позволяет обновлять значения без рестарта,
4) отделяет dev-only параметры так, чтобы их можно было удалить без затрагивания stable-контракта.

## Goals
- Один реестр всех runtime `BSL_*` с типами/дефолтами/описаниями и tiering.
- Runtime update без рестарта:
  - LSP: `workspace/didChangeConfiguration` применяет overrides сразу,
  - bsl-agent: отдельный tool-call обновляет settings активной сессии.
- Назад-совместимость: `BSL_*` env остаются bootstrap источником значений.
- Dev-only отдельно от stable: отдельный payload/namespace и отдельные требования в спеках.

## Non-Goals
- Не вводить удалённое хранение/синхронизацию настроек между процессами.
- Не пытаться сделать compile-time `env!` переменные изменяемыми в рантайме.
- Не гарантировать UI-формы под каждую переменную (достаточно object overrides + документация).

## Proposed Architecture

### 1) Unified RuntimeConfig store
- Центральная структура `RuntimeConfig` (Arc + RwLock/atomics для hot-path), доступная:
  - в LSP через `SystemCoordinator`/runtime layer,
  - в bsl-agent через session state.
- Реестр ключей `KeySpec`:
  - `env_name: "BSL_CACHE_DIR"`
  - `value_type: Bool|Int|Float|String|DurationMs|Path`
  - `default`
  - `tier: stable|dev-only`
  - `components: [lsp, agent, runtime]`

### 2) Merge priority
Effective value вычисляется как:
`defaults` < `env bootstrap` < `runtime overrides (stable)` < `runtime overrides (dev-only)`.

Dev-only overrides всегда “последние”, чтобы дев-режим не требовал чистки других уровней.
Для “удаления dev-only” достаточно удалить поддержку dev-only overrides и соответствующих ключей tier=dev-only.

### 3) Settings payload (LSP + bsl-agent)
Единый JSON контракт:
```json
{
  "envOverrides": { "BSL_CACHE_DIR": "/tmp/bsl_cache", "BSL_CACHE_DISABLE": true },
  "devEnvOverrides": { "BSL_COMPLETION_TRACE": true }
}
```
Где:
- `envOverrides` — stable overrides,
- `devEnvOverrides` — dev-only overrides (можно отключить целиком в будущем).

### 4) Application points
- Все чтения `std::env::var("BSL_*")` должны быть сведены к одному месту (bootstrap loader).
- Hot-path чтение (например thresholds, enable flags) должно быть:
  - без аллокаций,
  - без парсинга строк,
  - предпочтительно через atomics/кэшированный typed value.

## Observability
- Метрики должны экспортироваться в едином JSON snapshot формате и быть доступными через:
  - LSP command `bsl.getObservabilityMetrics`,
  - bsl-agent endpoint/tool (существующий или новый).
- Включение/выключение сборщиков (и dev-only trace flags) управляется через RuntimeConfig.

## Compatibility & Migration
- `BSL_*` env продолжают работать как bootstrap.
- Overrides (VS Code / bsl-agent) должны переопределять env (с диагностикой/логом).
- Unknown keys в overrides: по умолчанию ошибка уровня warning + игнор (но поведение фиксируется в спеках).

## Open Questions
### Q1: Нужно ли “read back” (получить effective config) как отдельная команда/endpoint?
Да.

Решение:
- LSP: добавить custom command `bsl.getRuntimeConfig` (возвращает effective config + источники значений: default/env/runtime stable/runtime dev-only).
- bsl-agent: добавить MCP tool `workspace_get_settings` (по `session_id`), возвращающий effective config в том же формате.

Причины:
- упрощает дебаг “почему резолвер/кэш/threshold ведёт себя так”,
- позволяет детерминированно проверять применение runtime update (и писать тесты без чтения env),
- облегчает будущую чистку dev-only: можно видеть, что реально задействовано.

### Q2: Нужен ли allowlist per-interface (например, VS Code не может менять часть dev-only по умолчанию)?
Да, но без ограничения “по возможности”, а как явная защитная шторка (opt-in) для dev-only.

Решение:
- `envOverrides` (stable) принимаются всегда (VS Code + LSP + bsl-agent).
- `devEnvOverrides` (dev-only) принимаются только при явном opt-in:
  - VS Code: отдельный булевый флаг `bsl.dev.enableDevEnvOverrides` (default: false). Пока false — `devEnvOverrides` игнорируется с warning.
  - bsl-agent: параметр `allow_dev_overrides` в tool/runtime update (default: false). Пока false — `devEnvOverrides` игнорируется с warning.

Причины:
- dev-only флаги часто ломают детерминизм/производительность и не должны включаться “случайно”,
- opt-in сохраняет требование “управляемы” (включая dev-only), но делает включение осознанным,
- “легко убрать”: удаление dev-only слоя = удаление opt-in + поля `devEnvOverrides` из контрактов без затрагивания stable.
