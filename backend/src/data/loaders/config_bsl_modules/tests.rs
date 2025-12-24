use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

use tempfile::TempDir;

use super::{index_configuration_bsl_modules, index_configuration_bsl_modules_with_progress_parallel_cached};
use crate::data::loaders::config_metadata_parser::types::{
    CommonModuleProperties, ReturnValuesReuse,
};
use crate::data::loaders::{ParsedModuleData, UniversalMetadataObject};

fn write(path: &Path, content: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

fn file_fingerprint(path: &Path) -> String {
    let contents = std::fs::read(path).unwrap_or_default();
    blake3::hash(&contents).to_hex().to_string()
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
    cm.common_module_path = Some(module_path.clone());
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
    cm.common_module_path = Some(module_path.clone());
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

#[test]
fn marks_optional_params_from_default_values() {
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
Процедура Тест(Обязательный, Опциональный = 1, ЕщеОдин = Неопределено) Экспорт
КонецПроцедуры
"#,
    );

    let mut cm = UniversalMetadataObject::new(
        "CommonModule".to_string(),
        "ОбщийМодуль1".to_string(),
        "00000000-0000-0000-0000-000000000000".to_string(),
    );
    cm.common_module_path = Some(module_path.clone());
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
        .find(|(t, m)| t == "ОбщиеМодули.ОбщийМодуль1" && m.name == "Тест")
        .map(|(_, m)| m)
        .unwrap();

    assert_eq!(sig.params.len(), 3);
    assert!(!sig.params[0].is_optional);
    assert!(sig.params[1].is_optional);
    assert!(sig.params[2].is_optional);
}

#[test]
fn cached_index_reuses_unchanged_modules() {
    let tmp = TempDir::new().unwrap();
    let object_module = tmp
        .path()
        .join("Catalogs")
        .join("Контрагенты")
        .join("Ext")
        .join("ObjectModule.bsl");
    let manager_module = tmp
        .path()
        .join("Catalogs")
        .join("Контрагенты")
        .join("Ext")
        .join("ManagerModule.bsl");

    write(
        &object_module,
        r#"
Процедура Метод1() Экспорт
КонецПроцедуры
"#,
    );
    write(
        &manager_module,
        r#"
Процедура Метод2() Экспорт
КонецПроцедуры
"#,
    );

    let mut obj = UniversalMetadataObject::new(
        "Catalog".to_string(),
        "Контрагенты".to_string(),
        "00000000-0000-0000-0000-000000000000".to_string(),
    );
    obj.object_module_path = Some(object_module.clone());
    obj.manager_module_path = Some(manager_module.clone());

    let cache: Arc<Mutex<HashMap<PathBuf, (String, ParsedModuleData)>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let parsed_count = Arc::new(AtomicUsize::new(0));

    let load_cached = {
        let cache = Arc::clone(&cache);
        move |module_path: &Path| -> anyhow::Result<Option<ParsedModuleData>> {
            let fingerprint = file_fingerprint(module_path);
            let guard = cache.lock().unwrap();
            let Some((stored_fingerprint, data)) = guard.get(module_path) else {
                return Ok(None);
            };
            if *stored_fingerprint == fingerprint {
                Ok(Some(data.clone()))
            } else {
                Ok(None)
            }
        }
    };

    let store_cached = {
        let cache = Arc::clone(&cache);
        let parsed_count = Arc::clone(&parsed_count);
        move |module_path: &Path, parsed: &ParsedModuleData| -> anyhow::Result<()> {
            let fingerprint = file_fingerprint(module_path);
            let mut guard = cache.lock().unwrap();
            guard.insert(module_path.to_path_buf(), (fingerprint, parsed.clone()));
            parsed_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    };

    let indexed = index_configuration_bsl_modules_with_progress_parallel_cached(
        tmp.path(),
        &[obj.clone()],
        None::<fn(super::ModuleIndexProgress)>,
        &load_cached,
        &store_cached,
    )
    .unwrap();
    assert!(indexed
        .config_methods
        .iter()
        .any(|(t, m)| t == "СправочникОбъект.Контрагенты" && m.name == "Метод1"));
    assert!(indexed
        .config_methods
        .iter()
        .any(|(t, m)| t == "СправочникМенеджер.Контрагенты" && m.name == "Метод2"));
    assert_eq!(parsed_count.load(Ordering::SeqCst), 2);

    parsed_count.store(0, Ordering::SeqCst);
    write(
        &manager_module,
        r#"
Процедура Метод3() Экспорт
КонецПроцедуры
"#,
    );

    let indexed = index_configuration_bsl_modules_with_progress_parallel_cached(
        tmp.path(),
        &[obj],
        None::<fn(super::ModuleIndexProgress)>,
        &load_cached,
        &store_cached,
    )
    .unwrap();
    assert!(indexed
        .config_methods
        .iter()
        .any(|(t, m)| t == "СправочникМенеджер.Контрагенты" && m.name == "Метод3"));
    assert!(!indexed
        .config_methods
        .iter()
        .any(|(t, m)| t == "СправочникМенеджер.Контрагенты" && m.name == "Метод2"));
    assert_eq!(parsed_count.load(Ordering::SeqCst), 1);
}
