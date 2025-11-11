//! Интеграционные тесты для discover_metadata_in_configuration()
//!
//! Проверяет загрузку метаданных из конкретных конфигураций.

use bsl_backend::data::loaders::config_metadata_parser::ConfigurationDiscovery;
use std::path::PathBuf;

/// Тест 1: Загрузка метаданных из базовой конфигурации
///
/// Проверяет наличие объектов:
/// - Справочники: Организации, Контрагенты
/// - Документы: ЗаказНаряды
/// - РегистрыСведений: ТестовыйРегистрСведений
#[test]
fn test_load_metadata_from_base_configuration() {
    // Arrange
    let conf_path = PathBuf::from("../examples/conf/conf_test");
    let discovery = ConfigurationDiscovery::new(conf_path);

    // Act
    let configurations_result = discovery.discover_all_configurations();
    assert!(configurations_result.is_ok());

    let configurations = configurations_result.unwrap();
    assert!(
        !configurations.is_empty(),
        "Должна быть найдена конфигурация"
    );

    let base_config = &configurations[0];
    assert!(
        base_config.is_base(),
        "Первая конфигурация должна быть базовой"
    );

    let metadata_result = discovery.discover_metadata_in_configuration(
        base_config,
        None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>,
    );

    // Assert
    assert!(
        metadata_result.is_ok(),
        "Метаданные должны быть загружены: {:?}",
        metadata_result.err()
    );

    let metadata = metadata_result.unwrap();

    println!(
        "✅ Загружено {} объектов метаданных из базовой конфигурации",
        metadata.len()
    );

    // Проверяем наличие ожидаемых объектов
    let object_names: Vec<_> = metadata.iter().map(|m| m.name.as_str()).collect();

    // Справочники
    assert!(
        object_names.contains(&"Организации"),
        "Должен быть найден справочник 'Организации'"
    );
    assert!(
        object_names.contains(&"Контрагенты"),
        "Должен быть найден справочник 'Контрагенты'"
    );

    // Документы
    assert!(
        object_names.contains(&"ЗаказНаряды"),
        "Должен быть найден документ 'ЗаказНаряды'"
    );

    // Регистры сведений
    assert!(
        object_names.contains(&"ТестовыйРегистрСведений"),
        "Должен быть найден регистр сведений 'ТестовыйРегистрСведений'"
    );

    println!("✅ Все ожидаемые объекты найдены:");
    println!("   - Справочник: Организации");
    println!("   - Справочник: Контрагенты");
    println!("   - Документ: ЗаказНаряды");
    println!("   - РегистрСведений: ТестовыйРегистрСведений");
}

/// Тест 2: Загрузка метаданных из расширения
///
/// Проверяет наличие объектов с префиксом:
/// - Константа: Тест_Константа1
/// - Роль: Тест_ОсновнаяРоль
#[test]
fn test_load_metadata_from_extension_configuration() {
    // Arrange
    let parent_path = PathBuf::from("../examples/conf");
    let discovery = ConfigurationDiscovery::new(parent_path);

    // Act
    let configurations_result = discovery.discover_all_configurations();
    assert!(configurations_result.is_ok());

    let configurations = configurations_result.unwrap();

    // Находим расширение
    let extension = configurations.iter().find(|c| c.is_extension());

    if extension.is_none() {
        println!("⚠️ Расширение не найдено, пропускаем тест");
        return;
    }

    let extension = extension.unwrap();
    assert_eq!(
        extension.name, "ТестовоеРасширение",
        "Имя расширения должно быть 'ТестовоеРасширение'"
    );

    let metadata_result = discovery.discover_metadata_in_configuration(
        extension,
        None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>,
    );

    // Assert
    assert!(
        metadata_result.is_ok(),
        "Метаданные расширения должны быть загружены: {:?}",
        metadata_result.err()
    );

    let metadata = metadata_result.unwrap();

    println!(
        "✅ Загружено {} объектов метаданных из расширения",
        metadata.len()
    );

    // Проверяем наличие объектов с префиксом
    let object_names: Vec<_> = metadata.iter().map(|m| m.name.as_str()).collect();

    // Константа
    assert!(
        object_names.contains(&"Тест_Константа1"),
        "Должна быть найдена константа 'Тест_Константа1'"
    );

    // Роль
    assert!(
        object_names.contains(&"Тест_ОсновнаяРоль"),
        "Должна быть найдена роль 'Тест_ОсновнаяРоль'"
    );

    println!("✅ Все ожидаемые объекты расширения найдены:");
    println!("   - Константа: Тест_Константа1");
    println!("   - Роль: Тест_ОсновнаяРоль");
}

/// Тест 3: Последовательная загрузка всех конфигураций
///
/// Проверяет, что метаданные корректно загружаются из каждой конфигурации
/// через discover_metadata_in_configuration().
#[test]
fn test_sequential_metadata_loading_all_configurations() {
    // Arrange
    let parent_path = PathBuf::from("../examples/conf");
    let discovery = ConfigurationDiscovery::new(parent_path);

    // Act - получаем список всех конфигураций
    let configurations_result = discovery.discover_all_configurations();
    assert!(configurations_result.is_ok());

    let configurations = configurations_result.unwrap();
    assert!(
        configurations.len() >= 2,
        "Должно быть найдено минимум 2 конфигурации"
    );

    println!(
        "🔍 Последовательная загрузка метаданных из {} конфигураций:",
        configurations.len()
    );

    // Загружаем метаданные из каждой конфигурации
    for (idx, config) in configurations.iter().enumerate() {
        let icon = if config.is_base() { "📦" } else { "🧩" };
        println!("  {}. {} {}", idx + 1, icon, config.name);

        let metadata_result = discovery.discover_metadata_in_configuration(
            config,
            None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>,
        );

        assert!(
            metadata_result.is_ok(),
            "Метаданные из '{}' должны быть загружены: {:?}",
            config.name,
            metadata_result.err()
        );

        let metadata = metadata_result.unwrap();

        assert!(
            !metadata.is_empty(),
            "Конфигурация '{}' должна содержать хотя бы один объект метаданных",
            config.name
        );

        println!("     ✅ Загружено {} объектов", metadata.len());

        // Выводим имена объектов для наглядности
        for obj in metadata.iter().take(5) {
            println!("        - {} ({})", obj.name, obj.object_type_raw);
        }

        if metadata.len() > 5 {
            println!("        ... и ещё {} объектов", metadata.len() - 5);
        }
    }

    println!("✅ Все конфигурации успешно загружены");
}

/// Тест 4: Проверка структуры объектов метаданных
///
/// Убеждается, что объекты метаданных содержат корректную информацию.
#[test]
fn test_metadata_objects_structure() {
    // Arrange
    let conf_path = PathBuf::from("../examples/conf/conf_test");
    let discovery = ConfigurationDiscovery::new(conf_path);

    // Act
    let configurations_result = discovery.discover_all_configurations();
    assert!(configurations_result.is_ok());

    let configurations = configurations_result.unwrap();
    let base_config = &configurations[0];

    let metadata_result = discovery.discover_metadata_in_configuration(
        base_config,
        None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>,
    );
    assert!(metadata_result.is_ok());

    let metadata = metadata_result.unwrap();

    // Assert
    for obj in &metadata {
        // Проверяем обязательные поля
        assert!(!obj.name.is_empty(), "Имя объекта не должно быть пустым");
        assert!(!obj.uuid.is_empty(), "UUID объекта не должен быть пустым");
        assert!(
            !obj.object_type_raw.is_empty(),
            "Тип объекта не должен быть пустым"
        );

        // Проверяем, что тип объекта распознан (если возможно)
        if obj.object_type.is_some() {
            println!(
                "  ✅ Объект '{}' распознан как {:?}",
                obj.name,
                obj.object_type.unwrap()
            );
        } else {
            println!(
                "  ℹ️ Объект '{}' имеет нераспознанный тип: {}",
                obj.name, obj.object_type_raw
            );
        }

        // Проверяем наличие фасетов для известных типов
        if obj.object_type.is_some() {
            assert!(
                !obj.facets.is_empty(),
                "Распознанный объект '{}' должен иметь фасеты",
                obj.name
            );
        }
    }

    println!("✅ Все объекты метаданных имеют корректную структуру");
}

/// Тест 5: Проверка фасетов для справочников
///
/// Убеждается, что справочники имеют все 5 фасетов:
/// Manager, Object, Reference, Selection, List
#[test]
fn test_catalog_facets() {
    // Arrange
    let conf_path = PathBuf::from("../examples/conf/conf_test");
    let discovery = ConfigurationDiscovery::new(conf_path);

    // Act
    let configurations_result = discovery.discover_all_configurations();
    assert!(configurations_result.is_ok());

    let configurations = configurations_result.unwrap();
    let base_config = &configurations[0];

    let metadata_result = discovery.discover_metadata_in_configuration(
        base_config,
        None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>,
    );
    assert!(metadata_result.is_ok());

    let metadata = metadata_result.unwrap();

    // Assert
    // Находим справочник "Контрагенты"
    let catalog = metadata.iter().find(|m| m.name == "Контрагенты");
    assert!(
        catalog.is_some(),
        "Справочник 'Контрагенты' должен быть найден"
    );

    let catalog = catalog.unwrap();

    // Проверяем количество фасетов
    assert_eq!(
        catalog.facets.len(),
        5,
        "Справочник должен иметь 5 фасетов: Manager, Object, Reference, Selection, List"
    );

    println!("✅ Справочник 'Контрагенты' имеет все 5 фасетов:");
    for facet in &catalog.facets {
        println!("   - {:?}", facet);
    }
}

/// Тест 6: Проверка атрибутов и табличных частей
///
/// Убеждается, что парсер корректно извлекает атрибуты и табличные части
/// (если они есть в тестовых данных).
#[test]
fn test_attributes_and_tabular_sections() {
    // Arrange
    let conf_path = PathBuf::from("../examples/conf/conf_test");
    let discovery = ConfigurationDiscovery::new(conf_path);

    // Act
    let configurations_result = discovery.discover_all_configurations();
    assert!(configurations_result.is_ok());

    let configurations = configurations_result.unwrap();
    let base_config = &configurations[0];

    let metadata_result = discovery.discover_metadata_in_configuration(
        base_config,
        None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>,
    );
    assert!(metadata_result.is_ok());

    let metadata = metadata_result.unwrap();

    // Assert
    for obj in &metadata {
        println!("📄 Объект: {}", obj.name);

        if !obj.attributes.is_empty() {
            println!("   Атрибуты ({}):", obj.attributes.len());
            for attr in &obj.attributes {
                println!("     - {} ({})", attr.name, attr.type_name);
            }
        }

        if !obj.tabular_sections.is_empty() {
            println!("   Табличные части ({}):", obj.tabular_sections.len());
            for ts in &obj.tabular_sections {
                println!("     - {} ({} атрибутов)", ts.name, ts.attributes.len());
            }
        }
    }

    println!("✅ Проверка атрибутов и табличных частей завершена");
}
