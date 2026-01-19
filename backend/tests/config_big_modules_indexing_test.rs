//! Интеграционный тест: индексация экспортных методов из реальной конфигурации
//!
//! Требует наличия `examples/conf_big` в репозитории (выгрузка конфигурации 1С).

use bsl_backend::data::loaders::{index_configuration_bsl_modules, UniversalMetadataObject};
use bsl_shared::domain::code_location::CodeLocation;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[test]
fn test_conf_big_indexes_exported_module_methods() {
    let candidates = [
        std::path::PathBuf::from("examples/conf_big"),
        std::path::PathBuf::from("../examples/conf_big"),
    ];

    let Some(config_path) = candidates
        .into_iter()
        .find(|p| p.join("Configuration.xml").exists())
    else {
        // Конфигурация может отсутствовать в некоторых окружениях (например, минимальные CI).
        // В таком случае не считаем это падением функциональности парсера.
        return;
    };

    let canonical = config_path.canonicalize().expect("canonicalize conf_big");
    let module_paths = find_module_paths(&canonical, 200);
    let metadata = build_minimal_metadata_from_paths(&module_paths);

    let indexed = index_configuration_bsl_modules(&canonical, &metadata).expect("index modules");

    assert!(
        !indexed.config_methods.is_empty() || !indexed.global_functions.is_empty(),
        "expected exported methods to be indexed from examples/conf_big modules"
    );

    for (_owner, sig) in indexed.config_methods {
        assert_eq!(
            sig.source,
            bsl_shared::domain::signature_index::SignatureSource::Configuration
        );
    }
}

fn find_module_paths(root: &Path, limit: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }

            let Some(file_name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };

            let is_module = matches!(
                file_name,
                "Module.bsl" | "ObjectModule.bsl" | "ManagerModule.bsl" | "RecordSetModule.bsl"
            );

            if !is_module {
                continue;
            }

            out.push(path);
            if out.len() >= limit {
                return out;
            }
        }
    }

    out
}

fn build_minimal_metadata_from_paths(paths: &[PathBuf]) -> Vec<UniversalMetadataObject> {
    let mut objects: HashMap<(String, String), UniversalMetadataObject> = HashMap::new();

    for p in paths {
        let Ok(loc) = CodeLocation::determine_from_path(p) else {
            continue;
        };

        match loc.module_type {
            bsl_shared::domain::code_location::ModuleType::CommonModule { name, .. } => {
                objects
                    .entry(("CommonModule".to_string(), name.clone()))
                    .or_insert_with(|| {
                        UniversalMetadataObject::new(
                            "CommonModule".to_string(),
                            name.clone(),
                            "00000000-0000-0000-0000-000000000000".to_string(),
                        )
                    });
            }
            bsl_shared::domain::code_location::ModuleType::ObjectModule { owner_type } => {
                let Some((xml_kind, object_name)) = owner_type.split_once('.') else {
                    continue;
                };
                let key = (xml_kind.to_string(), object_name.to_string());
                let entry = objects.entry(key).or_insert_with(|| {
                    UniversalMetadataObject::new(
                        xml_kind.to_string(),
                        object_name.to_string(),
                        "00000000-0000-0000-0000-000000000000".to_string(),
                    )
                });
                entry.object_module_path = Some(p.clone());
            }
            bsl_shared::domain::code_location::ModuleType::ManagerModule { owner_type } => {
                let Some((xml_kind, object_name)) = owner_type.split_once('.') else {
                    continue;
                };
                let key = (xml_kind.to_string(), object_name.to_string());
                let entry = objects.entry(key).or_insert_with(|| {
                    UniversalMetadataObject::new(
                        xml_kind.to_string(),
                        object_name.to_string(),
                        "00000000-0000-0000-0000-000000000000".to_string(),
                    )
                });
                entry.manager_module_path = Some(p.clone());
            }
            bsl_shared::domain::code_location::ModuleType::RecordSetModule { owner_type } => {
                let Some((xml_kind, object_name)) = owner_type.split_once('.') else {
                    continue;
                };
                let key = (xml_kind.to_string(), object_name.to_string());
                let entry = objects.entry(key).or_insert_with(|| {
                    UniversalMetadataObject::new(
                        xml_kind.to_string(),
                        object_name.to_string(),
                        "00000000-0000-0000-0000-000000000000".to_string(),
                    )
                });
                entry.record_set_module_path = Some(p.clone());
            }
            _ => {}
        }
    }

    objects.into_values().collect()
}
