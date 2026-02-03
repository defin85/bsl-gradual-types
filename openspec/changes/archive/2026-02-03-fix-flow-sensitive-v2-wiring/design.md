# Дизайн: flow-sensitive v2 как управляемая (opt-in) подсистема

## 1) Архитектурные драйверы
- **Предсказуемость**: одинаковое включение/выключение и одинаковая семантика результатов во всех интерфейсах.
- **Инкрементальность**: flow-sensitive вычисления должны быть demand-driven (salsa) и не запускаться при default OFF.
- **Детерминизм**: привязка “позиция → CFG контекст” должна быть стабильной, без локальных эвристик в consumers.
- **Безопасность/точность**: если контекст недостаточно определён, лучше вернуть базовый результат, чем “ложную уверенность”.

## 2) Canonical CFG в v2 snapshot
### Контракт
- `SemanticProgram.cfg` всегда `Some(ControlFlowGraph)`.
- Для файлов без исполняемых конструкций CFG содержит минимум:
  - узлы `Entry`, `Exit`,
  - ребро `Entry -> Exit`.

### Причина
Это убирает класс “молча ничего не работает” из-за `None` и позволяет унифицировать consumers: они работают всегда, но могут вернуть “нет подходящего узла/сужения”.

## 3) Единый API: `node_at_byte_offset(offset, bias)`
### Контракт
`ControlFlowGraph` предоставляет детерминированный алгоритм выбора CFG-узла по byte offset, с bias:
- `Exact`: выбрать “самый специфичный” (минимальный span), содержащий offset;
- `PreferLeft`: для границ токена (completion на `.`) интерпретировать позицию как относящуюся к выражению слева;
- `PreferRight`: симметрично (если потребуется).

### Почему это важно
Алгоритм выбора узла должен быть централизован, иначе:
- разные consumers начинают расходиться по поведению,
- появляются эвристики вида “32 байта назад”, которые нестабильны и плохо тестируются.

## 4) Flow-sensitive как v2-only queries (и только по запросу)
### Базовый принцип
Flow-sensitive вычисления не являются частью “обычного” ответа: они должны быть отдельными ветками в v2 queries/handlers, вызываемыми только если effective флаг включён.

### Предлагаемый интерфейс
- `flow_type_at_byte_offset(file_id, byte_offset, bias) -> Option<TypeResolution>`
  - использует CFG + narrowing engine;
  - если нет применимого контекста/сужения — возвращает `None` (consumer делает fallback на базовый `type_at_byte_offset`).
- `semantic_diagnostics_flow_sensitive(file_id) -> Vec<TypeDiagnostic>`
  - включает null-safety (и прочие flow-sensitive правила), но вызывается только при включении.

Гейтинг осуществляется на уровне адаптеров (LSP/Web API/MCP): при OFF они вызывают только базовые queries.

## 5) Контракты включения по интерфейсам
- **LSP (IDE)**: `enableFlowSensitive` (workspace setting), default `false`. При `false` сервер не выполняет flow-sensitive queries.
- **Web API**: параметр запроса `includeFlowSensitive` (camelCase), default `false`. Legacy `include_flow_sensitive` — явный `400 Bad Request`, чтобы не было “тихого игнора”.
- **MCP**: параметр `include_flow_sensitive` (snake_case), default `false`. В ответах должен быть явный индикатор effective режима (чтобы отличать “режим OFF” от “сужение не применилось”).

## 6) План внедрения (в рамках задач)
1) Довести CFG контракт (always present) и API `node_at_byte_offset`.
2) Поднять flow-sensitive v2 queries с явным gating.
3) Перевести LSP/Web API/MCP на единый контракт и удалить локальные эвристики.
4) Закрепить тестами ON/OFF и позиционирование (границы токенов, пустые ветки, циклы).

