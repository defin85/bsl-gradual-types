# Change: Функциональная GA-ready матрица для LSP IntelliSense

## Why
Перед выходом на продажи основным риском остается не деплой, а функциональная предсказуемость LSP на реальных 1С-кодовых базах: ложные diagnostics в платформенных контекстах, неполный резолв колонок `ТаблицаЗначений`, нестабильность поведения на неполном коде и недостаточно формализованный acceptance-бейзлайн на `conf_big`.

Нужна формальная спецификация "что считается функционально готовым к продаже", чтобы команда могла проверять релиз не по субъективному впечатлению, а по детерминированным критериям качества IntelliSense.

## What Changes
- Добавить spec delta в `bsl-intellisense-ide-grade` для функционального GA baseline:
  - обязательный acceptance-профиль на `examples/conf_big`;
  - запрет на FP `Необъявленная переменная` для поддерживаемых context implicit symbols;
  - e2e-поведение `completion/hover/diagnostics` для typed rows `ТаблицаЗначений`;
  - обязательная устойчивость на неполном коде без потери базовой функциональности.
- Добавить spec delta в `bsl-intellisense-v2` для сквозной согласованности snapshot-уровня:
  - общие правила для implicit symbols и schema-effects `ТаблицаЗначений` должны давать согласованные результаты во всех v2 consumers.
- Явно зафиксировать, что change зависит от уже активных:
  - `update-v2-contextual-implicit-variables`,
  - `add-v2-valuetable-column-resolution`.

## Impact
- Affected specs:
  - `bsl-intellisense-ide-grade`
  - `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `analysis-v2/**`
  - `semantic-diagnostics/**`
  - `backend/src/lsp/**`
  - тестовые наборы на `examples/conf_big/**`

## Non-Goals
- Релизная автоматизация, маркетплейс-публикация и лицензионный деплой.
- Изменение протоколов лицензирования (см. `add-tpm-lease-licensing`).
- Добавление новых UI-функций, не связанных с качеством IntelliSense/LSP.
