## 1. Audit (evidence-driven)
- [x] Зафиксировать целевой “перечень функций IntelliSense” для аудита (по текущему чату) и границы scope (LSP/VS Code/MCP).
- [x] Провести инвентаризацию LSP‑возможностей по коду `backend/src/bin/lsp_server/`:
  - [x] заявленные `ServerCapabilities` (initialize)
  - [x] реализованные handlers / LSP методы (completion/hover/signatureHelp/definition/diagnostics)
  - [x] ограничения/частичные реализации (например, definition только для типов)
- [x] Провести инвентаризацию VS Code extension по коду `vscode-extension/`:
  - [x] что реально включено в “main” (активация, запуск LSP)
  - [x] какие providers есть и какие из них заглушки (inlay hints / code actions / enhanced diagnostics)
  - [x] какие custom команды/вьюхи добавляют “IDE UX” поверх LSP
- [ ] (Опционально) Зафиксировать влияние MCP `bsl-agent` на IDE‑сценарии (если он используется как источник диагностики/контекста).

## 2. Spec (фиксируем текущую правду)
- [x] Добавить delta‑spec для нового capability `bsl-intellisense` с требованиями на уже реализованные функции.
- [x] Явно отметить “не входит в текущий контракт” для функций, которых нет (references/rename/formatting и т.п.) — через раздел “Out of scope / Not supported” в `design.md` (не как требования).

## 3. Deliverables
- [x] Заполнить `design.md` audit‑матрицей (feature → status → evidence → notes).
- [x] Сформировать список follow‑up change‑proposal’ов на недостающие функции IntelliSense (без реализации).

## 4. Validation
- [x] `openspec validate audit-bsl-intellisense --strict --no-interactive`
