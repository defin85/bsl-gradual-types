## 1. Scope Narrowing
- [ ] 1.1 Сузить change только до residual observability scope для `syntax_diagnostics`.
- [ ] 1.2 Удалить из change повторное описание incremental parse, fallback и lifecycle `ParseSnapshot`.

## 2. Observability Contract
- [ ] 2.1 Добавить mode-aware измерение latency `syntax_diagnostics` stage в канонический observability contract.
- [ ] 2.2 Использовать существующую low-cardinality taxonomy parse mode: `incremental`, `reused`, `full`, `other`.
- [ ] 2.3 Сохранить `intellisense_v2_syntax_diagnostics_query_ms` как aggregate compatibility projection.

## 3. Validation
- [ ] 3.1 Добавить contract/regression tests, которые доказывают наличие mode-aware разреза для syntax diagnostics latency.
- [ ] 3.2 Добавить проверку, что legacy aggregate projection остаётся детерминированной и backward-compatible.
- [ ] 3.3 Выполнить `openspec validate add-incremental-syntax-diagnostics-for-large-modules --strict --no-interactive`.

## 4. Follow-up Boundary
- [ ] 4.1 Не расширять этот change назад в parse/runtime algorithm work; отдельный scope требует нового change.
