## Context
`Completion Timeline v7` уже экспортирует bounded pre-method split, но producer path использует два источника request metadata:
- task-local request context в normal service scope;
- pending completion registry как fallback.

Проблема в том, что fallback сейчас привязан к позиции completion-запроса. При burst/overlap completion на одном и том же `uri + position` оператор может получить убедительно выглядящий pre-method split, который на самом деле не доказан для этого `request_id`.

При этом нам не нужен широкий рефакторинг LSP transport. Нужен узкий bounded контракт, который:
- сохраняет existing observability discipline;
- не рисует guessed root cause;
- позволяет человеку и derived summary отличать strong same-request attribution от weak fallback.

## Goals / Non-Goals
- Goals:
  - сделать pre-method attribution request-aware и fail-closed по integrity;
  - добавить bounded provenance для pre-method facts;
  - синхронизировать правила strong/weak attribution между server payload и extension surfaces.
- Non-Goals:
  - не исправлять сам scheduler/service backlog;
  - не добавлять unbounded request-debug logs;
  - не менять completion result contract.

## Decisions

### 1. Поднять contract до `v8`
Новый change использует additive `v8` contract для authoritative completion timeline. Это нужно, чтобы extension мог явно различать:
- `v7`: pre-method split без integrity provenance;
- `v8`: тот же split, но уже с bounded source/confidence semantics.

### 2. Ввести bounded provenance для pre-method attribution
Для pre-method facts вводится bounded provenance vocabulary. Минимально нам нужно различать:
- request-bound authoritative source;
- best-effort fallback source.

Идея change не в том, чтобы обязательно удалить fallback, а в том, чтобы больше не выдавать его за same-request факт.

### 3. Fail-closed semantics для сильных ingress verdicts
Human-readable surfaces и incident summary могут считать `server_before_method_entry_dominant` сильным выводом только тогда, когда pre-method attribution provenance подтверждает same-request binding.

Если provenance best-effort или unavailable:
- raw fields могут быть показаны только в рамках contract semantics;
- derived summary не должен агрегировать такой trace как сильный ingress finding;
- UI должен явно помечать lowered confidence, а не молча смешивать strong и weak cases.

### 4. Сначала integrity, потом deeper lag narrowing
Этот change не пытается сразу ещё глубже расколоть `transport_received -> service_scope_entered`. Пока есть подозрение на cross-request attribution, более тонкий split даст ложную точность. Сначала нужен trustworthy handoff.

## Alternatives Considered

### Полностью убрать fallback и публиковать только task-local attribution
Плюс: максимально строгая integrity semantics.
Минус: можем потерять полезный bounded факт там, где task-local не доживает до consumer path, хотя best-effort signal всё ещё полезен для ручного анализа.

Решение: не запрещать fallback полностью, а явно маркировать его как weak/best-effort.

### Оставить текущий contract и чинить только summary heuristics
Плюс: минимально по коду.
Минус: не решает корневую проблему недоказанного producer provenance; raw payload останется вводящим в заблуждение.

Решение: нужен server-side contract change, а не только UI-level patch.

## Risks / Trade-offs
- Новый contract bump добавляет ещё одну explicit degradation ветку в extension.
- Если fallback path окажется часто используемым, часть текущих ingress findings станет слабее или исчезнет, пока не появится better request-bound propagation. Это ожидаемое ужесточение, а не regression.

## Migration Plan
1. Зафиксировать `v8` provenance semantics.
2. Протянуть provenance через request-context handoff и timeline serialization.
3. Обновить extension consumers и incident aggregation.
4. Добавить overlapping-request regression tests и smoke expectations.

## Open Questions
- Нужен ли один bounded field `pre_method_attribution_source`, или лучше сразу раскладывать его на `source` и `confidence`.
- Можно ли перевести fallback с position-keyed handoff на request-id-keyed handoff без дополнительного transport refactor.
