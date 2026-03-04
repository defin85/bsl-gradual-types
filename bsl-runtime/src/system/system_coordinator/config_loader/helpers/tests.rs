use super::*;

mod cache_tests {
    use super::*;
    use crate::data::loaders::config_metadata_parser::ConfigurationDiscovery;
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn test_config_metadata_disk_cache_reuse() {
        let config_root = Path::new("examples/conf/conf_test");
        if !config_root.exists() {
            eprintln!("⚠️ Конфигурация не найдена в examples/conf/conf_test");
            return;
        }

        let temp = TempDir::new().unwrap();
        let cache = crate::system::DiskCache::with_root(temp.path().to_path_buf(), 1).unwrap();
        let coordinator = SystemCoordinator::new();

        let discovery = ConfigurationDiscovery::new(config_root.to_path_buf(), false);
        let configs = match discovery.discover_all_configurations() {
            Ok(list) if !list.is_empty() => list,
            _ => return,
        };
        let config_set_id = config_set_id_from_configs(&configs);
        let config_info = &configs[0];

        let key = coordinator
            .build_config_cache_key(config_root, config_info, Some(&config_set_id))
            .unwrap();

        let entry = cache
            .get_or_build_with(
                &key,
                || {
                    discovery
                        .discover_metadata_in_configuration(
                            config_info,
                            None::<fn(crate::data::loaders::progress::ProgressUpdate)>,
                        )
                        .map_err(|e| anyhow::anyhow!("Ошибка загрузки метаданных: {}", e))
                },
                |metadata| !metadata.is_empty(),
            )
            .unwrap();
        assert!(!entry.from_cache);

        let entry = cache
            .get_or_build_with(
                &key,
                || {
                    discovery
                        .discover_metadata_in_configuration(
                            config_info,
                            None::<fn(crate::data::loaders::progress::ProgressUpdate)>,
                        )
                        .map_err(|e| anyhow::anyhow!("Ошибка загрузки метаданных: {}", e))
                },
                |metadata| !metadata.is_empty(),
            )
            .unwrap();
        assert!(entry.from_cache);
    }
}

mod merkle_tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, contents.as_bytes()).unwrap();
    }

    #[test]
    fn merkle_root_for_empty_artifacts_matches_spec() {
        let artifacts: Vec<MerkleArtifact> = Vec::new();
        let root = merkle_root_for_artifacts(&artifacts, true);

        let empty = [0u8; 32];
        let root_raw = blake3::Hash::from(merkle_node_hash(&empty, &empty));

        let mut hasher = blake3::Hasher::new();
        hasher.update(&[0x02]);
        hasher.update(b"merkle-root-v1");
        hasher.update(&[0x00]);
        hasher.update(root_raw.as_bytes());
        let expected = hasher.finalize().to_hex().to_string();

        assert_eq!(root, expected);
    }

    #[test]
    fn merkle_root_raw_duplicates_last_leaf_for_odd_count() {
        let a = blake3::hash(b"a");
        let b = blake3::hash(b"b");
        let c = blake3::hash(b"c");

        let raw3 = merkle_root_raw(&[a, b, c]);
        let raw4 = merkle_root_raw(&[a, b, c, c]);

        assert_eq!(raw3, raw4);
    }

    #[test]
    fn merkle_fingerprint_paths_is_order_independent_strict() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let a = root.join("A.xml");
        let b = root.join("B.xml");
        write_file(&a, "<a/>");
        write_file(&b, "<b/>");

        let fp1 = merkle_fingerprint_paths(root, &[b.clone(), a.clone()], true);
        let fp2 = merkle_fingerprint_paths(root, &[a.clone(), b.clone()], true);
        let fp3 = merkle_fingerprint_paths(root, &[a, b], true);

        assert_eq!(fp1, fp2);
        assert_eq!(fp1, fp3);
    }

    #[test]
    fn merkle_fingerprint_paths_is_stable_for_same_inputs_strict() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let a = root.join("A.xml");
        let b = root.join("B.xml");
        write_file(&a, "<a/>");
        write_file(&b, "<b/>");

        let paths = [a, b];
        let fp1 = merkle_fingerprint_paths(root, &paths, true);
        let fp2 = merkle_fingerprint_paths(root, &paths, true);

        assert_eq!(fp1, fp2);
    }

    #[test]
    fn merkle_fingerprint_changes_when_one_file_changes_strict() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let a = root.join("A.xml");
        let b = root.join("B.xml");
        write_file(&a, "<a/>");
        write_file(&b, "<b/>");

        let paths = [a.clone(), b.clone()];
        let before = merkle_fingerprint_paths(root, &paths, true);

        write_file(&b, "<b>changed</b>");
        let after = merkle_fingerprint_paths(root, &paths, true);

        assert_ne!(before, after);
    }

    #[test]
    fn merkle_fingerprint_paths_dedups_duplicates_strict() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let a = root.join("A.xml");
        write_file(&a, "<a/>");

        let fp_unique = merkle_fingerprint_paths(root, std::slice::from_ref(&a), true);
        let fp_dup = merkle_fingerprint_paths(root, &[a.clone(), a], true);

        assert_eq!(fp_unique, fp_dup);
    }

    #[test]
    fn merkle_fingerprint_paths_with_modules_matches_paths_when_no_modules_strict() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let a = root.join("A.xml");
        write_file(&a, "<a/>");
        let xml_paths = [a];

        let empty_modules: Vec<PathBuf> = Vec::new();
        let fp_xml = merkle_fingerprint_paths(root, &xml_paths, true);
        let fp_with = merkle_fingerprint_paths_with_modules(root, &xml_paths, &empty_modules, true);

        assert_eq!(fp_xml, fp_with);
    }

    #[test]
    fn merkle_fingerprint_paths_with_modules_includes_bsl_artifacts_strict() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let xml = root.join("Configuration.xml");
        let bsl = root
            .join("CommonModules")
            .join("M")
            .join("Ext")
            .join("Module.bsl");
        write_file(&xml, "<Configuration/>");
        write_file(&bsl, "Процедура X() Экспорт\nКонецПроцедуры\n");

        let xml_paths = [xml];

        let empty_modules: Vec<PathBuf> = Vec::new();
        let no_modules =
            merkle_fingerprint_paths_with_modules(root, &xml_paths, &empty_modules, true);
        let with_modules = merkle_fingerprint_paths_with_modules(root, &xml_paths, &[bsl], true);

        assert_ne!(no_modules, with_modules);
    }

    #[test]
    fn normalize_path_strips_root_and_uses_forward_slashes() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let path = root
            .join("CommonModules")
            .join("M")
            .join("Ext")
            .join("Module.bsl");
        write_file(&path, ""); // файл должен существовать для реалистичного кейса

        assert_eq!(
            normalize_path(&path, Some(root)),
            "CommonModules/M/Ext/Module.bsl"
        );
    }

    #[test]
    fn merkle_root_depends_on_artifact_kind_strict() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let path = root.join("A.any");
        write_file(&path, "same");

        let fp_one = merkle_root_for_artifacts(
            &[MerkleArtifact {
                kind: "xml",
                path: path.clone(),
                path_norm: "A".to_string(),
            }],
            true,
        );
        let fp_two = merkle_root_for_artifacts(
            &[
                MerkleArtifact {
                    kind: "xml",
                    path: path.clone(),
                    path_norm: "A".to_string(),
                },
                MerkleArtifact {
                    kind: "bsl",
                    path,
                    path_norm: "A".to_string(),
                },
            ],
            true,
        );

        assert_ne!(fp_one, fp_two);
    }

    #[test]
    fn merkle_fingerprint_paths_with_modules_dedups_duplicate_bsl_paths_strict() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let xml = root.join("Configuration.xml");
        let bsl = root.join("CommonModules").join("M").join("Module.bsl");
        write_file(&xml, "<Configuration/>");
        write_file(&bsl, "Процедура X() Экспорт\nКонецПроцедуры\n");

        let xml_paths = [xml];

        let fp_unique = merkle_fingerprint_paths_with_modules(
            root,
            &xml_paths,
            std::slice::from_ref(&bsl),
            true,
        );
        let fp_dup =
            merkle_fingerprint_paths_with_modules(root, &xml_paths, &[bsl.clone(), bsl], true);

        assert_eq!(fp_unique, fp_dup);
    }
}

mod warmup_tests {
    use super::*;
    use crate::system::IndexSnapshot;
    use std::sync::Arc;

    #[test]
    fn warmup_skips_when_snapshot_id_changed() {
        let current = IndexSnapshot::empty(IndexSnapshotId::from_hash("a"));
        let candidate = IndexSnapshot::empty(IndexSnapshotId::from_hash("b"));
        assert!(!should_apply_warmup(&current, &candidate));
    }

    #[test]
    fn warmup_skips_when_current_has_data() {
        let mut current = IndexSnapshot::empty(IndexSnapshotId::from_hash("a"));
        Arc::make_mut(&mut current.type_index).insert(
            "Type".to_string(),
            Arc::new(IndexItem::new(
                "Type",
                IndexItemKind::Type(TypeKind::Platform),
                IndexKind::Type,
            )),
        );
        let candidate = IndexSnapshot::empty(IndexSnapshotId::from_hash("a"));
        assert!(!should_apply_warmup(&current, &candidate));
    }

    #[test]
    fn warmup_applies_when_snapshot_matches_and_empty() {
        let current = IndexSnapshot::empty(IndexSnapshotId::from_hash("a"));
        let candidate = IndexSnapshot::empty(IndexSnapshotId::from_hash("a"));
        assert!(should_apply_warmup(&current, &candidate));
    }
}
