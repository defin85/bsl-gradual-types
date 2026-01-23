## 1. Target spec (IDE‑grade)
- [ ] Сформировать целевой список требований “IDE‑grade IntelliSense” для BSL на базе:
  - [ ] текущего LSP core (`openspec/specs/bsl-intellisense/spec.md`)
  - [ ] roadmap completion v2 (`docs/roadmap/intellisense-v2-roadmap/intellisense-v2-roadmap.md`)
  - [ ] требований пользователя (symbols/references/rename/formatting/code actions/inlay hints включены)
- [ ] Разнести требования на MUST/SHOULD (обосновать в `design.md`).
- [ ] Описать минимум один `Scenario:` для каждого требования.

## 2. Mapping на реализацию
- [ ] Сопоставить требования target‑spec с существующими active changes:
  - [ ] `add-bsl-lsp-symbols`
  - [ ] `add-bsl-lsp-references-and-rename`
  - [ ] `evaluate-bsl-formatting`
  - [ ] `implement-bsl-vscode-enhanced-providers`
- [ ] Зафиксировать, какие требования покрываются roadmap’ом IntelliSense v2 (completion‑completeness) vs какими changes (navigation/refactor/UX).

## 3. OpenSpec files
- [ ] `proposal.md`
- [ ] `design.md`
- [ ] `specs/bsl-intellisense-ide-grade/spec.md` (delta)

## 4. Validation
- [ ] `openspec validate define-bsl-intellisense-ide-grade --strict --no-interactive`
