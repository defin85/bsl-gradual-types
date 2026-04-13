## MODIFIED Requirements

### Requirement: didChange path использует incremental parse с fail-safe full fallback (MUST)
На `didChange` система MUST пытаться обновлять parse state инкрементально через старое дерево и
edit chain.

Если incoming `didChange` содержит ranged content changes, producer MUST сначала нормализовать их в
один canonical ordered replay plan, соответствующий receive order внутри LSP notification, а уже
затем:

- реконструировать `new_content`;
- строить parser edit chain для incremental parsing.

Для multi-range `didChange` canonical replay plan MUST сохранять receive order `contentChanges`.
Producer MUST NOT переупорядочивать valid ranged edits в reverse document order.

Producer MUST использовать один и тот же canonical ordered replay plan и для реконструкции
`new_content`, и для parser edit chain. Producer MUST NOT позволять этим двум derivation paths
расходиться по порядку replay одного и того же `didChange`.

Если incremental path не может быть применен корректно, система MUST детерминированно переходить
на full parse для той же версии и фиксировать причину fallback в observability.

Observability для этого пути MUST:

- использовать bounded canonical reason taxonomy, а не raw/free-form error strings;
- фиксировать source контекст, достаточный чтобы понимать, откуда бралась база для edit chain;
- фиксировать low-cardinality change-shape classification для текущего `didChange`;
- фиксировать replay-order attribution, достаточную чтобы отличать producer replay drift от stale
  base-text drift;
- сохранять version-bound evidence, достаточную чтобы later `didSave` snapshot miss можно было
  сопоставить с preceding parse-snapshot fallback без чтения логов или исходников.

Для valid ranged `didChange`, где base text совпадает с выбранным producer source и incoming batch
соответствует receive-order semantics LSP, система MUST NOT публиковать canonical fallback reason
`edits_do_not_match_new_content` только потому, что producer переупорядочил replay относительно
порядка, в котором клиент прислал `contentChanges`.

#### Scenario: Multi-range didChange uses one canonical receive-order replay plan
- **GIVEN** `didChange` для одной версии документа содержит несколько ranged content changes
- **WHEN** producer строит `new_content` и parser edit chain
- **THEN** оба derivation paths используют один и тот же canonical ordered replay plan
- **AND** multi-range replay выполняется в receive order `contentChanges`, а не в reverse document
  order

#### Scenario: Valid sequential ranged didChange does not false-fallback to edits_do_not_match_new_content
- **GIVEN** producer выбрал корректный base text для valid ranged `didChange`
- **AND** incoming change set корректно описывает последовательные state transitions внутри одной
  LSP notification
- **WHEN** система обновляет parse state для этой версии
- **THEN** incremental path либо succeeds, либо fallback-ит по другой допустимой canonical причине
- **AND** observability MUST NOT объяснять такой путь `edits_do_not_match_new_content` только из-за
  divergence между producer replay order и LSP receive order

#### Scenario: Incident bundle distinguishes replay-order attribution from stale-base attribution
- **GIVEN** оператор экспортирует incident bundle после parse-snapshot fallback на ranged
  `didChange`
- **WHEN** bundle включает compact didChange parse-snapshot evidence
- **THEN** evidence содержит bounded replay-order attribution и known base-version attribution when
  available
- **AND** оператор может понять, был ли fallback вызван replay drift или stale base selection, не
  читая raw text payloads
