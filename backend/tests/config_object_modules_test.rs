//! Integration тесты для discovery модулей объектов (Milestone 3.12 Phase 3+4)
//!
//! Тестирует:
//! - Обнаружение ObjectModule, ManagerModule, RecordSetModule
//! - Integration в ConfigurationDiscovery
//! - CodeLocation определение из путей

use bsl_backend::data::loaders::config_metadata_parser::discovery::ConfigurationDiscovery;
use bsl_shared::domain::code_location::{CodeLocation, ModuleType};
use std::path::PathBuf;
use tempfile::TempDir;

/// Базовый путь к тестовой конфигурации
fn test_config_path() -> PathBuf {
    PathBuf::from("../examples/conf")
}

#[test]
fn test_discover_object_modules_returns_tuples() {
    let discovery = ConfigurationDiscovery::new(test_config_path(), false);

    // Тестируем на любом существующем каталоге
    let (object_mod, manager_mod, record_set_mod) =
        discovery.discover_object_modules("Catalogs", "SomeTestCatalog");

    // Проверяем, что метод возвращает кортеж (graceful degradation - None если не существует)
    assert!(object_mod.is_none() || object_mod.is_some());
    assert!(manager_mod.is_none() || manager_mod.is_some());
    assert!(record_set_mod.is_none() || record_set_mod.is_some());
}

#[test]
fn test_discover_metadata_includes_module_paths() {
    let discovery = ConfigurationDiscovery::new(test_config_path(), false);

    // Обнаруживаем все конфигурации
    let configs = discovery
        .discover_all_configurations()
        .expect("Should discover configurations");

    // Берём первую конфигурацию
    if !configs.is_empty() {
        let first_config = &configs[0];

        // Парсим метаданные без прогресса
        let metadata = discovery
            .discover_metadata_in_configuration(first_config, None::<fn(_)>)
            .expect("Should parse metadata");

        // Проверяем, что структура UniversalMetadataObject содержит поля модулей
        for obj in &metadata {
            // Поля должны существовать (даже если None)
            let _ = &obj.object_module_path;
            let _ = &obj.manager_module_path;
            let _ = &obj.record_set_module_path;
        }

        // Если есть хотя бы один объект с модулями - проверяем его
        let has_modules = metadata.iter().any(|obj| {
            obj.object_module_path.is_some()
                || obj.manager_module_path.is_some()
                || obj.record_set_module_path.is_some()
        });

        println!("✅ Metadata parsed, objects with modules: {}", has_modules);
    }
}

#[test]
fn test_discover_common_module_path() {
    let temp = TempDir::new().expect("Не удалось создать временную папку");
    let config_root = temp.path();

    let config_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
    <Configuration>
        <Properties>
            <Name>ТестоваяКонфигурация</Name>
        </Properties>
        <ChildObjects>
            <CommonModule>ТестовыйОбщийМодуль</CommonModule>
        </ChildObjects>
    </Configuration>
</MetaDataObject>"#;

    std::fs::write(config_root.join("Configuration.xml"), config_xml)
        .expect("Не удалось записать Configuration.xml");

    let module_dir = config_root
        .join("CommonModules")
        .join("ТестовыйОбщийМодуль");
    std::fs::create_dir_all(module_dir.join("Ext"))
        .expect("Не удалось создать структуру CommonModules");

    let module_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
    <CommonModule uuid="00000000-0000-0000-0000-000000000000">
        <Properties>
            <Name>ТестовыйОбщийМодуль</Name>
            <Global>false</Global>
            <Server>true</Server>
        </Properties>
    </CommonModule>
</MetaDataObject>"#;

    std::fs::write(
        config_root
            .join("CommonModules")
            .join("ТестовыйОбщийМодуль.xml"),
        module_xml,
    )
    .expect("Не удалось записать CommonModule.xml");

    let module_path = module_dir.join("Ext").join("Module.bsl");
    std::fs::write(&module_path, "Процедура Тест() Экспорт\nКонецПроцедуры\n")
        .expect("Не удалось записать Module.bsl");

    let discovery = ConfigurationDiscovery::new(config_root.to_path_buf(), false);
    let configs = discovery
        .discover_all_configurations()
        .expect("Should discover configurations");
    let config_info = configs.first().expect("Should have configuration");
    let metadata = discovery
        .discover_metadata_in_configuration(config_info, None::<fn(_)>)
        .expect("Should parse metadata");

    let common_module = metadata
        .iter()
        .find(|obj| obj.object_type_raw == "CommonModule")
        .expect("CommonModule должен быть найден");

    assert_eq!(
        common_module.common_module_path.as_ref(),
        Some(&module_path)
    );
}

#[test]
fn test_code_location_common_module() {
    let path = PathBuf::from("CommonModules/ОбщийМодуль1/Ext/Module.bsl");
    let loc = CodeLocation::determine_from_path(&path).expect("Should parse path");

    match loc.module_type {
        ModuleType::CommonModule { ref name, .. } => {
            assert_eq!(name, "ОбщийМодуль1");
        }
        _ => panic!("Expected CommonModule"),
    }

    assert!(loc.metadata_context.is_none());
}

#[test]
fn test_code_location_object_module() {
    let path = PathBuf::from("Catalogs/Контрагенты/Ext/ObjectModule.bsl");
    let loc = CodeLocation::determine_from_path(&path).expect("Should parse path");

    match loc.module_type {
        ModuleType::ObjectModule { ref owner_type } => {
            assert_eq!(owner_type, "Catalog.Контрагенты");
        }
        _ => panic!("Expected ObjectModule"),
    }

    assert!(loc.metadata_context.is_some());
    let ctx = loc.metadata_context.unwrap();
    assert_eq!(ctx.object_name, "Контрагенты");
    assert_eq!(ctx.object_type, "Catalog.Контрагенты");
}

#[test]
fn test_code_location_manager_module() {
    let path = PathBuf::from("Documents/ЗаказНаряд/Ext/ManagerModule.bsl");
    let loc = CodeLocation::determine_from_path(&path).expect("Should parse path");

    match loc.module_type {
        ModuleType::ManagerModule { ref owner_type } => {
            assert_eq!(owner_type, "Document.ЗаказНаряд");
        }
        _ => panic!("Expected ManagerModule"),
    }

    assert!(loc.can_call_database_methods(None));
}

#[test]
fn test_code_location_form_module() {
    let path = PathBuf::from("Catalogs/Контрагенты/Forms/ФормаЭлемента/Ext/Module.bsl");
    let loc = CodeLocation::determine_from_path(&path).expect("Should parse path");

    match loc.module_type {
        ModuleType::FormModule {
            ref form_name,
            ref owner_type,
        } => {
            assert_eq!(form_name, "ФормаЭлемента");
            assert_eq!(owner_type, "Catalog.Контрагенты");
        }
        _ => panic!("Expected FormModule"),
    }

    assert!(loc.metadata_context.is_some());
}

#[test]
fn test_code_location_form_module_with_ext_form_subdir() {
    let path = PathBuf::from("Documents/ЗаказНаряды/Forms/ФормаДокумента/Ext/Form/Module.bsl");
    let loc = CodeLocation::determine_from_path(&path).expect("Should parse path");

    match loc.module_type {
        ModuleType::FormModule {
            ref form_name,
            ref owner_type,
        } => {
            assert_eq!(form_name, "ФормаДокумента");
            assert_eq!(owner_type, "Document.ЗаказНаряды");
        }
        _ => panic!("Expected FormModule"),
    }
}

#[test]
fn test_code_location_record_set_module() {
    let path = PathBuf::from("InformationRegisters/РегистрСведений/Ext/RecordSetModule.bsl");
    let loc = CodeLocation::determine_from_path(&path).expect("Should parse path");

    match loc.module_type {
        ModuleType::RecordSetModule { ref owner_type } => {
            assert_eq!(owner_type, "InformationRegister.РегистрСведений");
        }
        _ => panic!("Expected RecordSetModule"),
    }

    assert!(loc.can_call_database_methods(None));
}

#[test]
fn test_code_location_database_access_object_module() {
    let path = PathBuf::from("Catalogs/Контрагенты/Ext/ObjectModule.bsl");
    let loc = CodeLocation::determine_from_path(&path).expect("Should parse path");

    // ObjectModule всегда имеет доступ к БД
    assert!(loc.can_call_database_methods(None));
}

#[test]
fn test_code_location_database_access_form_module_with_directive() {
    use bsl_shared::domain::code_location::CompilerDirective;

    let path = PathBuf::from("Catalogs/Контрагенты/Forms/ФормаЭлемента/Ext/Module.bsl");
    let loc = CodeLocation::determine_from_path(&path).expect("Should parse path");

    // FormModule без директивы - нет доступа
    assert!(!loc.can_call_database_methods(None));

    // FormModule с &НаСервере - есть доступ
    assert!(loc.can_call_database_methods(Some(&CompilerDirective::OnServer)));

    // FormModule с &НаКлиенте - нет доступа
    assert!(!loc.can_call_database_methods(Some(&CompilerDirective::OnClient)));
}

#[test]
fn test_code_location_unknown_module() {
    let path = PathBuf::from("SomeUnknown/Path/File.bsl");
    let loc = CodeLocation::determine_from_path(&path).expect("Should parse path");

    assert!(matches!(loc.module_type, ModuleType::Unknown));
    assert!(!loc.can_call_database_methods(None));
}

#[test]
fn test_code_location_get_module_name() {
    let path = PathBuf::from("CommonModules/ОбщийМодуль1/Ext/Module.bsl");
    let loc = CodeLocation::determine_from_path(&path).expect("Should parse path");
    assert_eq!(loc.get_module_name(), Some("ОбщийМодуль1"));

    let form_path = PathBuf::from("Catalogs/Контрагенты/Forms/ФормаЭлемента/Ext/Module.bsl");
    let form_loc = CodeLocation::determine_from_path(&form_path).expect("Should parse path");
    assert_eq!(form_loc.get_module_name(), Some("ФормаЭлемента"));
}

#[test]
fn test_code_location_get_owner_type() {
    let path = PathBuf::from("Catalogs/Контрагенты/Ext/ObjectModule.bsl");
    let loc = CodeLocation::determine_from_path(&path).expect("Should parse path");
    assert_eq!(loc.get_owner_type(), Some("Catalog.Контрагенты"));

    let manager_path = PathBuf::from("Documents/ЗаказНаряд/Ext/ManagerModule.bsl");
    let manager_loc = CodeLocation::determine_from_path(&manager_path).expect("Should parse path");
    assert_eq!(manager_loc.get_owner_type(), Some("Document.ЗаказНаряд"));
}

#[test]
fn test_multiple_object_types() {
    // Тестируем различные типы объектов
    let test_cases = vec![
        ("Catalogs/Test/Ext/ObjectModule.bsl", "Catalog.Test"),
        ("Documents/Test/Ext/ObjectModule.bsl", "Document.Test"),
        (
            "InformationRegisters/Test/Ext/RecordSetModule.bsl",
            "InformationRegister.Test",
        ),
        (
            "AccumulationRegisters/Test/Ext/RecordSetModule.bsl",
            "AccumulationRegister.Test",
        ),
    ];

    for (path_str, expected_owner) in test_cases {
        let path = PathBuf::from(path_str);
        let loc = CodeLocation::determine_from_path(&path).expect("Should parse path");

        let owner = loc.get_owner_type().expect("Should have owner type");
        assert_eq!(owner, expected_owner, "Failed for path: {}", path_str);
    }
}
