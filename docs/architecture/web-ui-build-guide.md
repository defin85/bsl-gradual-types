# 🏗️ Build Guide - BSL Gradual Types

**Последнее обновление:** 2025-11-07
**Версия:** 0.4.2
**Статус:** Production Ready

---

## 📚 Содержание

1. [Архитектура проекта](#архитектура-проекта)
2. [Команды сборки](#команды-сборки)
3. [Frontend (Leptos WASM UI)](#frontend-leptos-wasm-ui)
4. [Backend (API + LSP)](#backend-api--lsp)
5. [Troubleshooting](#troubleshooting)

---

## 🎯 Архитектура проекта

### Правильное разделение ответственности:

#### **Frontend (WASM UI)**
- **Цель**: Leptos компоненты для браузера
- **Точка входа**: `frontend/src/lib.rs` с `wasm-bindgen` entry point
- **Сборка**: `trunk build` → WASM + HTML + CSS в `target/site/`
- **Дизайн**: Tailwind CSS + BSL цветовая палитра (из `front_template/`)
- **Тип**: Library с `crate-type = ["cdylib"]` (НЕТ binary targets)

#### **Backend (API + LSP)**
- **Цель**: Axum веб-сервер + Language Server Protocol
- **Файлы**:
  - `backend/src/main.rs` → `bsl-web-server` (веб API + статика)
  - `backend/src/bin/lsp_server.rs` → LSP сервер
- **Раздает**:
  - WASM файлы из `backend/static/` как статику
  - API endpoints: `/api/types`, `/api/search`, `/api/health`

#### **Shared (WASM-совместимые типы)**
- **Цель**: Доменная логика для frontend и backend
- **Особенность**: Совместима с WASM (без `std` зависимостей, где возможно)
- **DTOs**: TypeDto, MethodDto, AnalysisResultDto, MetricsDto

---

## 🔨 Команды сборки

### 🛠️ Development (Быстрая разработка)

```bash
# 1. Собрать frontend (WASM + Tailwind CSS)
cd frontend
trunk build

# 2. Скопировать собранный frontend в backend/static
cd ..
cp -r target/site/* backend/static/

# 3. Запустить веб-сервер с парсингом платформенных типов
cargo run -p bsl-backend --bin bsl-web-server -- \
  --port 3002 \
  --enable-cors true \
  --syntax-helper-path examples/syntax_helper

# 4. Открыть http://127.0.0.1:3002
```

**Ожидаемый результат:**
```
🚀 BSL Type System Web UI (CSR) listening on http://127.0.0.1:3002
📊 Загружено 3927 типов из синтаксис-помощника
```

---

### 🚀 Production (Оптимизированная сборка)

```bash
# 1. Release сборка frontend (минифицированный WASM + CSS)
cd frontend
trunk build --release

# 2. Скопировать в backend/static
cd ..
cp -r target/site/* backend/static/

# 3. Release сборка backend
cargo build --release -p bsl-backend --bin bsl-web-server

# 4. Запуск production сервера
./target/release/bsl-web-server \
  --port 3002 \
  --enable-cors true \
  --syntax-helper-path examples/syntax_helper
```

**Bundle размеры (после оптимизации):**
```
bsl-frontend.wasm:     ~850 KB (release)
tailwind.css:          ~58 KB
main.css:              ~16 KB
index.html:            ~1 KB
──────────────────────────────
ИТОГО:                 ~925 KB
```

---

### 🔧 LSP Server

```bash
# Сборка LSP сервера
cargo build --release --bin lsp-server

# Запуск (для VSCode Extension)
./target/release/lsp-server

# Проверка работы
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | \
  ./target/release/lsp-server
```

---

## 🎨 Frontend (Leptos WASM UI)

### Структура frontend/

```
frontend/
├── src/
│   ├── lib.rs              # WASM entry point
│   ├── app.rs              # Main App (unified interface) ✅ ИСПОЛЬЗУЕТСЯ
│   ├── api/                # API client для бэкенда
│   ├── components/         # UI компоненты
│   │   ├── dashboard.rs    # Dashboard с метриками
│   │   ├── cards_view.rs   # Cards режим с карточками типов
│   │   ├── table_view.rs   # Table режим
│   │   ├── graph_view.rs   # Graph режим
│   │   ├── sidebar.rs      # Sticky фильтры (NEW!)
│   │   ├── back_to_top.rs  # Back to top button (NEW!)
│   │   └── ...
│   └── utils/              # Утилиты
│
├── style/
│   ├── main.css            # CSS variables (от front_template)
│   └── tailwind.css        # Tailwind directives
│
├── index.html              # Trunk template
├── Trunk.toml              # dist = "../target/site"
├── tailwind.config.js      # BSL цветовая палитра
└── Cargo.toml              # [lib] crate-type = ["cdylib"]
```

### Дизайн система (front_template)

**Цветовая палитра:**
```css
/* Primitives */
--color-cream-50: rgb(252, 252, 249)
--color-teal-500: rgb(33, 128, 141)
--color-brown-600: rgb(94, 82, 64)
--color-slate-900: rgb(19, 52, 59)

/* Category colors */
Platform: #3498db (blue)
Configuration: #e74c3c (red)
Union: #9b59b6 (purple)
Dynamic: #f39c12 (orange)
```

**Tailwind mapping:**
- `bg-bsl-cream-50` → cream background
- `text-bsl-slate-900` → dark text
- `border-bsl-brown-600/20` → brown borders 20% opacity

### Ключевые компоненты

#### **Dashboard** (`components/dashboard.rs`)
- 8 метрик карточек с градиентами
- BSL цветовая схема (исправлено: purple→teal, indigo→orange)

#### **CardsView** (`components/cards_view.rs`)
- Адаптивный grid: 2/3/4 колонки
- **Топ-линия по категориям** (синий/красный/фиолетовый/оранжевый)
- **Certainty bar** с градиентом (red→orange→teal)
- **Facet tags** (Object, Collection, Manager, Reference)
- **Union Types секция** для Union/Dynamic типов
- **Flow-sensitive badge** (синий бейдж)
- **Tooltip** для обрезанного текста (`title` атрибуты)
- Пагинация (50 из 3362)

#### **Sidebar** (`components/sidebar.rs`)
- **Sticky** с `overflow-y: auto` (best practice)
- Фильтры по категориям и определенности
- Brown borders (не teal)

#### **BackToTop** (`components/back_to_top.rs`) - NEW!
- Floating action button справа внизу
- Появляется после 300px прокрутки
- BSL teal цвета

### UX Best Practices (реализовано)

1. ✅ **Sticky Header** - навигация всегда доступна
2. ✅ **Sticky Sidebar** - фильтры всегда доступны (как Amazon/GitHub)
3. ✅ **Динамический shadow** на header при скролле
4. ✅ **Back to top button** - для быстрого возврата
5. ❌ **Pagination НЕ sticky** - стандартная практика

---

## 🖥️ Backend (API + LSP)

### Структура backend/

```
backend/
├── src/
│   ├── main.rs                 # Веб сервер (Axum)
│   ├── bin/lsp_server.rs       # LSP сервер
│   ├── system/                 # System Layer
│   ├── domain/                 # Domain Layer
│   ├── data/                   # Data Layer
│   └── presentation/           # API endpoints
│
├── static/                     # Frontend статика (копируется из target/site/)
│   ├── index.html              # HTML entry point
│   ├── bsl-frontend-*.js       # WASM bindings
│   ├── bsl-frontend-*_bg.wasm  # WASM binary
│   ├── main-*.css              # CSS variables
│   └── tailwind-*.css          # Tailwind styles
│
└── Cargo.toml                  # [lib] + [[bin]] targets
```

### API Endpoints

```bash
# Health check
curl http://127.0.0.1:3002/api/health

# Все типы
curl http://127.0.0.1:3002/api/types

# Поиск (с URL-encoding для кириллицы)
curl "http://127.0.0.1:3002/api/search?q=%D0%9C%D0%B0%D1%81%D1%81%D0%B8%D0%B2"

# Метрики
curl http://127.0.0.1:3002/api/metrics
```

**URL-encoding для кириллицы:**
```bash
# В GitBash используй python для encoding
python3 -c "import urllib.parse; print(urllib.parse.quote('Массив'))"
# Результат: %D0%9C%D0%B0%D1%81%D1%81%D0%B8%D0%B2
```

---

## 🔄 Workflow сборки

### Полный цикл разработки

```bash
# Шаг 1: Изменения во frontend
cd frontend
# Редактируй src/components/*.rs

# Шаг 2: Пересборка frontend
trunk build --release

# Шаг 3: Копирование в backend/static
cd ..
cp -r target/site/* backend/static/

# Шаг 4: Перезапуск веб сервера
# Остановить текущий (Ctrl+C)
cargo run -p bsl-backend --bin bsl-web-server -- \
  --port 3002 \
  --enable-cors true \
  --syntax-helper-path examples/syntax_helper

# Шаг 5: Hard reload в браузере (Ctrl+Shift+R)
# Открыть http://127.0.0.1:3002
```

### Быстрая проверка изменений

```bash
# Проверка компиляции без полной сборки
cd frontend && cargo check

# Только backend (если frontend не менялся)
cargo check -p bsl-backend
```

---

## 🧹 Cleanup (после рефакторинга)

### ❌ Legacy код (УДАЛЕНО)

```bash
# frontend/src/pages/ - старая архитектура с фиолетовым дизайном
# Содержала:
# - pages/app.rs (legacy App с Navigation компонентом)
# - pages/dashboard.rs (старый Dashboard)
# - Конфликтовала с src/app.rs

# Решение:
rm -rf frontend/src/pages/

# lib.rs обновлен:
// pub mod pages; // Legacy - removed
pub use app::App; // Только unified interface
```

### ✅ Активный код

- `frontend/src/app.rs` - **ЕДИНСТВЕННЫЙ** App компонент (unified interface)
- Использует компоненты из `components/` (CardsView, Dashboard, Sidebar, etc.)
- Дизайн на основе `front_template/` (cream/teal/brown цвета)

---

## 🐛 Troubleshooting

### Проблема: Браузер показывает старый дизайн (фиолетовый header)

**Причина:** Кэш браузера или конфликт двух App компонентов

**Решение:**
```bash
# 1. Убедиться что pages/ удалена
ls frontend/src/pages/  # Должно быть: No such file

# 2. Пересобрать с нуля
cd frontend
trunk build --release

# 3. Удалить старые файлы из static
rm backend/static/*

# 4. Скопировать новые
cp -r ../target/site/* ../backend/static/

# 5. Hard reload в браузере
# Chrome: Ctrl+Shift+R
# Firefox: Ctrl+F5
```

### Проблема: Ошибки компиляции frontend

**Частые ошибки:**
```rust
// ❌ Неправильно
use leptos::For;  // For не существует в leptos 0.8

// ✅ Правильно
use leptos::prelude::*;  // For уже в prelude
```

```rust
// ❌ Неправильно
info.facets.into_iter()  // Перемещает ownership

// ✅ Правильно
info.facets.iter()  // Заимствует
```

### Проблема: Прокрутка двигает header вместо контента

**Причина:** Неправильная структура layout

**Решение:** (УЖЕ ИСПРАВЛЕНО)
```rust
// app.rs
view! {
  <div class="min-h-screen flex flex-col">
    <header class="sticky top-0 z-50">  // Sticky header
    <main class="flex-1 overflow-auto">  // Scrollable main
      <div class="grid grid-cols-[280px_1fr]">
        <Sidebar />  // Sticky sidebar
        <Content />  // Scrollable content
      </div>
    </main>
  </div>
}
```

### Проблема: Tailwind классы не применяются

**Причина:** Файлы не включены в `content` конфигурации

**Решение:**
```js
// tailwind.config.js
export default {
  content: [
    "./src/**/*.rs",  // ✅ Все Rust файлы
    "./index.html",   // ✅ HTML template
  ],
  // ...
}
```

---

## 📦 Детали сборки frontend

### Trunk конфигурация (`Trunk.toml`)

```toml
[build]
target = "index.html"
dist = "../target/site"  # Собираем в общий target
no_sri = true            # Отключить SRI для development

[watch]
watch = ["src"]
```

### Tailwind CSS процесс

```
1. trunk build запускается
   ↓
2. Читает tailwind.config.js
   ↓
3. Сканирует content: ["./src/**/*.rs", "./index.html"]
   ↓
4. Генерирует tailwind-*.css с используемыми классами
   ↓
5. Копирует в target/site/
```

### WASM компиляция

```
1. cargo build --target=wasm32-unknown-unknown --release
   ↓
2. wasm-bindgen генерирует JS bindings
   ↓
3. Оптимизация WASM (wasm-opt)
   ↓
4. Результат: bsl-frontend-*.wasm + bsl-frontend-*.js
```

---

## 📊 Отдельные компоненты

```bash
# Проверка компиляции всего workspace
cargo check --workspace

# Сборка конкретного крейта
cargo build -p bsl-frontend   # WASM UI (НЕ РАБОТАЕТ напрямую - используй trunk!)
cargo build -p bsl-backend    # API + LSP
cargo build -p bsl-shared     # Shared типы
cargo build -p bsl-cli        # CLI tools

# Тесты
cargo test --workspace
cargo test -p bsl-shared  # Только shared
```

**ВАЖНО:** `bsl-frontend` - это library, не binary. Для сборки используй `trunk build`!

---

## 📁 Структура после рефакторинга (2025-11-07)

```
bsl-gradual-types/
├── frontend/              # Leptos WASM UI
│   ├── src/
│   │   ├── lib.rs        # wasm-bindgen entry point
│   │   ├── app.rs        # ✅ Main App (unified interface)
│   │   ├── api/          # API client
│   │   ├── components/   # UI компоненты
│   │   │   ├── dashboard.rs
│   │   │   ├── cards_view.rs
│   │   │   ├── table_view.rs
│   │   │   ├── graph_view.rs
│   │   │   ├── sidebar.rs       # Sticky sidebar
│   │   │   ├── back_to_top.rs   # NEW! Floating button
│   │   │   └── ...
│   │   └── utils/
│   │
│   ├── style/
│   │   ├── main.css      # CSS variables (front_template)
│   │   └── tailwind.css  # Tailwind directives
│   │
│   ├── index.html
│   ├── Trunk.toml
│   ├── tailwind.config.js  # BSL color palette
│   └── Cargo.toml
│
├── backend/              # Axum API + LSP server
│   ├── src/
│   │   ├── main.rs       # bsl-web-server
│   │   ├── bin/lsp_server.rs
│   │   ├── system/       # System Layer
│   │   ├── domain/       # Domain Layer
│   │   ├── data/         # Data Layer (loaders, parsers)
│   │   └── presentation/ # API routes
│   │
│   ├── static/           # Frontend статика (от trunk build)
│   │   ├── index.html
│   │   ├── *.wasm
│   │   ├── *.js
│   │   └── *.css
│   │
│   └── Cargo.toml
│
├── shared/               # WASM-совместимые типы
│   └── src/
│       ├── lib.rs
│       ├── api/          # DTOs (TypeDto, MethodDto, ...)
│       ├── domain/       # TypeResolver, AnalysisEngine
│       └── data/         # TypeRepository
│
├── target/site/          # Trunk собирает сюда (временно)
│   ├── index.html
│   ├── bsl-frontend-*.wasm
│   ├── bsl-frontend-*.js
│   ├── main-*.css
│   └── tailwind-*.css
│
└── target/release/       # Rust executables
    ├── bsl-web-server    # веб API + статика
    └── lsp-server        # Language Server
```

---

## ✅ Преимущества текущей архитектуры

### 1. **Четкое разделение ответственности**
- Frontend = UI (WASM)
- Backend = API + LSP (native)
- Shared = Доменная логика

### 2. **Стандартные практики**
- Leptos apps = только library (`cdylib`)
- Trunk работает с `lib.rs`
- Нет коллизий имен файлов

### 3. **Простая сборка**
- `trunk build` для UI
- `cargo build` для сервера
- Один endpoint для всего

### 4. **Production ready**
- Оптимизированные WASM бинарии
- Минифицированный CSS (Tailwind purge)
- Статический веб-сервер
- LSP интеграция для VSCode

### 5. **UX Enhancements** (2025-11-07)
- ✅ Sticky sidebar (фильтры всегда доступны)
- ✅ Back to top button (удобная навигация)
- ✅ Динамический shadow (визуальная обратная связь)
- ✅ Адаптивный layout (2/3/4 колонки)
- ✅ Tooltip для обрезанного текста

---

## 🚀 Готово к разработке!

Архитектура соответствует best practices 2024-2025 и готова к дальнейшему развитию.

**Следующие шаги:**
- Используй `.claude/skills/web-ui.md` для автоматизированной сборки
- Смотри `TAILWIND_INTEGRATION_ROADMAP.md` для статуса миграции
- Проверяй `docs/guides/development-workflow.md` для полного workflow
