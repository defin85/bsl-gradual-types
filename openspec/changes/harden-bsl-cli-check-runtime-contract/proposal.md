# Change: Harden `bsl-cli check` runtime contract

## Why

`bsl-cli check` уже оказался полезным как быстрый single-file smoke для BSL diagnostics, но текущий контракт этой поверхности остается неявным. В частности, diagnostic details завязаны на `--verbose`, `--format` не является полноценным контрактом вывода, runtime запускается без явного configuration root, а diagnostics path не прогревает exact type index.

Из-за этого легко переоценить доказательную силу команды: успешный single-file запуск можно ошибочно принять за live/LSP verification, parse configuration proof или exact snapshot proof. Для product-grade CLI surface нужен честный, машинно проверяемый контракт: что именно загружено, какой режим использован, какие diagnostics получены, и какие ограничения у результата.

## What Changes

- Зафиксировать `bsl-cli check` как first-class diagnostics surface с документированным output/runtime contract.
- Сделать machine-readable `--format json` пригодным для automation без скрытой зависимости от `--verbose`.
- Зафиксировать human output contract для диагностик, чтобы формат и флаги не скрывали сами diagnostics.
- Добавить runtime evidence metadata: наличие/отсутствие configuration root, статус загрузки конфигурации, rules config, syntax helper, и применимость exact type-index warmup.
- Добавить явный config-root/workspace input для `bsl-cli check` либо формально описать существующий input, если он уже покрывает этот сценарий.
- Сохранить честную границу: single-file CLI diagnostics не считаются LSP/live/config-parse proof без соответствующего explicit mode и evidence.
- Добавить интеграционные smoke checks на `bsl-cli check`, включая regression на `examples/conf_big/CommonModules/АвансовыйОтчетФормы/Ext/Module.bsl`.

## Impact

- Affected specs: `bsl-intellisense-v2`, `dev-workflow`
- Affected code:
  - `cli/src/args.rs`
  - `cli/src/main.rs`
  - `cli/src/runtime.rs`
  - CLI diagnostics/output DTOs or formatters
  - runtime startup/deps metadata exposed to CLI
  - docs/agent verification guidance and smoke scripts
