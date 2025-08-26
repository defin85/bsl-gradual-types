# Roadmap: Полное избавление от legacy кода

## Цель
Полностью удалить все legacy структуры (`core/`, `architecture/`, `unified/`, `target/`) и перевести проект на чистую плоскую архитектуру.

## Текущее состояние
- ✅ Основные компоненты перенесены в новые слои
- ✅ Временные реэкспорты настроены в `core/mod.rs`
- 🔴 **62 legacy файла** нуждаются в удалении
- 🔴 **100+ legacy импортов** нуждаются в обновлении
- 🔴 **4 legacy папки** нуждаются в удалении

## Этап 1: Массовое обновление импортов (2-3 часа)

### 1.1 Основной код src/ (26 импортов, ~1 час)

**Приоритет: КРИТИЧЕСКИЙ** - основной код должен работать без legacy зависимостей

| Файл | Legacy импорты | Новые импорты | Статус |
|------|----------------|---------------|--------|
| `src/domain/types.rs` | `use crate::core::types::*` | Удалить (уже есть в domain) | 🔴 |
| `src/documentation/**/*.rs` | `super::core::hierarchy`, `super::core::providers` | `crate::application::documentation_service` | 🔴 |
| `src/presentation/adapters.rs` | `use crate::unified::data::` | `use crate::data::` | 🔴 |
| `src/system/coordination.rs` | комментарии `unified::system::` | обновить комментарии | 🔴 |

**Действия:**
```bash
# Найти все основные файлы с legacy импортами
find src/ -name "*.rs" -not -path "*/core/*" -not -path "*/architecture/*" -not -path "*/unified/*" -not -path "*/bin/*" -exec grep -l "use.*::core::\|use.*::unified::\|use.*::architecture::" {} \;

# Наиболее проблемные файлы:
# 1. src/domain/types.rs - удалить use crate::core::types::*
# 2. src/documentation/ - заменить super::core:: на crate::application::
# 3. src/presentation/adapters.rs - заменить unified::data на data::
```

### 1.2 Тесты (24 импорта, ~45 минут)

**Приоритет: ВЫСОКИЙ** - тесты должны проходить после миграции

| Файл | Legacy импорты | Новые импорты | Статус |
|------|----------------|---------------|--------|
| `tests/union_types_test.rs` | `core::types`, `core::union_types` | `domain::types`, `domain::analysis::union_types` | 🔴 |
| `tests/type_narrowing_test.rs` | `core::type_checker` | `domain::analysis::type_checker` | 🔴 |
| `tests/interprocedural_test.rs` | `core::dependency_graph`, `core::interprocedural`, `core::type_checker` | `domain::analysis::*` | 🔴 |
| `tests/flow_sensitive_test.rs` | `core::dependency_graph`, `core::flow_sensitive` | `domain::analysis::*` | 🔴 |
| `tests/facet_system_test.rs` | `core::platform_resolver`, `core::types` | `domain::resolvers::platform`, `domain::types` | 🔴 |
| `tests/contracts_test.rs` | `core::*` | `domain::contracts` | 🔴 |
| `tests/repository_basic_test.rs` | `unified::data` | `data::*` | 🔴 |
| `tests/domain_metrics_test.rs` | `unified::data`, `unified::domain` | `data::*`, `domain::*` | 🔴 |

### 1.3 Примеры (17 импортов, ~45 минут)

**Приоритет: СРЕДНИЙ** - примеры должны компилироваться

| Категория | Файлы | Legacy паттерны | Новые паттерны |
|-----------|-------|----------------|----------------|
| Type System | `test_*_type_system.rs`, `visualize_*.rs` | `core::unified_type_system`, `core::type_system_service` | `domain::unified_type_system`, `domain::type_system_service` |
| Platform | `test_completion.rs`, `test_guided_*.rs` | `core::platform_resolver` | `domain::resolvers::platform` |  
| Documentation | `test_*_docs.rs`, `test_hierarchy_*.rs` | `documentation::core` | `application::documentation_service` |
| Query | `query_demo.rs` | `core::context` | `domain::context` |
| Parsing | `syntax_helper_*.rs` | `core::types` | `domain::types` |

### 1.4 Бинарники (9 импортов, ~30 минут)

**Приоритет: ВЫСОКИЙ** - бинарники должны запускаться

| Файл | Legacy импорты | Статус |
|------|----------------|--------|
| `src/bin/lsp_server.rs` | `system::*` | ✅ Уже обновлен |
| `src/bin/profiler.rs` | `core::parallel_analysis`, `core::performance` | � |
| `src/bin/web_server.rs` | `core::type_checker` | 🔴 |  
| `src/bin/test_guided_discovery.rs` | `core::platform_resolver` | 🔴 |
| `src/bin/build_index.rs` | `architecture::data` | 🔴 |

## Этап 2: Удаление legacy файлов (1 час)

### 2.1 Подготовка к удалению (15 минут)
```bash
# Создать backup
git add -A && git commit -m "Pre-cleanup snapshot"

# Проверить что нет активных импортов (должно быть 0)
grep -r "use.*::core::" --include="*.rs" src/ tests/ examples/ | grep -v "src/core/"
grep -r "use.*::architecture::" --include="*.rs" src/ tests/ examples/
grep -r "use.*::unified::" --include="*.rs" src/ tests/ examples/
```

### 2.2 Удаление файлов core/ (15 минут)
```bash
# Удалить все файлы кроме mod.rs
find src/core/ -name "*.rs" ! -name "mod.rs" -delete

# Очистить mod.rs до минимума
echo '//! Legacy core module - DEPRECATED' > src/core/mod.rs
```

### 2.3 Удаление legacy папок (15 минут)
```bash
# Удалить старые архитектурные папки
rm -rf src/architecture/
rm -rf src/unified/
rm -rf src/target/

# Удалить пустые папки
rm -rf src/parser/
rm -rf src/query/

# Удалить папку adapters (если все перенесено в data/loaders)
# rm -rf src/adapters/ # Проверить содержимое сначала
```

### 2.4 Обновление lib.rs (15 минут)

Удалить legacy подключения из `src/lib.rs`:
```rust
// УДАЛИТЬ эти строки:
// pub mod core; 
// pub mod architecture;
// pub mod unified;
// pub mod target;
// pub mod parser;
// pub mod query;
```

## Этап 3: Финальная очистка core (30 минут)

### 3.1 Удаление core/mod.rs и папки
```bash
# После того как все импорты обновлены и тесты проходят
rm -rf src/core/
```

### 3.2 Обновление документации
```bash
# Обновить docs/README.md с новой структурой
# Обновить CONTRIBUTING.md 
# Обновить copilot-instructions.md
```

## Этап 4: Проверка и тестирование (1-2 часа)

### 4.1 Проверка компиляции (30 минут)
```bash
# Полная пересборка
cargo clean
cargo check
cargo build
cargo build --release
```

### 4.2 Запуск тестов (45 минут)
```bash
# Unit тесты
cargo test

# Integration тесты  
cargo test --test '*'

# Benchmarks
cargo bench --no-run
```

### 4.3 Проверка примеров (30 минут)
```bash
# Проверить что все примеры компилируются
find examples/ -name "*.rs" -exec cargo check --example {} \;

# Запустить ключевые примеры
cargo run --example query_demo
cargo run --example syntax_helper_demo
```

### 4.4 Проверка бинарников (15 минут)
```bash
# Проверить что все бинарники работают
cargo run --bin lsp-server --help
cargo run --bin bsl-web-server --help
cargo run --bin type-check --help
```

## Детальный план действий по импортам

### Шаблон замены для тестов:
```bash
# В каждом тесте заменить:
sed -i \
  -e 's/use bsl_gradual_types::core::types::/use bsl_gradual_types::domain::types::/g' \
  -e 's/use bsl_gradual_types::core::type_checker::/use bsl_gradual_types::domain::analysis::type_checker::/g' \
  -e 's/use bsl_gradual_types::core::dependency_graph::/use bsl_gradual_types::domain::analysis::dependency_graph::/g' \
  -e 's/use bsl_gradual_types::core::flow_sensitive::/use bsl_gradual_types::domain::analysis::flow_sensitive::/g' \
  -e 's/use bsl_gradual_types::core::interprocedural::/use bsl_gradual_types::domain::analysis::interprocedural::/g' \
  -e 's/use bsl_gradual_types::core::platform_resolver::/use bsl_gradual_types::domain::resolvers::platform::/g' \
  -e 's/use bsl_gradual_types::core::union_types::/use bsl_gradual_types::domain::analysis::union_types::/g' \
  -e 's/use bsl_gradual_types::unified::data::/use bsl_gradual_types::data::/g' \
  -e 's/use bsl_gradual_types::unified::domain::/use bsl_gradual_types::domain::/g' \
  tests/*.rs
```

## Порядок обработки файлов:
1. **Основной код src/** (критичен для архитектуры)
2. **Бинарники** (должны запускаться)  
3. **Тесты** (должны проходить)
4. **Примеры** (должны компилироваться)

### Обоснование нового порядка приоритетов:

**1. Основной код src/ (26 импортов) - ПЕРВЫЙ ПРИОРИТЕТ**
- Это **ядро системы** - если оно не работает, ничего не работает
- **34%** от всех legacy импортов - самая большая проблема
- Включает критические файлы: `domain/types.rs`, `documentation/`, `presentation/`

**2. Бинарники (9 импортов) - ВТОРОЙ ПРИОРИТЕТ**  
- **Пользовательский интерфейс** - LSP, веб-сервер, анализатор
- Если не работают бинарники - пользователи не могут использовать систему
- Относительно мало импортов, но критичны для функциональности

**3. Тесты (24 импорта) - ТРЕТИЙ ПРИОРИТЕТ**
- **Проверка корректности** после изменений
- Важны для CI/CD, но не блокируют работу системы
- Можно временно отключить проблемные тесты

**4. Примеры (17 импортов) - ЧЕТВЕРТЫЙ ПРИОРИТЕТ**
- **Демонстрация возможностей** системы  
- Важны для документации, но не критичны для работы
- Можно обновить после основной функциональности

## Метрики прогресса

### Контрольные точки:
- [ ] 0% legacy импортов в тестах
- [ ] 0% legacy импортов в примерах  
- [ ] 0% legacy импортов в бинарниках
- [ ] 0% legacy файлов в src/
- [ ] 0% legacy папок в src/
- [ ] ✅ cargo check проходит
- [ ] ✅ cargo test проходит
- [ ] ✅ все бинарники работают

### Команды для проверки прогресса:
```bash
# Подсчет legacy импортов
echo "Legacy core импорты:"; grep -r "use.*::core::" --include="*.rs" . | wc -l
echo "Legacy unified импорты:"; grep -r "use.*::unified::" --include="*.rs" . | wc -l  
echo "Legacy architecture импорты:"; grep -r "use.*::architecture::" --include="*.rs" . | wc -l

# Подсчет legacy файлов
echo "Legacy файлы:"; find src/ -path "*/core/*" -o -path "*/architecture/*" -o -path "*/unified/*" -o -path "*/target/*" | wc -l

# Проверка компиляции
cargo check 2>&1 | grep "error\|warning" | wc -l
```

## Оценка времени

| Этап | Оптимистично | Реалистично | Пессимистично |
|------|-------------|-------------|---------------|
| Импорты в тестах | 30 мин | 45 мин | 1 час |
| Импорты в примерах | 1 час | 1.5 часа | 2 часа |
| Импорты в бинарниках | 20 мин | 30 мин | 45 мин |
| Внутренние импорты | 30 мин | 45 мин | 1 час |
| Удаление файлов | 45 мин | 1 час | 1.5 часа |
| Тестирование | 1 час | 1.5 часа | 2 часа |
| **ИТОГО** | **4 часа** | **5.5 часа** | **8 часов** |

## Риски и митигация

### Высокие риски:
1. **Поломка тестов** → Обрабатывать тесты по одному с immediate проверкой
2. **Циклические импорты** → Проверять cargo check после каждого большого изменения
3. **Потеря функциональности** → Делать git commit на каждом этапе

### Средние риски:
1. **Долгая отладка** → Использовать автоматические замены sed/grep где возможно
2. **Regression в примерах** → Проверять ключевые примеры в конце

### План отката:
```bash
# В случае критических проблем
git reset --hard HEAD~1  # Откат к последнему commit
```

## Готовность к запуску

**Предварительные требования:**
- [ ] Текущие тесты проходят  
- [ ] Проект собирается с cargo check
- [ ] Создан backup (git commit)
- [ ] Есть 4-8 часов непрерывного времени

**Готов начать полную очистку legacy кода! 🚀**
