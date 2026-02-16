## Context
После внедрения descriptor-based implicit типизации есть базовый контракт "symbol объявлен и типизирован", но completion-контракт по members implicit symbols остаётся неявным.

В результате возможны рассинхронизации:
- symbol присутствует в type hints, но `completion` на `symbol.` выдаёт неполный набор;
- `hover/diagnostics` и `completion` используют разные lookup paths;
- для `FormModule.Объект` не закреплены отдельно collection order и precedence policy;
- не закреплены owner-sensitive правила dedupe и границы интерактивной выдачи (limit/isIncomplete).

## Goals / Non-Goals
- Goals:
  - Зафиксировать обязательный completion-контракт для implicit symbols в supported module contexts.
  - Обеспечить выдачу и свойств, и методов через единый facet-aware path.
  - Закрепить additive-only intrinsic supplement для `FormModule.Объект`.
  - Убрать шумные/невалидные подсказки implicit symbols в `*БезКонтекста`.
- Non-Goals:
  - Не заменять facet metadata эвристиками.
  - Не менять пользовательский формат completion item за пределами текущего контракта.
  - Не расширять scope на неявные runtime-only members, недоступные через metadata/facet model.

## Decisions

### 1) Единый lookup путь для member-completion implicit symbols
Completion MUST брать owner type для implicit symbol из того же v2 snapshot/descriptor контракта, что и hover/type-at-position.
Member list строится через `TypeMetadataLookup` и facet-aware providers, без отдельного completion-only inference пути.

### 2) Provider chain для `FormModule.Объект`
Для `FormModule.Объект` фиксируется collection order источников members:
1. form shape members (реквизиты формы, табличные части и релевантные элементы),
2. intrinsic supplement (минимальный whitelist гарантированных свойств),
3. applied object facet lookup (свойства и методы),
4. fallback-источники (для controlled degradation).

Precedence policy фиксируется отдельно от collection order:
- repository/facet members имеют приоритет выше intrinsic;
- intrinsic никогда не переопределяет repository/facet members;
- конфликтующие members разрешаются по canonical key и precedence policy, а не по случайному порядку обхода.

### 3) Intrinsic supplement policy
Intrinsic слой:
- additive-only (только дополняет);
- whitelist-driven;
- не переопределяет и не скрывает members из facet metadata;
- проходит нормализацию имен и dedupe вместе с другими провайдерами.

### 4) Completion output policy
Выдача member-completion MUST включать:
- свойства,
- методы,
- детерминированную дедупликацию (case-insensitive canonical key + owner-sensitive identity),
- стабильную классификацию kind (property vs method).

Для owner-sensitive dedupe:
- кандидаты с одинаковым `label`/`kind`, но с разными semantic owners, не должны схлопываться в один item без явного правила объединения;
- объединение допустимо только для semantic-equivalent owners.

### 5) Контексты `*БезКонтекста`
В процедурах/функциях `&НаСервереБезКонтекста` и эквивалентных context-free режимах context-bound implicit symbols MUST NOT предлагаться в non-member completion.

### 6) Нефункциональные требования (Interactive latency / bounded output)
- Completion MUST ограничивать выдачу фиксированным limit и выставлять `isIncomplete`, если кандидатов больше лимита.
- Completion MUST иметь детерминированный порядок выдачи в рамках одного snapshot/revision.
- Интерактивная задержка completion для типовых инкрементальных сценариев должна иметь измеримый SLA/SLO и stage-level telemetry:
  - `resolve`,
  - `collect`,
  - `rank`,
  - `format`.

### 7) Rollout / rollback strategy
- Внедрение через feature flag с возможностью canary rollout.
- Rollback должен выполняться переключением флага без изменения клиентского контракта LSP.
- План rollout включает сравнение baseline/new по latency и качеству выдачи на regression matrix.

## Verification Matrix (pre-implementation contract)
- Module contexts matrix: `FormModule` / `ManagerModule` / `ObjectModule` / `RecordSetModule`.
- `*БезКонтекста`: context-bound implicit symbols отсутствуют в non-member completion.
- `FormModule.Объект.`: одновременно присутствуют `shape + intrinsic + facet method`.
- Provider conflict: intrinsic vs repository/facet одноимённые members (repository/facet wins).
- Owner-sensitive dedupe: union/chain cases не теряют валидные candidates.
- Consistency: выбранный completion member не вызывает ложный `NonExistentProperty` в diagnostics.

## Risks / Trade-offs
- Риск: рост количества candidates и ухудшение signal/noise.
  - Mitigation: bounded output + owner-sensitive dedupe + deterministic ordering.
- Риск: расхождение completion и diagnostics при частичной интеграции.
  - Mitigation: единый owner resolution path и e2e regression matrix.
- Риск: избыточное расширение intrinsic списка без платформенного обоснования.
  - Mitigation: только whitelist + review через platform reference.
