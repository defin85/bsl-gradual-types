use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Context;
use walkdir::WalkDir;

use bsl_analysis_v2::{DepsSnapshotId, SemanticDeps};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::resolver::TypeResolver;

use super::{IndexSnapshot, SystemCoordinator};

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
}

pub fn build_deps_bundle_v2(
    coordinator: &SystemCoordinator,
    platform_docs_root: Option<&Path>,
    config_root: Option<&Path>,
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

    let (semantic_deps, repo_stats) =
        build_semantic_deps_snapshot(coordinator).context("build_semantic_deps_snapshot")?;

    let deps_payload = format!(
        "schema={};platform_version={};platform_fp={};config_fp={};index_snapshot_id={};repo.total_types={};repo.platform_types={};repo.configuration_types={};repo.user_defined_types={};platform_signatures_loaded={};strict_fingerprint={}",
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
        strict,
    );

    let deps_id =
        DepsSnapshotId::from_hash(blake3::hash(deps_payload.as_bytes()).to_hex().to_string());

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
        },
    })
}

fn build_semantic_deps_snapshot(
    coordinator: &SystemCoordinator,
) -> anyhow::Result<(
    Arc<SemanticDeps>,
    bsl_shared::domain::repository::RepositoryStats,
)> {
    let Some(engine) = coordinator.analysis_engine() else {
        let repository: Arc<dyn TypeRepository> = Arc::new(InMemoryTypeRepository::new());
        let resolver = Arc::new(TypeResolver::new(repository.clone()));
        let signature_index = repository.get_signature_index_clone();
        let platform_signatures_loaded = repository.platform_docs_loaded();
        let stats = repository.get_stats();
        return Ok((
            Arc::new(SemanticDeps {
                repository,
                signature_index,
                resolver: Some(resolver),
                platform_signatures_loaded,
            }),
            stats,
        ));
    };

    let source_repo = engine.get_repository();
    let stats = source_repo.get_stats();
    let raw_types = source_repo.get_all_types();
    let platform_docs_loaded = source_repo.platform_docs_loaded();
    let signature_index = source_repo.get_signature_index_clone();
    let method_definition_locations = source_repo.get_method_definition_locations_clone();
    let platform_signatures_loaded = platform_docs_loaded;

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
        Arc::new(SemanticDeps {
            repository,
            signature_index,
            resolver: Some(resolver),
            platform_signatures_loaded,
        }),
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
