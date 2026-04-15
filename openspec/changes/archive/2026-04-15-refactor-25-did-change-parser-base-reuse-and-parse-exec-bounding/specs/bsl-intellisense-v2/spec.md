## ADDED Requirements

### Requirement: Ranged `didChange` MUST attempt bounded parser-base recovery before treating `ready_snapshot_lags_shadow_state` as full-parse fate

Система MUST сначала попытаться выполнить bounded parser-base recovery / prime path для текущего
exact document state, если ranged `didChange` попадает в `fallback_reason=stale_parser_base` и
bounded root-cause attribution указывает на `ready_snapshot_lags_shadow_state`.

Этот recovery path MUST:

- оставаться version- and text-bound для exact target revision;
- не использовать raw debug-only state как substitute для matching parser base;
- сохранять существующий truthful fallback, если matching parser base всё ещё не может быть
  доказан.

Система MUST NOT считать `ready_snapshot_lags_shadow_state` достаточной причиной для немедленного
безусловного full parse, если bounded recovery path ещё не был исчерпан.

#### Scenario: Lagging ready snapshot восстанавливает exact parser base без немедленного full parse

- **GIVEN** ranged `didChange` для revision `V+1` видит `shadow_state` на той же revision
- **AND** bounded miss attribution для старого reuse path равен `ready_snapshot_lags_shadow_state`
- **WHEN** runtime запускает bounded parser-base recovery для exact target revision
- **THEN** runtime сначала пытается восстановить/prime matching parser base для этой revision
- **AND** если recovery succeeds, exact ready-snapshot build продолжается без forced full parse из
  этого miss класса

#### Scenario: Recovery failure сохраняет truthful fallback

- **GIVEN** ranged `didChange` попал в miss class `ready_snapshot_lags_shadow_state`
- **AND** bounded parser-base recovery не смог доказать matching parser base
- **WHEN** runtime завершает выбор parse path
- **THEN** система MAY перейти к существующему truthful full fallback
- **AND** observability сохраняет bounded attribution того, что recovery был исчерпан, а exact
  parser base так и не был доказан

### Requirement: Same-file exact ready-snapshot work MUST bound obsolete parse-exec waste

Система MUST иметь bounded cancellation/retarget observation inside the expensive parse/build path,
если same-file churn retargets/coalesces exact ready-snapshot producer на более новую revision,
чтобы obsolete worker мог завершиться во время `parse_exec`, а не только после почти полного
завершения parse cost.

Этот behavior MUST:

- сохранять exact same-version guarantees для surviving worker;
- различать lifecycle/metrics причины как минимум между abort `during_parse_exec` и loss
  `before_materialization`;
- не обвинять `ready_install` или `documentSymbol` phases в obsolete work, которое фактически было
  потрачено внутри parse execution.

#### Scenario: Более новая same-file revision останавливает obsolete worker во время parse execution

- **GIVEN** exact ready-snapshot worker уже выполняет дорогой parse/build path для revision `V`
- **AND** тот же файл получает более новую revision `V+1`, retargeting producer на новый target
- **WHEN** obsolete worker достигает следующего bounded cancellation/retarget checkpoint inside
  `parse_exec`
- **THEN** obsolete worker завершается без materializing revision `V`
- **AND** lifecycle attribution / metrics показывают bounded parse-exec abort, а не поздний
  post-parse loss

#### Scenario: Отсутствие retarget не ломает normal exact materialization

- **GIVEN** exact ready-snapshot worker выполняет parse/build path для current revision
- **AND** новых same-file revisions не приходит до конца build path
- **WHEN** parse/build successfully finishes
- **THEN** ready snapshot materializes как и раньше
- **AND** новые cancellation checkpoints не меняют exact success semantics
