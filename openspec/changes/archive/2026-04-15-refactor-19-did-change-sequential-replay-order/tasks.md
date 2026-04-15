## 1. Implementation

- [x] 1.1 Add failing regressions for spec-correct sequential ranged `didChange`, including a
      `UTF-8 BOM + CRLF` live-class fixture.
- [x] 1.2 Replace reverse-order ranged replay normalization with one canonical receive-order replay
      plan shared by `updated_text` reconstruction and `parser_edits`.
- [x] 1.3 Extend bounded didChange parse-snapshot evidence with replay-order and base-version
      attribution fields needed for incident-bundle triage.

## 2. Validation

- [x] 2.1 Run targeted backend/runtime regressions for:
      - sequential multi-range replay
      - `BOM + CRLF` live-class replay
      - didChange evidence export
- [x] 2.2 Capture fresh evidence showing that valid ranged `didChange` no longer false-fallbacks to
      `edits_do_not_match_new_content` in the target scenario.
- [x] 2.3 Run `openspec validate refactor-19-did-change-sequential-replay-order --strict --no-interactive`.
