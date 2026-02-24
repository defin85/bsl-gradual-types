## 1. Snapshot Contract
- [ ] 1.1 Описать и зафиксировать структуру `ParseSnapshot` (file/version/tree/parse_result/changed_ranges/timestamp).
- [ ] 1.2 Определить invariants консистентности snapshot относительно `received/applied` ревизий.

## 2. Incremental Parse Path
- [ ] 2.1 Реализовать обновление snapshot через `old_tree + edit + parse(new, Some(old_tree))` для `didChange`.
- [ ] 2.2 Добавить deterministic fallback на full parse при несовпадении edit-последовательности или ошибке incremental path.
- [ ] 2.3 Добавить changed-ranges extraction и контракт передачи ranges в downstream pipeline.

## 3. analysis-v2 Integration
- [ ] 3.1 Подключить `parse_result/syntax_diagnostics/ir` к snapshot input вместо обязательного full parse на каждую ревизию.
- [ ] 3.2 Ограничить recompute до затронутых диапазонов, где это возможно без нарушения корректности.

## 4. Observability
- [ ] 4.1 Добавить low-cardinality метрики reuse/fallback/reason и changed-range size.
- [ ] 4.2 Добавить stage drilldown для сравнения incremental vs full parse стоимости.

## 5. Validation
- [ ] 5.1 Добавить parity тесты incremental vs full parse (AST/syntax diagnostics).
- [ ] 5.2 Добавить стресс-тест edit burst на большом модуле с проверкой отсутствия semantic drift.
- [ ] 5.3 Выполнить `openspec validate add-incremental-parse-snapshot-for-analysis-v2 --strict --no-interactive`.
