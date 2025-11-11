//! Интеграционные тесты для универсального парсера метаданных конфигурации

use bsl_backend::data::loaders::{ConfigurationDiscovery, UniversalMetadataParser};
use bsl_shared::domain::types::MetadataKind;
use std::path::Path;

#[test]
fn test_parse_test_configuration() {
    // Путь к тестовой конфигурации
    let config_path = Path::new("../examples/conf/conf_test");

    if !config_path.exists() {
        eprintln!("⚠️ Тестовая конфигурация не найдена: {:?}", config_path);
        return;
    }

    let discovery = ConfigurationDiscovery::new(config_path.to_path_buf());

    // Проверяем парсинг ChildObjects сначала
    let config_xml = config_path.join("Configuration.xml");
    let child_objects = discovery.parse_child_objects_list(&config_xml).unwrap();
    println!("📦 ChildObjects найдено: {} типов", child_objects.len());
    for (obj_type, objects) in &child_objects {
        println!("  {} ({} шт.): {:?}", obj_type, objects.len(), objects);
    }

    let metadata = discovery
        .discover_all_metadata(None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>)
        .expect("Не удалось обнаружить метаданные");

    println!("📊 Обнаружено объектов метаданных: {}", metadata.len());

    // Проверяем, что найдены объекты
    assert!(
        metadata.len() >= 4,
        "Ожидалось минимум 4 объекта (2 справочника, 1 документ, 1 регистр), найдено: {}",
        metadata.len()
    );

    // Проверяем справочник "Контрагенты"
    let kontragenty = metadata
        .iter()
        .find(|m| m.name == "Контрагенты")
        .expect("Должен быть справочник Контрагенты");

    println!(
        "✅ Найден справочник: {} (UUID: {})",
        kontragenty.name, kontragenty.uuid
    );
    assert_eq!(kontragenty.object_type, Some(MetadataKind::Catalog));
    assert_eq!(kontragenty.facets.len(), 5); // Manager, Object, Reference, Selection, List

    // Проверяем конвертацию в RawTypeData
    let raw_type = kontragenty.to_raw_type_data(None);
    assert_eq!(raw_type.name, "Справочники.Контрагенты");
    assert_eq!(raw_type.kind, Some(MetadataKind::Catalog));
    println!("✅ Конвертация в RawTypeData успешна: {}", raw_type.name);
}

#[test]
fn test_parse_child_objects_list() {
    let config_path = Path::new("../examples/conf/conf_test");

    if !config_path.exists() {
        eprintln!("⚠️ Тестовая конфигурация не найдена: {:?}", config_path);
        return;
    }

    let discovery = ConfigurationDiscovery::new(config_path.to_path_buf());
    let config_xml = config_path.join("Configuration.xml");

    let child_objects = discovery
        .parse_child_objects_list(&config_xml)
        .expect("Не удалось распарсить ChildObjects");

    println!("📦 Найдено типов объектов: {}", child_objects.len());

    // Выводим найденные типы и объекты
    for (obj_type, objects) in &child_objects {
        println!("  {} ({} шт.): {:?}", obj_type, objects.len(), objects);
    }

    // Проверяем наличие справочников
    assert!(
        child_objects.contains_key("Catalog"),
        "Должны быть справочники в ChildObjects"
    );

    let catalogs = &child_objects["Catalog"];
    assert!(
        catalogs.contains(&"Контрагенты".to_string()),
        "Должен быть справочник Контрагенты"
    );
}

#[test]
fn test_parse_catalog_kontragenty() {
    let xml_path = Path::new("../examples/conf/conf_test/Catalogs/Контрагенты.xml");

    if !xml_path.exists() {
        eprintln!("⚠️ Файл не найден: {:?}", xml_path);
        return;
    }

    let metadata = UniversalMetadataParser::parse_any_object(xml_path)
        .expect("Не удалось распарсить объект Контрагенты");

    println!("📄 Объект: {}", metadata.name);
    println!("   Тип: {:?}", metadata.object_type);
    println!("   UUID: {}", metadata.uuid);
    println!("   Синоним: {:?}", metadata.synonym);
    println!("   Фасетов: {}", metadata.facets.len());
    println!("   Атрибутов: {}", metadata.attributes.len());

    assert_eq!(metadata.name, "Контрагенты");
    assert_eq!(metadata.object_type, Some(MetadataKind::Catalog));
    assert_eq!(metadata.facets.len(), 5);
}

#[test]
fn test_parse_information_register() {
    let xml_path =
        Path::new("../examples/conf/conf_test/InformationRegisters/ТестовыйРегистрСведений.xml");

    if !xml_path.exists() {
        eprintln!("⚠️ Файл не найден: {:?}", xml_path);
        return;
    }

    let metadata = UniversalMetadataParser::parse_any_object(xml_path)
        .expect("Не удалось распарсить регистр сведений");

    println!("📊 Регистр: {}", metadata.name);
    println!("   Тип: {:?}", metadata.object_type);
    println!("   Фасетов: {}", metadata.facets.len());

    assert_eq!(metadata.name, "ТестовыйРегистрСведений");
    assert_eq!(metadata.object_type_raw, "InformationRegister");
    // Facets для регистра: Manager, Object, Selection
    assert_eq!(metadata.facets.len(), 3);
}

#[test]
fn test_convert_catalog_to_raw_type() {
    let xml_path = Path::new("../examples/conf/conf_test/Catalogs/Контрагенты.xml");

    if !xml_path.exists() {
        eprintln!("⚠️ Файл не найден: {:?}", xml_path);
        return;
    }

    let metadata =
        UniversalMetadataParser::parse_any_object(xml_path).expect("Не удалось распарсить объект");

    let raw_type = metadata.to_raw_type_data(None);

    println!("🔄 Конвертация в RawTypeData:");
    println!("   name: {}", raw_type.name);
    println!("   kind: {:?}", raw_type.kind);
    println!("   facets: {} шт.", raw_type.facets.len());
    println!("   attributes: {} шт.", raw_type.attributes.len());
    println!(
        "   tabular_sections: {} шт.",
        raw_type.tabular_sections.len()
    );

    assert_eq!(raw_type.name, "Справочники.Контрагенты");
    assert_eq!(raw_type.kind, Some(MetadataKind::Catalog));
    assert_eq!(raw_type.facets.len(), 5);
    assert_eq!(
        raw_type.source,
        bsl_shared::domain::types::RawDataSource::Configuration
    );
}

#[test]
fn test_graceful_handling_unknown_type() {
    // Этот тест проверяет, что парсер не падает на неизвестных типах объектов
    // Вместо этого он должен вернуть object_type: None

    let config_path = Path::new("../examples/conf/conf_test");

    if !config_path.exists() {
        eprintln!("⚠️ Тестовая конфигурация не найдена: {:?}", config_path);
        return;
    }

    let discovery = ConfigurationDiscovery::new(config_path.to_path_buf());

    // Даже если встретятся неизвестные типы, парсинг не должен упасть
    let result = discovery
        .discover_all_metadata(None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>);

    assert!(
        result.is_ok(),
        "Парсинг должен быть успешным даже с неизвестными типами"
    );

    let metadata = result.unwrap();

    // Проверяем, что все объекты имеют валидные имена
    for obj in metadata {
        assert!(!obj.name.is_empty(), "Имя объекта не должно быть пустым");
        assert!(!obj.uuid.is_empty(), "UUID не должен быть пустым");

        if obj.object_type.is_none() {
            println!(
                "⚠️ Обнаружен неизвестный тип: {} (raw: {})",
                obj.name, obj.object_type_raw
            );
        }
    }
}
