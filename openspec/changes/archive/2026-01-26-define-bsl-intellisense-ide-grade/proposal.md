# Change: define-bsl-intellisense-ide-grade

## Why
В репозитории есть:
- текущий “core” IntelliSense (LSP: completion/hover/signatureHelp/diagnostics и частичный definition),
- roadmap IntelliSense v2 (как план),
- набор отдельных proposal’ов на недостающие IDE‑grade функции (symbols, references/rename, форматирование, провайдеры VS Code).

При этом отсутствует единый **целевой контракт** “IDE‑grade IntelliSense для BSL (1С)”, который:
- формализует ожидания (MUST/SHOULD),
- отделяет целевое состояние от текущей реализации,
- служит якорем для декомпозиции работ и проверки прогресса.

## What Changes
- Создать новую спецификацию‑цель `bsl-intellisense-ide-grade` (north star), описывающую идеальный/целевой IntelliSense для VS Code (и совместимую с будущими LSP‑клиентами).
- Зафиксировать MUST/SHOULD по ключевым IDE‑grade возможностям (symbols/references/rename/formatting/code actions/inlay hints) и нефункциональные требования (инкрементальность, отмена, отсутствие блокирующего I/O, детерминизм).
- Сопоставить целевые требования с текущими активными change‑proposal’ами и обозначить явные пробелы.

## Impact
- Спецификация: новая capability `openspec/specs/bsl-intellisense-ide-grade/spec.md` (через change).
- Код: **без изменений** (это define/spec change).
- Планирование: активные changes по реализации будут ссылаться на этот target‑spec.
