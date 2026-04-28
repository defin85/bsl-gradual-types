use super::*;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::TempDir;

fn write_configuration_xml(dir: &TempDir, body: &str) -> PathBuf {
    let path = dir.path().join("Configuration.xml");
    fs::write(&path, body).expect("write Configuration.xml");
    path
}

struct EnvGuard {
    key: &'static str,
    prev: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
        let prev = std::env::var(key).ok();
        std::env::set_var(key, value);
        crate::system::runtime_config::global_runtime_config().reload_env_bootstrap_from_env();
        Self { key, prev }
    }

    fn remove(key: &'static str) -> Self {
        let prev = std::env::var(key).ok();
        std::env::remove_var(key);
        crate::system::runtime_config::global_runtime_config().reload_env_bootstrap_from_env();
        Self { key, prev }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(prev) = &self.prev {
            std::env::set_var(self.key, prev);
        } else {
            std::env::remove_var(self.key);
        }
        crate::system::runtime_config::global_runtime_config().reload_env_bootstrap_from_env();
    }
}

#[test]
fn test_syntax_helper_disk_cache_reuse() {
    let syntax_path = Path::new("examples/syntax_helper");
    if !syntax_path.exists() {
        eprintln!("⚠️ Syntax Helper не найден в examples/syntax_helper");
        return;
    }

    let temp = TempDir::new().unwrap();
    let cache = crate::system::DiskCache::with_root(temp.path().to_path_buf(), 1).unwrap();
    let builds = AtomicUsize::new(0);
    let coordinator = SystemCoordinator::new();

    let mut parser = SyntaxHelperLoader::new();
    let key = coordinator
        .build_syntax_helper_cache_key(syntax_path, Some("8.3.25"), &parser)
        .unwrap();
    let entry = cache
        .get_or_build_with(
            &key,
            || {
                builds.fetch_add(1, Ordering::SeqCst);
                parser.parse_syntax_helper(syntax_path)?;
                Ok(SyntaxHelperCachePayload {
                    database: parser.export_database(),
                    parse_ok: true,
                })
            },
            |payload| payload.parse_ok && !payload.database.nodes.is_empty(),
        )
        .unwrap();
    assert!(!entry.from_cache);

    let mut parser = SyntaxHelperLoader::new();
    let key = coordinator
        .build_syntax_helper_cache_key(syntax_path, Some("8.3.25"), &parser)
        .unwrap();
    let entry = cache
        .get_or_build_with(
            &key,
            || {
                builds.fetch_add(1, Ordering::SeqCst);
                parser.parse_syntax_helper(syntax_path)?;
                Ok(SyntaxHelperCachePayload {
                    database: parser.export_database(),
                    parse_ok: true,
                })
            },
            |payload| payload.parse_ok && !payload.database.nodes.is_empty(),
        )
        .unwrap();
    assert!(entry.from_cache);
    assert_eq!(builds.load(Ordering::SeqCst), 1);
}

#[test]
fn test_platform_raw_cache_produces_signature_index() {
    let syntax_path = Path::new("examples/syntax_helper");
    if !syntax_path.exists() {
        eprintln!("⚠️ Syntax Helper не найден в examples/syntax_helper");
        return;
    }

    let temp = TempDir::new().unwrap();
    let _cache_dir_guard = EnvGuard::set("BSL_CACHE_DIR", temp.path());
    let _cache_disable_guard = EnvGuard::remove("BSL_CACHE_DISABLE");

    let coordinator = SystemCoordinator::new();
    let load_result = coordinator
        .load_syntax_helper(syntax_path, Some("8.3.25"), &None)
        .unwrap();

    let repository = Arc::new(InMemoryTypeRepository::new());
    let platform_docs_bundle = coordinator
        .populate_repository_from_syntax_helper(
            &repository,
            load_result.database,
            load_result.cache_meta.as_ref(),
        )
        .unwrap();

    let index = repository.get_signature_index_clone();
    let methods = index.get_type_methods("Массив");
    assert!(
        methods.iter().any(|method| method.name == "Добавить"),
        "Ожидали метод Массив.Добавить в SignatureIndex"
    );
    assert!(
        platform_docs_bundle
            .global_context_index
            .get("Метаданные")
            .is_some(),
        "Ожидали Метаданные в GlobalContextIndex semantic bundle"
    );
}

#[test]
fn test_combined_cache_roundtrip() {
    let syntax_path = Path::new("examples/syntax_helper");
    let config_path = Path::new("examples/conf/conf_test");
    if !syntax_path.exists() || !config_path.exists() {
        eprintln!("⚠️ Не найдены примеры syntax_helper или конфигурации");
        return;
    }

    let temp = TempDir::new().unwrap();
    let _cache_dir_guard = EnvGuard::set("BSL_CACHE_DIR", temp.path());
    let _cache_disable_guard = EnvGuard::remove("BSL_CACHE_DISABLE");

    let coordinator = SystemCoordinator::new();
    let load_result = coordinator
        .load_syntax_helper(syntax_path, Some("8.3.25"), &None)
        .unwrap();
    let platform_meta = match load_result.cache_meta.as_ref() {
        Some(meta) => meta.clone(),
        None => return,
    };

    coordinator
        .start_with_paths_blocking(Some(syntax_path), Some(config_path), Some("8.3.25"), None)
        .unwrap();

    let config_meta = coordinator
        .build_config_combined_cache_meta(config_path)
        .unwrap();
    let key = coordinator.build_combined_cache_key(&platform_meta, &config_meta);
    let cache = coordinator.disk_cache();
    let cached = cache.try_get::<CombinedCachePayload>(&key).unwrap();
    assert!(cached.is_some(), "Combined cache entry отсутствует");
}

#[test]
fn test_parse_platform_version() {
    let direct = parse_platform_version("8.3.25").expect("direct");
    assert_eq!(direct.to_string(), "8.3.25");

    let prefixed = parse_platform_version("Version8_3_25").expect("prefixed");
    assert_eq!(prefixed.to_string(), "8.3.25");
}

#[test]
fn test_parse_platform_version_invalid() {
    assert!(parse_platform_version("Version8_3").is_none());
    assert!(parse_platform_version("invalid").is_none());
}

#[test]
fn test_required_platform_version_missing_compatibility() {
    let temp = TempDir::new().unwrap();
    write_configuration_xml(
        &temp,
        r#"
<MetaDataObject>
  <Configuration>
<Properties>
  <Name>Test</Name>
</Properties>
  </Configuration>
</MetaDataObject>
"#,
    );

    let coordinator = SystemCoordinator::new();
    let result = coordinator.required_platform_version(temp.path());
    assert!(
        result.is_err(),
        "expected error for missing CompatibilityMode"
    );
}

#[test]
fn test_required_platform_version_invalid_compatibility() {
    let temp = TempDir::new().unwrap();
    write_configuration_xml(
        &temp,
        r#"
<MetaDataObject>
  <Configuration>
<Properties>
  <Name>Test</Name>
  <CompatibilityMode>Version8_3</CompatibilityMode>
</Properties>
  </Configuration>
</MetaDataObject>
"#,
    );

    let coordinator = SystemCoordinator::new();
    let result = coordinator.required_platform_version(temp.path());
    assert!(
        result.is_err(),
        "expected error for invalid CompatibilityMode"
    );
}

#[test]
fn test_platform_version_below_compatibility_is_rejected() {
    let temp = TempDir::new().unwrap();
    write_configuration_xml(
        &temp,
        r#"
<MetaDataObject>
  <Configuration>
<Properties>
  <Name>Test</Name>
  <CompatibilityMode>Version8_3_25</CompatibilityMode>
</Properties>
  </Configuration>
</MetaDataObject>
"#,
    );

    let coordinator = SystemCoordinator::new();
    let result =
        coordinator.start_with_paths_blocking(None, Some(temp.path()), Some("8.3.24"), None);
    assert!(result.is_err(), "expected error for lower platform version");
}
