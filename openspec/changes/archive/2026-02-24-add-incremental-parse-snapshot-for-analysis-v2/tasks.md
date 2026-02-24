## 1. Snapshot Contract
- [x] 1.1 Описать и зафиксировать структуру `ParseSnapshot` (file/version/tree/parse_result/changed_ranges/timestamp).
- [x] 1.2 Определить invariants консистентности snapshot относительно `received/applied` ревизий.

## 2. Incremental Parse Path
- [x] 2.1 Реализовать обновление snapshot через `old_tree + edit + parse(new, Some(old_tree))` для `didChange`.
- [x] 2.2 Добавить deterministic fallback на full parse при несовпадении edit-последовательности или ошибке incremental path.
- [x] 2.3 Добавить changed-ranges extraction и контракт передачи ranges в downstream pipeline.

## 3. analysis-v2 Integration
- [x] 3.1 Подключить `parse_result/syntax_diagnostics/ir` к snapshot input вместо обязательного full parse на каждую ревизию.
- [x] 3.2 Ограничить recompute до затронутых диапазонов, где это возможно без нарушения корректности.

## 4. Observability
- [x] 4.1 Добавить low-cardinality метрики reuse/fallback/reason и changed-range size.
- [x] 4.2 Добавить stage drilldown для сравнения incremental vs full parse стоимости.

## 5. Validation
- [x] 5.1 Добавить parity тесты incremental vs full parse (AST/syntax diagnostics).
- [x] 5.2 Добавить стресс-тест edit burst на большом модуле с проверкой отсутствия semantic drift.
- [x] 5.3 Выполнить `openspec validate add-incremental-parse-snapshot-for-analysis-v2 --strict --no-interactive`.
