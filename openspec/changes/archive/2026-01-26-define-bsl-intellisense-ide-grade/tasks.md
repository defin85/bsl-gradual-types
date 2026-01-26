## 1. Target spec (IDE‑grade)
- [x] Сформировать целевой список требований “IDE‑grade IntelliSense” для BSL на базе:
  - [x] текущего LSP core (`openspec/specs/bsl-intellisense/spec.md`)
  - [x] roadmap completion v2 (`docs/roadmap/intellisense-v2-roadmap/intellisense-v2-roadmap.md`)
  - [x] требований пользователя (symbols/references/rename/formatting/code actions/inlay hints включены)
- [x] Разнести требования на MUST/SHOULD (обосновать в `design.md`).
- [x] Описать минимум один `Scenario:` для каждого требования.

## 2. Mapping на реализацию
- [x] Сопоставить требования target‑spec с существующими active changes:
  - [x] `add-bsl-lsp-symbols`
  - [x] `add-bsl-lsp-references-and-rename`
  - [x] `evaluate-bsl-formatting`
  - [x] `implement-bsl-vscode-enhanced-providers`
- [x] Зафиксировать, какие требования покрываются roadmap’ом IntelliSense v2 (completion‑completeness) vs какими changes (navigation/refactor/UX).

## 3. OpenSpec files
- [x] `proposal.md`
- [x] `design.md`
- [x] `specs/bsl-intellisense-ide-grade/spec.md` (delta)

## 4. Validation
- [x] `openspec validate define-bsl-intellisense-ide-grade --strict --no-interactive`
