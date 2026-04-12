# Follow-up Live Evidence

## Scenario

- Change: `refactor-14-diagnostics-save-followup-semantic-snapshot-reuse`
- Evidence artifact: `backend/tests/perf/reports/refactor-14-diagnostics-save-followup-semantic-snapshot-reuse-real-conf-big-did-save-diagnostics-followup-syntax-live.json`
- Fixture class: real `conf_big` `didSave` follow-up capture
- Goal: prove that the same save cycle can publish its heavy follow-up through `ready_artifacts` with snapshot-backed semantic input instead of paying the older `shadow_state + direct parse_result` path by default.

## Checked-in Result

- Status: checked in and reviewed on `2026-04-12`
- `first_publish_elapsed_ms=4`
- `first_publish_syntax_only=true`
- `followup_publish_profile=idle_heavy`
- `followup_publish_elapsed_ms=31`
- `followup_semantic_path=ready_artifacts`
- `followup_semantic_parse_source=snapshot`
- `followup_semantic_ir_source=null`
- `followup_syntax_work_mode=reused`

## Verdict

The repo-local `conf_big` artifact shows the intended same-version follow-up ordering: the syntax fastlane publishes first, then the heavy follow-up completes through `ready_artifacts` with snapshot-backed semantic parse input and reused syntax work. This particular capture does not carry a semantic IR source value, so the additive `v8` IR attribution is primarily covered by deterministic regressions and timeline-contract tests; however, the checked-in live evidence is sufficient for `tasks.md` item `2.2` because it proves the production-shaped `conf_big` path no longer defaults to `shadow_state` when same-version ready artifacts are already available.
