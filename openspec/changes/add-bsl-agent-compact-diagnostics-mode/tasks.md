## 1. Contract
- [ ] 1.1 Расширить контракт `bsl_diagnostics_start` opt-in параметрами shaping:
  - [ ] `compact: bool = false`
  - [ ] `group_by: none|message`
  - [ ] `omit_null_fields: bool = false`
  - [ ] `omit_repeated_file: bool = false`
  - [ ] `severity_filter: error|warning|info`
- [ ] 1.2 Зафиксировать compact response shape:
  - [ ] `summary.errors|warnings|infos|unique_messages`
  - [ ] `common_file` для single-file effective result set
  - [ ] `groups[]` для `group_by=message`
- [ ] 1.3 Явно зафиксировать backward compatibility:
  - [ ] default `compact=false` сохраняет текущий flat payload
  - [ ] `include_impact`/`include_coverage` не получают новых user-facing гарантий в рамках этого change

## 2. Implementation
- [ ] 2.1 Обновить request/response DTO и JSON schema для `bsl_diagnostics_start`.
- [ ] 2.2 Реализовать `summary` и deterministic compact serialization.
- [ ] 2.3 Реализовать `severity_filter` как read-only shaping без изменения анализа.
- [ ] 2.4 Реализовать `omit_null_fields=true` так, чтобы `null` поля реально исчезали из JSON.
- [ ] 2.5 Реализовать `omit_repeated_file=true` через top-level `common_file`, если все возвращённые diagnostics относятся к одному файлу.
- [ ] 2.6 Реализовать `group_by=message` с детерминированной группировкой и compact occurrences.
- [ ] 2.7 Обновить `mcp_help` и README примерами compact режима.

## 3. Tests
- [ ] 3.1 Добавить stdio regression: default `bsl_diagnostics_start` остаётся backward-compatible.
- [ ] 3.2 Добавить stdio regression: `compact=true` возвращает `summary` и compact flat payload.
- [ ] 3.3 Добавить stdio regression: `group_by=message` возвращает groups без flat duplication.
- [ ] 3.4 Добавить stdio regression: `severity_filter=warning` оставляет только warning diagnostics и согласованную summary.
- [ ] 3.5 Добавить stdio regression: `omit_repeated_file=true` hoist-ит `common_file` для single-file effective result set.
- [ ] 3.6 Добавить regression на `omit_null_fields=true`, чтобы `code: null` не сериализовался.

## 4. Validation
- [ ] 4.1 Прогнать `openspec validate add-bsl-agent-compact-diagnostics-mode --strict --no-interactive`.
- [ ] 4.2 Подготовить traceability `Requirement -> Code -> Test` для compact diagnostics mode.
