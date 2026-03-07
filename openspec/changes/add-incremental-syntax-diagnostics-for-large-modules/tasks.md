## 1. Scope Narrowing
- [x] 1.1 Сузить change только до residual observability scope для `syntax_diagnostics`.
- [x] 1.2 Удалить из change повторное описание incremental parse, fallback и lifecycle `ParseSnapshot`.

## 2. Observability Contract
- [x] 2.1 Добавить mode-aware измерение latency `syntax_diagnostics` stage в канонический observability contract.
- [x] 2.2 Зафиксировать stage-aware semantics поля `mode` в каноническом observability contract:
  - [x] для `syntax_diagnostics` `mode` означает parse mode;
  - [x] для completion-related stages сохраняется существующая completion-routing semantics.
- [x] 2.3 Использовать существующую low-cardinality taxonomy parse mode: `incremental`, `reused`, `full`, `other`.
- [x] 2.4 Зафиксировать explicit policy для `non-LSP` origins и path без version-bound `ParseSnapshot`: `syntax_diagnostics` публикует `mode=other`.
- [x] 2.5 Сохранить `intellisense_v2_syntax_diagnostics_query_ms` как aggregate compatibility projection.

## 3. Validation
- [x] 3.1 Добавить contract/regression tests, которые доказывают наличие mode-aware разреза для syntax diagnostics latency.
- [x] 3.2 Добавить проверку, что stage-aware schema отбрасывает недопустимые сочетания stage/mode и не смешивает parse-mode с completion-mode.
- [x] 3.3 Добавить проверку, что legacy aggregate projection остаётся детерминированной и backward-compatible.
- [x] 3.4 Добавить проверку, что `non-LSP` origins без version-bound `ParseSnapshot` публикуют `mode=other`.
- [x] 3.5 Выполнить `openspec validate add-incremental-syntax-diagnostics-for-large-modules --strict --no-interactive`.

## 4. Follow-up Boundary
- [x] 4.1 Не расширять этот change назад в parse/runtime algorithm work; отдельный scope требует нового change.
