use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::mpsc;

use crate::data::loaders::progress::ProgressUpdate;
use crate::system::platform_version::normalize_platform_version;
use crate::system::{DepsBundleV2, SystemCoordinator, StartupError, build_deps_bundle_v2};

#[derive(Debug, Clone, Default)]
pub struct StartupInputs {
    pub syntax_helper_path: Option<PathBuf>,
    pub configuration_path: Option<PathBuf>,
    pub platform_version: Option<String>,
    pub cache_enabled: Option<bool>,
    pub strict_fingerprint: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct EffectiveStartupInputs {
    pub syntax_helper_path: Option<PathBuf>,
    pub configuration_path: Option<PathBuf>,
    pub platform_version: Option<String>,
    pub cache_enabled: bool,
    pub strict_fingerprint: bool,
}

#[derive(Clone)]
pub struct StartupResultV2 {
    pub coordinator: Arc<SystemCoordinator>,
    pub deps_bundle_v2: DepsBundleV2,
    pub inputs: EffectiveStartupInputs,
}

impl StartupInputs {
    pub fn from_lsp_settings(
        platform_docs_archive: Option<&str>,
        configuration_path: Option<&str>,
        platform_version: Option<&str>,
        cache_enabled: Option<bool>,
        strict_fingerprint: Option<bool>,
    ) -> Self {
        Self {
            syntax_helper_path: platform_docs_archive
                .and_then(|value| non_empty_string(value).map(PathBuf::from)),
            configuration_path: configuration_path
                .and_then(|value| non_empty_string(value).map(PathBuf::from)),
            platform_version: platform_version.and_then(non_empty_string).map(str::to_string),
            cache_enabled,
            strict_fingerprint,
        }
    }

    pub fn from_web_flags(
        syntax_helper_path: Option<PathBuf>,
        configuration_path: Option<PathBuf>,
        platform_version: Option<String>,
        cache_enabled: Option<bool>,
        strict_fingerprint: Option<bool>,
    ) -> Self {
        Self {
            syntax_helper_path,
            configuration_path,
            platform_version,
            cache_enabled,
            strict_fingerprint,
        }
    }

    pub fn normalize(self) -> Result<Self, StartupError> {
        let syntax_helper_path = self
            .syntax_helper_path
            .map(normalize_path_best_effort);

        let configuration_path = self
            .configuration_path
            .map(normalize_configuration_root)
            .map(normalize_path_best_effort);

        let platform_version = match self.platform_version {
            Some(value) => {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(
                        normalize_platform_version(trimmed).ok_or_else(|| {
                            StartupError::PlatformTypesError(anyhow::anyhow!(
                                "Invalid platform_version: {}",
                                trimmed
                            ))
                        })?,
                    )
                }
            }
            None => None,
        };

        if configuration_path.is_some() && platform_version.is_none() {
            return Err(StartupError::PlatformTypesError(anyhow::anyhow!(
                "platform_version is required when configuration_path is set"
            )));
        }

        Ok(Self {
            syntax_helper_path,
            configuration_path,
            platform_version,
            cache_enabled: self.cache_enabled,
            strict_fingerprint: self.strict_fingerprint,
        })
    }
}

pub async fn startup_v2(
    coordinator: Arc<SystemCoordinator>,
    inputs: StartupInputs,
    progress_tx: Option<mpsc::UnboundedSender<ProgressUpdate>>,
) -> Result<StartupResultV2, StartupError> {
    let inputs = inputs.normalize()?;

    let cache_enabled = match inputs.cache_enabled {
        Some(enabled) => coordinator.set_cache_enabled(enabled).await.effective,
        None => coordinator.disk_cache().is_enabled(),
    };

    let strict_fingerprint = match inputs.strict_fingerprint {
        Some(strict) => {
            coordinator.set_strict_fingerprint(strict);
            strict
        }
        None => coordinator.strict_fingerprint(),
    };

    coordinator
        .start_with_paths(
            inputs.syntax_helper_path.as_deref(),
            inputs.configuration_path.as_deref(),
            inputs.platform_version.as_deref(),
            progress_tx,
        )
        .await?;

    let build_started = Instant::now();
    let coordinator_for_build = coordinator.clone();
    let platform_docs_root = inputs.syntax_helper_path.clone();
    let config_root = inputs.configuration_path.clone();
    let build_result = tokio::task::spawn_blocking(move || {
        build_deps_bundle_v2(
            coordinator_for_build.as_ref(),
            platform_docs_root.as_deref(),
            config_root.as_deref(),
        )
    })
    .await;

    coordinator.record_intellisense_v2_deps_update_build_latency(build_started.elapsed());

    let deps_bundle_v2 = match build_result {
        Ok(Ok(bundle)) => bundle,
        Ok(Err(err)) => {
            coordinator.record_intellisense_v2_deps_update_error();
            return Err(StartupError::PlatformTypesError(err));
        }
        Err(err) => {
            coordinator.record_intellisense_v2_deps_update_error();
            return Err(StartupError::CacheError(format!(
                "Deps bundle build task failed: {}",
                err
            )));
        }
    };

    Ok(StartupResultV2 {
        coordinator,
        deps_bundle_v2,
        inputs: EffectiveStartupInputs {
            syntax_helper_path: inputs.syntax_helper_path,
            configuration_path: inputs.configuration_path,
            platform_version: inputs.platform_version,
            cache_enabled,
            strict_fingerprint,
        },
    })
}

fn non_empty_string(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn normalize_path_best_effort(path: PathBuf) -> PathBuf {
    std::fs::canonicalize(&path).unwrap_or(path)
}

fn normalize_configuration_root(path: PathBuf) -> PathBuf {
    if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
        if name.eq_ignore_ascii_case("Configuration.xml") {
            return path.parent().unwrap_or(path.as_path()).to_path_buf();
        }
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;
    use tempfile::TempDir;

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
        assert_eq!(normalized.configuration_path.as_deref(), Some(expected.as_path()));
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
        fs::write(config_root.join("Configuration.xml"), "<MetaDataObject/>").expect("write config file");

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

        fs::write(&config_file, "<MetaDataObject><X/></MetaDataObject>")
            .expect("update config file");

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
}
