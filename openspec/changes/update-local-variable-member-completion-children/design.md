## Context
The 2026-05-01 incident around `examples/conf_big/CommonModules/АвансовыйОтчетФормы/Ext/Module.bsl` exposed two related symptoms:

- snapshot status could remain stuck at `building requested=v38 ready=v36`;
- after-dot member completion for `ТаблЗнач.` returned no children.

This change deliberately handles the second symptom after `update-snapshot-status-terminal-liveness`. The completion bug should be diagnosed only after the server can reliably say whether current-revision `CompletionHeadArtifact` / exact artifact is ready, unavailable, failed, or superseded.

Bundle evidence for the member completion symptom:

- `probe-17`, version `22`, after-dot trigger `.`, returned `empty_list` with `0` items after `145ms`; server trace failed closed on `exact_deadline`.
- `probe-18`, version `22`, invoked after dot, returned `empty_list` with `0` items in `5ms`.
- `probe-19`, version `22`, invoked after dot, returned `empty_list` with `0` items in `6ms`.
- `probe-32`, version `48`, after-dot trigger `.`, was cancelled/nullish with `0` items.
- For `probe-18/19`, exact/head readiness was observed before wait, while `current_revision_head_owner_hints_ready=false`, so the remaining bug is not only a timeout.

## Goals
- Return member children for local variables with constructor-inferred platform types.
- Cover variables declared inside procedures and functions, not only module/global symbols.
- Preserve current-revision semantics and fail-closed policy.
- Make empty member completion distinguish “no members exist” from “owner type unavailable/unresolved”.
- Add regression tests that assert actual children, not just the local variable symbol label.

## Non-Goals
- Do not invent members from syntax-helper fallback when canonical artifacts are missing.
- Do not broaden this change to all unresolved local inference cases.
- Do not change diagnostics for unrelated metadata chains such as `Документы.РеализацияТоваровУслуг`.
- Do not optimize completion latency except where required to make the correctness path deterministic.

## Design Notes

### 1. The owner expression is the contract boundary
For `ТаблЗнач.`, completion must resolve the owner expression `ТаблЗнач` in the active lexical scope. The result should be a type candidate for `ТаблицаЗначений`, derived from the local assignment:

```bsl
ТаблЗнач = Новый ТаблицаЗначений;
```

The member query should then enumerate children for that owner type from the canonical current-revision artifact.

### 2. Local variable symbol completion is not enough
Existing or future tests that assert a non-member completion item labeled `ТаблЗнач` prove only that the local symbol exists. This change requires an after-dot member-access assertion that children such as `Колонки` and `ВыгрузитьКолонку` are returned for the owner expression.

### 3. Readiness failures must stay explicit
When current-revision completion artifacts are unavailable, completion must follow the existing
bounded fail-closed/degraded policy for the active completion profile. If that policy returns a
degraded `isIncomplete=true` response for a recognized member-access context, it still must not
synthesize `ТаблицаЗначений` children without canonical owner hints, and it must remain
distinguishable in traces from a successful empty children response.

After `update-snapshot-status-terminal-liveness`, a test can assert:

- snapshot/head/exact readiness is terminal and visible;
- the request is after-dot/member-access;
- owner type resolution succeeds;
- children are returned.

### 4. Owner hints are the shared contract, not an adapter repair
The default LSP completion surface intentionally fails closed when a member-access request reaches response construction without shared owner hints. This change must not reverse that boundary by teaching the adapter to infer local owner types from `parse_result`, raw text, or adapter-local IR traversal.

The correct fix is to make the canonical current-revision artifacts expose the owner type for
`ТаблЗнач`. This can be a general type entry at the receiver span or a dedicated owner-hint
projection keyed by the member-access receiver span; it must still be produced by the shared
canonical artifact pipeline, not by adapter-local request handling:

- `CompletionHeadArtifact` should expose `ТаблицаЗначений` through the shared head owner-hint query when the head is ready;
- `ExactSemanticArtifact` / serve-only type index should expose the same owner type through the exact owner-hint query when exact is ready;
- LSP should pass those hints through to the shared completion runtime.

If head/exact artifacts are terminal and current but the owner still cannot be resolved, the trace must classify the result as owner-unresolved (or an equivalent bounded low-cardinality reason) rather than artifact-unavailable, exact-deadline, or successful-empty.

Existing static receiver fallback may remain only for syntactically self-contained receivers that
do not require local lexical scope truth, such as supported type-name/static receivers. It is not an
acceptance path for `Лок.` or `ТаблЗнач.` and must not be used to infer local variable owner types.

### 5. Owner hints are a likely narrow fault line
The incident showed `current_revision_head_owner_hints_ready=false` for fast fail-closed invoked probes even when exact/head readiness looked ready before wait. The implementation should inspect:

- how `CompletionHeadArtifact` stores local symbol owner hints;
- whether constructor assignment local types survive IR conversion for incomplete code near line 55;
- whether member-access probing after an incomplete dot consults the correct current-revision owner-hint table;
- whether fail-closed traces distinguish missing artifact, missing owner hint, and resolved-owner-with-zero-members.

## Risks
- Fixing only the real fixture could miss the general local-scope path; include a minimal snippet test.
- Treating unresolved owner as empty result would hide regressions; traces/tests should fail on that ambiguity.
- Incomplete-code parse recovery around `ТаблЗнач.` can drop the assignment node; tests should cover both stable code and after-dot incomplete code.
