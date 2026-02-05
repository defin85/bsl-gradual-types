# Design: First-token `Для`, structured origin, multiline-safe masking

## Goals
- Совпасть со спекой: “первый неожиданный токен” для обобщённого `Для`.
- Стабильная приоритизация line-cap без хрупких проверок текста message.
- Masking `//` корректен для срезов с newline (до конца строки, не до конца среза).

## Proposed Changes

### 1) First-token selection for generalized `Для`
В текущей реализации span выбирается через “последний токен” в диапазоне `По..Цикл`.
Нужно:
- определить диапазон между `По` и `Цикл` на masked-строке;
- найти **первый** токен после пробелов (идентификатор/число/символ);
- span вернуть на этот токен.

### 2) Structured origin
Сейчас “heuristics vs parser” определяется по message. Это хрупко.
Предлагаемый подход (минимальный и code-local):
- В `convert_tree` собирать:
  - `parser_errors`: из `collect_syntax_errors_cached`
  - `heuristic_errors`: semicolon/`Новый` эвристики
- Передавать оба набора в `normalize_syntax_errors` так, чтобы origin был известен структурно.
Внутри нормализатора хранить `SyntaxDiag { origin: Parser|Heuristic, error: ParseError }`,
делать rewrite только при наличии parser errors, затем line-cap и сортировку.

### 3) Multiline-safe masking
`mask_line_for_rules` должен работать на любом тексте (строка или многострочный срез):
- `//` маскируется до `\n`/`\r\n`, потом продолжаем обычный режим;
- строковые литералы `"..."` маскируются с учётом `""` как escape.

### 4) line-cap ranking
При выборе 1 диагностики на строку:
- primary: origin (Parser лучше Heuristic),
- secondary: `ErrorType` (InvalidSyntax > MissingToken > ParseError > UnexpectedToken),
- затем “узость” span и стабильные tie-breakers.

## Notes
- Внешний тип `ParseError` не меняем (без добавления новых полей).
- LSP conversion уже умеет `relatedInformation`, менять backend не требуется.

