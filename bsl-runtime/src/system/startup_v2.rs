use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::mpsc;

use crate::data::loaders::progress::ProgressUpdate;
use crate::system::platform_version::normalize_platform_version;
use crate::system::runtime_config::{global_runtime_config, RuntimeKey};
use crate::system::{
    build_deps_bundle_v2_with_semantic_rules_config, load_semantic_rules_config_report,
    resolve_semantic_rules_config_path, DepsBundleV2, StartupError, SystemCoordinator,
};

#[derive(Debug, Clone, Default)]
pub struct StartupInputs {
    pub syntax_helper_path: Option<PathBuf>,
    pub configuration_path: Option<PathBuf>,
    pub platform_version: Option<String>,
    pub rules_config_path: Option<PathBuf>,
    pub cache_enabled: Option<bool>,
    pub strict_fingerprint: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct EffectiveStartupInputs {
    pub syntax_helper_path: Option<PathBuf>,
    pub configuration_path: Option<PathBuf>,
    pub platform_version: Option<String>,
    pub rules_config_path: Option<PathBuf>,
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
            platform_version: platform_version
                .and_then(non_empty_string)
                .map(str::to_string),
            rules_config_path: None,
            cache_enabled,
            strict_fingerprint,
        }
    }

    pub fn from_web_flags(
        syntax_helper_path: Option<PathBuf>,
        configuration_path: Option<PathBuf>,
        platform_version: Option<String>,
        rules_config_path: Option<PathBuf>,
        cache_enabled: Option<bool>,
        strict_fingerprint: Option<bool>,
    ) -> Self {
        Self {
            syntax_helper_path,
            configuration_path,
            platform_version,
            rules_config_path,
            cache_enabled,
            strict_fingerprint,
        }
    }

    pub fn normalize(self) -> Result<Self, StartupError> {
        let syntax_helper_path = self
            .syntax_helper_path
            .map(normalize_path_best_effort)
            .map(normalize_syntax_helper_root_best_effort);

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
                    Some(normalize_platform_version(trimmed).ok_or_else(|| {
                        StartupError::PlatformTypesError(anyhow::anyhow!(
                            "Invalid platform_version: {}",
                            trimmed
                        ))
                    })?)
                }
            }
            None => None,
        };

        if configuration_path.is_some() && platform_version.is_none() {
            return Err(StartupError::PlatformTypesError(anyhow::anyhow!(
                "platform_version is required when configuration_path is set"
            )));
        }

        let runtime_rules_config_path = self
            .rules_config_path
            .is_none()
            .then(|| global_runtime_config().get_pathbuf(RuntimeKey::RulesConfigPath))
            .flatten();
        let explicit_rules_config_path = self.rules_config_path.or(runtime_rules_config_path);
        let rules_config_path = resolve_semantic_rules_config_path(
            explicit_rules_config_path.as_deref(),
            configuration_path.as_deref(),
        );

        Ok(Self {
            syntax_helper_path,
            configuration_path,
            platform_version,
            rules_config_path,
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
    let rules_config_path = inputs.rules_config_path.clone();
    let semantic_rules_report = load_semantic_rules_config_report(rules_config_path.as_deref());
    for diagnostic in &semantic_rules_report.diagnostics {
        tracing::warn!(
            "startup_v2 semantic rules config diagnostic: path={}, message={}",
            rules_config_path
                .as_deref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "none".to_string()),
            diagnostic.message
        );
    }
    let semantic_rules_config = semantic_rules_report.config;
    let build_result = tokio::task::spawn_blocking(move || {
        build_deps_bundle_v2_with_semantic_rules_config(
            coordinator_for_build.as_ref(),
            platform_docs_root.as_deref(),
            config_root.as_deref(),
            Some(&semantic_rules_config),
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
            rules_config_path: inputs.rules_config_path,
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

fn normalize_syntax_helper_root_best_effort(path: PathBuf) -> PathBuf {
    // `platform_docs_archive` in external tools (MCP/LSP) is allowed to be either:
    // - a directory containing extracted docs (rebuilt.shcntx_ru, rebuilt.shlang_ru), OR
    // - a file path to a .hbk archive (typically one of shcntx_*.hbk or shlang_*.hbk).
    //
    // The system coordinator expects a DIRECTORY root (it scans for *.hbk inside).
    // For best UX, if a file is provided, we treat its parent directory as the root.
    match std::fs::metadata(&path) {
        Ok(meta) if meta.is_file() => match path.extension().and_then(|e| e.to_str()) {
            Some("hbk") | Some("zip") => path.parent().map(PathBuf::from).unwrap_or(path),
            _ => path,
        },
        _ => path,
    }
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
#[path = "startup_v2/tests.rs"]
mod tests;
