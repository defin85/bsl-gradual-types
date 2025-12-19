# Roadmap: Исправление API табличных частей

**Дата создания:** 2025-12-11
**Дата завершения:** 2025-12-11
**Статус:** ✅ Завершено
**Приоритет:** Средний

---

## Проблема

API не возвращал табличные части для конфигурационных типов, хотя они корректно загружались из XML и хранились в репозитории.

### Симптомы

3 падающих теста в `backend/tests/api_tabular_sections_test.rs`:

| Тест | Ожидание | Факт |
|------|----------|------|
| `test_api_returns_tabular_sections_for_zakaznarjady` | 2 табличные части | 0 |
| `test_all_tabular_sections_returned` | Табличная часть "Работы" | Не найдена |
| `test_composite_attribute_type_preserved` | Табличная часть "Стороны" | Не найдена |

---

## Корневая причина

**Двойной префикс** в `extract_type_name()`:

```
ConfigurationType.name = "Документы.ЗаказНаряды"  (уже с префиксом)
extract_type_name() добавлял ещё один → "Документы.Документы.ЗаказНаряды"
repository.find_type() не находил → None → пустые tabular_sections
```

---

## Решение

### Исправленный код

**Файл:** `shared/src/domain/metadata_lookup/core.rs`

```rust
ConcreteType::Configuration(config) => {
    // Если name уже содержит префикс (точку), возвращаем как есть
    // Это предотвращает двойной префикс: "Документы.Документы.ЗаказНаряды"
    if config.name.contains('.') {
        Some(config.name.clone())
    } else {
        Some(format!("{}.{}", config.kind.to_prefix(), config.name))
    }
}
```

### Оптимизация тестов

**Файл:** `backend/tests/shared_test_fixtures.rs`

Добавлен `SHARED_CONFIG_COORDINATOR` с LazyLock для переиспользования конфигурации между тестами:

```rust
pub static SHARED_CONFIG_COORDINATOR: LazyLock<SystemCoordinator> = LazyLock::new(|| {
    let coordinator = SystemCoordinator::new();
    let config_path = std::path::Path::new("../examples/conf/conf_test");
    coordinator
        .start_with_paths_blocking(None, Some(config_path), None)
        .expect("Failed to start coordinator with config");
    coordinator
});
```

**Результат:** Время выполнения тестов ~24 сек вместо ~6.5 мин.

---

## Результаты

### Тесты

```bash
cargo test -p bsl-backend --test api_tabular_sections_test
# 15 passed; 0 failed; 0 ignored
```

### Все 15 тестов проходят

- `test_api_returns_tabular_sections_for_zakaznarjady` ✅
- `test_all_tabular_sections_returned` ✅
- `test_composite_attribute_type_preserved` ✅
- И ещё 12 тестов ✅

---

## История изменений

| Дата | Изменение |
|------|-----------|
| 2025-12-11 | Создан roadmap на основе анализа при тестировании рефакторинга |
| 2025-12-11 | Найдена корневая причина: двойной префикс в `extract_type_name()` |
| 2025-12-11 | Исправлено + добавлен `SHARED_CONFIG_COORDINATOR` для оптимизации тестов |
| 2025-12-11 | Все 15 тестов проходят. Статус: Завершено |
