# Change: Ввести versioned contract слой `contracts/**` для внешних интерфейсов

## Why
В репозитории нет явного versioned contract слоя для внешних интерфейсов (LSP/Web/MCP/observability labels).
Из-за этого регрессии совместимости обнаруживаются поздно: уже после изменения кода и тестов, когда поведение дрейфует, но формального источника истины для внешнего контракта нет.

Нужен отдельный change, который фиксирует структуру и правила для `contracts/**`, чтобы:
- отделить контракт (public surface) от реализации;
- сделать изменения контракта ревью‑и‑CI‑контролируемыми;
- стандартизовать version bump policy (breaking vs non-breaking).

## What Changes
- **ADDED**: requirement в `dev-workflow` про обязательный versioned слой `contracts/**` для внешних интерфейсов.
- **ADDED**: requirement в `dev-workflow` про compatibility policy и version bump правила для контрактов.
- **ADDED**: requirement в `bsl-intellisense-v2` про обязательный versioned contract для completion v2 и связанных observability метрик интерактивного пути.
- Фиксируется начальная область применения контрактов:
  - LSP completion contract (trigger modes/outcomes),
  - observability contract (ключевые метрики completion v2),
  - Web/MCP surface — поэтапно, следующими change.

## Impact
- Affected specs:
  - `dev-workflow`
  - `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `contracts/**` (новый versioned каталог),
  - CI/quality gates для проверки контрактов,
  - тесты и snapshot/fixture проверки, которые привязаны к contract версии.

## Non-Goals
- Немедленная контрактация всех существующих API/метрик в репозитории.
- Переписывание текущего completion pipeline или event-driven архитектуры.
- Автоматическая генерация всех контрактов из кода в рамках этого change (допустим ручной baseline + проверка).
