## 1. Specification
- [ ] 1.1 Добавить в `bsl-intellisense-ide-grade` функциональный GA baseline для `conf_big`.
- [ ] 1.2 Зафиксировать в `bsl-intellisense-ide-grade` запрет на FP `Необъявленная переменная` для поддерживаемых context implicit symbols.
- [ ] 1.3 Зафиксировать в `bsl-intellisense-ide-grade` e2e-контракт typed rows `ТаблицаЗначений` (`completion/hover/diagnostics`).
- [ ] 1.4 Добавить в `bsl-intellisense-v2` требование сквозной согласованности snapshot-результатов для implicit symbols и value-table schema-effects.
- [ ] 1.5 Зафиксировать зависимость от change `update-v2-contextual-implicit-variables` и `add-v2-valuetable-column-resolution`.

## 2. Design
- [ ] 2.1 Описать функциональный acceptance-профиль на `examples/conf_big` (smoke-кейсы + expected diagnostics profile).
- [ ] 2.2 Описать стратегию стабилизации поведения на неполном коде без деградации UX.
- [ ] 2.3 Описать контракт "единый v2 snapshot -> единые ответы diagnostics/hover/completion" для продаваемого качества.

## 3. Validation
- [ ] 3.1 `openspec validate add-lsp-functional-ga-readiness --strict --no-interactive`.
- [ ] 3.2 Провести review change с владельцами `analysis-v2`, LSP и diagnostics.
