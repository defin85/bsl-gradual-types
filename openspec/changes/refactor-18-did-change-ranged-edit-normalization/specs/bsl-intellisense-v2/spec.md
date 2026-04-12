## MODIFIED Requirements

### Requirement: didChange path использует incremental parse с fail-safe full fallback (MUST)
На `didChange` система MUST пытаться обновлять parse state инкрементально через старое дерево и
edit chain.

Если incoming `didChange` содержит ranged content changes, producer MUST сначала нормализовать их в
один canonical ordered replay plan для исходной base revision, а уже затем:

- реконструировать `new_content`;
- строить parser edit chain для incremental parsing.

Для multi-range `didChange` canonical replay plan MUST применяться в reverse document order, чтобы
все диапазоны оставались привязаны к одной и той же pre-change base revision.

Producer MUST использовать один и тот же canonical ordered replay plan и для реконструкции
`new_content`, и для parser edit chain. Producer MUST NOT позволять этим двум derivation paths
расходиться по порядку replay одного и того же `didChange`.

Если incremental path не может быть применен корректно, система MUST детерминированно переходить
на full parse для той же версии и фиксировать причину fallback в observability.

Для valid ranged `didChange`, где base text совпадает с выбранным producer source, система MUST NOT
публиковать canonical fallback reason `edits_do_not_match_new_content` только потому, что локальная
replay order для `new_content` отличалась от порядка parser edits.

#### Scenario: Multi-range didChange uses one canonical replay plan
- **GIVEN** `didChange` для одной версии документа содержит несколько ranged content changes
- **WHEN** producer строит `new_content` и parser edit chain
- **THEN** оба derivation paths используют один и тот же canonical ordered replay plan
- **AND** multi-range replay выполняется в reverse document order относительно исходной base revision

#### Scenario: Valid ranged didChange does not false-fallback to edits_do_not_match_new_content
- **GIVEN** producer выбрал корректный base text для valid ranged `didChange`
- **AND** incoming change set может быть корректно применен к этой base revision
- **WHEN** система обновляет parse state для этой версии
- **THEN** incremental path либо succeeds, либо fallback-ит по другой допустимой canonical причине
- **AND** observability MUST NOT объяснять такой путь `edits_do_not_match_new_content` только из-за divergence между text replay order и parser edit order
