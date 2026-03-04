use super::*;

#[test]
fn test_platform_location() {
    let loc = TypeDefinitionLocation::platform("Массив");

    assert!(matches!(loc, TypeDefinitionLocation::Platform { .. }));
    assert_eq!(loc.docs_uri(), Some("bsl://docs/Массив"));
    assert!(!loc.is_navigable() || loc.docs_uri().is_some());
}

#[test]
fn test_configuration_location_primary_path() {
    let metadata_path = PathBuf::from("Catalogs/Контрагенты.xml");
    let object_module = PathBuf::from("Catalogs/Контрагенты/Ext/ObjectModule.bsl");

    let module_paths = ModulePaths::new().with_object_module(object_module.clone());

    let loc =
        TypeDefinitionLocation::configuration_with_modules(metadata_path.clone(), module_paths);

    // primary_path должен вернуть object_module (приоритет выше)
    assert_eq!(loc.primary_path(), Some(&object_module));
    assert!(loc.is_navigable());
}

#[test]
fn test_configuration_location_fallback_to_metadata() {
    let metadata_path = PathBuf::from("Catalogs/Контрагенты.xml");

    let loc = TypeDefinitionLocation::configuration(metadata_path.clone());

    // Без модулей primary_path должен вернуть metadata_path
    assert_eq!(loc.primary_path(), Some(&metadata_path));
}

#[test]
fn test_user_defined_location() {
    let file_path = PathBuf::from("src/module.bsl");
    let loc = TypeDefinitionLocation::user_defined(file_path.clone(), 10, 20);

    assert_eq!(loc.primary_path(), Some(&file_path));
    assert!(loc.is_navigable());
}

#[test]
fn test_primitive_not_navigable() {
    let loc = TypeDefinitionLocation::primitive();

    assert_eq!(loc.primary_path(), None);
    assert!(!loc.is_navigable());
}

#[test]
fn test_module_paths_builder() {
    let paths = ModulePaths::new()
        .with_object_module(PathBuf::from("obj.bsl"))
        .with_manager_module(PathBuf::from("mgr.bsl"));

    assert!(paths.has_any_module());
    assert!(paths.object_module.is_some());
    assert!(paths.manager_module.is_some());
    assert!(paths.recordset_module.is_none());
}

#[test]
fn test_platform_with_custom_docs() {
    let loc = TypeDefinitionLocation::platform_with_docs("Массив", "https://docs.1c.ru/array.html");

    assert_eq!(loc.docs_uri(), Some("https://docs.1c.ru/array.html"));
}

#[test]
fn test_configuration_module_priority() {
    let metadata_path = PathBuf::from("meta.xml");
    let object_module = PathBuf::from("obj.bsl");
    let manager_module = PathBuf::from("mgr.bsl");

    // Только manager_module
    let paths1 = ModulePaths::new().with_manager_module(manager_module.clone());
    let loc1 = TypeDefinitionLocation::configuration_with_modules(metadata_path.clone(), paths1);
    assert_eq!(loc1.primary_path(), Some(&manager_module));

    // object_module + manager_module -> object_module имеет приоритет
    let paths2 = ModulePaths::new()
        .with_manager_module(manager_module.clone())
        .with_object_module(object_module.clone());
    let loc2 = TypeDefinitionLocation::configuration_with_modules(metadata_path.clone(), paths2);
    assert_eq!(loc2.primary_path(), Some(&object_module));
}

#[test]
fn test_user_defined_position() {
    let loc = TypeDefinitionLocation::user_defined(PathBuf::from("test.bsl"), 10, 20);

    if let TypeDefinitionLocation::UserDefined {
        file_path,
        start,
        end,
    } = loc
    {
        assert_eq!(file_path, PathBuf::from("test.bsl"));
        assert_eq!(start, 10);
        assert_eq!(end, 20);
    } else {
        panic!("Expected UserDefined location");
    }
}
