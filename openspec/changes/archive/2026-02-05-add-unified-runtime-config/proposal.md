# Change: Unified runtime config & observability controls (LSP + bsl-agent)

## Why
- В проекте есть дрейф настроек между VS Code settings, LSP `initializationOptions`, переменными окружения `BSL_*` и конфигом `bsl-agent`.
- Большинство `BSL_*` параметров читаются только из env на старте (часто через `LazyLock/OnceLock`), поэтому менять их без рестарта процесса невозможно.
- Нужно обеспечить управление **всеми runtime `BSL_*` параметрами** (включая dev-only) из:
  - VS Code Settings (для LSP),
  - `bsl-agent` MCP tools (для агент-сессии),
  - и применять изменения **в рантайме** без перезапуска сессии/процесса.

## What Changes
- Добавить единый реестр runtime-конфигурации для всех `BSL_*`, с типами/дефолтами/описаниями и пометкой **tier**:
  - `stable` (пользовательские/production),
  - `dev-only` (временные/отладочные; должны быть легко удаляемы).
- Сделать единый механизм загрузки и мерджа настроек:
  - `defaults` < `env` (bootstrap) < `runtime overrides` (VS Code / bsl-agent).
- VS Code extension:
  - добавить settings для overrides (stable и отдельно dev-only),
  - прокидывать overrides в LSP через `workspace/didChangeConfiguration`,
  - продолжать поддерживать `BSL_*` env как bootstrap-совместимость (не ломать существующие сценарии).
- LSP server:
  - заменить прямое чтение `std::env::var("BSL_*")` на чтение из runtime-config store,
  - применить `didChangeConfiguration` так, чтобы все новые overrides начинали работать без рестарта.
- `bsl-agent`:
  - добавить MCP tool для runtime update настроек (без перезапуска сессии),
  - выровнять входной JSON со схемой LSP settings (чтобы один и тот же payload работал в обоих интерфейсах).
- Observability/metrics:
  - сделать управление сбором и экспортом метрик/трассировок частью единого runtime-config (включая dev-only флаги).

## Scope Notes
- Под "все `BSL_*`" в рамках этого change понимаются **runtime переменные**, которые читаются через `std::env::var`/эквивалент.
- Compile-time переменные (используемые через `env!`/build.rs, например `BSL_AGENT_GIT_SHA`) не могут быть изменены в рантайме и остаются read-only; они будут отражаться только в build/info.

## Impact
- Affected specs:
  - `mcp-bsl-agent` (runtime update + settings contract),
  - `bsl-intellisense-v2` (runtime tunables apply without restart),
  - **NEW** `bsl-runtime-config` (единый контракт конфигурации и tiering).
- Affected code (high-level):
  - `bsl-runtime` (все места чтения `BSL_*`),
  - `backend` LSP (конфиг и runtime updates),
  - `bsl-agent` (session settings + new tool),
  - `vscode-extension` (settings schema + прокидывание overrides).

