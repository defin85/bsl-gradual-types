# VS Code Extension Notes

Эти инструкции дополняют root `AGENTS.md` и `docs/agent/*`.

## Scope

- `vscode-extension/` — VS Code extension, Node/TypeScript tooling, packaged binaries
- Canonical scripts живут в `vscode-extension/package.json`
- Runtime/diagnostic contract расширения должен соответствовать живым backend binaries и `docs/agent/verification.md`

## Local Verify

```bash
npm --prefix ./vscode-extension run compile:fast
npm --prefix ./vscode-extension run lint
npm --prefix ./vscode-extension test
```

- Для packaging/build workflow смотри `package.json` scripts и `docs/agent/verification.md`

## Important Files

- `vscode-extension/package.json` — scripts, extension metadata, configuration schema
- `vscode-extension/src/` — extension/runtime code
- `vscode-extension/README.md` — user-facing docs, держать в sync с реальным tooling surface

## Boundaries

- Не вводи новые бинарные names в docs/config без проверки через workspace crates
- Изменения в bundled binaries или startup flow должны сопровождаться обновлением user-facing docs и smoke checks
