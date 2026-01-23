# Change: implement-bsl-vscode-enhanced-providers

## Why
В VS Code extension уже зарегистрированы “enhanced” providers (inlay hints, code actions, enhanced diagnostics), но часть из них сейчас является заглушками и всегда возвращает пустые результаты:
- inlay hints: `vscode-extension/src/providers/typeHintsProvider.ts:15`
- code actions: `vscode-extension/src/providers/codeActionsProvider.ts:15`
- diagnostics stats: `vscode-extension/src/providers/enhancedDiagnosticsProvider.ts:23`

Это создаёт ложное ощущение “фича есть”, усложняет поддержку и мешает измерять прогресс IntelliSense.

## What Changes
- Привести “enhanced” providers в одно из двух корректных состояний:
  1) либо **реально реализованы** и дают пользователю ценность,
  2) либо **не регистрируются по умолчанию** (фича-флаг/эксперимент), чтобы не было скрытых заглушек.
- Зафиксировать контракт/ожидания в `openspec/specs/bsl-intellisense/spec.md` (delta).

## Impact
- Спецификация: `openspec/specs/bsl-intellisense/spec.md` (delta).
- Код: `vscode-extension/src/setup/providers.ts` и `vscode-extension/src/providers/*`.
- Тесты: unit/e2e тесты extension (минимум: провайдеры включаются/выключаются и возвращают ожидаемое).
