# 🏗️ Build Guide - BSL Gradual Types

## Архитектура после рефакторинга

### 🎯 Правильное разделение ответственности:

#### Frontend (WASM UI)
- **Цель**: Leptos компоненты для браузера
- **Файлы**: `frontend/src/lib.rs` с `wasm-bindgen` entry point
- **Сборка**: `trunk build` → WASM + HTML + CSS в `target/site/`
- **Нет binary targets** - только library с cdylib

#### Backend (API + LSP)
- **Цель**: Axum веб-сервер + Language Server Protocol
- **Файлы**: 
  - `backend/src/main.rs` → `bsl-web-server` (веб API + статические файлы)
  - `backend/src/bin/lsp_server.rs` → LSP сервер
- **Раздает**: WASM файлы из `target/site/` как статику

#### Shared (WASM-совместимые типы)
- **Цель**: Доменная логика для frontend и backend
- **Особенность**: Совместима с WASM (без `std` зависимостей)

## 🔨 Команды сборки

### Разработка

```bash
# 1. Собрать frontend (WASM)
cd frontend
trunk build

# 2. Собрать backend
cd ..
cargo build

# 3. Запустить веб-сервер
./target/debug/bsl-web-server --port 8080

# 4. Открыть http://127.0.0.1:8080
```

### Production

```bash
# 1. Release сборка frontend
cd frontend
trunk build --release

# 2. Release сборка backend
cd ..
cargo build --release

# 3. Запуск production сервера
./target/release/bsl-web-server --port 8080
```

### LSP сервер

```bash
# Сборка LSP сервера
cargo build --release --bin lsp-server

# Запуск
./target/release/lsp-server
```

## 🔧 Отдельные компоненты

```bash
# Только проверка компиляции
cargo check --workspace

# Сборка конкретного крейта
cargo build -p bsl-frontend
cargo build -p bsl-backend  
cargo build -p bsl-shared
cargo build -p bsl-cli

# Тесты
cargo test --workspace
```

## 📁 Структура после рефакторинга

```
bsl-gradual-types/
├── frontend/           # Leptos WASM UI
│   ├── src/lib.rs     # wasm-bindgen entry point  
│   ├── index.html     # Trunk template
│   └── Cargo.toml     # только [lib] crate-type = ["cdylib"]
│
├── backend/           # Axum API + LSP server
│   ├── src/main.rs    # веб-сервер (раздает WASM + API)
│   ├── src/bin/lsp_server.rs  # LSP сервер
│   └── Cargo.toml     # [lib] + [[bin]] targets
│
├── shared/            # WASM-совместимые типы
│   ├── src/lib.rs     # доменная логика
│   └── Cargo.toml     # без std зависимостей
│
├── target/site/       # Trunk собирает WASM сюда
│   ├── index.html     # HTML + JS loader
│   ├── *.wasm         # WASM модуль
│   └── *.js           # JS bindings
│
└── target/release/    # Rust executables
    ├── bsl-web-server # веб API + статика
    └── lsp-server     # Language Server
```

## ✅ Преимущества рефакторинга

1. **Четкое разделение ответственности**
   - Frontend = UI (WASM)
   - Backend = API + LSP (native)

2. **Стандартные практики**
   - Leptos apps = только library
   - Trunk работает с lib.rs
   - Нет коллизий имен файлов

3. **Простая сборка**
   - `trunk build` для UI
   - `cargo build` для сервера
   - Один endpoint для всего

4. **Production ready**
   - Оптимизированные WASM бинарии
   - Статический веб-сервер
   - LSP интеграция

## 🚀 Готово к разработке!

Архитектура теперь соответствует best practices и готова к дальнейшему развитию.
