## 1. Decide: implement vs gate
- [x] Принять решение для каждого provider’а: “реально реализуем сейчас” или “прячем за флагом по умолчанию”.
- [x] Обновить настройки extension (если нужен флаг), чтобы поведение было явным.

## 2. Inlay hints (type hints)
- [x] Если реализуем: определить источник данных (LSP стандарт `inlayHint` или custom request) и минимальный набор подсказок (переменные/возвраты).
- [x] Если gating: отключить регистрацию `registerInlayHintsProvider` по умолчанию.

## 3. Code actions
- [x] Если реализуем: определить минимальный набор quick fixes/refactors и источник (LSP codeAction или локальная логика).
- [x] Если gating: отключить регистрацию `registerCodeActionsProvider` по умолчанию.

## 4. Enhanced diagnostics stats
- [x] Привязать stats к реальным diagnostics (или убрать/скрыть до появления источника правды).

## 5. Spec
- [x] Обновить `openspec/changes/implement-bsl-vscode-enhanced-providers/specs/bsl-intellisense/spec.md`.

## 6. Validation
- [x] `openspec validate implement-bsl-vscode-enhanced-providers --strict --no-interactive`
