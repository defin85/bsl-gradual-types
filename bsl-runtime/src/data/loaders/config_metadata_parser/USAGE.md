# Config Metadata Parser - Usage Guide

## Автоматическое обнаружение конфигурации

### Обзор

`ConfigurationDiscovery` теперь поддерживает автоматическое обнаружение конфигурации в подпапках.

### Принцип работы

Метод `find_configuration_folder()` выполняет поиск в следующем порядке:

1. **Прямой путь** (обратная совместимость):
   ```
   base_path/Configuration.xml  ← Проверяется первым
   ```

2. **Автообнаружение в подпапках**:
   ```
   base_path/
   ├── conf_test/Configuration.xml  ← Сканируется
   ├── ext_test/Configuration.xml   ← Сканируется
   └── other_folder/                ← Сканируется
   ```

3. **Возвращает первую найденную** конфигурацию
4. **Нерекурсивное сканирование** (только прямые подпапки)

---

## Примеры использования

### ✅ Вариант 1: Прямой путь к конфигурации (обратная совместимость)

```rust
use bsl_runtime::data::loaders::ConfigurationDiscovery;
use std::path::Path;

let config_path = Path::new("examples/conf/conf_test");
let discovery = ConfigurationDiscovery::new(config_path.to_path_buf());

let metadata = discovery.discover_all_metadata()?;
println!("Найдено объектов: {}", metadata.len());
```

**Результат:**
```
Найдено объектов: 5
```

---

### ✅ Вариант 2: Родительская папка (автообнаружение)

```rust
use bsl_runtime::data::loaders::ConfigurationDiscovery;
use std::path::Path;

// Передаём родительскую папку - метод автоматически найдёт конфигурацию
let parent_path = Path::new("examples/conf");
let discovery = ConfigurationDiscovery::new(parent_path.to_path_buf());

let metadata = discovery.discover_all_metadata()?;
println!("Найдено объектов: {}", metadata.len());
```

**Результат:**
```
✅ Найдена конфигурация в подпапке: "examples/conf/conf_test"
Найдено объектов: 5
```

---

### ❌ Вариант 3: Глубоко вложенная структура (не поддерживается)

```rust
// ❌ НЕ СРАБОТАЕТ
let project_root = Path::new("project");

// Структура:
// project/
//   └── src/
//       └── cf/
//           └── Configuration.xml  ← Не найдётся (>1 уровень вложенности)

let discovery = ConfigurationDiscovery::new(project_root.to_path_buf());
let result = discovery.discover_all_metadata();

assert!(result.is_err());
// Ошибка: "Configuration.xml не найден ни в ..., ни в подпапках"
```

**Решение:** Указать путь до родительской папки конфигурации:
```rust
// ✅ СРАБОТАЕТ
let config_parent = Path::new("project/src");
let discovery = ConfigurationDiscovery::new(config_parent.to_path_buf());
let metadata = discovery.discover_all_metadata()?; // ✅ Найдёт cf/Configuration.xml
```

---

## Обработка ошибок

### Конфигурация не найдена

```rust
use bsl_runtime::data::loaders::ConfigurationDiscovery;
use std::path::Path;

let empty_path = Path::new("empty_folder");
let discovery = ConfigurationDiscovery::new(empty_path.to_path_buf());

match discovery.discover_all_metadata() {
    Ok(metadata) => println!("Найдено: {}", metadata.len()),
    Err(e) => {
        eprintln!("Ошибка: {}", e);
        // Вывод:
        // Configuration.xml не найден ни в "empty_folder", ни в подпапках.
        // Убедитесь, что указан правильный путь к выгруженной конфигурации.
    }
}
```

---

## Рекомендации

### ✅ Когда использовать автообнаружение

1. **Стандартная структура проекта:**
   ```
   project/
   ├── conf/           ← Передать этот путь
   │   └── Configuration.xml
   └── README.md
   ```

2. **Несколько конфигураций:**
   ```
   configs/
   ├── base_config/         ← Будет найдена первая
   │   └── Configuration.xml
   └── extension/
       └── Configuration.xml
   ```

3. **Гибкость для пользователя:**
   - Пользователь может указать как прямой путь, так и родительскую папку
   - Метод автоматически адаптируется

### ❌ Когда НЕ использовать автообнаружение

1. **Глубоко вложенные структуры:**
   ```
   project/src/configs/1c/cf/Configuration.xml  ← Не найдётся
   ```
   **Решение:** Указать `project/src/configs/1c/`

2. **Множественные конфигурации (нужна конкретная):**
   ```
   configs/
   ├── config_a/Configuration.xml
   ├── config_b/Configuration.xml  ← Нужна именно эта
   └── config_c/Configuration.xml
   ```
   **Решение:** Указать прямой путь `configs/config_b/`

---

## CLI Использование

### Базовая команда (обратная совместимость)

```bash
cargo run --bin bsl-web-server -- \
  --syntax-helper-path examples/conf/conf_test
```

### НОВОЕ: Автообнаружение

```bash
# Передаём родительскую папку - метод автоматически найдёт конфигурацию
cargo run --bin bsl-web-server -- \
  --syntax-helper-path examples/conf
```

**Вывод:**
```
✅ Найдена конфигурация в подпапке: "examples/conf/conf_test"
🔍 Начало обнаружения метаданных...
✅ Обнаружено 5 объектов метаданных
```

---

## Логи и отладка

### Уровни логирования

1. **INFO** — Основные этапы поиска:
   ```
   ✅ Найдена конфигурация в подпапке: "examples/conf/conf_test"
   ```

2. **DEBUG** — Детали сканирования:
   ```
   🔍 Сканирование подпапок в "examples/conf" для поиска конфигурации...
   ✅ Configuration.xml найден напрямую в "examples/conf/conf_test"
   ```

3. **ERROR** — Проблемы:
   ```
   ❌ Configuration.xml не найден: "examples/conf/Configuration.xml"
   ```

### Включение DEBUG логов

```bash
RUST_LOG=debug cargo run --bin bsl-web-server -- \
  --syntax-helper-path examples/conf
```

---

## FAQ

### Q: Что если в папке несколько конфигураций?
**A:** Метод вернёт ПЕРВУЮ найденную конфигурацию. Для выбора конкретной укажите прямой путь.

### Q: Поддерживается ли рекурсивный поиск?
**A:** Нет, сканируются только прямые подпапки (1 уровень). Для глубоко вложенных структур укажите путь ближе к конфигурации.

### Q: Работает ли с русскими путями?
**A:** Да, полностью поддерживаются кириллические символы в путях (протестировано).

### Q: Как проверить, что конфигурация найдена?
**A:** Смотрите INFO логи или проверьте успешность `discover_all_metadata()`.

---

## Дополнительная информация

- **Тесты:** См. `backend/tests/config_discovery_auto_find_test.rs`
- **Edge Cases:** См. `backend/tests/config_discovery_edge_cases_test.rs`
- **Отчёт:** См. `backend/tests/CONFIG_DISCOVERY_TEST_REPORT.md`
