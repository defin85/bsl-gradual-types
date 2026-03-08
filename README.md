# BSL Gradual Type System

*Система градуальной типизации для языка 1С:Предприятие (BSL)*

[![CI](https://github.com/defin85/bsl-gradual-types/actions/workflows/ci.yml/badge.svg)](https://github.com/defin85/bsl-gradual-types/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70+-brightgreen.svg)](https://www.rust-lang.org/)

GitHub Actions сейчас использует workflow `CI` как default fail-closed path для OpenSpec governance gate по затронутым `openspec/changes/<id>`. Локальные `cargo fmt`, `cargo clippy` и `cargo test` перед PR по-прежнему обязательны (см. `CONTRIBUTING.md`).

## 📋 Обзор

**BSL Gradual Type System** — современная система статического анализа и типизации для языка программирования 1С:Предприятие (BSL). Система обеспечивает:

- 🔍 **Анализ типов** с поддержкой flow-sensitive анализа
- 🌐 **Веб-интерфейс** для визуализации и исследования типов
- 🔧 **LSP сервер** для интеграции с редакторами кода
- ⚡ **Высокая производительность** для enterprise проектов
- 📚 **Автоматический парсинг** документации платформы 1С

## 🚀 Быстрый старт

### 1️⃣ Установка
```bash
git clone https://github.com/yourusername/bsl-gradual-types.git
cd bsl-gradual-types
cargo build --release
```

### 2️⃣ Веб-интерфейс (рекомендуется)
```bash
# Базовый запуск
cargo run -p bsl-backend --bin bsl-web-server -- --port 3001
# Открыть: http://localhost:3001

# С указанием синтаксис-помощника 1С
cargo run -p bsl-backend --bin bsl-web-server -- \
  --syntax-helper-path "examples/syntax_helper/rebuilt.shcntx_ru" \
  --port 3001
```

### 3️⃣ CLI анализ
```bash
echo 'Функция Тест() Возврат "привет"; КонецФункции' > test.bsl
cargo run -p cli --bin bsl-type-check -- test.bsl --verbose
```

### 4️⃣ VSCode расширение
```bash
cd vscode-extension
npm install && npm run compile && vsce package
code --install-extension bsl-gradual-types-*.vsix
```

## ⚙️ Конфигурация

### 📁 Подготовка ресурсов

**Требуемые ресурсы:**
- **Синтаксис-помощник 1С** - HTML файлы документации платформы
- **Конфигурация проекта** - XML файлы метаданных 1С (Configuration.xml)

**Получение синтаксис-помощника:**
1. Откройте Конфигуратор 1С
2. Меню "Справка" → "Содержание справки"
3. Экспортируйте как `.shcntx_ru.zip`
4. **Распакуйте ZIP в папку** `./syntax_helper/`

### 🛠️ CLI аргументы

```bash
# Веб-сервер
cargo run -p bsl-backend --bin bsl-web-server -- [OPTIONS]

ОСНОВНЫЕ ОПЦИИ:
  --syntax-helper-path <PATH>  Путь к HTML файлам синтаксис-помощника
                               (абсолютный или относительный)
  --project-path <PATH>        Путь к конфигурации 1С
                               (абсолютный или относительный)
  --port <PORT>                Порт сервера (по умолчанию: 8080)
  --enable-cors                Включить CORS для разработки
  --log-level <LEVEL>          Уровень логирования

# CLI анализ
cargo run -p cli --bin bsl-type-check -- <FILE> [--verbose]
```

**Примеры путей:**
```bash
# Относительные пути (от корня проекта)
--syntax-helper-path "./syntax_helper"
--syntax-helper-path "examples/syntax_helper/rebuilt.shcntx_ru"

# Абсолютные пути
--syntax-helper-path "C:/1C/Platform/8.3.24/bin/ru_RU"
--syntax-helper-path "/opt/1cv8/8.3.24/share/doc/1cv8/ru_RU"
```

### 🌍 Переменные окружения

```bash
export BSL_SYNTAX_HELPER_PATH="/path/to/syntax/helper"
export BSL_CONFIG_PATH="/path/to/config"
export BSL_LOG_LEVEL=debug
cargo run -p bsl-backend --bin bsl-web-server
```

### 📊 API

```bash
# Получить все типы
curl "http://localhost:3001/api/types"

# Поиск типов
curl "http://localhost:3001/api/search?q=Строка"

# Метрики и здоровье системы
curl "http://localhost:3001/api/metrics"
curl "http://localhost:3001/api/health"
```

## ✨ Ключевые возможности

### 🔍 Продвинутый анализ типов
- **Flow-Sensitive Analysis** - отслеживание изменений типов по мере выполнения
- **Union Types** - полноценные union типы с весами (`String 60% | Number 40%`)
- **Межпроцедурный анализ** - анализ типов через границы функций
- **Type Narrowing** - уточнение типов в условиях (`ТипЗнч(x) = Тип("Строка")`)

### ⚡ Enterprise Performance
- **Парсинг**: ~189μs | **Type Checking**: ~125μs | **Flow Analysis**: ~175ns
- **Кеширование** результатов анализа между сессиями
- **Параллельный анализ** больших проектов с rayon
- **Memory optimization** для enterprise нагрузок

### 🛠️ Production Tooling
- **LSP сервер**
- **VSCode Extension** с type hints и code actions
- **Web-based Type Browser** для команд разработки
- **CLI инструменты** для автоматизации и CI/CD

### 🧭 Configuration-guided Discovery (NEW!)
- **Полностью автоматический парсинг** конфигураций 1С:Предприятие
- **Configuration.xml как опорный файл** - 100% соответствие структуре
- **Динамическое обнаружение типов** метаданных без хардкода
- **Поддержка всех элементов**: Attribute, Resource, Dimension
- **Автоматические стандартные атрибуты** (Код, Наименование, Дата, Период)
- **Поддержка иерархии и владельцев** справочников

## 🛠️ Инструменты разработки

### 💻 VSCode Extension

### Сборка расширения
```bash
cd vscode-extension

# Установка зависимостей
npm install

# Компиляция TypeScript
npm run compile

# Упаковка extension
npm install -g vsce
vsce package

# Установка в VSCode
code --install-extension bsl-gradual-types-*.vsix
```

### Возможности extension
- **Type Hints** - inline отображение типов в коде
- **Enhanced Hover** - детальная информация о типах с union весами
- **Code Actions** - автоматические исправления (объявление переменных, type fixes)
- **Real-time диагностика** с flow-sensitive анализом
- **Performance Monitor** - статистика LSP операций в status bar

### Настройки
```json
{
  "bsl.typeHints.showVariableTypes": true,
  "bsl.typeHints.showReturnTypes": true,
  "bsl.analysis.enableCaching": true,
  "bsl.performance.enableProfiling": false
}
```


## 🏗️ Архитектура

### Слоистая архитектура
```
┌─────────────────────────────────────────┐
│    System (Coordination, Performance)   │
├─────────────────────────────────────────┤
│    Application (Services, LSP)          │
├─────────────────────────────────────────┤
│    Presentation (Adapters, Interfaces)  │
├─────────────────────────────────────────┤
│    Domain (Types, Analysis, Contracts)  │
├─────────────────────────────────────────┤
│    Data (Repository, Loaders)           │
├─────────────────────────────────────────┤
│    Parsing (BSL, Tree-sitter)           │
└─────────────────────────────────────────┘
```

### Ключевые модули (Плоская архитектура)
- **Domain**: `types.rs`, `analysis/`, `resolvers/`, `contracts.rs`
- **Parsing**: `bsl/tree_sitter_adapter.rs` (на основе tree-sitter-bsl)
- **LSP**: `lsp_enhanced.rs` с инкрементальным парсингом
- **Tools**: `profiler.rs`, `web_server.rs`

## 🧪 Тестирование

```bash
# Все тесты
cargo test

# Performance тесты
cargo run --bin bsl-profiler benchmark --iterations 10

# Проверка extension
cd vscode-extension && npm test
```

## 🚀 Production Deployment

### Docker
```bash
# Build image
docker build -t bsl-gradual-types .

# Run web server
docker run -p 8080:8080 bsl-gradual-types

# With project analysis
docker run -p 8080:8080 -v /path/to/1c:/app/project:ro bsl-gradual-types \
  ./bsl-web-server --project /app/project --port 8080
```

### Systemd Service
```ini
# /etc/systemd/system/bsl-web.service
[Unit]
Description=BSL Type Browser
After=network.target

[Service]
ExecStart=/usr/local/bin/bsl-web-server --port 8080
Restart=always
User=bsl-analyzer

[Install]
WantedBy=multi-user.target
```

## 🧭 Configuration-guided Discovery

### Новый подход к парсингу конфигураций 1С

**Configuration-guided Discovery** - революционный подход к парсингу конфигураций 1С:Предприятие, использующий `Configuration.xml` как авторитетный источник структуры.

### ✨ Принципы
- **Никаких предположений** о структуре каталогов
- **Configuration.xml как источник истины** - читаем `<ChildObjects>` для получения полного списка объектов
- **Динамическое обнаружение файлов** - рекурсивный поиск XML по всей структуре
- **Полное извлечение атрибутов** - пользовательские (Attribute, Resource, Dimension) + стандартные

### 🔧 Использование
```bash
# Быстрый тест
cargo run --example test_simple

# Unit-тесты с assertions 
cargo test --test config_parser_guided_test

# Использование в коде
use bsl_gradual_types::data::loaders::config_parser_guided_discovery::ConfigurationGuidedParser;

let mut parser = ConfigurationGuidedParser::new("path/to/configuration");
let type_resolutions = parser.parse_with_configuration_guide()?;
```

### 📊 Результаты парсинга
- **100% точность** - только объекты из Configuration.xml
- **Полная извлечение** - все Resource, Attribute, Dimension + стандартные поля
- **Автоматическая типизация** - Строка(25), СправочникСсылка.Объект, ДатаВремя
- **TypeResolution для всех фасетов** - Manager, Object, Reference

### 🚀 TODO для интеграции
- [ ] Подключить в `PlatformResolver` для замены статических парсеров
- [ ] Добавить поддержку табличных частей с их атрибутами
- [ ] Реализовать кеширование результатов парсинга
- [ ] Интегрировать в LSP сервер для live типизации
- [ ] Добавить CLI команду `cargo run --bin config-parser -- path/to/config`

## 📊 Performance Benchmarks

| Component | Time | Status |
|-----------|------|--------|
| Parsing | ~189μs | ✅ Excellent |
| Type Checking | ~125μs | ✅ Production Ready |
| Flow Analysis | ~175ns | ✅ Blazing Fast |
| LSP Response | <100ms | ✅ Responsive |
| **Config Discovery** | **~5ms** | **✅ NEW!** |

## 🤝 Contributing

1. Fork репозитория
2. Создайте feature branch: `git checkout -b feature/name`
3. Внесите изменения и добавьте тесты
4. Убедитесь что `cargo test` и `cargo clippy` проходят
5. Создайте Pull Request

### Стандарты кода
```bash
cargo fmt      # Форматирование
cargo clippy   # Линтинг
cargo test     # Тесты
```

## 📄 Лицензия

MIT License - см. [LICENSE](LICENSE)

## 📞 Поддержка

- 🐛 [Issues](https://github.com/yourusername/bsl-gradual-types/issues) - Баги и вопросы
- 💬 [Discussions](https://github.com/yourusername/bsl-gradual-types/discussions) - Обсуждения
- 📖 [Детальная документация](docs/reference/target_architecture/overview.md) - Архитектура системы

---

⭐ **Понравился проект? Поставьте звездочку на GitHub!**

**BSL Gradual Type System** — enterprise-ready решение для анализа типов в 1С:Предприятие проектах.
