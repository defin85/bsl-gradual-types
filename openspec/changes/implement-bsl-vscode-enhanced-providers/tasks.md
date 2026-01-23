## 1. Decide: implement vs gate
- [ ] Принять решение для каждого provider’а: “реально реализуем сейчас” или “прячем за флагом по умолчанию”.
- [ ] Обновить настройки extension (если нужен флаг), чтобы поведение было явным.

## 2. Inlay hints (type hints)
- [ ] Если реализуем: определить источник данных (LSP стандарт `inlayHint` или custom request) и минимальный набор подсказок (переменные/возвраты).
- [ ] Если gating: отключить регистрацию `registerInlayHintsProvider` по умолчанию.

## 3. Code actions
- [ ] Если реализуем: определить минимальный набор quick fixes/refactors и источник (LSP codeAction или локальная логика).
- [ ] Если gating: отключить регистрацию `registerCodeActionsProvider` по умолчанию.

## 4. Enhanced diagnostics stats
- [ ] Привязать stats к реальным diagnostics (или убрать/скрыть до появления источника правды).

## 5. Spec
- [ ] Обновить `openspec/changes/implement-bsl-vscode-enhanced-providers/specs/bsl-intellisense/spec.md`.

## 6. Validation
- [ ] `openspec validate implement-bsl-vscode-enhanced-providers --strict --no-interactive`
