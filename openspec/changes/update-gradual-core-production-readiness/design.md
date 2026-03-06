# Design: update-gradual-core-production-readiness

## Context
Сейчас проект сильнее в архитектурном понимании правильной gradual-системы, чем в полной доставке этого понимания в shared runtime contract.

Подтверждённые симптомы:
- `Certainty` / `UncertaintyReason` и graceful degradation уже формализованы;
- часть acceptance/parity тестов реально ловит drift;
- в design/spec уже закреплена цель единого resolved path;
- при этом snapshot-local structural knowledge ещё не стало first-class shared truth;
- completion сохраняет локальные semantic branches;
- отчётность change может расходиться с фактическим критическим backlog.

## Goals
- Сохранить архитектурный вывод как формальный future contract, а не только как текст ревью.
- Зафиксировать критерии, при которых ядро gradual typing можно будет честно считать production-grade.
- Связать архитектурную готовность и delivery honesty одним change.

## Non-Goals
- Не реализовывать этот контракт в рамках данного change.
- Не дублировать текущий `add-v2-universal-collection-schema-resolution` и его follow-up epic task-by-task.
- Не предписывать немедленный выбор конкретной структуры данных, если соблюдён shared-contract result.

## Decisions

### 1. Shared structural knowledge MUST стать first-class contract
Typed `Структура` и typed-row не могут считаться fully shared semantics, пока их member knowledge не живёт в общем контракте как first-class данные.

Минимальный required payload для structural member:
- canonical member name;
- stable identity;
- member type;
- certainty;
- source span / source location.

Representation вида `Структура<vec<ConcreteType>>` или generic lookup только по `base_type` недостаточна для роли shared truth.

### 2. Semantic consumers MUST использовать один resolved path
`completion`, `hover`, `type-at-position`, `semantic diagnostics`, а также adapter surfaces (`LSP`, `MCP`, Web helpers) должны читать owner/type из одного semantic contract.

Допустимы только thin adapters:
- меняют формат ответа;
- не вводят отдельную schema/effect truth;
- не выполняют consumer-local inference как новый источник смысла.

Если временные исключения остаются, они должны быть:
- явно перечислены;
- покрыты migration plan;
- removed-by-default целью, а не бессрочной особенностью.

### 3. Acceptance MUST доказывать shared semantics, а не только отсутствие явного drift
Smoke/parity проверки полезны, но недостаточны как единственное доказательство общей модели знания.

Production-grade acceptance должна уметь проверять как минимум:
- один и тот же owner resolution результат;
- одну и ту же member identity;
- отсутствие hidden consumer-only hints как условия корректного результата;
- одинаковую policy реакцию на known/unknown member.

### 4. Delivery readiness MUST быть честной относительно MUST backlog
Если review выявил, что MUST-требования change фактически не доставлены, и для этого создан критический follow-up backlog, исходный change не должен продолжать жить в статусе “complete” только на основании закрытых checklist items.

Нужен readiness gate, который сверяет:
- OpenSpec status / checklist;
- traceability matrix;
- review-gate verdict;
- критический Beads backlog, созданный для закрытия тех же MUST-требований.

### 5. Этот change future-facing и зависит от более узких remediation changes
Текущий change не заменяет remediation-level change/epic. Он фиксирует более широкий стандарт готовности, к которому должны прийти follow-up работы.

## Alternatives Considered

### Оставить анализ только в review-комментарии
Rejected.
Такой вывод быстро теряется и не становится частью change governance.

### Ограничиться только product-spec без dev-workflow части
Rejected.
Тогда теряется ключевой вывод про расхождение между declared completion и реальной readiness.

### Вынести только process-гейт без архитектурного контракта
Rejected.
Это решает honesty вопрос, но не сохраняет самую важную technical target state.

## Risks / Trade-offs
- Change объединяет архитектурную и процессную тему.
  - Mitigation: scope ограничен readiness contract и не уходит в implementation details.
- Возможен overlap с текущими active changes.
  - Mitigation: этот change явно future-facing и не заменяет remediation work, а задаёт следующий критерий зрелости.

## Migration Plan
1. Утвердить future readiness contract.
2. Использовать его как критерий для follow-up changes в `bsl-intellisense-v2`.
3. После реализации remediation work добавить governance gate, который связывает OpenSpec completion с реальным MUST backlog.

## Open Questions
- Что окажется устойчивее как shared representation: расширенный `TypeResolution` или отдельный explicit structural sidecar contract?
- Должен ли readiness gate быть автоматизирован через tooling, или на первом этапе достаточно обязательного review artifact?
