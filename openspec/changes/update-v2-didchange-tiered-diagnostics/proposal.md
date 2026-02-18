# Change: Update v2 didChange tiered diagnostics

## Why
Последние метрики показывают устойчивую деградацию интерактивного пути при редактировании:
- `syntax_diagnostics_query` на warm-path держится в районе `~2.4-2.7s` (`p50/p95`);
- `completion_duration_ms` и `runtime_queue_wait_interactive_ms` имеют высокий хвост;
- при серии `didChange` видно несколько подряд тяжёлых запусков для последовательных версий (`expected_version=42,43,44...`), что указывает на доработку устаревших вычислений.

Текущий debounce снижает частоту стартов, но не гарантирует раннюю остановку уже запущенного тяжёлого пайплайна для устаревшей версии.

## What Changes
- Ввести tiered-профили диагностики для LSP:
  - `didChange` запускает только дешёвый/инкрементальный fast-path;
  - тяжёлые стадии запускаются отложенно (`debounce`) и в background;
  - самые дорогие проверки выполняются только по `didSave` и/или `idle`.
- Зафиксировать version-bound отмену для диагностики:
  - новая версия документа MUST supersede старые задачи;
  - устаревшие задачи MUST прерываться до входа в следующую дорогую стадию;
  - публикация разрешена только для актуального revision token.
- Уточнить strict-latest publish контракт: проверка не только `(file_version, deps_id, settings_id)`, но и `diagnostics_generation` (или эквивалентный revision token).
- Расширить observability канонического контракта для triage:
  - фиксировать `trigger` (`did_change|did_save|idle|documents_set|job_start`);
  - фиксировать `profile` (`fast|debounced_full|idle_heavy`);
  - фиксировать discard/cancel причины устаревания.
- Распространить ту же модель на `bsl-agent`:
  - `documents_set` обновляет revision и отменяет устаревшие batch-задачи;
  - интерактивные инструменты остаются fast/interactive, тяжёлые проверки — background/deferred.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
  - `mcp-bsl-agent`
- Affected code (implementation stage):
  - `backend/src/bin/lsp_server/server/language_server.rs`
  - `backend/src/bin/lsp_server/server/core.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade.rs`
  - `bsl-runtime/src/application/intellisense_v2/policy.rs`
  - `bsl-runtime/src/system/basic_observability.rs`
  - `bsl-agent/src/session/mod.rs`

## Assumptions
- Под «дорогими проверками» в первую очередь понимаются full `syntax_diagnostics`, flow-sensitive semantic проверки и другие CPU-heavy шаги, не критичные для мгновенной обратной связи на каждый символ.
- Если клиент не присылает `didSave`, роль триггера для heavy-профиля берёт на себя `idle` таймер.
