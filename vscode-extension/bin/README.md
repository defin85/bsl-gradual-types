# BSL Analyzer Binaries

Эта директория содержит скомпилированные бинарники для VSCode Extension.

## ⚠️ Бинарники НЕ включены в git

По соображениям размера и безопасности, бинарные файлы (.exe) **не хранятся в репозитории**.

Пользователи должны собрать их самостоятельно после клонирования проекта.

## 📦 Как собрать бинарники

### Вариант 1: Автоматическая сборка (рекомендуется)

```bash
cd vscode-extension
npm run compile
```

Скрипт `copy-binaries.js` автоматически:
1. Проверит наличие Rust бинарников в `../target/release/`
2. Скопирует `bsl-lsp-server.exe` → `bin/lsp_server.exe`
3. Покажет статистику

### Вариант 2: Ручная сборка

```bash
# 1. Собрать LSP Server
cargo build -p bsl-backend --bin bsl-lsp-server --release

# 2. Скопировать в Extension
cp target/release/bsl-lsp-server.exe vscode-extension/bin/lsp_server.exe
```

## 📋 Список бинарников

| Файл | Источник | Размер | Описание |
|------|----------|--------|----------|
| `lsp_server.exe` | `bsl-backend` | ~8-9 MB | Language Server Protocol сервер |
| `lsp_server_wrapper.bat` | — | <1 KB | Обёртка для запуска LSP (Windows) |

## 🔒 Безопасность

- Все бинарники собираются из исходников в этом репозитории
- Release бинарники оптимизированы и очищены от отладочной информации
- GitHub Actions автоматически собирает бинарники для releases

## 🚀 Для CI/CD

GitHub Actions workflow автоматически собирает и публикует бинарники:

```yaml
- name: Build LSP Server
  run: cargo build -p bsl-backend --bin bsl-lsp-server --release

- name: Package Extension
  run: |
    cd vscode-extension
    npm run compile
    vsce package
```

## ❓ Troubleshooting

**Проблема**: Extension не запускается, ошибка "LSP Server not found"

**Решение**:
1. Проверьте наличие файла `vscode-extension/bin/lsp_server.exe`
2. Пересоберите бинарник: `npm run compile` в `vscode-extension/`
3. Проверьте логи: VSCode Output → "BSL Gradual Types"

**Проблема**: "Access denied" при запуске LSP

**Решение**: Windows может блокировать скачанные .exe файлы.
- ПКМ на `lsp_server.exe` → Properties → Unblock → Apply
