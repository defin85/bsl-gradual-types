use std::path::Path;

use tempfile::TempDir;

use super::index_configuration_bsl_modules;
use crate::data::loaders::config_metadata_parser::types::{
    CommonModuleProperties, ReturnValuesReuse,
};
use crate::data::loaders::UniversalMetadataObject;

fn write(path: &Path, content: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

#[test]
fn indexes_manager_module_exports() {
    let tmp = TempDir::new().unwrap();
    let module_path = tmp
        .path()
        .join("Catalogs")
        .join("Контрагенты")
        .join("Ext")
        .join("ManagerModule.bsl");

    write(
        &module_path,
        r#"
Процедура Тест(П1) Экспорт
КонецПроцедуры

Процедура Скрытая()
КонецПроцедуры
"#,
    );

    let mut obj = UniversalMetadataObject::new(
        "Catalog".to_string(),
        "Контрагенты".to_string(),
        "00000000-0000-0000-0000-000000000000".to_string(),
    );
    obj.manager_module_path = Some(module_path);

    let indexed = index_configuration_bsl_modules(tmp.path(), &[obj]).unwrap();
    assert!(indexed
        .config_methods
        .iter()
        .any(|(t, m)| t == "СправочникМенеджер.Контрагенты" && m.name == "Тест"));
    assert!(!indexed
        .config_methods
        .iter()
        .any(|(_, m)| m.name == "Скрытая"));
}

#[test]
fn indexes_common_module_exports_and_global_functions() {
    let tmp = TempDir::new().unwrap();
    let module_path = tmp
        .path()
        .join("CommonModules")
        .join("ОбщийМодуль1")
        .join("Ext")
        .join("Module.bsl");

    write(
        &module_path,
        r#"
Функция Ф1() Экспорт
    Возврат 1;
КонецФункции
"#,
    );

    let mut cm = UniversalMetadataObject::new(
        "CommonModule".to_string(),
        "ОбщийМодуль1".to_string(),
        "00000000-0000-0000-0000-000000000000".to_string(),
    );
    cm.common_module_properties = Some(CommonModuleProperties {
        server: true,
        client_managed_application: false,
        client_ordinary_application: false,
        external_connection: false,
        server_call: false,
        global: true,
        privileged: false,
        compile: true,
        return_values_reuse: ReturnValuesReuse::DontUse,
    });

    let indexed = index_configuration_bsl_modules(tmp.path(), &[cm]).unwrap();
    assert!(indexed
        .config_methods
        .iter()
        .any(|(t, m)| t == "ОбщиеМодули.ОбщийМодуль1" && m.name == "Ф1"));
    let sig = indexed
        .config_methods
        .iter()
        .find(|(t, m)| t == "ОбщиеМодули.ОбщийМодуль1" && m.name == "Ф1")
        .map(|(_, m)| m)
        .unwrap();
    assert_eq!(sig.return_type.as_deref(), Some("Число"));
    assert!(indexed
        .global_functions
        .iter()
        .any(|(n, m)| n == "Ф1" && m.owner_type.is_none()));
}

#[test]
fn infers_function_return_type_union() {
    let tmp = TempDir::new().unwrap();
    let module_path = tmp
        .path()
        .join("CommonModules")
        .join("ОбщийМодуль1")
        .join("Ext")
        .join("Module.bsl");

    write(
        &module_path,
        r#"
Функция Ф1(Флаг) Экспорт
    Если Флаг Тогда
        Возврат 1;
    Иначе
        Возврат "x";
    КонецЕсли;
КонецФункции
"#,
    );

    let mut cm = UniversalMetadataObject::new(
        "CommonModule".to_string(),
        "ОбщийМодуль1".to_string(),
        "00000000-0000-0000-0000-000000000000".to_string(),
    );
    cm.common_module_properties = Some(CommonModuleProperties {
        server: true,
        client_managed_application: true,
        client_ordinary_application: false,
        external_connection: false,
        server_call: false,
        global: false,
        privileged: false,
        compile: true,
        return_values_reuse: ReturnValuesReuse::DontUse,
    });

    let indexed = index_configuration_bsl_modules(tmp.path(), &[cm]).unwrap();
    let sig = indexed
        .config_methods
        .iter()
        .find(|(t, m)| t == "ОбщиеМодули.ОбщийМодуль1" && m.name == "Ф1")
        .map(|(_, m)| m)
        .unwrap();
    assert_eq!(sig.return_type.as_deref(), Some("Строка | Число"));
}
