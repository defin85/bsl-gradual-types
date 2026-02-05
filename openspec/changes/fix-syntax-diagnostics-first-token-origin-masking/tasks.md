## 1. Proposal / Specs
- [ ] 1.1 Добавить delta-спеку для `bsl-intellisense-v2`: “первый неожиданный токен” в обобщённом `Для`, masking `//` до EOL в многострочных срезах.
- [ ] 1.2 `openspec validate fix-syntax-diagnostics-first-token-origin-masking --strict --no-interactive`.

## 2. Implementation (syntax)
- [ ] 2.1 `Для` (general): определять span на **первый** неожиданный токен между `По` и `Цикл` (не последний, не “ближайший к падению”).
- [ ] 2.2 Ввести structured origin для синтаксических diagnostics (parser vs heuristics) без сравнения message:
  - вариант: `normalize_syntax_errors(parser_errors: Vec<ParseError>, heuristics: Vec<ParseError>, ...)`;
  - либо внутренний wrapper `SyntaxDiag { origin, error }` внутри нормализатора.
- [ ] 2.3 Masking: исправить обработку `//` для многострочного текста — маскировать до `\n`, затем продолжать.
- [ ] 2.4 Обновить ранжирование line-cap: parser-origin SHOULD выигрывать у heuristics-origin на одной строке, независимо от `ErrorType`.

## 3. Tests
- [ ] 3.1 `Для i = 10 По 0 abc def Цикл` → подсветка `abc` + сообщение “ожидается `Цикл`”.
- [ ] 3.2 `Для ... По 0 // Шаг -1\nabc Цикл` (или эквивалентный пример) не должен триггерить правило `Шаг <expr>` из комментария.
- [ ] 3.3 Тест line-cap приоритизации: parser-origin подавляет semicolon/`Новый` heuristics на той же строке без зависимости от текста сообщения.

## 4. Quality gates
- [ ] 4.1 `cargo test -p bsl-syntax`.
- [ ] 4.2 (опционально) smoke: `cargo test -p bsl-analysis-v2`.

