## 1. Реализация
- [ ] 1.1 Удалить из индекса git артефакты `vscode-extension/out/**` и `vscode-extension/*.vsix` (без удаления исходников).
- [ ] 1.2 Обновить `.gitignore`/`vscode-extension/.gitignore` так, чтобы артефакты не возвращались.
- [ ] 1.3 Обновить `vscode-extension/INSTALLATION.md` и/или `README.md`: описать сборку extension из исходников и упаковку `.vsix`.
- [ ] 1.4 Убедиться, что dev‑цикл (отладка/сборка) не требует присутствия закоммиченных `out/**`.

## 2. Валидация
- [ ] 2.1 `git ls-files | rg "^vscode-extension/out/"` возвращает пусто.
- [ ] 2.2 `git ls-files | rg "\\.vsix$"` возвращает пусто.
- [ ] 2.3 `npm -C vscode-extension run compile` работает при чистом checkout (генерирует `out/**` локально).
- [ ] 2.4 `openspec validate update-vscode-artifacts-policy --strict --no-interactive`.

