## ADDED Requirements

### Requirement: Local variable member completion returns children for constructor-inferred types (MUST)
IntelliSense v2 SHALL return member completion children for a local variable declared inside a procedure or function when the variable's current-revision type is inferred from a constructor assignment.

For an assignment of the form:

```bsl
ТаблЗнач = Новый ТаблицаЗначений;
```

an after-dot completion request on `ТаблЗнач.` SHALL resolve `ТаблЗнач` as the owner expression in the active local scope and SHALL return members of `ТаблицаЗначений`, including at least `Колонки` and `ВыгрузитьКолонку` when those platform members are present in the loaded syntax-helper/type repository data.

The result SHALL be derived from canonical current-revision completion artifacts (`CompletionHeadArtifact` and/or exact semantic artifact) built from the same revision. The server MUST NOT serve stale children from another revision and MUST NOT synthesize member candidates from a non-canonical fallback path.

For local-variable owner expressions, owner type hints SHALL be produced by the shared canonical owner-resolution path exposed by `CompletionHeadArtifact` and/or the exact semantic artifact. The shared path MAY expose those hints through a general type entry or a dedicated member-access owner-hint projection, but the projection MUST be keyed by the current-revision artifact identity and receiver span. Adapter surfaces MAY pass shared owner hints through, but MUST NOT reconstruct local-variable owner truth from raw text, `parse_result`, adapter-local IR traversal, or static receiver fallback as a substitute for missing canonical owner hints. Existing static receiver fallback MAY remain only for non-local, syntactically self-contained receivers where no local lexical-scope truth is required.

If current-revision artifacts required for member completion are unavailable, the server SHALL follow the existing bounded fail-closed/degraded policy for the active completion profile. Any unavailable or degraded response, including an `isIncomplete=true` response, MUST NOT synthesize local-variable member children without canonical owner hints and SHALL be distinguishable from a successful empty member set. If the owner expression cannot be resolved while current-revision artifacts are ready, the trace or test-visible outcome SHALL classify the issue as owner-unresolved (or an equivalent bounded low-cardinality reason) rather than as artifact-unavailable, exact-deadline, wait-not-ready, or successful empty completion.

#### Scenario: `ТаблЗнач.` returns value table children in the real fixture
- **GIVEN** the active document is `examples/conf_big/CommonModules/АвансовыйОтчетФормы/Ext/Module.bsl`
- **AND** the current revision contains `ТаблЗнач = Новый ТаблицаЗначений`
- **AND** snapshot/head/exact readiness for that revision is terminal and current
- **WHEN** the IDE requests member completion immediately after `ТаблЗнач.`
- **THEN** the server resolves `ТаблЗнач` as a local variable in the active procedure or function scope
- **AND** returns member completion children for `ТаблицаЗначений`
- **AND** the returned children include `Колонки`
- **AND** the returned children include `ВыгрузитьКолонку`

#### Scenario: Minimal local constructor assignment returns children
- **GIVEN** a BSL procedure contains `Лок = Новый ТаблицаЗначений;`
- **AND** the current revision is ready for completion
- **WHEN** the IDE requests completion at `Лок.`
- **THEN** the response contains member children for `ТаблицаЗначений`
- **AND** the response is not an empty list

#### Scenario: Canonical artifacts expose local owner hints
- **GIVEN** a BSL procedure contains `Лок = Новый ТаблицаЗначений;`
- **AND** `CompletionHeadArtifact` is ready for the current revision
- **WHEN** the shared completion-head owner-hint query is evaluated for `Лок.`
- **THEN** it resolves `Лок` to `ТаблицаЗначений`
- **AND** when the exact semantic artifact is ready, the exact owner-hint query resolves the same owner to `ТаблицаЗначений`
- **AND** the hint is tied to the current-revision artifact identity and receiver span

#### Scenario: Adapter does not repair missing local owner hints
- **GIVEN** a member-access request targets `Лок.`
- **AND** current-revision artifacts are terminal and current
- **AND** shared owner-hint extraction returns no owner type for `Лок`
- **WHEN** the LSP adapter builds the completion response
- **THEN** it does not infer the local owner type from raw text, `parse_result`, adapter-local IR traversal, or static receiver fallback
- **AND** the response or trace is classified as owner-unresolved rather than successful empty children

#### Scenario: Local variable label assertion does not replace member-child assertion
- **GIVEN** non-member completion includes an item labeled `ТаблЗнач`
- **WHEN** tests validate member completion for `ТаблЗнач.`
- **THEN** they assert returned children under the owner expression
- **AND** they do not treat the local variable item itself as proof that member children work

#### Scenario: Artifact unavailable remains fail-closed
- **GIVEN** current-revision completion artifacts are not ready for the active revision
- **WHEN** the IDE requests completion at `ТаблЗнач.`
- **THEN** the server follows the existing bounded fail-closed/degraded policy for the active completion profile
- **AND** it does not return stale children from an older revision
- **AND** it does not synthesize `ТаблицаЗначений` children without canonical owner hints
- **AND** the response or trace is distinguishable from a successful empty member set
