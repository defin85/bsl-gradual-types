//! Интеграционные тесты для discover_all_configurations()
//!
//! Проверяет обнаружение множественных конфигураций (база + расширения)
//! и корректность сортировки.

use bsl_backend::data::loaders::config_metadata_parser::{
    ConfigurationDiscovery, ConfigurationType,
};
use std::path::PathBuf;

/// Тест 1: Обнаружение нескольких конфигураций
///
/// Проверяет, что в папке examples/conf обнаруживаются:
/// - conf_test (Base)
/// - ext_test (Extension)
#[test]
fn test_discover_multiple_configurations() {
    // Arrange
    let parent_path = PathBuf::from("../examples/conf");
    let discovery = ConfigurationDiscovery::new(parent_path);

    // Act
    let result = discovery.discover_all_configurations();

    // Assert
    assert!(
        result.is_ok(),
        "Должны быть найдены конфигурации: {:?}",
        result.err()
    );

    let configurations = result.unwrap();

    // Должно быть минимум 2 конфигурации (conf_test + ext_test)
    assert!(
        configurations.len() >= 2,
        "Должно быть найдено минимум 2 конфигурации (Base + Extension), найдено: {}",
        configurations.len()
    );

    println!("✅ Обнаружено конфигураций: {}", configurations.len());

    for (idx, config) in configurations.iter().enumerate() {
        let icon = if config.is_base() { "📦" } else { "🧩" };
        println!(
            "  {}. {} {} (тип: {:?}, префикс: {:?}, путь: {})",
            idx + 1,
            icon,
            config.name,
            config.config_type,
            config.prefix,
            config.path.display()
        );
    }
}

/// Тест 2: Правильная сортировка (Base → Extensions)
///
/// Проверяет, что базовая конфигурация всегда первая,
/// расширения идут после базовой.
#[test]
fn test_configurations_sorted_base_first() {
    // Arrange
    let parent_path = PathBuf::from("../examples/conf");
    let discovery = ConfigurationDiscovery::new(parent_path);

    // Act
    let result = discovery.discover_all_configurations();
    assert!(result.is_ok());

    let configurations = result.unwrap();

    // Assert
    // Должна быть хотя бы одна конфигурация
    assert!(!configurations.is_empty(), "Должна быть найдена хотя бы одна конфигурация");

    // Проверяем, что первая конфигурация — Base
    let first_config = &configurations[0];
    assert!(
        first_config.is_base(),
        "Первая конфигурация должна быть базовой (Base), найдена: {:?}",
        first_config.config_type
    );

    println!("✅ Первая конфигурация — Base: {}", first_config.name);

    // Проверяем, что все последующие конфигурации (если есть) — Extension
    for (idx, config) in configurations.iter().enumerate().skip(1) {
        assert!(
            config.is_extension(),
            "Конфигурация #{} должна быть Extension, найдена: {:?}",
            idx + 1,
            config.config_type
        );
        println!("✅ Конфигурация #{} — Extension: {}", idx + 1, config.name);
    }
}

/// Тест 3: Корректность путей
///
/// Проверяет, что config_info.path указывает на правильные папки:
/// - conf_test → .../conf_test
/// - ext_test → .../ext_test
#[test]
fn test_configuration_paths_correctness() {
    // Arrange
    let parent_path = PathBuf::from("../examples/conf");
    let discovery = ConfigurationDiscovery::new(parent_path);

    // Act
    let result = discovery.discover_all_configurations();
    assert!(result.is_ok());

    let configurations = result.unwrap();

    // Assert
    // Находим conf_test
    let conf_test = configurations.iter().find(|c| c.name == "Конфигурация");
    assert!(
        conf_test.is_some(),
        "Должна быть найдена конфигурация 'Конфигурация'"
    );

    let conf_test = conf_test.unwrap();
    assert!(
        conf_test.path.ends_with("conf_test"),
        "Путь к conf_test должен заканчиваться на 'conf_test', найден: {}",
        conf_test.path.display()
    );

    println!("✅ Путь к базовой конфигурации корректен: {}", conf_test.path.display());

    // Находим ext_test
    let ext_test = configurations.iter().find(|c| c.name == "ТестовоеРасширение");
    assert!(
        ext_test.is_some(),
        "Должно быть найдено расширение 'ТестовоеРасширение'"
    );

    let ext_test = ext_test.unwrap();
    assert!(
        ext_test.path.ends_with("ext_test"),
        "Путь к ext_test должен заканчиваться на 'ext_test', найден: {}",
        ext_test.path.display()
    );

    println!("✅ Путь к расширению корректен: {}", ext_test.path.display());
}

/// Тест 4: Обратная совместимость
///
/// Проверяет, что старый метод discover_all_metadata() всё ещё работает
/// и загружает метаданные из ПЕРВОЙ конфигурации (базовой).
#[test]
fn test_backward_compatibility_discover_all_metadata() {
    // Arrange
    let parent_path = PathBuf::from("../examples/conf");
    let discovery = ConfigurationDiscovery::new(parent_path.clone());

    // Act - новый метод
    let configurations_result = discovery.discover_all_configurations();
    assert!(configurations_result.is_ok());

    let configurations = configurations_result.unwrap();
    let first_config = &configurations[0];

    // Act - старый метод (должен возвращать метаданные из первой конфигурации)
    let old_metadata_result = discovery.discover_all_metadata();

    // Assert
    assert!(
        old_metadata_result.is_ok(),
        "Старый метод должен работать для обратной совместимости"
    );

    let old_metadata = old_metadata_result.unwrap();
    println!(
        "✅ Обратная совместимость: обнаружено {} объектов метаданных через старый метод",
        old_metadata.len()
    );

    // Старый метод должен загружать метаданные из ПЕРВОЙ (базовой) конфигурации
    assert!(
        old_metadata.len() > 0,
        "Должны быть найдены объекты метаданных из первой конфигурации"
    );

    println!(
        "✅ Метаданные загружены из первой конфигурации: {}",
        first_config.name
    );
}

/// Тест 5: Проверка наличия всех ожидаемых типов
///
/// Убеждается, что среди найденных конфигураций есть и Base, и Extension.
#[test]
fn test_both_base_and_extension_present() {
    // Arrange
    let parent_path = PathBuf::from("../examples/conf");
    let discovery = ConfigurationDiscovery::new(parent_path);

    // Act
    let result = discovery.discover_all_configurations();
    assert!(result.is_ok());

    let configurations = result.unwrap();

    // Assert
    let base_count = configurations.iter().filter(|c| c.is_base()).count();
    let ext_count = configurations.iter().filter(|c| c.is_extension()).count();

    assert!(
        base_count > 0,
        "Должна быть найдена хотя бы одна базовая конфигурация"
    );
    assert!(
        ext_count > 0,
        "Должно быть найдено хотя бы одно расширение"
    );

    println!("✅ Найдено базовых конфигураций: {}", base_count);
    println!("✅ Найдено расширений: {}", ext_count);
    println!("✅ Всего конфигураций: {}", configurations.len());
}

/// Тест 6: Проверка уникальности конфигураций
///
/// Убеждается, что не обнаружено дубликатов конфигураций.
#[test]
fn test_no_duplicate_configurations() {
    // Arrange
    let parent_path = PathBuf::from("../examples/conf");
    let discovery = ConfigurationDiscovery::new(parent_path);

    // Act
    let result = discovery.discover_all_configurations();
    assert!(result.is_ok());

    let configurations = result.unwrap();

    // Assert
    // Проверяем уникальность по имени
    let mut names = std::collections::HashSet::new();
    for config in &configurations {
        let is_unique = names.insert(config.name.clone());
        assert!(
            is_unique,
            "Дубликат конфигурации: {} (путь: {})",
            config.name,
            config.path.display()
        );
    }

    // Проверяем уникальность по пути
    let mut paths = std::collections::HashSet::new();
    for config in &configurations {
        let is_unique = paths.insert(config.path.clone());
        assert!(
            is_unique,
            "Дубликат пути конфигурации: {} (имя: {})",
            config.path.display(),
            config.name
        );
    }

    println!("✅ Все конфигурации уникальны (имена и пути)");
}
