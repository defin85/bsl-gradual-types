## ADDED Requirements

### Requirement: Ranged `didChange` fallback `stale_parser_base` объясняет, почему reuse-база была недоступна (MUST)

Система MUST дополнительно экспортировать low-cardinality root-cause attribution, если ranged
`didChange` вынужден перейти на full parse с `fallback_reason=stale_parser_base`, чтобы было
понятно, почему дешёвый incremental parser-base reuse не был доступен.

Этот attribution MUST различать как минимум:

- отсутствовал matching ready snapshot для текущего shadow revision;
- latest ready snapshot отставал от shadow revision;
- priming от matching ready snapshot был выполнен, но tree cache всё равно не совпал с shadow
  text;
- иные bounded внутренние причины.

Система MUST экспортировать bounded base-state поля, достаточные для интерпретации miss-класса,
включая текущий shadow revision и latest ready revision, когда они доступны.

Система MUST NOT использовать для этого raw text, free-form debug strings или другую
high-cardinality payload информацию.

#### Scenario: Bundle показывает, что shadow revision ушёл вперёд latest ready base

- **GIVEN** same-file churn продвинул `shadow_state` до revision `V+k`
- **AND** latest ready parse snapshot остаётся на revision `V`
- **WHEN** ranged `didChange` для revision `V+k+1` падает в `stale_parser_base`
- **THEN** observability payload явно показывает bounded miss class, соответствующий lag между
  ready base и shadow revision
- **AND** оператору не нужно открывать raw logs, чтобы понять, что matching ready base для shadow
  revision ещё не существовал

#### Scenario: Bundle показывает mismatch даже после attempted prime

- **GIVEN** для shadow revision существует matching ready snapshot
- **AND** runtime attempted prime parser tree cache from that ready snapshot
- **WHEN** subsequent ranged `didChange` всё равно падает в `stale_parser_base`
- **THEN** observability payload показывает miss class для tree-cache mismatch after prime
- **AND** top-level fallback reason остаётся `stale_parser_base`
