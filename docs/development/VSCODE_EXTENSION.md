# VSCode Extension - Building & Publishing Guide

Полное руководство по сборке, тестированию и публикации VSCode расширения BSL Gradual Type System.

## 📋 Требования

### Системные требования
- **Node.js 16+** - [Скачать Node.js](https://nodejs.org/)
- **npm 8+** или **yarn 1.22+**
- **VSCode 1.75+** - для тестирования
- **Git** - для version control

### Инструменты разработки
```bash
# Установка vsce (VSCode Extension Manager)
npm install -g vsce

# Установка ovsx (Open VSX Registry publisher)
npm install -g ovsx

# TypeScript compiler (если нужен глобально)
npm install -g typescript
```

## 🚀 Подготовка к сборке

### 1. Переход в директорию extension
```bash
cd vscode-extension
```

### 2. Установка зависимостей
```bash
# Установка Node.js dependencies
npm install

# Или с yarn
yarn install

# Проверка установки
npm list --depth=0
```

### 3. Проверка конфигурации
```bash
# Проверка package.json
cat package.json | grep -E "(name|version|main)"

# Проверка TypeScript конфигурации  
cat tsconfig.json
```

## 🔧 Сборка расширения

### 1. Компиляция TypeScript
```bash
# Единоразовая компиляция
npm run compile

# Или
tsc -p ./

# Watch mode для разработки
npm run watch

# Или
tsc -watch -p ./
```

### 2. Линтинг и проверки
```bash
# TypeScript проверки
npm run lint

# Или
tsc --noEmit

# Проверка форматирования (если настроено)
npm run format:check

# Автоматическое форматирование
npm run format
```

### 3. Тестирование расширения
```bash
# Запуск тестов
npm test

# Или
npm run pretest && node ./out/test/runTest.js

# Отдельные тесты
npm run test:unit
npm run test:integration
```

## 📦 Упаковка расширения

### 1. Pre-publish подготовка
```bash
# Полная пересборка
npm run clean
npm install
npm run compile

# Финальные проверки
npm run lint
npm test
```

### 2. Копирование бинарников (если нужно)
```bash
# Сборка Rust бинарников
cd ..
cargo build --release

# Копирование в extension
cd vscode-extension
mkdir -p bin

# Копирование нужных бинарников
cp ../target/release/lsp-server.exe bin/        # Windows
cp ../target/release/lsp-server bin/            # Linux/macOS
cp ../target/release/type-check.exe bin/        # Windows  
cp ../target/release/type-check bin/            # Linux/macOS
cp ../target/release/bsl-profiler.exe bin/      # Windows
cp ../target/release/bsl-profiler bin/          # Linux/macOS
```

### 3. Создание .vsix пакета
```bash
# Упаковка расширения
vsce package

# Это создаст файл: bsl-gradual-types-1.0.0.vsix

# Упаковка с конкретной версией
vsce package --no-git-tag-version 1.0.0

# Упаковка с pre-release версией  
vsce package --pre-release

# Проверка содержимого пакета
vsce ls
```

### 4. Проверка .vsix файла
```bash
# Установка локально для тестирования
code --install-extension bsl-gradual-types-1.0.0.vsix

# Проверка что extension загрузился
code --list-extensions | grep bsl-gradual-types

# Тест основной функциональности
# Откройте BSL файл в VSCode и проверьте:
# - LSP server connection
# - Syntax highlighting  
# - Auto completion
# - Hover information
```

## 📢 Публикация расширения

### 1. Visual Studio Marketplace

#### Подготовка к публикации
```bash
# Создание publisher аккаунта (если нет)
# https://marketplace.visualstudio.com/manage

# Login в vsce
vsce login <publisher-name>

# Проверка токена
vsce verify-pat <personal-access-token>
```

#### Публикация
```bash
# Первая публикация
vsce publish

# Публикация конкретной версии
vsce publish 1.0.0

# Pre-release публикация
vsce publish --pre-release

# Публикация с готовым .vsix
vsce publish bsl-gradual-types-1.0.0.vsix
```

### 2. Open VSX Registry (для других editors)

#### Подготовка
```bash
# Создание аккаунта в Open VSX
# https://open-vsx.org/

# Login в ovsx
ovsx create-namespace <namespace>
ovsx verify-pat <access-token>
```

#### Публикация
```bash
# Публикация в Open VSX
ovsx publish bsl-gradual-types-1.0.0.vsix

# Или прямая публикация
ovsx publish
```

### 3. GitHub Releases

#### Создание release
```bash
# Tag версии
git tag v1.0.0
git push origin v1.0.0

# GitHub CLI (если установлен)
gh release create v1.0.0 bsl-gradual-types-1.0.0.vsix \
  --title "BSL Gradual Type System v1.0.0" \
  --notes "Enterprise-ready release with full IDE ecosystem"
```

## 🔄 Development Workflow

### 1. Настройка окружения разработки
```bash
# Установка в dev mode
npm run install:dev

# Запуск в режиме разработки
code --extensionDevelopmentPath=./vscode-extension

# Или F5 в VSCode с открытой директорией extension
```

### 2. Hot reload разработка
```bash
# Terminal 1: TypeScript watch
npm run watch

# Terminal 2: VSCode development host
code --extensionDevelopmentPath=./vscode-extension

# Terminal 3: Rust watch (если изменяется LSP)
cd ..
cargo watch -x "build --bin lsp-server"
```

### 3. Debugging расширения
```bash
# Включение debug логов
export DEBUG="bsl-gradual-types:*"

# Или в VSCode launch.json
{
  "type": "extensionHost",
  "request": "launch",
  "name": "Launch Extension",
  "runtimeExecutable": "${execPath}",
  "args": ["--extensionDevelopmentPath=${workspaceFolder}"],
  "env": {
    "DEBUG": "bsl-gradual-types:*"
  }
}
```

## 📊 Автоматизация сборки

### 1. npm scripts
```json
// package.json
{
  "scripts": {
    "vscode:prepublish": "npm run compile",
    "compile": "tsc -p ./",
    "watch": "tsc -watch -p ./",
    "pretest": "npm run compile && npm run lint",
    "lint": "tsc --noEmit",
    "test": "node ./out/test/runTest.js",
    "package": "vsce package",
    "publish": "vsce publish",
    "publish:ovsx": "ovsx publish"
  }
}
```

### 2. GitHub Actions для автоматической публикации
```yaml
# .github/workflows/vscode-extension.yml
name: VSCode Extension CI

on:
  push:
    tags: ['v*']

jobs:
  build-and-publish:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    
    - name: Setup Node.js
      uses: actions/setup-node@v4
      with:
        node-version: '18'
        
    - name: Install dependencies
      run: |
        cd vscode-extension
        npm install
        
    - name: Build extension
      run: |
        cd vscode-extension
        npm run compile
        npm test
        
    - name: Package extension
      run: |
        cd vscode-extension
        npm install -g vsce
        vsce package
        
    - name: Publish to Marketplace
      run: |
        cd vscode-extension
        vsce publish -p ${{ secrets.VSCE_TOKEN }}
        
    - name: Publish to Open VSX
      run: |
        cd vscode-extension
        npx ovsx publish -p ${{ secrets.OVSX_TOKEN }}
        
    - name: Upload artifact
      uses: actions/upload-artifact@v4
      with:
        name: vscode-extension
        path: vscode-extension/*.vsix
```

## 🧪 Тестирование расширения

### 1. Unit тесты
```bash
cd vscode-extension

# Запуск unit тестов
npm run test:unit

# Или прямо
npx mocha out/test/suite/extension.test.js
```

### 2. Integration тесты
```bash
# Запуск integration тестов с VSCode instance
npm run test:integration

# Manual тестирование
code --extensionDevelopmentPath=. test_extension.bsl
```

### 3. LSP интеграция тесты
```bash
# Тест LSP connection
npm run test:lsp

# Manual проверка LSP
# 1. Открыть BSL файл
# 2. Проверить hover на переменных
# 3. Проверить auto completion
# 4. Проверить diagnostics
# 5. Проверить type hints (если включены)
```

## 🚀 Release процесс

### 1. Pre-release checklist
- [ ] Версии обновлены в package.json и CHANGELOG
- [ ] Все тесты проходят  
- [ ] LSP сервер собран в release режиме
- [ ] README расширения обновлен
- [ ] Screenshots обновлены (если нужно)

### 2. Версионирование
```bash
# Обновление версии
npm version patch   # 1.0.0 → 1.0.1
npm version minor   # 1.0.0 → 1.1.0  
npm version major   # 1.0.0 → 2.0.0

# Или вручную в package.json
```

### 3. Release команды
```bash
# Полный release процесс
npm run build:release
npm run test:all
npm run package
npm run publish:all

# Где publish:all это
npm run publish         # VS Marketplace
npm run publish:ovsx    # Open VSX Registry
```

## 📋 Checklist публикации

### Pre-publish
- [ ] ✅ TypeScript компилируется без ошибок
- [ ] ✅ Все тесты проходят
- [ ] ✅ Линтинг проходит без warnings
- [ ] ✅ LSP сервер бинарники актуальны
- [ ] ✅ README и CHANGELOG обновлены
- [ ] ✅ Version bumped в package.json

### Post-publish
- [ ] ✅ Extension доступен в VS Marketplace
- [ ] ✅ Extension доступен в Open VSX
- [ ] ✅ GitHub release создан с .vsix файлом
- [ ] ✅ Documentation обновлена
- [ ] ✅ Community уведомлено

## 🔧 Troubleshooting Extension

### Общие проблемы

#### 1. TypeScript ошибки
```bash
# Очистка и пересборка
npm run clean
npm install
npm run compile
```

#### 2. LSP connection проблемы
```bash
# Проверка LSP сервера
../target/release/lsp-server --version

# Debug LSP в VSCode
# Ctrl+Shift+P → "Developer: Reload Window"
# Check Output → "BSL Gradual Types"
```

#### 3. Packaging ошибки
```bash
# Проверка files в package.json
vsce ls

# Исключение ненужных файлов
# Добавить в .vscodeignore:
node_modules/
src/
*.ts
*.map
```

#### 4. Publication ошибки
```bash
# Проверка токена
vsce verify-pat

# Проверка прав publisher
vsce show <publisher-name>

# Re-login если нужно
vsce logout
vsce login <publisher-name>
```

## 📊 Metrics & Analytics

### Download статистика
```bash
# Проверка downloads в VS Marketplace
vsce show bsl-gradual-types-team.bsl-gradual-types

# Analytics в publisher dashboard
# https://marketplace.visualstudio.com/manage/publishers/<publisher-name>
```

### User feedback
- Monitor VS Marketplace reviews
- GitHub Issues для bug reports
- Community feedback в Discussions

---

## 🎯 Advanced Topics

### Custom LSP Protocol Extension
Extension поддерживает enhanced LSP methods:
- `bsl/enhancedHover` - Детальная hover информация
- `bsl/performanceProfiling` - Performance profiling
- `bsl/projectAnalysis` - Parallel project analysis
- `bsl/clearCache` - Cache management

### Performance Optimization
```typescript
// В extension коде
const performanceMonitor = new PerformanceMonitor(outputChannel);
performanceMonitor.enable();

// Automatic performance tracking для LSP operations
```

### Multi-platform Support
Extension автоматически определяет platform и использует соответствующие бинарники:
- `bin/lsp-server.exe` - Windows
- `bin/lsp-server` - Linux/macOS

---

## ✅ Final Extension Checklist

### Before Publishing
- [ ] ✅ Extension компилируется без ошибок
- [ ] ✅ Все тесты проходят (unit + integration)
- [ ] ✅ LSP connection работает
- [ ] ✅ Type hints отображаются корректно
- [ ] ✅ Code actions функционируют
- [ ] ✅ Performance monitoring активен
- [ ] ✅ README и screenshots актуальны
- [ ] ✅ Version и changelog обновлены

### After Publishing
- [ ] ✅ Extension установлен из marketplace
- [ ] ✅ Основная функциональность работает
- [ ] ✅ Performance в пределах нормы
- [ ] ✅ No critical errors в logs
- [ ] ✅ User feedback мониторится

**🎉 Если все пункты выполнены - extension готов к production использованию!**

---

## 📞 Support

При проблемах со сборкой extension:
- 📖 [VSCode Extension API](https://code.visualstudio.com/api)
- 🐛 [Issues](https://github.com/yourusername/bsl-gradual-types/issues)
- 💬 [Discussions](https://github.com/yourusername/bsl-gradual-types/discussions)