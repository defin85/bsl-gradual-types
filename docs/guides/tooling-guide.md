# Tooling Guide

Руководство по инструментам разработки и анализа BSL Gradual Types проекта.

## 🛠️ MCP Инструментарий

Claude Code предоставляет богатый набор MCP (Model Context Protocol) инструментов для эффективной работы с проектом.

### 🌐 Chrome DevTools - Автоматизация веб-интерфейса

**Назначение:** Тестирование BSL Web Server через браузер

#### Запуск сервера для тестирования

```bash
# Базовый запуск
cargo run -p bsl-backend --bin bsl-web-server -- --port 3002 --enable-cors true

# С полными типами платформы
cargo run -p bsl-backend --bin bsl-web-server -- \
  --port 3002 \
  --enable-cors true \
  --syntax-helper-path examples/syntax_helper
```

#### Доступные команды Chrome DevTools

| Команда | Описание | Пример использования |
|---------|----------|---------------------|
| `take_screenshot` | Снимок экрана | Документация UI |
| `click` | Клик по элементу | Тестирование кнопок |
| `fill` | Заполнение полей | Тестирование поиска |
| `hover` | Наведение на элемент | Проверка tooltips |
| `list_network_requests` | Анализ API запросов | Отладка endpoints |
| `performance_start_trace` | Измерение производительности | Профилирование WASM |
| `evaluate_script` | Выполнение JavaScript | Интеграционные тесты |

#### Типовые сценарии

**1. Тестирование поиска типов**

```javascript
// Через Chrome DevTools
navigate_page("http://127.0.0.1:3002")
fill("#search-input", "Массив")
click("#search-button")
take_screenshot("search-results.png")
```

**2. Проверка производительности WASM**

```javascript
performance_start_trace()
navigate_page("http://127.0.0.1:3002")
// Дождаться загрузки WASM модуля
performance_stop_trace()
// Результат: LCP, FCP, Core Web Vitals
```

**3. Анализ API запросов**

```javascript
navigate_page("http://127.0.0.1:3002")
fill("#search-input", "Справочники")
click("#search-button")
list_network_requests()
// Результат: GET /api/search?q=%D0%A1%D0%BF%D1%80%D0%B0%D0%B2%D0%BE%D1%87%D0%BD%D0%B8%D0%BA%D0%B8
```

---

### 🔍 Language Server Protocol - Rust диагностика

**Назначение:** Проверка качества Rust кода через LSP

#### Доступные команды

| Команда | Описание | Пример |
|---------|----------|--------|
| `diagnostics` | Ошибки компиляции | Проверка backend/ |
| `hover` | Информация о типах | Анализ TypeResolver |
| `definition` | Переход к определению | Поиск реализации |
| `references` | Все использования символа | Рефакторинг |
| `rename_symbol` | Безопасный рефакторинг | Переименование |

#### Типовые сценарии

**1. Проверка TypeResolver**

```bash
# Получить диагностику
diagnostics(shared/src/domain/resolver.rs)

# Hover на resolve_member_access()
hover(shared/src/domain/resolver.rs, line: 124, column: 8)

# Найти все использования
references("TypeResolver::resolve_member_access")
```

**2. Рефакторинг SystemCoordinator**

```bash
# Найти определение
definition("SystemCoordinator")

# Найти все места использования
references("SystemCoordinator")

# Безопасно переименовать
rename_symbol(
  file: "backend/src/system/mod.rs",
  line: 45,
  column: 12,
  new_name: "SystemOrchestrator"
)
```

**3. Проверка совместимости API**

```bash
# Проверить, что application фасад использует AnalysisEngine корректно
hover(backend/src/application/type_system_service.rs, line: 120, column: 25)

# Проверить сигнатуру метода
definition("AnalysisEngine::analyze_program")
```

---

### 🔍 Sourcebot - Поиск архитектурных паттернов

**Назначение:** Regex и семантический поиск по всему репозиторию

#### Поиск ключевых компонентов

```bash
# Точный поиск компонента
search_code(query: "SystemCoordinator", include_snippets: true)

# Семантический поиск концепций
search_code(query: "градуальная типизация|gradual typing", include_snippets: true)

# Поиск архитектурных паттернов
search_code(query: "dependency injection|IoC container", include_snippets: true)
```

#### Примеры запросов

| Запрос | Тип | Что найдёт |
|--------|-----|------------|
| `SystemCoordinator` | Точный | Все упоминания компонента |
| `координатор\|зависимост\|архитектур` | Regex | Концептуально связанные темы |
| `TypeResolver::resolve` | Точный | Конкретные методы |
| `управление жизненным циклом` | Семантический | Русскоязычные концепции |
| `flow.*sensitive\|CFG\|control.*flow` | Regex | Flow-sensitive analysis код |

#### Типовые сценарии

**1. Исследование Right-Sized Architecture**

```bash
# Поиск упоминаний архитектурных принципов
search_code(query: "Right-Sized|right.*sized|layer.*architecture")

# Поиск DI паттернов
search_code(query: "dependency.*injection|IoC|inversion.*control")
```

**2. Поиск примеров использования SemanticIR**

```bash
# Найти все места, где используется SemanticProgram
search_code(query: "SemanticProgram", include_snippets: true)

# Найти AstToIrConverter usage
search_code(query: "AstToIrConverter|ast.*to.*ir", include_snippets: true)
```

**3. Многоязычный поиск (русский + английский)**

```bash
# Поиск типов (русские + английские термины)
search_code(query: "TypeRepository|репозиторий.*тип|type.*repository")

# Поиск фасетной системы
search_code(query: "FacetKind|фасет|facet|Manager|Object|Reference")
```

---

### 📚 Context7 - Документация библиотек

**Назначение:** Актуальная документация внешних зависимостей

#### Основные библиотеки проекта

```bash
# Leptos (frontend WASM)
resolve_library_id("leptos")
get_library_docs("/leptos/leptos", topic: "components")

# Axum (web server)
resolve_library_id("axum")
get_library_docs("/tokio-rs/axum", topic: "routing")

# Tower (middleware)
get_library_docs("/tower-rs/tower", topic: "service")

# Tree-sitter (парсинг)
resolve_library_id("tree-sitter")
get_library_docs("/tree-sitter/tree-sitter", topic: "parsing")

# Serde (сериализация)
get_library_docs("/serde-rs/serde", topic: "derive")
```

#### Типовые сценарии

**1. Обновление зависимостей**

```bash
# Проверить breaking changes в Leptos
get_library_docs("/leptos/leptos/v0.7.0", topic: "migration guide")

# Изучить новые API в Axum
get_library_docs("/tokio-rs/axum/v0.8.0", topic: "what's new")
```

**2. Поиск примеров использования**

```bash
# Как использовать Axum middleware
get_library_docs("/tokio-rs/axum", topic: "middleware examples")

# Leptos reactive primitives
get_library_docs("/leptos/leptos", topic: "signals")
```

**3. Troubleshooting**

```bash
# Tree-sitter error handling
get_library_docs("/tree-sitter/tree-sitter", topic: "error recovery")

# Serde custom serialization
get_library_docs("/serde-rs/serde", topic: "custom serialize")
```

---

### 🌐 Tavily - Веб-исследования

**Назначение:** Поиск информации о концепциях и best practices

#### Примеры запросов

```bash
# Градуальная типизация
tavily_search(query: "gradual typing systems implementation")

# Архитектурные паттерны
tavily_search(query: "rust dependency injection patterns")

# TypeScript/Rust интеграция
tavily_search(query: "rust wasm typescript integration best practices")

# Производительность WASM
tavily_search(query: "leptos wasm performance optimization 2025")
```

#### Типовые сценарии

**1. Исследование flow-sensitive анализа**

```bash
tavily_search(
  query: "flow sensitive type analysis control flow graph",
  search_depth: "advanced"
)
```

**2. Архитектурные решения**

```bash
tavily_search(
  query: "type checker architecture design patterns",
  include_domains: ["github.com", "stackoverflow.com"]
)
```

**3. Актуальные практики 2025**

```bash
tavily_search(
  query: "rust LSP server implementation 2025",
  time_range: "month"
)
```

---

## 🔬 ast-grep - Анализ структуры кода

**Назначение:** Быстрый поиск и подсчёт элементов кода

### Установка

```bash
# Установка ast-grep
cargo install ast-grep
```

### Основные команды

#### Подсчёт элементов

```bash
# Структуры в проекте
ast-grep run -p "struct " -l rust . | wc -l

# Enum'ы
ast-grep run -p "enum " -l rust . | wc -l

# Impl блоки
ast-grep run -p "impl " -l rust . | wc -l

# Публичные функции
ast-grep run -p "pub fn" -l rust . | head -20
```

#### Поиск архитектурных компонентов

```bash
# Ключевые компоненты системы
ast-grep run -p "SystemCoordinator\|AnalysisHostV2\|ParserCoordinator" -l rust .

# Доменные типы
ast-grep run -p "enum" -l rust shared/src/domain/

# Parser trait реализации
ast-grep run -p "impl Parser" -l rust .
```

#### Поиск паттернов

```bash
# Result<T, E> usage
ast-grep run -p "Result<" -l rust backend/src/

# Async функции
ast-grep run -p "async fn" -l rust backend/src/

# Error types
ast-grep run -p "Error\|Err" -l rust shared/src/
```

### Рекомендации по использованию

✅ **Хорошо для:**
- Быстрая статистика структуры проекта
- Подсчёт элементов кода
- Поиск архитектурных компонентов
- Анализ паттернов использования

❌ **Ограничения:**
- Простые текстовые паттерны надёжнее сложных AST-паттернов
- Не подходит для семантического анализа
- Требует комбинации с grep/Read для глубокого анализа

---

## 📊 Комплексные сценарии

### Сценарий 1: Полное тестирование системы

```bash
# 1. Запуск backend
cargo run -p bsl-backend --bin bsl-web-server -- --port 3002

# 2. Chrome DevTools автотесты
# - Загрузка интерфейса и снимок
# - Тестирование поиска "Справочники"
# - Проверка фильтров и навигации
# - Измерение LCP и Core Web Vitals

# 3. Language Server диагностика
# - Проверка всех Rust файлов на ошибки
# - Анализ типов в shared/src/domain/
# - Валидация API контрактов

# 4. Автоматизация через Skills
/test-runner
/api-tester
```

### Сценарий 2: Рефакторинг архитектуры

```bash
# 1. Sourcebot - поиск паттернов использования
search_code(query: "SystemCoordinator", include_snippets: true)

# 2. Language Server - анализ зависимостей
references("SystemCoordinator")
definition("SystemCoordinator::new")

# 3. ast-grep - статистика использования
ast-grep run -p "SystemCoordinator" -l rust .

# 4. Chrome DevTools - проверка не поломался ли UI
navigate_page("http://127.0.0.1:3002")
take_screenshot("after-refactoring.png")

# 5. Context7 - изучение альтернативных подходов
get_library_docs("/tower-rs/tower", topic: "service composition")
```

### Сценарий 3: Исследование производительности

```bash
# 1. Chrome DevTools Performance trace
performance_start_trace()
# ... выполнение операций ...
performance_stop_trace()

# 2. Анализ Network requests
list_network_requests()
# Оптимизация медленных API endpoints

# 3. Tavily - поиск бенчмарков
tavily_search(query: "rust wasm performance benchmarks leptos")

# 4. Context7 - документация по оптимизации
get_library_docs("/leptos/leptos", topic: "performance optimization")
```

---

## 🎯 Рекомендации по использованию

### Проактивное использование

| Инструмент | Когда использовать |
|------------|-------------------|
| Chrome DevTools | После изменений UI, перед релизом |
| Language Server | При рефакторинге Rust кода |
| Sourcebot | Для изучения архитектурных решений |
| Context7 | Перед обновлением зависимостей |
| Tavily | При исследовании новых концепций |
| ast-grep | Для статистики и обзора структуры |

### Эффективные комбинации

**LSP + Chrome DevTools**
- LSP диагностика → исправление ошибок → Chrome DevTools тестирование

**Sourcebot + Context7**
- Sourcebot поиск паттернов → Context7 документация библиотек → реализация

**Performance trace + Tavily**
- Chrome DevTools trace → выявление узких мест → Tavily поиск оптимизаций

---

## ⚠️ Специфика BSL проекта

### Фасетная система

Требует особого внимания к типам при использовании LSP:

```bash
# Hover на Справочники.Контрагенты
# Должен показывать активный фасет: Manager | Object | Reference
hover(file: "test.bsl", line: 5, column: 20)
```

### WASM компоненты

Лучше тестировать через реальный браузер (Chrome DevTools):

```javascript
// Проверка загрузки WASM модуля
evaluate_script(`
  console.log(window.wasmLoaded);
  return window.wasmInstance !== undefined;
`)
```

### Русскоязычные термины

При поиске используй оба языка:

```bash
# Sourcebot
search_code(query: "TypeRepository|репозиторий.*тип")

# Tavily
tavily_search(query: "1C:Enterprise static typing gradual")
```

---

## 🤖 Автоматизация через Claude Skills

Для частых задач используй Skills:

```bash
# Комплексное тестирование
/test-runner

# Тестирование Web API
/api-tester

# Проверка Roadmap
/roadmap-checker
```

**См. также:** `.claude/skills/` для детальных описаний навыков

---

## 🔗 Связанные руководства

- **[Development Workflow](development-workflow.md)** — команды cargo/npm/bash
- **[Roadmap Verification](roadmap-verification.md)** — проверка выполнения задач
- **[Web API Reference](../api/web-api-reference.md)** — endpoints для тестирования

---

## 📚 Дополнительные ресурсы

- **MCP Documentation:** https://modelcontextprotocol.io/
- **ast-grep Guide:** https://ast-grep.github.io/
- **Chrome DevTools Protocol:** https://chromedevtools.github.io/devtools-protocol/
- **Rust Analyzer LSP:** https://rust-analyzer.github.io/
