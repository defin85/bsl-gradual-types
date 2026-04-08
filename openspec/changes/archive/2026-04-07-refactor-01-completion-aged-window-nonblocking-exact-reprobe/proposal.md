# Change: шаг 1 убрать blocking exact re-probe из aged completion window

## Почему

Incident bundle `2026-04-07T09:36:20Z` показал, что после закрытия front-edge `exact_deadline` остаётся отдельная aged-path регрессия:

- `completion-trace-5` завершился `ok_non_empty`, но занял `1590ms`;
- внутри него `server_handler_exec_ms=1590`, тогда как видимые timeline stages покрывают только `154ms`;
- скрытый handler gap `1436ms` делает verdict уровня `handler_prelude_dominant` недостоверным;
- healthy `completion-trace-6` (`head_hit`, `8ms`) подтверждает, что front-edge remediation уже сработал, а residual latency сидит в другом path.

Текущая реализация aged non-member `shadow_current_revision_fast_path` синхронно делает
`completion_current_revision_snapshot_for_origin_and_operation(...).await` до `query_bundle_started`.
Это одновременно:

- блокирует first response после immediate window;
- прячет real wait вне stage taxonomy;
- позволяет timeline обвинять не тот участок handler path.

## Что меняется

- Aged non-member current-revision completion MUST NOT блокировать first response на exact re-probe, если exact не был уже ready в подготовленном current-revision state.
- Вместо blocking re-probe request path MUST сразу переходить в bounded lightweight/no-IR current-revision fallback, сохраняя truthful same-revision semantics.
- Если blocking current-revision snapshot reacquisition где-то остаётся, authoritative completion timeline MUST явно атрибутировать его как отдельный low-cardinality stage или representative validation MUST fail-ить по seconds-scale uncovered handler gap.

## Impact

- Спецификация: `bsl-intellisense-v2`
- Backend/LSP: aged non-member completion path и timeline capture
- Runtime facade: current-revision snapshot acquisition instrumentation/policy
- Validation: incident-like regression coverage и truthful stage-coverage gate

## Не цели

- latest-only/supersession политика для `bsl.getCurrentContext` и cursor-burst auxiliary load
- изменение member-access exact path вне aged non-member current-revision scenario
