# Installation Guide

Детальное руководство по установке BSL Gradual Type System на различных платформах.

## 🎯 Варианты установки

### 📦 Option 1: Binary Release (Рекомендуется)
Самый простой способ для пользователей.

### 🔧 Option 2: Build from Source  
Для разработчиков и кастомизации.

### 🐳 Option 3: Docker
Для контейнеризованных окружений.

### 💻 Option 4: Package Managers
Через system package managers (планируется).

---

## 📦 Option 1: Binary Release

### Windows

#### Скачивание
```powershell
# Скачать latest release
$url = "https://github.com/yourusername/bsl-gradual-types/releases/latest/download/bsl-gradual-types-windows-x64.zip"
Invoke-WebRequest -Uri $url -OutFile "bsl-gradual-types.zip"

# Распаковка
Expand-Archive -Path "bsl-gradual-types.zip" -DestinationPath "C:\Tools\bsl-gradual-types"
```

#### Установка
```powershell
# Добавление в PATH
$env:PATH += ";C:\Tools\bsl-gradual-types\bin"

# Постоянное добавление в PATH
[Environment]::SetEnvironmentVariable("PATH", $env:PATH + ";C:\Tools\bsl-gradual-types\bin", "User")

# Проверка установки
bsl-analyzer --version
lsp-server --version
```

#### VSCode Extension
```powershell
# Скачивание extension
$extUrl = "https://github.com/yourusername/bsl-gradual-types/releases/latest/download/bsl-gradual-types-1.0.0.vsix"
Invoke-WebRequest -Uri $extUrl -OutFile "bsl-gradual-types.vsix"

# Установка
code --install-extension bsl-gradual-types.vsix
```

### Linux (Ubuntu/Debian)

#### Скачивание и установка
```bash
# Скачивание
wget https://github.com/yourusername/bsl-gradual-types/releases/latest/download/bsl-gradual-types-linux-x64.tar.gz

# Распаковка
tar -xzf bsl-gradual-types-linux-x64.tar.gz

# Установка в system
sudo cp bsl-gradual-types/bin/* /usr/local/bin/
sudo chmod +x /usr/local/bin/lsp-server
sudo chmod +x /usr/local/bin/bsl-profiler

# Проверка
which lsp-server
lsp-server --version
```

#### Package installation (планируется)
```bash
# Через APT (когда будет доступно)
curl -fsSL https://repo.bsl-gradual-types.org/gpg | sudo apt-key add -
echo "deb https://repo.bsl-gradual-types.org/apt stable main" | sudo tee /etc/apt/sources.list.d/bsl-gradual-types.list
sudo apt-get update
sudo apt-get install bsl-gradual-types
```

### macOS

#### Скачивание
```bash
# Intel Macs
curl -LO https://github.com/yourusername/bsl-gradual-types/releases/latest/download/bsl-gradual-types-macos-x64.tar.gz

# Apple Silicon (M1/M2)
curl -LO https://github.com/yourusername/bsl-gradual-types/releases/latest/download/bsl-gradual-types-macos-arm64.tar.gz
```

#### Установка
```bash
# Распаковка
tar -xzf bsl-gradual-types-macos-*.tar.gz

# Установка в system
sudo cp bsl-gradual-types/bin/* /usr/local/bin/

# Добавление в PATH (если нужно)
echo 'export PATH="/usr/local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc

# Проверка
lsp-server --version
```

#### Homebrew (планируется)
```bash
# Когда будет доступно
brew tap bsl-gradual-types/tap
brew install bsl-gradual-types
```

---

## 🔧 Option 2: Build from Source

### Prerequisites

#### Все платформы
- **Rust 1.70+**: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Git**: для клонирования репозитория
- **CMake**: для сборки tree-sitter dependencies

#### Windows специфичные
```powershell
# Visual Studio Build Tools
# Скачать с https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022

# Или через chocolatey
choco install visualstudio2022buildtools
choco install cmake
```

#### Linux специфичные
```bash
# Ubuntu/Debian
sudo apt-get install build-essential cmake pkg-config libssl-dev git

# CentOS/RHEL
sudo yum groupinstall "Development Tools"
sudo yum install cmake openssl-devel git

# Arch Linux
sudo pacman -S base-devel cmake git
```

#### macOS специфичные
```bash
# Xcode Command Line Tools
xcode-select --install

# Homebrew dependencies
brew install cmake git
```

### Сборка из исходников

#### 1. Клонирование
```bash
git clone https://github.com/yourusername/bsl-gradual-types.git
cd bsl-gradual-types
```

#### 2. Проверка окружения
```bash
# Rust версия
rustc --version
# Ожидается: rustc 1.70.0 или новее

# Cargo версия  
cargo --version

# CMake (для tree-sitter)
cmake --version
```

#### 3. Сборка
```bash
# Debug сборка (быстрее, для разработки)
cargo build

# Release сборка (оптимизированная)
cargo build --release

# Проверка сборки
ls -la target/release/
```

#### 4. Тестирование
```bash
# Запуск тестов
cargo test

# Performance тест
cargo run --bin bsl-profiler benchmark --iterations 3
```

#### 5. Установка
```bash
# Копирование бинарников в system PATH
sudo cp target/release/lsp-server /usr/local/bin/
sudo cp target/release/type-check /usr/local/bin/
sudo cp target/release/bsl-profiler /usr/local/bin/
sudo cp target/release/bsl-web-server /usr/local/bin/

# Windows
copy target\release\*.exe C:\Tools\bsl-gradual-types\bin\
```

---

## 🐳 Option 3: Docker

### Quick Docker Setup
```bash
# Скачивание Docker image (когда будет доступно)
docker pull bslgradualypes/bsl-gradual-types:1.0.0

# Или сборка из source
git clone https://github.com/yourusername/bsl-gradual-types.git
cd bsl-gradual-types
docker build -t bsl-gradual-types:local .
```

### Запуск контейнеров
```bash
# Web server
docker run -d -p 8080:8080 --name bsl-web bsl-gradual-types:1.0.0

# LSP server  
docker run -d -p 3000:3000 --name bsl-lsp bsl-gradual-types:1.0.0 ./lsp-server

# С mounted project directory
docker run -d -p 8080:8080 \
  -v /path/to/1c/project:/app/project:ro \
  --name bsl-web \
  bsl-gradual-types:1.0.0 \
  ./bsl-web-server --project /app/project --port 8080
```

### Docker Compose
```bash
# Скачивание docker-compose.yml
curl -O https://raw.githubusercontent.com/yourusername/bsl-gradual-types/master/docker-compose.yml

# Запуск full stack
docker-compose up -d

# Проверка
docker-compose ps
curl http://localhost:8080/api/stats
```

---

## 💻 IDE Integration Setup

### VSCode Extension

#### Автоматическая установка
```bash
# Из VS Marketplace (когда будет опубликовано)
code --install-extension bsl-gradual-types-team.bsl-gradual-types

# Проверка установки
code --list-extensions | grep bsl-gradual-types
```

#### Manual установка
```bash
# Скачивание .vsix
curl -LO https://github.com/yourusername/bsl-gradual-types/releases/latest/download/bsl-gradual-types-1.0.0.vsix

# Установка
code --install-extension bsl-gradual-types-1.0.0.vsix

# Или через VSCode UI:
# Ctrl+Shift+P → "Extensions: Install from VSIX" → выбрать файл
```

#### Конфигурация
```json
// settings.json
{
  "bsl.typeHints.showVariableTypes": true,
  "bsl.typeHints.showReturnTypes": true,
  "bsl.analysis.enableCaching": true,
  "bsl.performance.enableProfiling": false
}
```

### Другие IDE

#### IntelliJ IDEA (экспериментальный)
```bash
# LSP plugin для IntelliJ
# Settings → Plugins → Marketplace → "LSP Support"
# Settings → Languages → BSL → LSP Server: /usr/local/bin/lsp-server
```

#### Sublime Text
```json
// LSP-bsl.sublime-settings
{
  "command": ["/usr/local/bin/lsp-server"],
  "selector": "source.bsl"
}
```

#### Vim/Neovim
```lua
-- init.lua для neovim с nvim-lspconfig
require('lspconfig').bsl_gradual_types = {
  default_config = {
    cmd = {'/usr/local/bin/lsp-server'},
    filetypes = {'bsl'},
    root_dir = require('lspconfig.util').find_git_ancestor,
  },
}

require('lspconfig').bsl_gradual_types.setup{}
```

---

## ✅ Installation Verification

### Basic Functionality Test
```bash
# 1. CLI tools
lsp-server --version
type-check --version  
bsl-profiler --version
bsl-web-server --version

# 2. Basic analysis
echo 'Процедура Тест() КонецПроцедуры' > test.bsl
type-check --file test.bsl

# 3. Performance test
bsl-profiler benchmark --iterations 3

# 4. Web server test
bsl-web-server --port 9000 &
curl http://localhost:9000/api/stats
kill %1
```

### Expected Results
```
✅ All binaries respond to --version
✅ type-check analyzes simple BSL file successfully  
✅ bsl-profiler shows performance metrics
✅ Web server responds to API requests
✅ VSCode extension loads without errors (if installed)
```

### Performance Validation
Убедитесь что performance соответствует benchmarks:
- **Parsing**: 150-250μs
- **Type Checking**: 100-200μs
- **Flow Analysis**: 100-500ns
- **Web Response**: <100ms

## 🔧 Configuration

### System-wide Configuration
```bash
# Linux/macOS
sudo mkdir -p /etc/bsl-gradual-types
sudo cp config/default.toml /etc/bsl-gradual-types/config.toml

# Windows
mkdir "C:\ProgramData\bsl-gradual-types"
copy config\default.toml "C:\ProgramData\bsl-gradual-types\config.toml"
```

### User Configuration
```bash
# User-specific settings
mkdir -p ~/.config/bsl-gradual-types
cp config/user.toml ~/.config/bsl-gradual-types/config.toml

# Windows
mkdir "%APPDATA%\bsl-gradual-types"
copy config\user.toml "%APPDATA%\bsl-gradual-types\config.toml"
```

## 🚨 Troubleshooting Installation

### Common Issues

#### 1. "lsp-server not found"
```bash
# Check PATH
echo $PATH | grep bsl

# Add to PATH if missing
export PATH="/usr/local/bin:$PATH"

# Make permanent
echo 'export PATH="/usr/local/bin:$PATH"' >> ~/.bashrc
```

#### 2. Permission denied
```bash
# Fix permissions
sudo chmod +x /usr/local/bin/lsp-server
sudo chmod +x /usr/local/bin/type-check

# Or install to user directory
cp target/release/* ~/.local/bin/
```

#### 3. Missing dependencies
```bash
# Linux - install missing libraries
sudo apt-get install libssl1.1 ca-certificates

# Check dependencies
ldd /usr/local/bin/lsp-server
```

#### 4. VSCode extension не активируется
```bash
# Check extension list
code --list-extensions

# Check logs
# Ctrl+Shift+P → "Developer: Show Logs" → "Extension Host"

# Reinstall extension
code --uninstall-extension bsl-gradual-types-team.bsl-gradual-types
code --install-extension bsl-gradual-types-1.0.0.vsix
```

---

## 📋 Post-Installation Setup

### 1. Workspace Configuration
```json
// .vscode/settings.json в вашем 1С проекте
{
  "bsl.analysis.enableCaching": true,
  "bsl.analysis.cacheDirectory": "./.bsl_cache",
  "bsl.typeHints.showVariableTypes": true,
  "bsl.typeHints.minCertainty": 0.8
}
```

### 2. Project Structure Setup
```bash
# Создание .bsl_cache директории
mkdir .bsl_cache

# Добавление в .gitignore
echo ".bsl_cache/" >> .gitignore
echo "*.vsix" >> .gitignore

# Создание BSL configuration (если нужно)
mkdir -p src/cf
```

### 3. First Analysis
```bash
# Анализ вашего первого BSL файла
type-check --file src/CommonModules/YourModule.bsl

# Или через web interface
bsl-web-server --project . --port 8080
# Открыть http://localhost:8080
```

---

## 🎯 Next Steps

После успешной установки:

1. **📖 [Quick Start](QUICKSTART.md)** - Быстрый старт за 5 минут
2. **🎨 [Examples](../EXAMPLES.md)** - Практические примеры использования  
3. **🔧 [CLI Tools](../usage/CLI_TOOLS.md)** - Детальная справка по инструментам
4. **💻 [VSCode Guide](../usage/LSP_SERVER.md)** - Настройка IDE интеграции

---

## 📞 Getting Help

### Installation Issues
- 🐛 [GitHub Issues](https://github.com/yourusername/bsl-gradual-types/issues) - Bug reports
- 💬 [Discussions](https://github.com/yourusername/bsl-gradual-types/discussions) - Questions
- 📧 Email: install-help@bsl-gradual-types.org

### Community Resources
- 📖 [Documentation](../INDEX.md) - Full documentation index
- 🎥 [Video Tutorials](https://youtube.com/bsl-gradual-types) - Visual guides
- 💬 [Telegram](https://t.me/bsl_gradual_types) - Community chat

**✅ Installation complete! Ready to analyze BSL code with enterprise-grade type system!**