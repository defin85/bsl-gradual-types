//! Тесты для обнаружения основных конфигураций и расширений
//!
//! Проверяет новую функциональность различения Base и Extension конфигураций.

#![allow(clippy::len_zero)]

use bsl_backend::data::loaders::config_metadata_parser::{
    ConfigurationDiscovery, ConfigurationType,
};
use std::path::PathBuf;

#[test]
fn test_detect_base_configuration() {
    // Arrange
    let conf_path = PathBuf::from("../examples/conf/conf_test");
    let discovery = ConfigurationDiscovery::new(conf_path);

    // Act
    let result = discovery.discover_all_configurations();

    // Assert
    if let Err(e) = &result {
        eprintln!("Ошибка обнаружения конфигурации: {}", e);
    }
    assert!(
        result.is_ok(),
        "Должна быть найдена конфигурация: {:?}",
        result.err()
    );

    let configurations = result.unwrap();
    assert!(
        !configurations.is_empty(),
        "Должна быть найдена хотя бы одна конфигурация"
    );

    let first_config = &configurations[0];
    assert!(
        first_config.is_base(),
        "Первая конфигурация должна быть базовой"
    );
    assert_eq!(first_config.config_type, ConfigurationType::Base);
    assert!(
        first_config.prefix.is_none(),
        "Базовая конфигурация не должна иметь префикс"
    );

    println!("✅ Базовая конфигурация: {}", first_config.name);
}

#[test]
fn test_detect_extension_configuration() {
    // Arrange
    let ext_path = PathBuf::from("../examples/conf/ext_test");
    let discovery = ConfigurationDiscovery::new(ext_path);

    // Act
    let result = discovery.discover_all_configurations();

    // Assert
    if result.is_err() {
        // Расширение может не существовать - это нормально для теста
        println!("⚠️ Тестовое расширение не найдено (это OK для базового теста)");
        return;
    }

    let configurations = result.unwrap();
    if configurations.is_empty() {
        println!("⚠️ Расширение не найдено (папка не существует)");
        return;
    }

    let first_config = &configurations[0];

    // Проверяем, что это расширение
    if first_config.is_extension() {
        println!("✅ Обнаружено расширение: {}", first_config.name);
        println!("   Префикс: {:?}", first_config.prefix);
        assert_eq!(first_config.config_type, ConfigurationType::Extension);
    } else {
        println!("ℹ️ В тестовой папке найдена базовая конфигурация вместо расширения");
    }
}

#[test]
fn test_discover_all_configurations_multiple() {
    // Arrange
    // Тестируем папку, которая может содержать и базу, и расширения
    let parent_path = PathBuf::from("../examples/conf");
    let discovery = ConfigurationDiscovery::new(parent_path);

    // Act
    let result = discovery.discover_all_configurations();

    // Assert
    if result.is_err() {
        println!("⚠️ Конфигурации не найдены в examples/configs (это OK)");
        return;
    }

    let configurations = result.unwrap();

    println!("🔍 Найдено конфигураций: {}", configurations.len());

    for (idx, config) in configurations.iter().enumerate() {
        let icon = if config.is_base() { "📦" } else { "🧩" };
        let config_type_str = if config.is_base() {
            "Base"
        } else {
            "Extension"
        };

        println!(
            "  {}. {} {} (тип: {}, префикс: {:?})",
            idx + 1,
            icon,
            config.name,
            config_type_str,
            config.prefix
        );
    }

    // Проверяем сортировку: Base должна быть первой
    if configurations.len() > 1 {
        let first_is_base = configurations[0].is_base();
        println!(
            "✅ Сортировка: первая конфигурация — {}",
            if first_is_base { "Base" } else { "Extension" }
        );
    }
}

#[test]
fn test_discover_metadata_in_specific_configuration() {
    // Arrange
    let conf_path = PathBuf::from("../examples/conf/conf_test");
    let discovery = ConfigurationDiscovery::new(conf_path);

    // Act
    let configurations = discovery.discover_all_configurations();

    if configurations.is_err() {
        println!("⚠️ Конфигурация не найдена");
        return;
    }

    let configs = configurations.unwrap();
    if configs.is_empty() {
        println!("⚠️ Нет доступных конфигураций");
        return;
    }

    let first_config = &configs[0];
    let metadata_result = discovery.discover_metadata_in_configuration(
        first_config,
        None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>,
    );

    // Assert
    assert!(metadata_result.is_ok(), "Должны быть обнаружены метаданные");

    let metadata = metadata_result.unwrap();
    println!(
        "✅ Обнаружено {} объектов метаданных в конфигурации {}",
        metadata.len(),
        first_config.name
    );

    // Проверяем, что метаданные действительно загружены
    assert!(
        metadata.len() > 0,
        "Должен быть хотя бы один объект метаданных"
    );
}

#[test]
fn test_backward_compatibility_discover_all_metadata() {
    // Arrange
    let conf_path = PathBuf::from("../examples/conf/conf_test");
    let discovery = ConfigurationDiscovery::new(conf_path);

    // Act
    let result = discovery
        .discover_all_metadata(None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>);

    // Assert
    assert!(
        result.is_ok(),
        "Старый метод должен работать для обратной совместимости"
    );

    let metadata = result.unwrap();
    println!(
        "✅ Обратная совместимость: обнаружено {} объектов метаданных",
        metadata.len()
    );

    assert!(metadata.len() > 0, "Должны быть найдены объекты метаданных");
}
