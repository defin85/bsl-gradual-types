## ADDED Requirements

### Requirement: Parse snapshot v2 является version-bound и общим для completion/diagnostics (MUST)
Система MUST поддерживать явный `ParseSnapshot` на уровне файла, связанный с конкретной версией документа.

`ParseSnapshot` MUST включать как минимум:
- версию файла, для которой вычислен snapshot;
- parse result и line index;
- дерево parser backend, пригодное для incremental update;
- changed ranges (если snapshot был получен инкрементально).

Completion и diagnostics MUST использовать один и тот же snapshot контракт для одной и той же ревизии.

#### Scenario: Completion и diagnostics читают согласованный parse state
- **GIVEN** документ версии `V` уже применен в pipeline
- **WHEN** параллельно запрашиваются completion и diagnostics для `V`
- **THEN** обе операции используют общий `ParseSnapshot(V)`
- **AND** система не создает независимые несовместимые parse состояния для одного `file_id/version`

### Requirement: didChange path использует incremental parse с fail-safe full fallback (MUST)
На `didChange` система MUST пытаться обновлять parse state инкрементально через старое дерево и edit chain.

Если incremental path не может быть применен корректно, система MUST детерминированно переходить на full parse для той же версии и фиксировать причину fallback в observability.

#### Scenario: Incremental update fallback не ломает корректность
- **GIVEN** edit chain для `didChange` не может быть применена к текущему дереву
- **WHEN** система обновляет parse state
- **THEN** выполняется full parse fallback для актуальной версии
- **AND** итоговый snapshot остается version-consistent
- **AND** причина fallback доступна в observability

### Requirement: Changed-ranges используются для ограниченного downstream recompute (MUST)
Система MUST передавать changed-range информацию в downstream стадии и ограничивать пересчет затронутыми диапазонами, когда это не нарушает корректность.

Если range-ограничение не может гарантировать корректный результат, система MUST выполнить полный пересчет соответствующей стадии.

#### Scenario: Burst мелких правок не вызывает полный тяжелый пересчет каждый раз
- **GIVEN** пользователь вносит серию локальных правок в небольшой диапазон большого модуля
- **WHEN** pipeline обрабатывает последовательные ревизии
- **THEN** затронутые стадии используют changed ranges для локального пересчета
- **AND** полный пересчет выполняется только при невозможности безопасного range-режима
