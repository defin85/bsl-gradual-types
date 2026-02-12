## Context
После внедрения descriptor-based implicit типизации есть базовый контракт "symbol объявлен и типизирован", но completion-контракт по members implicit symbols остаётся неявным.

В результате возможны рассинхронизации:
- symbol присутствует в type hints, но `completion` на `symbol.` выдаёт неполный набор;
- `hover/diagnostics` и `completion` используют разные lookup paths;
- для `FormModule.Объект` не закреплён явный порядок, как сочетаются форма, intrinsic и applied facet members.

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
Для `FormModule.Объект` порядок источников members фиксируется:
1. form shape members (реквизиты формы, табличные части и релевантные элементы),
2. intrinsic supplement (минимальный whitelist гарантированных свойств),
3. applied object facet lookup (свойства и методы).

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
- детерминированную дедупликацию (case-insensitive canonical key),
- стабильную классификацию kind (property vs method).

### 5) Контексты `*БезКонтекста`
В процедурах/функциях `&НаСервереБезКонтекста` и эквивалентных context-free режимах context-bound implicit symbols MUST NOT предлагаться в non-member completion.

## Risks / Trade-offs
- Риск: рост количества candidates и ухудшение signal/noise.
  - Mitigation: dedupe + deterministic ordering + whitelist для intrinsic.
- Риск: расхождение completion и diagnostics при частичной интеграции.
  - Mitigation: единый owner resolution path и e2e regression matrix.
- Риск: избыточное расширение intrinsic списка без платформенного обоснования.
  - Mitigation: только whitelist + review через platform reference.
