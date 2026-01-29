# Установка и тестирование BSL Gradual Types расширения для VS Code

## 📦 Готовое расширение

`.vsix` не хранится в git: это генерируемый артефакт. Собери и упакуй расширение локально.

## 🚀 Установка

### Способ 1: Установка через VS Code UI

1. Открой VS Code
2. Открой панель Extensions (Ctrl+Shift+X)
3. Кликни на иконку ... (меню) в правом верхнем углу панели
4. Выбери "Install from VSIX..."
5. Найди `.vsix`, созданный командой `vsce package`
6. Подтверди установку

### Способ 2: Установка через командную строку

```bash
cd vscode-extension
code --install-extension bsl-gradual-types-*.vsix
```

## 🧪 Тестирование

Открой файл [examples/test_lsp.bsl](../examples/test_lsp.bsl) в VS Code и проверь:

1. **Hover подсказки** ✅ — наведи на переменную (работает!)
2. **LSP сервер запущен** ✅ — View → Output → BSL Type Safety Analyzer
3. **Парсинг работает** ✅ — логи показывают `Tree-sitter parsed: X nodes`

### Проверка работоспособности

**Output панель (Ctrl+Shift+U):**
```
🔄 LSP Client state: Starting → Running  ← ✅ Должно быть Running!
✅ LSP client started successfully
[Info] BSL Language Server initialized!
```

**Логи Rust сервера:**
```bash
# Просмотр логов в реальном времени
tail -f vscode-extension/rust_lsp_server.log
```

### Что работает

✅ **Hover tooltips** — показывает информацию о типах при наведении
✅ **File parsing** — Tree-sitter парсит BSL синтаксис
✅ **v2 analysis** — система типов инициализирована

### Известные ограничения

⚠️ `textDocument/diagnostic` не реализован (не критично)
⚠️ `workspace/didChangeConfiguration` игнорируется (не критично)

Детали решения проблем: [LSP_SUCCESS.md](LSP_SUCCESS.md)

## 🎉 Готово!

LSP сервер запустится автоматически при открытии .bsl файлов.

---

## 🔧 Разработка расширения

### Автоматическое обновление бинарников

Расширение включает механизм автоматической синхронизации Rust бинарников:

```bash
# Компиляция с автоматическим копированием бинарников
npm run compile

# Принудительное обновление всех бинарников
npm run copy-binaries:force

# Создание production сборки (автоматически обновляет бинарники)
npm run package
```

**Что происходит автоматически:**
1. ✅ Проверка наличия `target/release/bsl-lsp-server.exe`
2. ✅ Автоматическая сборка Rust если бинарник отсутствует
3. ✅ Копирование в `vscode-extension/bin/lsp-server.exe`
4. ✅ Пропуск копирования если бинарник уже актуален

**Детали:** См. [scripts/README.md](scripts/README.md)

### Пересборка расширения после изменений

После изменений в Rust коде (backend):

```bash
cd vscode-extension

# Вариант 1: Быстрая пересборка (только если бинарник устарел)
npm run compile

# Вариант 2: Полная пересборка (принудительно)
npm run copy-binaries:force && npm run compile

# Вариант 3: Production сборка для публикации
npm run package
```

После изменений в TypeScript коде (расширение):

```bash
cd vscode-extension
npm run compile
```

### Переупаковка и переустановка

```bash
cd vscode-extension

# 1. Пересобрать бинарники и расширение
npm run package

# 2. Упаковать в .vsix
vsce package

# 3. Переустановить в VS Code
code --install-extension bsl-gradual-types-*.vsix --force
```

**Примечание:** `--force` флаг переустанавливает расширение даже если оно уже установлено.
