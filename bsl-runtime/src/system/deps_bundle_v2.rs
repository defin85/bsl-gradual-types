use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Context;
use tracing::warn;
use walkdir::WalkDir;

use bsl_analysis_v2::semantic_rules::CommonModuleFactoryRegistry;
use bsl_analysis_v2::{DepsSnapshotId, SemanticDeps};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::GlobalContextAvailability;
use bsl_shared::domain::GlobalContextIndex;

use super::{IndexSnapshot, SemanticRulesConfig, SystemCoordinator};

#[derive(Clone)]
pub struct DepsBundleV2 {
    pub deps_id: DepsSnapshotId,
    pub semantic_deps: Arc<SemanticDeps>,
    pub index_snapshot: Arc<IndexSnapshot>,
    pub meta: DepsBundleV2Meta,
}

#[derive(Clone)]
pub struct DepsBundleV2Meta {
    pub platform_version: String,
    pub platform_fingerprint: Option<String>,
    pub config_fingerprint: Option<String>,
    pub index_snapshot_id: String,
    pub strict_fingerprint: bool,
    pub global_context_state: String,
    pub global_context_property_count: usize,
    pub global_context_fingerprint: String,
    pub global_context_degraded_reason: Option<String>,
}

pub fn build_deps_bundle_v2(
    coordinator: &SystemCoordinator,
    platform_docs_root: Option<&Path>,
    config_root: Option<&Path>,
) -> anyhow::Result<DepsBundleV2> {
    build_deps_bundle_v2_with_semantic_rules_config(
        coordinator,
        platform_docs_root,
        config_root,
        None,
    )
}

pub fn build_deps_bundle_v2_with_semantic_rules_config(
    coordinator: &SystemCoordinator,
    platform_docs_root: Option<&Path>,
    config_root: Option<&Path>,
    semantic_rules_config: Option<&SemanticRulesConfig>,
) -> anyhow::Result<DepsBundleV2> {
    let strict = coordinator.strict_fingerprint();

    let platform_fingerprint = platform_docs_root
        .map(|root| fingerprint_platform_docs(root, strict))
        .transpose()
        .unwrap_or_else(|err| Some(format!("error:{}", err)));

    let config_fingerprint = config_root
        .map(|root| fingerprint_config_root(root, strict))
        .transpose()
        .unwrap_or_else(|err| Some(format!("error:{}", err)));

    let platform_version = coordinator
        .platform_version()
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

    let index_snapshot = coordinator.intellisense_index().snapshot();
    let index_snapshot_id = index_snapshot.id.as_str().to_string();

    let default_semantic_rules_config;
    let semantic_rules_config = match semantic_rules_config {
        Some(config) => config,
        None => {
            default_semantic_rules_config = SemanticRulesConfig::default();
            &default_semantic_rules_config
        }
    };
    let common_module_factory_registry =
        Arc::new(semantic_rules_config.common_module_factories.clone());
    let (semantic_deps, repo_stats) =
        build_semantic_deps_snapshot(coordinator, common_module_factory_registry)
            .context("build_semantic_deps_snapshot")?;

    let deps_payload = format!(
        "schema={};platform_version={};platform_fp={};config_fp={};index_snapshot_id={};repo.total_types={};repo.platform_types={};repo.configuration_types={};repo.user_defined_types={};platform_signatures_loaded={};global_context_fp={};strict_fingerprint={};semantic_rules={}",
        bsl_analysis_v2::DEPS_SCHEMA_VERSION,
        platform_version,
        platform_fingerprint.as_deref().unwrap_or("none"),
        config_fingerprint.as_deref().unwrap_or("none"),
        index_snapshot_id,
        repo_stats.total_types,
        repo_stats.platform_types,
        repo_stats.configuration_types,
        repo_stats.user_defined_types,
        semantic_deps.platform_signatures_loaded,
        semantic_deps.global_context_index.fingerprint(),
        strict,
        semantic_rules_config.identity.cache_key_payload(),
    );

    let deps_id =
        DepsSnapshotId::from_hash(blake3::hash(deps_payload.as_bytes()).to_hex().to_string());
    let global_context_state = global_context_state(semantic_deps.global_context_index.as_ref());
    let global_context_property_count = semantic_deps.global_context_index.len();
    let global_context_fingerprint = semantic_deps.global_context_index.fingerprint().to_string();
    let global_context_degraded_reason =
        global_context_degraded_reason(semantic_deps.global_context_index.as_ref());

    Ok(DepsBundleV2 {
        deps_id,
        semantic_deps,
        index_snapshot: Arc::new(index_snapshot),
        meta: DepsBundleV2Meta {
            platform_version,
            platform_fingerprint,
            config_fingerprint,
            index_snapshot_id,
            strict_fingerprint: strict,
            global_context_state,
            global_context_property_count,
            global_context_fingerprint,
            global_context_degraded_reason,
        },
    })
}

fn global_context_state(index: &GlobalContextIndex) -> String {
    match index.availability() {
        GlobalContextAvailability::Loaded if index.is_empty() => "loaded_empty".to_string(),
        GlobalContextAvailability::Loaded => "loaded".to_string(),
        GlobalContextAvailability::Unavailable => "absent".to_string(),
        GlobalContextAvailability::Degraded { .. } => "degraded".to_string(),
    }
}

fn global_context_degraded_reason(index: &GlobalContextIndex) -> Option<String> {
    match index.availability() {
        GlobalContextAvailability::Degraded { reason } => Some(reason.clone()),
        _ => None,
    }
}

fn build_semantic_deps_snapshot(
    coordinator: &SystemCoordinator,
    common_module_factory_registry: Arc<CommonModuleFactoryRegistry>,
) -> anyhow::Result<(
    Arc<SemanticDeps>,
    bsl_shared::domain::repository::RepositoryStats,
)> {
    let Some(bundle) = coordinator.domain_bundle() else {
        warn!("SystemCoordinator has no DomainBundle; building empty SemanticDeps snapshot (did you forget to call start?)");
        let repository: Arc<dyn TypeRepository> = Arc::new(InMemoryTypeRepository::new());
        let resolver = Arc::new(TypeResolver::new(repository.clone()));
        let signature_index = repository.get_signature_index_clone();
        let platform_signatures_loaded = repository.platform_docs_loaded();
        let stats = repository.get_stats();
        return Ok((
            Arc::new(
                SemanticDeps::from_parts_with_common_module_factory_registry(
                    repository,
                    signature_index,
                    Some(resolver),
                    platform_signatures_loaded,
                    Arc::new(GlobalContextIndex::unavailable()),
                    common_module_factory_registry,
                ),
            ),
            stats,
        ));
    };

    let source_repo = bundle.repository.clone();
    let stats = source_repo.get_stats();
    let raw_types = source_repo.get_all_types();
    let platform_docs_loaded = source_repo.platform_docs_loaded();
    let signature_index = source_repo.get_signature_index_clone();
    let method_definition_locations = source_repo.get_method_definition_locations_clone();
    let platform_signatures_loaded = platform_docs_loaded;
    let global_context_index = bundle.global_context_index.clone();

    let snapshot_repo_impl = Arc::new(InMemoryTypeRepository::new());
    snapshot_repo_impl.set_platform_docs_loaded(platform_docs_loaded);
    snapshot_repo_impl
        .load_types(raw_types)
        .context("load_types into snapshot repository")?;
    snapshot_repo_impl.set_signature_index(signature_index.clone());
    for (owner, name, location) in method_definition_locations {
        if let Some(owner) = owner.as_deref() {
            snapshot_repo_impl.add_config_method_definition_location(owner, &name, location);
        } else {
            snapshot_repo_impl.add_global_function_definition_location(&name, location);
        }
    }

    let repository = snapshot_repo_impl as Arc<dyn TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));

    Ok((
        Arc::new(
            SemanticDeps::from_parts_with_common_module_factory_registry(
                repository,
                signature_index,
                Some(resolver),
                platform_signatures_loaded,
                global_context_index,
                common_module_factory_registry,
            ),
        ),
        stats,
    ))
}

fn fingerprint_platform_docs(root: &Path, strict: bool) -> anyhow::Result<String> {
    let mut scan_roots = Vec::new();
    let context_help_path = root.join("rebuilt.shcntx_ru");
    let language_help_path = root.join("rebuilt.shlang_ru");

    if context_help_path.exists() {
        scan_roots.push(context_help_path);
    }
    if language_help_path.exists() {
        scan_roots.push(language_help_path);
    }
    if scan_roots.is_empty() && root.exists() {
        scan_roots.push(root.to_path_buf());
    }

    let mut files = Vec::new();
    for scan_root in &scan_roots {
        for entry in WalkDir::new(scan_root).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("html") {
                continue;
            }
            files.push(path.to_path_buf());
        }
    }

    Ok(fingerprint_paths(root, &files, strict))
}

fn fingerprint_config_root(root: &Path, strict: bool) -> anyhow::Result<String> {
    if !root.exists() {
        return Ok(format!("missing:{}", root.display()));
    }

    let mut files = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
            continue;
        };
        if ext != "xml" && ext != "bsl" {
            continue;
        }
        files.push(path.to_path_buf());
    }

    Ok(fingerprint_paths(root, &files, strict))
}

fn fingerprint_paths(root: &Path, files: &[PathBuf], strict: bool) -> String {
    let mut files: Vec<PathBuf> = files.to_vec();
    files.sort();
    files.dedup();

    let mut hasher = blake3::Hasher::new();
    for path in files {
        let rel = path.strip_prefix(root).unwrap_or(&path);
        hasher.update(rel.to_string_lossy().as_bytes());

        if let Ok(metadata) = std::fs::metadata(&path) {
            hasher.update(&metadata.len().to_le_bytes());
            let modified = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs())
                .unwrap_or(0);
            hasher.update(&modified.to_le_bytes());
        }

        if strict {
            if let Ok(contents) = std::fs::read(&path) {
                let content_hash = blake3::hash(&contents);
                hasher.update(content_hash.as_bytes());
            }
        }
    }

    hasher.finalize().to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::DomainBundle;
    use bsl_shared::domain::GlobalContextPropertyData;

    fn global_context_index_for(name: &str, prop_type: &str) -> Arc<GlobalContextIndex> {
        Arc::new(GlobalContextIndex::loaded(vec![
            GlobalContextPropertyData {
                name: format!("Глобальный контекст.{name}"),
                english_name: None,
                prop_type: Some(prop_type.to_string()),
                is_readonly: true,
                description: None,
                contexts: Vec::new(),
                source_key: format!("synthetic/{name}"),
                source_path: None,
                normalized_key: name.to_lowercase(),
                english_normalized_key: None,
                collection_item_type: None,
            },
        ]))
    }

    fn install_domain_bundle(
        coordinator: &SystemCoordinator,
        global_context_index: Arc<GlobalContextIndex>,
    ) {
        let repository: Arc<dyn TypeRepository> = Arc::new(InMemoryTypeRepository::new());
        let resolver = Arc::new(TypeResolver::new(repository.clone()));
        let mut cache = coordinator.domain_bundle_cache.write().unwrap();
        *cache = Some(Arc::new(DomainBundle {
            repository,
            resolver,
            global_context_index,
        }));
    }

    #[test]
    fn deps_snapshot_id_includes_global_context_fingerprint() {
        let coordinator = SystemCoordinator::new();
        coordinator.set_platform_version(Some("8.3.25".to_string()));

        install_domain_bundle(
            &coordinator,
            global_context_index_for("Метаданные", "ОбъектМетаданныхКонфигурация"),
        );
        let left = build_deps_bundle_v2(&coordinator, None, None).expect("left deps bundle");
        assert_eq!(left.meta.global_context_state, "loaded");
        assert_eq!(left.meta.global_context_property_count, 1);
        assert!(left.meta.global_context_degraded_reason.is_none());

        install_domain_bundle(
            &coordinator,
            global_context_index_for("Метаданные", "Строка"),
        );
        let right = build_deps_bundle_v2(&coordinator, None, None).expect("right deps bundle");

        assert_ne!(left.deps_id.as_str(), right.deps_id.as_str());
    }

    #[test]
    fn deps_snapshot_carries_typed_common_module_factory_registry() {
        let coordinator = SystemCoordinator::new();
        install_domain_bundle(
            &coordinator,
            global_context_index_for("Метаданные", "ОбъектМетаданныхКонфигурация"),
        );
        let config = crate::system::parse_semantic_rules_config_toml(
            "[semantic.common_module_factories]\nbuiltin_bsp = false\n",
        )
        .expect("rules config");

        let bundle = build_deps_bundle_v2_with_semantic_rules_config(
            &coordinator,
            None,
            None,
            Some(&config),
        )
        .expect("deps bundle");

        assert_eq!(
            bundle
                .semantic_deps
                .common_module_factory_registry
                .rules()
                .len(),
            0
        );
    }

    #[test]
    fn deps_snapshot_id_includes_semantic_rules_identity() {
        let coordinator = SystemCoordinator::new();
        install_domain_bundle(
            &coordinator,
            global_context_index_for("Метаданные", "ОбъектМетаданныхКонфигурация"),
        );
        let default_bundle = build_deps_bundle_v2(&coordinator, None, None).expect("default");
        let disabled_bsp = crate::system::parse_semantic_rules_config_toml(
            "[semantic.common_module_factories]\nbuiltin_bsp = false\n",
        )
        .expect("rules config");
        let custom_bundle = build_deps_bundle_v2_with_semantic_rules_config(
            &coordinator,
            None,
            None,
            Some(&disabled_bsp),
        )
        .expect("custom");

        assert_ne!(
            default_bundle.deps_id.as_str(),
            custom_bundle.deps_id.as_str()
        );
    }
}
