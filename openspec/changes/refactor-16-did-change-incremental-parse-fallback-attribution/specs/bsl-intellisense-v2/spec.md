## MODIFIED Requirements

### Requirement: didChange path использует incremental parse с fail-safe full fallback (MUST)
На `didChange` система MUST пытаться обновлять parse state инкрементально через старое дерево и
edit chain.

Если incremental path не может быть применен корректно, система MUST детерминированно переходить
на full parse для той же версии и фиксировать причину fallback в observability.

Observability для этого пути MUST:

- использовать bounded canonical reason taxonomy, а не raw/free-form error strings;
- фиксировать source контекст, достаточный чтобы понимать, откуда бралась база для edit chain;
- фиксировать low-cardinality change-shape classification для текущего `didChange`;
- сохранять version-bound evidence, достаточную чтобы later `didSave` snapshot miss можно было
  сопоставить с preceding parse-snapshot fallback без чтения логов или исходников.

#### Scenario: stale edit base maps to a canonical fallback reason
- **GIVEN** `didChange` строит incremental parse against base text, который не совпадает с
  incoming edit chain
- **WHEN** incremental parsing не может привести дерево к `new_content`
- **THEN** система выполняет full parse fallback для той же версии
- **AND** observability публикует canonical fallback reason вместо generic `incremental_failed`
- **AND** producer-side attribution сохраняет source базы для edit chain

#### Scenario: incident bundle exposes version-bound parse fallback evidence
- **GIVEN** для одной версии документа incremental parse path падает в full fallback
- **AND** более поздний `didSave` не находит same-version `ready_artifacts`
- **WHEN** оператор экспортирует incident bundle
- **THEN** bundle содержит compact version-bound parse-snapshot evidence для failed `didChange`
- **AND** оператор может сопоставить didChange fallback с didSave miss без raw text payloads
