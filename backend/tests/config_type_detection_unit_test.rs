//! Unit-тесты для detect_configuration_type()
//!
//! Проверяет корректность определения типа конфигурации на основе XML маркеров.

use bsl_backend::data::loaders::config_metadata_parser::{
    ConfigurationDiscovery, ConfigurationType,
};
use std::path::PathBuf;

/// Тест 1: Обнаружение базовой конфигурации
///
/// Проверяет:
/// - Тип конфигурации: Base
/// - Имя: "Конфигурация"
/// - Префикс: None
/// - UUID корректно извлекается
#[test]
fn test_detect_base_configuration_details() {
    // Arrange
    let conf_path = PathBuf::from("../examples/conf/conf_test");
    let discovery = ConfigurationDiscovery::new(conf_path.clone());

    // Act
    let result = discovery.discover_all_configurations();

    // Assert
    assert!(result.is_ok(), "Должна быть найдена конфигурация");

    let configurations = result.unwrap();
    assert!(!configurations.is_empty(), "Должна быть найдена хотя бы одна конфигурация");

    let config = &configurations[0];

    // Проверяем тип конфигурации
    assert_eq!(
        config.config_type,
        ConfigurationType::Base,
        "Должна быть определена как базовая конфигурация"
    );
    assert!(config.is_base(), "is_base() должен возвращать true");
    assert!(!config.is_extension(), "is_extension() должен возвращать false");

    // Проверяем имя
    assert_eq!(
        config.name, "Конфигурация",
        "Имя должно быть 'Конфигурация'"
    );

    // Проверяем префикс
    assert!(
        config.prefix.is_none(),
        "Базовая конфигурация не должна иметь префикс"
    );

    // Проверяем UUID
    assert!(config.uuid.is_some(), "UUID должен быть извлечён");
    assert_eq!(
        config.uuid.as_ref().unwrap(),
        "787997b1-dd2a-4b98-a8cc-c38eb2830949",
        "UUID должен совпадать с XML"
    );

    // Проверяем путь
    assert!(
        config.path.ends_with("conf_test"),
        "Путь должен указывать на conf_test"
    );

    println!("✅ Базовая конфигурация корректно определена:");
    println!("   Имя: {}", config.name);
    println!("   Тип: {:?}", config.config_type);
    println!("   UUID: {:?}", config.uuid);
}

/// Тест 2: Обнаружение расширения
///
/// Проверяет:
/// - Тип конфигурации: Extension
/// - Имя: "ТестовоеРасширение"
/// - Префикс: Some("Тест_")
/// - UUID корректно извлекается
#[test]
fn test_detect_extension_configuration_details() {
    // Arrange
    let ext_path = PathBuf::from("../examples/conf/ext_test");
    let discovery = ConfigurationDiscovery::new(ext_path.clone());

    // Act
    let result = discovery.discover_all_configurations();

    // Assert
    assert!(
        result.is_ok(),
        "Должно быть найдено расширение: {:?}",
        result.err()
    );

    let configurations = result.unwrap();
    assert!(
        !configurations.is_empty(),
        "Должно быть найдено хотя бы одно расширение"
    );

    let config = &configurations[0];

    // Проверяем тип конфигурации
    assert_eq!(
        config.config_type,
        ConfigurationType::Extension,
        "Должна быть определена как расширение"
    );
    assert!(config.is_extension(), "is_extension() должен возвращать true");
    assert!(!config.is_base(), "is_base() должен возвращать false");

    // Проверяем имя
    assert_eq!(
        config.name, "ТестовоеРасширение",
        "Имя должно быть 'ТестовоеРасширение'"
    );

    // Проверяем префикс
    assert!(
        config.prefix.is_some(),
        "Расширение должно иметь префикс"
    );
    assert_eq!(
        config.prefix.as_ref().unwrap(),
        "Тест_",
        "Префикс должен быть 'Тест_'"
    );

    // Проверяем UUID
    assert!(config.uuid.is_some(), "UUID должен быть извлечён");
    assert_eq!(
        config.uuid.as_ref().unwrap(),
        "08fbf8dc-a81f-4998-ad30-64c9374eaeb1",
        "UUID должен совпадать с XML"
    );

    // Проверяем путь
    assert!(
        config.path.ends_with("ext_test"),
        "Путь должен указывать на ext_test"
    );

    println!("✅ Расширение корректно определено:");
    println!("   Имя: {}", config.name);
    println!("   Тип: {:?}", config.config_type);
    println!("   Префикс: {:?}", config.prefix);
    println!("   UUID: {:?}", config.uuid);
}

/// Тест 3: Проверка маркеров расширения
///
/// Убеждается, что функция корректно распознаёт маркеры:
/// - <ObjectBelonging>Adopted</ObjectBelonging>
/// - <ConfigurationExtensionPurpose>
#[test]
fn test_extension_markers_detection() {
    // Arrange
    let ext_path = PathBuf::from("../examples/conf/ext_test");
    let discovery = ConfigurationDiscovery::new(ext_path);

    // Act
    let result = discovery.discover_all_configurations();

    // Assert
    assert!(result.is_ok(), "Расширение должно быть найдено");

    let configurations = result.unwrap();
    let config = &configurations[0];

    // Проверяем, что маркеры расширения корректно обработаны
    assert!(
        config.is_extension(),
        "Маркеры расширения должны быть обнаружены"
    );

    // В XML ext_test есть:
    // - <ObjectBelonging>Adopted</ObjectBelonging> (строка 35)
    // - <ConfigurationExtensionPurpose>Customization</ConfigurationExtensionPurpose> (строка 44)
    // Оба маркера должны приводить к определению как Extension

    println!("✅ Маркеры расширения корректно обработаны");
    println!("   ObjectBelonging: Adopted (ожидаемо)");
    println!("   ConfigurationExtensionPurpose: Customization (ожидаемо)");
}

/// Тест 4: Отсутствие маркеров расширения
///
/// Убеждается, что отсутствие маркеров расширения приводит к определению как Base
#[test]
fn test_base_configuration_no_extension_markers() {
    // Arrange
    let conf_path = PathBuf::from("../examples/conf/conf_test");
    let discovery = ConfigurationDiscovery::new(conf_path);

    // Act
    let result = discovery.discover_all_configurations();

    // Assert
    assert!(result.is_ok());

    let configurations = result.unwrap();
    let config = &configurations[0];

    // В базовой конфигурации не должно быть маркеров расширения
    assert!(
        config.is_base(),
        "Отсутствие маркеров должно приводить к определению как Base"
    );
    assert!(
        config.prefix.is_none(),
        "Базовая конфигурация не имеет префикса"
    );

    println!("✅ Базовая конфигурация корректно определена по отсутствию маркеров");
}

/// Тест 5: Извлечение NamePrefix из XML
///
/// Проверяет, что префикс корректно извлекается из тега <NamePrefix>
#[test]
fn test_name_prefix_extraction() {
    // Arrange
    let ext_path = PathBuf::from("../examples/conf/ext_test");
    let discovery = ConfigurationDiscovery::new(ext_path);

    // Act
    let result = discovery.discover_all_configurations();

    // Assert
    assert!(result.is_ok());

    let configurations = result.unwrap();
    let config = &configurations[0];

    // В ext_test XML есть: <NamePrefix>Тест_</NamePrefix> (строка 46)
    assert!(config.prefix.is_some(), "Префикс должен быть извлечён");
    assert_eq!(
        config.prefix.as_ref().unwrap(),
        "Тест_",
        "Префикс должен совпадать с <NamePrefix>"
    );

    println!("✅ NamePrefix корректно извлечён из XML: {:?}", config.prefix);
}

/// Тест 6: Пустой NamePrefix в базовой конфигурации
///
/// Проверяет, что пустой <NamePrefix/> не создаёт префикса
#[test]
fn test_empty_name_prefix_in_base() {
    // Arrange
    let conf_path = PathBuf::from("../examples/conf/conf_test");
    let discovery = ConfigurationDiscovery::new(conf_path);

    // Act
    let result = discovery.discover_all_configurations();

    // Assert
    assert!(result.is_ok());

    let configurations = result.unwrap();
    let config = &configurations[0];

    // В conf_test XML есть: <NamePrefix/> (строка 43) - пустой тег
    assert!(
        config.prefix.is_none(),
        "Пустой <NamePrefix/> должен давать None"
    );

    println!("✅ Пустой NamePrefix корректно обработан как None");
}
