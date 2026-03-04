use super::*;

use std::fs;
use tempfile::TempDir;

#[test]
fn startup_inputs_normalize_maps_hbk_file_to_parent_dir() {
    let dir = TempDir::new().expect("tempdir");
    let file = dir.path().join("shcntx_ru.hbk");
    fs::write(&file, "dummy").expect("write dummy hbk");

    let inputs = StartupInputs {
        syntax_helper_path: Some(file),
        ..Default::default()
    }
    .normalize()
    .expect("normalize");

    assert_eq!(inputs.syntax_helper_path.as_deref(), Some(dir.path()));
}

#[test]
fn startup_inputs_normalize_rejects_missing_platform_version_for_config() {
    let inputs = StartupInputs {
        configuration_path: Some(PathBuf::from("some/config")),
        platform_version: None,
        ..Default::default()
    };

    assert!(inputs.normalize().is_err());
}

#[test]
fn startup_inputs_normalize_normalizes_platform_version() {
    let inputs = StartupInputs {
        platform_version: Some("Version8_3_25".to_string()),
        ..Default::default()
    };

    let normalized = inputs.normalize().expect("normalize");
    assert_eq!(normalized.platform_version.as_deref(), Some("8.3.25"));
}

#[test]
fn startup_inputs_normalize_converts_configuration_xml_to_root_dir() {
    let temp = TempDir::new().expect("TempDir");
    let config_root = temp.path().join("conf");
    fs::create_dir_all(&config_root).expect("create config root");
    let config_xml = config_root.join("Configuration.xml");
    fs::write(&config_xml, "<MetaDataObject/>").expect("write Configuration.xml");

    let inputs = StartupInputs {
        configuration_path: Some(config_xml),
        platform_version: Some("8.3.25".to_string()),
        ..Default::default()
    };

    let normalized = inputs.normalize().expect("normalize");
    let expected = fs::canonicalize(&config_root).expect("canonicalize config root");
    assert_eq!(
        normalized.configuration_path.as_deref(),
        Some(expected.as_path())
    );
}

#[test]
fn lsp_and_web_startup_inputs_normalize_identically() {
    let temp = TempDir::new().expect("TempDir");
    let platform_root = temp.path().join("platform");
    let config_root = temp.path().join("conf");
    fs::create_dir_all(&platform_root).expect("create platform root");
    fs::create_dir_all(&config_root).expect("create config root");

    fs::write(platform_root.join("a.html"), "<html/>").expect("write platform file");
    let config_xml = config_root.join("Configuration.xml");
    fs::write(&config_xml, "<MetaDataObject/>").expect("write Configuration.xml");

    let lsp_inputs = StartupInputs::from_lsp_settings(
        platform_root.to_str(),
        config_xml.to_str(),
        Some("Version8_3_25"),
        Some(true),
        Some(false),
    );

    let web_inputs = StartupInputs::from_web_flags(
        Some(platform_root),
        Some(config_xml),
        Some("Version8_3_25".to_string()),
        Some(true),
        Some(false),
    );

    let lsp_normalized = lsp_inputs.normalize().expect("lsp normalize");
    let web_normalized = web_inputs.normalize().expect("web normalize");

    assert_eq!(
        lsp_normalized.syntax_helper_path.as_deref(),
        web_normalized.syntax_helper_path.as_deref()
    );
    assert_eq!(
        lsp_normalized.configuration_path.as_deref(),
        web_normalized.configuration_path.as_deref()
    );
    assert_eq!(
        lsp_normalized.platform_version.as_deref(),
        web_normalized.platform_version.as_deref()
    );
    assert_eq!(lsp_normalized.cache_enabled, web_normalized.cache_enabled);
    assert_eq!(
        lsp_normalized.strict_fingerprint,
        web_normalized.strict_fingerprint
    );
}

#[test]
fn same_startup_inputs_produce_stable_deps_and_index_ids() {
    let temp = TempDir::new().expect("TempDir");
    let platform_root = temp.path().join("platform");
    let config_root = temp.path().join("config");

    fs::create_dir_all(&platform_root).expect("create platform root");
    fs::create_dir_all(&config_root).expect("create config root");

    fs::write(platform_root.join("a.html"), "<html/>").expect("write platform file");
    fs::write(config_root.join("Configuration.xml"), "<MetaDataObject/>")
        .expect("write config file");

    let coordinator = SystemCoordinator::new();
    coordinator.set_platform_version(Some("8.3.25".to_string()));
    coordinator.set_strict_fingerprint(false);

    let seed = build_deps_bundle_v2(
        &coordinator,
        Some(platform_root.as_path()),
        Some(config_root.as_path()),
    )
    .expect("seed bundle");
    let config_fp = seed
        .meta
        .config_fingerprint
        .as_deref()
        .unwrap_or("none")
        .to_string();
    coordinator
        .intellisense_index()
        .update_snapshot_id(&config_fp, &seed.meta.platform_version);

    let left = build_deps_bundle_v2(
        &coordinator,
        Some(platform_root.as_path()),
        Some(config_root.as_path()),
    )
    .expect("left bundle");
    let right = build_deps_bundle_v2(
        &coordinator,
        Some(platform_root.as_path()),
        Some(config_root.as_path()),
    )
    .expect("right bundle");

    assert_eq!(left.deps_id.as_str(), right.deps_id.as_str());
    assert_eq!(left.meta.index_snapshot_id, right.meta.index_snapshot_id);
}

#[test]
fn changing_platform_version_changes_deps_and_index_ids() {
    let temp = TempDir::new().expect("TempDir");
    let platform_root = temp.path().join("platform");
    let config_root = temp.path().join("config");

    fs::create_dir_all(&platform_root).expect("create platform root");
    fs::create_dir_all(&config_root).expect("create config root");

    fs::write(platform_root.join("a.html"), "<html/>").expect("write platform file");
    fs::write(config_root.join("Configuration.xml"), "<MetaDataObject/>")
        .expect("write config file");

    let coordinator = SystemCoordinator::new();
    coordinator.set_strict_fingerprint(false);

    coordinator.set_platform_version(Some("8.3.25".to_string()));
    let seed = build_deps_bundle_v2(
        &coordinator,
        Some(platform_root.as_path()),
        Some(config_root.as_path()),
    )
    .expect("seed bundle");
    let config_fp = seed
        .meta
        .config_fingerprint
        .clone()
        .expect("config_fingerprint");
    coordinator
        .intellisense_index()
        .update_snapshot_id(&config_fp, &seed.meta.platform_version);

    let left = build_deps_bundle_v2(
        &coordinator,
        Some(platform_root.as_path()),
        Some(config_root.as_path()),
    )
    .expect("left bundle");

    coordinator.set_platform_version(Some("8.3.26".to_string()));
    coordinator
        .intellisense_index()
        .update_snapshot_id(&config_fp, "8.3.26");

    let right = build_deps_bundle_v2(
        &coordinator,
        Some(platform_root.as_path()),
        Some(config_root.as_path()),
    )
    .expect("right bundle");

    assert_ne!(left.meta.index_snapshot_id, right.meta.index_snapshot_id);
    assert_ne!(left.deps_id.as_str(), right.deps_id.as_str());
}

#[test]
fn changing_configuration_fingerprint_changes_deps_and_index_ids() {
    let temp = TempDir::new().expect("TempDir");
    let platform_root = temp.path().join("platform");
    let config_root = temp.path().join("config");

    fs::create_dir_all(&platform_root).expect("create platform root");
    fs::create_dir_all(&config_root).expect("create config root");

    fs::write(platform_root.join("a.html"), "<html/>").expect("write platform file");
    let config_file = config_root.join("Configuration.xml");
    fs::write(&config_file, "<MetaDataObject/>").expect("write config file");

    let coordinator = SystemCoordinator::new();
    coordinator.set_platform_version(Some("8.3.25".to_string()));
    coordinator.set_strict_fingerprint(false);

    let seed = build_deps_bundle_v2(
        &coordinator,
        Some(platform_root.as_path()),
        Some(config_root.as_path()),
    )
    .expect("seed bundle");
    let config_fp = seed
        .meta
        .config_fingerprint
        .clone()
        .expect("config_fingerprint");
    coordinator
        .intellisense_index()
        .update_snapshot_id(&config_fp, &seed.meta.platform_version);

    let left = build_deps_bundle_v2(
        &coordinator,
        Some(platform_root.as_path()),
        Some(config_root.as_path()),
    )
    .expect("left bundle");

    fs::write(&config_file, "<MetaDataObject><X/></MetaDataObject>").expect("update config file");

    let changed = build_deps_bundle_v2(
        &coordinator,
        Some(platform_root.as_path()),
        Some(config_root.as_path()),
    )
    .expect("changed bundle");
    let changed_fp = changed
        .meta
        .config_fingerprint
        .clone()
        .expect("config_fingerprint");

    assert_ne!(config_fp, changed_fp);

    coordinator
        .intellisense_index()
        .update_snapshot_id(&changed_fp, &changed.meta.platform_version);

    let right = build_deps_bundle_v2(
        &coordinator,
        Some(platform_root.as_path()),
        Some(config_root.as_path()),
    )
    .expect("right bundle");

    assert_ne!(left.meta.index_snapshot_id, right.meta.index_snapshot_id);
    assert_ne!(left.deps_id.as_str(), right.deps_id.as_str());
}
