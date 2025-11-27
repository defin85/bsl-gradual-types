//! Тесты для автоматического обнаружения папки конфигурации
//!
//! Проверяет метод ConfigurationDiscovery::find_configuration_folder()
//! который должен:
//! 1. Сначала проверять Configuration.xml прямо в base_path (обратная совместимость)
//! 2. Если не найден - сканировать подпапки и возвращать первую с Configuration.xml
//! 3. Если не найден нигде - возвращать ошибку

#![allow(clippy::len_zero)]

use bsl_backend::data::loaders::ConfigurationDiscovery;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

// ============================================================================
// 1. Unit-тесты для find_configuration_folder()
// ============================================================================

#[test]
fn test_find_configuration_direct() {
    // Тест 1: Configuration.xml прямо в base_path (обратная совместимость)
    let config_path = Path::new("../examples/conf/conf_test");

    if !config_path.exists() {
        eprintln!("⚠️ Тестовая конфигурация не найдена: {:?}", config_path);
        return;
    }

    let discovery = ConfigurationDiscovery::new(config_path.to_path_buf(), false);

    // ВАЖНО: Используем приватный метод через публичный API
    // find_configuration_folder() вызывается внутри discover_all_metadata()
    let result = discovery
        .discover_all_metadata(None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>);

    assert!(
        result.is_ok(),
        "discover_all_metadata должен успешно найти конфигурацию в прямом пути"
    );

    let metadata = result.unwrap();
    assert!(
        metadata.len() >= 4,
        "Конфигурация должна содержать минимум 4 объекта"
    );

    println!("✅ Тест 1 ПРОЙДЕН: Configuration.xml найден прямо в base_path");
}

#[test]
fn test_find_configuration_in_subfolder() {
    // Тест 2: Configuration.xml в подпапке (новая логика)
    // examples/conf/ содержит:
    //   - conf_test/ (с Configuration.xml)
    //   - ext_test/ (с Configuration.xml)
    let parent_path = Path::new("../examples/conf");

    if !parent_path.exists() {
        eprintln!("⚠️ Родительская папка не найдена: {:?}", parent_path);
        return;
    }

    let discovery = ConfigurationDiscovery::new(parent_path.to_path_buf(), false);

    // Метод должен найти ПЕРВУЮ подпапку с Configuration.xml
    let result = discovery
        .discover_all_metadata(None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>);

    assert!(
        result.is_ok(),
        "discover_all_metadata должен найти конфигурацию в подпапке"
    );

    let metadata = result.unwrap();

    // Конфигурация должна быть найдена (conf_test или ext_test)
    assert!(
        metadata.len() > 0,
        "Должна быть найдена хотя бы одна конфигурация в подпапках"
    );

    println!(
        "✅ Тест 2 ПРОЙДЕН: Configuration.xml найден в подпапке ({} объектов)",
        metadata.len()
    );
}

#[test]
fn test_find_configuration_not_found() {
    // Тест 3: Configuration.xml не найден - должна быть ошибка
    let temp_dir = TempDir::new().expect("Не удалось создать временную папку");

    // Создаём пустую временную папку без Configuration.xml
    let discovery = ConfigurationDiscovery::new(temp_dir.path().to_path_buf(), false);

    let result = discovery
        .discover_all_metadata(None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>);

    assert!(
        result.is_err(),
        "discover_all_metadata должен вернуть Err если Configuration.xml не найден"
    );

    if let Err(e) = result {
        let error_msg = e.to_string();
        println!("✅ Тест 3 ПРОЙДЕН: Ожидаемая ошибка: {}", error_msg);

        // Проверяем, что сообщение об ошибке содержит полезную информацию
        assert!(
            error_msg.contains("Configuration.xml") && error_msg.contains("не найден"),
            "Сообщение об ошибке должно указывать, что Configuration.xml не найден"
        );
    }
}

// ============================================================================
// 2. Интеграционные тесты для полного workflow
// ============================================================================

#[test]
fn test_workflow_with_nested_structure() {
    // Создаём временную структуру папок с Configuration.xml в подпапке
    let temp_dir = TempDir::new().expect("Не удалось создать временную папку");
    let config_subfolder = temp_dir.path().join("test_config");

    fs::create_dir(&config_subfolder).expect("Не удалось создать подпапку");

    // Создаём минимальный Configuration.xml
    let config_xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
    <Configuration>
        <Properties>
            <Name>ТестоваяКонфигурация</Name>
            <Synonym>
                <v8:item>
                    <v8:lang>ru</v8:lang>
                    <v8:content>Тестовая конфигурация</v8:content>
                </v8:item>
            </Synonym>
        </Properties>
        <ChildObjects>
            <Catalog>ТестовыйСправочник</Catalog>
        </ChildObjects>
    </Configuration>
</MetaDataObject>"#;

    let config_xml_path = config_subfolder.join("Configuration.xml");
    fs::write(&config_xml_path, config_xml_content).expect("Не удалось записать Configuration.xml");

    // Создаём структуру для справочника
    let catalogs_folder = config_subfolder.join("Catalogs");
    fs::create_dir(&catalogs_folder).expect("Не удалось создать папку Catalogs");

    let catalog_xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
    <Catalog>
        <Properties>
            <Name>ТестовыйСправочник</Name>
            <Synonym>
                <v8:item>
                    <v8:lang>ru</v8:lang>
                    <v8:content>Тестовый справочник</v8:content>
                </v8:item>
            </Synonym>
            <ObjectPresentation>
                <v8:item>
                    <v8:lang>ru</v8:lang>
                    <v8:content>Тестовый объект</v8:content>
                </v8:item>
            </ObjectPresentation>
            <Hierarchical>false</Hierarchical>
            <HierarchyType>Items</HierarchyType>
            <ChildObjectsCount>0</ChildObjectsCount>
        </Properties>
    </Catalog>
</MetaDataObject>"#;

    let catalog_xml_path = catalogs_folder.join("ТестовыйСправочник.xml");
    fs::write(&catalog_xml_path, catalog_xml_content).expect("Не удалось записать XML справочника");

    // Теперь тестируем: передаём родительскую папку (temp_dir)
    let discovery = ConfigurationDiscovery::new(temp_dir.path().to_path_buf(), false);

    let result = discovery
        .discover_all_metadata(None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>);

    assert!(
        result.is_ok(),
        "discover_all_metadata должен найти конфигурацию во вложенной структуре"
    );

    let metadata = result.unwrap();

    println!(
        "✅ Интеграционный тест ПРОЙДЕН: найдено {} объектов во вложенной структуре",
        metadata.len()
    );

    // Проверяем, что справочник был распарсен
    let has_catalog = metadata.iter().any(|m| m.name == "ТестовыйСправочник");

    assert!(
        has_catalog,
        "Должен быть найден справочник ТестовыйСправочник"
    );
}

#[test]
fn test_workflow_with_multiple_subfolders() {
    // Создаём структуру с несколькими подпапками,
    // только одна из которых содержит Configuration.xml
    let temp_dir = TempDir::new().expect("Не удалось создать временную папку");

    // Создаём несколько подпапок
    let empty_folder1 = temp_dir.path().join("empty1");
    let empty_folder2 = temp_dir.path().join("empty2");
    let config_folder = temp_dir.path().join("config_here");

    fs::create_dir(&empty_folder1).expect("Не удалось создать подпапку empty1");
    fs::create_dir(&empty_folder2).expect("Не удалось создать подпапку empty2");
    fs::create_dir(&config_folder).expect("Не удалось создать подпапку config_here");

    // Только в config_folder создаём Configuration.xml
    let config_xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
    <Configuration>
        <Properties>
            <Name>КонфигурацияВПодпапке</Name>
        </Properties>
        <ChildObjects>
        </ChildObjects>
    </Configuration>
</MetaDataObject>"#;

    let config_xml_path = config_folder.join("Configuration.xml");
    fs::write(&config_xml_path, config_xml_content).expect("Не удалось записать Configuration.xml");

    // Тестируем
    let discovery = ConfigurationDiscovery::new(temp_dir.path().to_path_buf(), false);

    let result = discovery
        .discover_all_metadata(None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>);

    assert!(
        result.is_ok(),
        "discover_all_metadata должен найти конфигурацию среди нескольких подпапок"
    );

    println!("✅ Тест ПРОЙДЕН: конфигурация найдена среди нескольких подпапок");
}

// ============================================================================
// 3. Тесты с реальными данными
// ============================================================================

#[test]
fn test_real_conf_test_configuration() {
    // Проверяем работу с реальной конфигурацией conf_test
    let config_path = Path::new("../examples/conf/conf_test");

    if !config_path.exists() {
        eprintln!("⚠️ Тестовая конфигурация не найдена: {:?}", config_path);
        return;
    }

    let discovery = ConfigurationDiscovery::new(config_path.to_path_buf(), false);

    let result = discovery
        .discover_all_metadata(None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>);

    assert!(
        result.is_ok(),
        "Реальная конфигурация должна парситься без ошибок"
    );

    let metadata = result.unwrap();

    println!(
        "✅ Реальная конфигурация conf_test: найдено {} объектов",
        metadata.len()
    );

    // Проверяем наличие известных объектов
    let has_kontragenty = metadata.iter().any(|m| m.name == "Контрагенты");
    let has_organizacii = metadata.iter().any(|m| m.name == "Организации");
    let has_zakaz_naryady = metadata.iter().any(|m| m.name == "ЗаказНаряды");

    assert!(has_kontragenty, "Должен быть найден справочник Контрагенты");
    assert!(has_organizacii, "Должен быть найден справочник Организации");
    assert!(has_zakaz_naryady, "Должен быть найден документ ЗаказНаряды");
}

#[test]
fn test_real_ext_test_configuration() {
    // Проверяем работу с реальным расширением ext_test
    let ext_path = Path::new("../examples/conf/ext_test");

    if !ext_path.exists() {
        eprintln!("⚠️ Расширение не найдено: {:?}", ext_path);
        return;
    }

    let discovery = ConfigurationDiscovery::new(ext_path.to_path_buf(), false);

    let result = discovery
        .discover_all_metadata(None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>);

    assert!(
        result.is_ok(),
        "Реальное расширение должно парситься без ошибок"
    );

    let metadata = result.unwrap();

    println!(
        "✅ Реальное расширение ext_test: найдено {} объектов",
        metadata.len()
    );

    // Проверяем наличие известных объектов расширения
    let has_constant = metadata.iter().any(|m| m.name == "Тест_Константа1");
    let has_role = metadata.iter().any(|m| m.name == "Тест_ОсновнаяРоль");

    assert!(
        has_constant,
        "Должна быть найдена константа Тест_Константа1"
    );
    assert!(has_role, "Должна быть найдена роль Тест_ОсновнаяРоль");
}

#[test]
fn test_parent_folder_auto_discovery() {
    // Главный тест: передаём родительскую папку examples/conf,
    // метод должен автоматически найти conf_test или ext_test
    let parent_path = Path::new("../examples/conf");

    if !parent_path.exists() {
        eprintln!("⚠️ Родительская папка не найдена: {:?}", parent_path);
        return;
    }

    let discovery = ConfigurationDiscovery::new(parent_path.to_path_buf(), false);

    let result = discovery
        .discover_all_metadata(None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>);

    assert!(
        result.is_ok(),
        "Метод должен автоматически найти конфигурацию в подпапках"
    );

    let metadata = result.unwrap();

    println!(
        "✅ ГЛАВНЫЙ ТЕСТ ПРОЙДЕН: автообнаружение нашло {} объектов в родительской папке",
        metadata.len()
    );

    // Должна быть найдена ЛИБО conf_test ЛИБО ext_test
    // (метод возвращает первую найденную конфигурацию)
    assert!(
        metadata.len() > 0,
        "Должны быть найдены объекты из одной из подпапок"
    );
}
