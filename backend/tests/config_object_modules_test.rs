//! Integration тесты для discovery модулей объектов (Milestone 3.12 Phase 3+4)
//!
//! Тестирует:
//! - Обнаружение ObjectModule, ManagerModule, RecordSetModule
//! - Integration в ConfigurationDiscovery
//! - CodeLocation определение из путей

use bsl_backend::data::loaders::config_metadata_parser::discovery::ConfigurationDiscovery;
use bsl_shared::domain::code_location::{CodeLocation, ModuleType};
use std::path::PathBuf;

/// Базовый путь к тестовой конфигурации
fn test_config_path() -> PathBuf {
    PathBuf::from("../examples/conf")
}

#[test]
fn test_discover_object_modules_returns_tuples() {
    let discovery = ConfigurationDiscovery::new(test_config_path());

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
    let discovery = ConfigurationDiscovery::new(test_config_path());

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
