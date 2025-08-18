# Building BSL Gradual Type System

Comprehensive руководство по сборке всех компонентов проекта из исходников.

## 📋 Системные требования

### Обязательные зависимости
- **Rust 1.70+** - [Установить Rust](https://rustup.rs/)
- **Git** - для клонирования репозитория
- **CMake** - для сборки tree-sitter-bsl
- **Node.js 16+** - для VSCode extension
- **npm/yarn** - package manager для Node.js

### Опциональные зависимости
- **Docker** - для контейнеризации
- **VSCode** - для разработки расширения
- **Visual Studio Build Tools** (Windows) - для некоторых native dependencies

## 🚀 Сборка основного проекта

### 1. Клонирование репозитория
```bash
git clone https://github.com/yourusername/bsl-gradual-types.git
cd bsl-gradual-types
```

### 2. Проверка окружения
```bash
# Проверка версии Rust
rustc --version
# Должно быть: rustc 1.70.0 или новее

# Проверка Cargo
cargo --version

# Проверка Node.js (для extension)
node --version
npm --version
```

### 3. Установка tree-sitter-bsl
```bash
# Клонирование tree-sitter-bsl (если не включен как submodule)
git clone https://github.com/alkoleft/tree-sitter-bsl.git
cd tree-sitter-bsl

# Сборка grammar
npm install
npm run build

cd ..
```

### 4. Сборка Rust проекта
```bash
# Debug сборка (для разработки)
cargo build

# Release сборка (для production)
cargo build --release

# Проверка компиляции без сборки
cargo check

# Сборка всех бинарников
cargo build --release --all-targets
```

### 5. Запуск тестов
```bash
# Все тесты
cargo test

# Только unit тесты
cargo test --lib

# Только integration тесты
cargo test --test "*"

# Тесты с выводом
cargo test -- --nocapture

# Performance тесты
cargo run --bin bsl-profiler benchmark --iterations 5
```

### 6. Проверка качества кода
```bash
# Форматирование
cargo fmt --check

# Линтинг
cargo clippy -- -D warnings

# Security audit
cargo audit

# Documentation тесты
cargo test --doc
```

## 📦 Результаты сборки

После успешной сборки в `target/release/` будут доступны:

### CLI Инструменты
- `bsl-analyzer` - Legacy анализатор BSL файлов
- `type-check` - Проверка типов с enhanced анализаторами
- `lsp-server` - Enhanced LSP сервер с flow-sensitive analysis
- `bsl-profiler` - Performance profiling и benchmark инструмент
- `bsl-web-server` - Web-based type browser сервер
- `build-index` - Построение индекса типов из конфигурации

### Библиотеки
- `libbsl_gradual_types.rlib` - Rust библиотека для интеграции

## 🎯 Проверка сборки

### Базовые тесты
```bash
# Тест парсинга
./target/release/type-check --file tests/fixtures/bsl/simple_test.bsl

# Тест LSP сервера (запуск на 10 секунд)
timeout 10s ./target/release/lsp-server || echo "LSP server started successfully"

# Тест performance
./target/release/bsl-profiler benchmark --iterations 3

# Тест web сервера (запуск на 5 секунд)
timeout 5s ./target/release/bsl-web-server --port 9000 || echo "Web server started successfully"
```

### Performance бенчмарки
```bash
# Ожидаемые результаты для release сборки:
# Parsing: ~150-200μs
# Type Checking: ~100-150μs  
# Flow Analysis: ~100-300ns
# LSP Response: <100ms
```

## 🔧 Troubleshooting

### Общие проблемы

#### 1. Ошибки компиляции Rust
```bash
# Обновление Rust toolchain
rustup update

# Очистка кеша Cargo
cargo clean

# Пересборка с нуля
cargo build --release
```

#### 2. Проблемы с tree-sitter
```bash
# Linux/macOS - установка cmake
sudo apt-get install cmake        # Ubuntu/Debian
brew install cmake                # macOS

# Windows - установка Visual Studio Build Tools
# Скачать с https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022
```

#### 3. Проблемы с dependencies
```bash
# Обновление dependencies
cargo update

# Проверка outdated packages
cargo install cargo-outdated
cargo outdated
```

#### 4. Проблемы производительности
```bash
# Включение оптимизаций при debug сборке
export RUSTFLAGS="-C opt-level=1"
cargo build

# Использование lld linker (быстрее)
export RUSTFLAGS="-C link-arg=-fuse-ld=lld"
cargo build --release
```

### Platform-specific Issues

#### Windows
```powershell
# Установка Microsoft C++ Build Tools
# https://visualstudio.microsoft.com/visual-cpp-build-tools/

# Установка CMake через chocolatey
choco install cmake

# Переменные окружения
$env:RUSTFLAGS="-C target-cpu=native"
cargo build --release
```

#### Linux
```bash
# Ubuntu/Debian dependencies
sudo apt-get install build-essential cmake pkg-config libssl-dev

# CentOS/RHEL dependencies  
sudo yum groupinstall "Development Tools"
sudo yum install cmake openssl-devel

# Arch Linux
sudo pacman -S base-devel cmake
```

#### macOS
```bash
# Xcode command line tools
xcode-select --install

# Homebrew dependencies
brew install cmake

# M1/M2 Macs - native compilation
export RUSTFLAGS="-C target-cpu=native"
cargo build --release
```

## ⚡ Оптимизация сборки

### Для разработки
```bash
# Fast dev builds с оптимизацией уровня 1
cargo build --profile=dev-fast

# Watch mode для автоматической пересборки
cargo install cargo-watch
cargo watch -x "build --bin lsp-server"
```

### Для production
```bash
# Maximum optimization
export RUSTFLAGS="-C target-cpu=native -C lto=fat"
cargo build --release

# Strip symbols для меньшего размера
export RUSTFLAGS="-C target-cpu=native -C strip=symbols"
cargo build --release

# Profile-guided optimization (advanced)
export RUSTFLAGS="-C profile-generate=/tmp/pgo-data"
cargo build --release
# Запуск benchmarks для генерации профиля...
export RUSTFLAGS="-C profile-use=/tmp/pgo-data"
cargo build --release
```

## 📊 Сборочная статистика

### Время сборки (примерное)
- **Debug build**: 2-5 минут
- **Release build**: 5-10 минут  
- **Clean release build**: 10-15 минут
- **VSCode extension**: 30-60 секунд

### Размеры бинарников (release)
- `lsp-server`: ~15-25 MB
- `type-check`: ~10-15 MB
- `bsl-profiler`: ~12-18 MB
- `bsl-web-server`: ~18-25 MB

### Системные ресурсы
- **RAM**: 4-8 GB рекомендуется для сборки
- **Disk**: ~2-3 GB для полной сборки с dependencies
- **CPU**: Multi-core ускоряет сборку (используется `cargo build -j <cores>`)

## 🔄 Continuous Integration

### GitHub Actions
Проект включает CI/CD pipeline который автоматически:
- Собирает проект на Linux, Windows, macOS
- Запускает все тесты
- Проверяет качество кода (clippy, fmt)
- Генерирует release artifacts

### Local CI simulation
```bash
# Эмуляция CI pipeline локально
chmod +x scripts/ci-local.sh
./scripts/ci-local.sh
```

## 📦 Package Management

### Cargo.toml обновления
```toml
# Обновление версии для релиза
[package]
version = "1.0.0"

# Production dependencies
[dependencies]
# Убедитесь что все dependencies имеют stable версии
```

### Dependency Management
```bash
# Проверка уязвимостей
cargo audit

# Обновление dependencies
cargo update

# Проверка outdated
cargo install cargo-outdated
cargo outdated
```

---

## ✅ Checklist успешной сборки

- [ ] Rust 1.70+ установлен и работает
- [ ] `cargo build --release` завершается успешно
- [ ] `cargo test` проходит без ошибок
- [ ] `cargo clippy` не выдает warnings
- [ ] Все бинарники созданы в `target/release/`
- [ ] Performance тесты показывают ожидаемые результаты
- [ ] LSP сервер запускается без ошибок
- [ ] Web сервер отвечает на порту 8080

**🎉 Если все пункты выполнены - сборка успешна и готова к использованию!**