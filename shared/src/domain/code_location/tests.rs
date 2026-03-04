use super::*;

#[test]
fn test_determine_common_module_location() {
    let path = PathBuf::from("CommonModules/ОбщийМодуль1/Ext/Module.bsl");
    let loc = CodeLocation::determine_from_path(&path).unwrap();

    match loc.module_type {
        ModuleType::CommonModule { ref name, .. } => {
            assert_eq!(name, "ОбщийМодуль1");
        }
        _ => panic!("Expected CommonModule, got {:?}", loc.module_type),
    }

    assert!(loc.metadata_context.is_none());
}

#[test]
fn test_determine_object_module_location() {
    let path = PathBuf::from("Catalogs/Контрагенты/Ext/ObjectModule.bsl");
    let loc = CodeLocation::determine_from_path(&path).unwrap();

    match loc.module_type {
        ModuleType::ObjectModule { ref owner_type } => {
            assert_eq!(owner_type, "Catalog.Контрагенты");
        }
        _ => panic!("Expected ObjectModule, got {:?}", loc.module_type),
    }

    assert!(loc.metadata_context.is_some());
    let ctx = loc.metadata_context.unwrap();
    assert_eq!(ctx.object_name, "Контрагенты");
}

#[test]
fn test_determine_manager_module_location() {
    let path = PathBuf::from("Documents/ЗаказНаряд/Ext/ManagerModule.bsl");
    let loc = CodeLocation::determine_from_path(&path).unwrap();

    match loc.module_type {
        ModuleType::ManagerModule { ref owner_type } => {
            assert_eq!(owner_type, "Document.ЗаказНаряд");
        }
        _ => panic!("Expected ManagerModule, got {:?}", loc.module_type),
    }
}

#[test]
fn test_determine_form_module_location() {
    let path = PathBuf::from("Catalogs/Контрагенты/Forms/ФормаЭлемента/Ext/Module.bsl");
    let loc = CodeLocation::determine_from_path(&path).unwrap();

    match loc.module_type {
        ModuleType::FormModule {
            ref form_name,
            ref owner_type,
        } => {
            assert_eq!(form_name, "ФормаЭлемента");
            assert_eq!(owner_type, "Catalog.Контрагенты");
        }
        _ => panic!("Expected FormModule, got {:?}", loc.module_type),
    }
}

#[test]
fn test_determine_form_module_location_case_insensitive_path_components() {
    let path = PathBuf::from("catalogs/Контрагенты/forms/ФормаЭлемента/ext/module.bsl");
    let loc = CodeLocation::determine_from_path(&path).unwrap();

    match loc.module_type {
        ModuleType::FormModule {
            ref form_name,
            ref owner_type,
        } => {
            assert_eq!(form_name, "ФормаЭлемента");
            assert_eq!(owner_type, "Catalog.Контрагенты");
        }
        _ => panic!("Expected FormModule, got {:?}", loc.module_type),
    }
}

#[test]
fn test_determine_record_set_module_location() {
    let path = PathBuf::from("InformationRegisters/РегистрСведений/Ext/RecordSetModule.bsl");
    let loc = CodeLocation::determine_from_path(&path).unwrap();

    match loc.module_type {
        ModuleType::RecordSetModule { ref owner_type } => {
            assert_eq!(owner_type, "InformationRegister.РегистрСведений");
        }
        _ => panic!("Expected RecordSetModule, got {:?}", loc.module_type),
    }
}

#[test]
fn test_can_call_database_methods_object_module() {
    let path = PathBuf::from("Catalogs/Контрагенты/Ext/ObjectModule.bsl");
    let loc = CodeLocation::determine_from_path(&path).unwrap();

    // ObjectModule всегда имеет доступ к БД
    assert!(loc.can_call_database_methods(None));
}

#[test]
fn test_can_call_database_methods_form_module() {
    let path = PathBuf::from("Catalogs/Контрагенты/Forms/ФормаЭлемента/Ext/Module.bsl");
    let loc = CodeLocation::determine_from_path(&path).unwrap();

    // FormModule без директивы - нет доступа
    assert!(!loc.can_call_database_methods(None));

    // FormModule с &НаСервере - есть доступ
    assert!(loc.can_call_database_methods(Some(&CompilerDirective::OnServer)));

    // FormModule с &НаКлиенте - нет доступа
    assert!(!loc.can_call_database_methods(Some(&CompilerDirective::OnClient)));
}

#[test]
fn test_unknown_module_type() {
    let path = PathBuf::from("SomeUnknown/Path/File.bsl");
    let loc = CodeLocation::determine_from_path(&path).unwrap();

    assert!(matches!(loc.module_type, ModuleType::Unknown));
    assert!(!loc.can_call_database_methods(None));
}

#[test]
fn test_get_module_name() {
    let path = PathBuf::from("CommonModules/ОбщийМодуль1/Ext/Module.bsl");
    let loc = CodeLocation::determine_from_path(&path).unwrap();
    assert_eq!(loc.get_module_name(), Some("ОбщийМодуль1"));
}

#[test]
fn test_get_owner_type() {
    let path = PathBuf::from("Catalogs/Контрагенты/Ext/ObjectModule.bsl");
    let loc = CodeLocation::determine_from_path(&path).unwrap();
    assert_eq!(loc.get_owner_type(), Some("Catalog.Контрагенты"));
}
