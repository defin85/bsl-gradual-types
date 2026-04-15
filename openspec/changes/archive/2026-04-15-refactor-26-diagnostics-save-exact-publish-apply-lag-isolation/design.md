## Context

После `refactor-25` live `conf_big` evidence перестало указывать на `stale_parser_base`, но
`didSave` heavy follow-up всё ещё может не вернуться к `ready_artifacts`.

Текущий residual profile выглядит так:

- `followup_ready_snapshot_zero_probe=not_ready`;
- `followup_ready_snapshot_wait_probe=timeout`;
- `followup_ready_snapshot_timeout_phase=parse_exec`;
- затем `followup_ready_snapshot_relief_valve_outcome=skipped_apply_lag`;
- cycle завершает semantic path через `shadow_state`.

Это значит, что система уже truthfully различает parse-phase и apply-lag phase, но следующий
runtime fix должен атаковать именно bridge между exact ready artifacts и final follow-up publish.

## Goals / Non-Goals

- Goals:
  - не держать exact same-version heavy follow-up hostage из-за writer/apply lag, если matching
    ready artifacts уже доказаны;
  - различать pre-ready apply lag и post-ready publish gating в observability;
  - сохранить exactness-first / newest-save-wins semantics.
- Non-Goals:
  - не поднимать relief valve budget как основной механизм;
  - не ослаблять proof requirements для matching `(file_id, version, text_hash)`;
  - не переписывать `refactor-25` parser-base recovery path.

## Decisions

### Decision: usable exact ready artifacts должны быть publish-authoritative для follow-up

Если runtime уже имеет exact same-version ready artifacts для matching current text hash, heavy
follow-up не должен ждать writer-owned apply просто для того, чтобы получить право публиковать
semantic diagnostics.

Это не означает отказ от applied state в целом; это означает, что ready-artifact path должен иметь
собственный bounded publish route, когда exact proof уже есть.

### Decision: `apply_lag` после exact readiness должен перестать быть catch-all label

Текущая truthful telemetry уже говорит оператору, что `apply_lag` есть. Этого недостаточно, если
same-version ready artifacts уже готовы, а проблема сидит уже между post-parse readiness и actual
follow-up publish.

Новый change должен либо убрать этот gate, либо хотя бы назвать его отдельно, чтобы `apply_lag`
оставался label только для случая "writer apply действительно primary blocker".

### Decision: fallback остаётся fail-closed

Если exact ready artifacts ещё не доказаны, stale, или superseded новым save-cycle, система
должна сохранить текущий truthful fallback вместо speculative publish.

## Alternatives Considered

### Raise `didSave` wait budgets again

Rejected. Это лишь растягивает latency вокруг того же residual path.

### Publish from shadow state and stop caring about exact ready artifacts

Rejected. Это operationally quieter fallback, а не исправление exact path.

### Always wait for writer-owned apply before any heavy follow-up publish

Rejected. Это как раз тот bottleneck, который текущий change должен либо снять, либо явно
ограничить.

## Risks / Trade-offs

- Publish path, оторванный от writer apply, опасен, если proof current text hash or version drift
  будет неполным.
- Если actual blocker сидит не в apply gate, а в later publish/admission handoff, change может
  сначала дать только better attribution before behavior change.
- `conf_big` live profile может остаться на fallback даже после fix; в таком случае новый bounded
  blocker должен быть explicit, а не замазан обратно в generic `apply_lag`.

## Migration Plan

1. Разделить attribution между pre-ready apply lag и post-ready publish gate.
2. Добавить exact publish route, который может использовать same-version ready artifacts без blind
   wait на writer apply.
3. Переснять `conf_big` mixed-load evidence и зафиксировать, вернулся ли path к `ready_artifacts`.
