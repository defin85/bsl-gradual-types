## 1. Audit (evidence-driven)
- [ ] Зафиксировать целевой “перечень функций IntelliSense” для аудита (по текущему чату) и границы scope (LSP/VS Code/MCP).
- [ ] Провести инвентаризацию LSP‑возможностей по коду `backend/src/bin/lsp_server/`:
  - [ ] заявленные `ServerCapabilities` (initialize)
  - [ ] реализованные handlers / LSP методы (completion/hover/signatureHelp/definition/diagnostics)
  - [ ] ограничения/частичные реализации (например, definition только для типов)
- [ ] Провести инвентаризацию VS Code extension по коду `vscode-extension/`:
  - [ ] что реально включено в “main” (активация, запуск LSP)
  - [ ] какие providers есть и какие из них заглушки (inlay hints / code actions / enhanced diagnostics)
  - [ ] какие custom команды/вьюхи добавляют “IDE UX” поверх LSP
- [ ] (Опционально) Зафиксировать влияние MCP `bsl-agent` на IDE‑сценарии (если он используется как источник диагностики/контекста).

## 2. Spec (фиксируем текущую правду)
- [ ] Добавить delta‑spec для нового capability `bsl-intellisense` с требованиями на уже реализованные функции.
- [ ] Явно отметить “не входит в текущий контракт” для функций, которых нет (references/rename/formatting и т.п.) — через раздел “Out of scope / Not supported” в `design.md` (не как требования).

## 3. Deliverables
- [ ] Заполнить `design.md` audit‑матрицей (feature → status → evidence → notes).
- [ ] Сформировать список follow‑up change‑proposal’ов на недостающие функции IntelliSense (без реализации).

## 4. Validation
- [ ] `openspec validate audit-bsl-intellisense --strict --no-interactive`
