use super::*;

#[derive(Debug)]
pub(super) struct ConfigCacheIdentity {
    pub(super) project_id: String,
    pub(super) config_id: String,
    pub(super) canonical_config: PathBuf,
}

pub(super) fn emit_cached_config_progress<F>(
    config_info: &crate::data::loaders::config_metadata_parser::ConfigurationInfo,
    progress_callback: &F,
) where
    F: Fn(ProgressUpdate) + Send + Sync + Clone + 'static,
{
    let phases = [
        IndexingPhase::ConfigurationDiscovery,
        IndexingPhase::ConfigurationParsing,
        IndexingPhase::ConfigurationLinking,
        IndexingPhase::ConfigurationFinalizing,
    ];
    for phase in phases {
        progress_callback(ProgressUpdate::new(
            phase,
            1,
            1,
            Some(format!("{} (кэш)", config_info.name)),
        ));
    }
}

pub(super) fn emit_cached_module_index_progress<F>(
    config_info: &crate::data::loaders::config_metadata_parser::ConfigurationInfo,
    progress_callback: &F,
) where
    F: Fn(ProgressUpdate) + Send + Sync + Clone + 'static,
{
    progress_callback(ProgressUpdate::new(
        IndexingPhase::ConfigurationIndexingModules,
        1,
        1,
        Some(format!(
            "Индексация BSL-модулей: {} (кэш)",
            config_info.name
        )),
    ));
}

pub(super) fn config_cache_identity(
    root_path: &Path,
    config_info: &crate::data::loaders::config_metadata_parser::ConfigurationInfo,
) -> ConfigCacheIdentity {
    let canonical_root =
        std::fs::canonicalize(root_path).unwrap_or_else(|_| root_path.to_path_buf());
    let canonical_config =
        std::fs::canonicalize(&config_info.path).unwrap_or_else(|_| config_info.path.clone());
    let project_id = blake3::hash(canonical_root.to_string_lossy().as_bytes())
        .to_hex()
        .to_string();
    let config_id = config_id_for_info(config_info);

    ConfigCacheIdentity {
        project_id,
        config_id,
        canonical_config,
    }
}

pub(super) fn project_id_from_root(root_path: &Path) -> String {
    let canonical_root =
        std::fs::canonicalize(root_path).unwrap_or_else(|_| root_path.to_path_buf());
    blake3::hash(canonical_root.to_string_lossy().as_bytes())
        .to_hex()
        .to_string()
}

pub(super) fn config_id_for_info(
    config_info: &crate::data::loaders::config_metadata_parser::ConfigurationInfo,
) -> String {
    if let Some(uuid) = config_info.uuid.clone() {
        return uuid;
    }
    let canonical_config =
        std::fs::canonicalize(&config_info.path).unwrap_or_else(|_| config_info.path.clone());
    blake3::hash(canonical_config.to_string_lossy().as_bytes())
        .to_hex()
        .to_string()
}

pub(super) fn normalize_config_root(config_path: &Path) -> PathBuf {
    if config_path.file_name().and_then(|name| name.to_str()) == Some("Configuration.xml") {
        return config_path.parent().unwrap_or(config_path).to_path_buf();
    }
    config_path.to_path_buf()
}

pub(super) fn extend_indexed_signatures(
    target: &mut IndexedConfigSignatures,
    source: &IndexedConfigSignatures,
) {
    target.config_methods.extend(source.config_methods.clone());
    target
        .global_functions
        .extend(source.global_functions.clone());
    target
        .definition_locations
        .extend(source.definition_locations.clone());
    target
        .global_definition_locations
        .extend(source.global_definition_locations.clone());
    target
        .module_signatures
        .extend(source.module_signatures.clone());
}

pub(super) fn build_config_index_cache(
    config_root: &Path,
    metadata: &[UniversalMetadataObject],
    module_signatures: &[ModuleSignatureSnapshot],
) -> ConfigIndexCache {
    let discovery = crate::data::loaders::config_metadata_parser::ConfigurationDiscovery::new(
        config_root.to_path_buf(),
        false,
    );
    let mut cache = ConfigIndexCache {
        config_root: config_root.to_path_buf(),
        ..Default::default()
    };

    for obj in metadata {
        let key = ObjectKey::new(&obj.object_type_raw, &obj.name);
        cache.metadata_by_key.insert(key.clone(), obj.clone());

        let xml_path = obj
            .metadata_xml_path
            .as_ref()
            .filter(|path| path.exists())
            .cloned()
            .or_else(|| {
                resolve_object_xml_path(&discovery, config_root, &obj.object_type_raw, &obj.name)
            });
        if let Some(xml_path) = xml_path {
            cache.object_xml_map.insert(xml_path, key.clone());
        }

        refresh_form_mappings(&mut cache, config_root, &discovery, &key, obj);
    }

    cache.child_objects = build_child_objects_map(&cache.metadata_by_key);

    for snapshot in module_signatures {
        cache
            .module_signatures
            .insert(snapshot.module_path.clone(), snapshot.clone());
    }

    cache
}

fn build_child_objects_map(
    metadata_by_key: &std::collections::HashMap<ObjectKey, UniversalMetadataObject>,
) -> std::collections::HashMap<String, Vec<String>> {
    let mut out: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for obj in metadata_by_key.values() {
        out.entry(obj.object_type_raw.clone())
            .or_default()
            .push(obj.name.clone());
    }
    for names in out.values_mut() {
        names.sort();
        names.dedup();
    }
    out
}

fn resolve_object_xml_path(
    discovery: &crate::data::loaders::config_metadata_parser::ConfigurationDiscovery,
    config_root: &Path,
    object_type_raw: &str,
    object_name: &str,
) -> Option<PathBuf> {
    let folder_name = discovery.xml_tag_to_folder_name(object_type_raw);
    let direct = config_root
        .join(&folder_name)
        .join(format!("{}.xml", object_name));
    if direct.exists() {
        return Some(direct);
    }

    let subdir = config_root
        .join(&folder_name)
        .join(object_name)
        .join(format!("{}.xml", object_name));
    if subdir.exists() {
        return Some(subdir);
    }

    None
}

fn refresh_form_mappings(
    cache: &mut ConfigIndexCache,
    config_root: &Path,
    discovery: &crate::data::loaders::config_metadata_parser::ConfigurationDiscovery,
    key: &ObjectKey,
    metadata: &UniversalMetadataObject,
) {
    cache.form_xml_map.retain(|_, v| v != key);
    let folder_name = discovery.xml_tag_to_folder_name(&metadata.object_type_raw);
    for form in &metadata.forms {
        let form_xml = config_root
            .join(&folder_name)
            .join(&metadata.name)
            .join("Forms")
            .join(&form.name)
            .join("Ext")
            .join("Form.xml");
        cache.form_xml_map.insert(form_xml, key.clone());
    }
}

pub(super) fn config_fingerprint(config_path: &Path, strict: bool) -> Result<String> {
    use walkdir::WalkDir;

    let mut files: Vec<PathBuf> = WalkDir::new(config_path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("xml") {
                Some(path.to_path_buf())
            } else {
                None
            }
        })
        .collect();

    if files.is_empty() {
        let config_xml = config_path.join("Configuration.xml");
        if config_xml.exists() {
            files.push(config_xml);
        }
    }

    Ok(merkle_fingerprint_paths(config_path, &files, strict))
}

pub(super) fn config_settings_fingerprint(strict: bool) -> String {
    format!(
        "config_parser_v3;modules_indexing_v1;strict_fingerprint={}",
        strict
    )
}

pub(super) fn config_layer_b_fingerprint(
    config_path: &Path,
    metadata_for_indexing: &[UniversalMetadataObject],
    strict: bool,
) -> Result<String> {
    use walkdir::WalkDir;

    let mut files: Vec<PathBuf> = WalkDir::new(config_path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("xml") {
                Some(path.to_path_buf())
            } else {
                None
            }
        })
        .collect();

    let module_paths = collect_module_paths(config_path, metadata_for_indexing);

    if files.is_empty() {
        let config_xml = config_path.join("Configuration.xml");
        if config_xml.exists() {
            files.push(config_xml);
        }
    }

    Ok(merkle_fingerprint_paths_with_modules(
        config_path,
        &files,
        &module_paths,
        strict,
    ))
}

pub(super) fn config_layer_b_settings_fingerprint(strict: bool) -> String {
    format!(
        "config_layer_b_v7;modules_indexing_v1;strict_fingerprint={}",
        strict
    )
}

pub(super) fn module_cache_settings_fingerprint(strict: bool) -> String {
    format!("config_module_parse_v2;strict_fingerprint={}", strict)
}

pub(super) fn file_fingerprint(path: &Path, strict: bool) -> Result<String> {
    if !path.exists() {
        return Ok(String::new());
    }
    Ok(merkle_fingerprint_single(path, strict))
}

struct MerkleArtifact {
    kind: &'static str,
    path: PathBuf,
    path_norm: String,
}

pub(super) fn merkle_fingerprint_paths(
    config_root: &Path,
    xml_paths: &[PathBuf],
    strict: bool,
) -> String {
    let mut artifacts: Vec<MerkleArtifact> = xml_paths
        .iter()
        .filter(|path| path.is_file())
        .map(|path| MerkleArtifact {
            kind: "xml",
            path: path.to_path_buf(),
            path_norm: normalize_path(path, Some(config_root)),
        })
        .collect();

    artifacts.sort_by(|a, b| {
        a.kind
            .cmp(b.kind)
            .then_with(|| a.path_norm.cmp(&b.path_norm))
    });
    artifacts.dedup_by(|a, b| a.kind == b.kind && a.path_norm == b.path_norm);

    merkle_root_for_artifacts(&artifacts, strict)
}

pub(super) fn merkle_fingerprint_paths_with_modules(
    config_root: &Path,
    xml_paths: &[PathBuf],
    module_paths: &[PathBuf],
    strict: bool,
) -> String {
    let mut artifacts: Vec<MerkleArtifact> = Vec::new();

    for path in xml_paths {
        if path.is_file() {
            artifacts.push(MerkleArtifact {
                kind: "xml",
                path: path.to_path_buf(),
                path_norm: normalize_path(path, Some(config_root)),
            });
        }
    }

    for path in module_paths {
        if path.is_file() {
            artifacts.push(MerkleArtifact {
                kind: "bsl",
                path: path.to_path_buf(),
                path_norm: normalize_path(path, Some(config_root)),
            });
        }
    }

    artifacts.sort_by(|a, b| {
        a.kind
            .cmp(b.kind)
            .then_with(|| a.path_norm.cmp(&b.path_norm))
    });
    artifacts.dedup_by(|a, b| a.kind == b.kind && a.path_norm == b.path_norm);

    merkle_root_for_artifacts(&artifacts, strict)
}

pub(super) fn merkle_fingerprint_single(path: &Path, strict: bool) -> String {
    let artifacts = [MerkleArtifact {
        kind: "file",
        path: path.to_path_buf(),
        path_norm: normalize_path(path, None),
    }];
    merkle_root_for_artifacts(&artifacts, strict)
}

fn merkle_root_for_artifacts(artifacts: &[MerkleArtifact], strict: bool) -> String {
    let mut leaves: Vec<blake3::Hash> = Vec::with_capacity(artifacts.len());
    for artifact in artifacts {
        if strict {
            let content_hash = match std::fs::read(&artifact.path) {
                Ok(contents) => blake3::hash(&contents),
                Err(_) => blake3::hash(&[]),
            };
            leaves.push(merkle_leaf_hash_strict(
                artifact.kind,
                &artifact.path_norm,
                &content_hash,
            ));
        } else {
            let (size, mtime_ns) = file_metadata_fields(&artifact.path);
            leaves.push(merkle_leaf_hash_fast(
                artifact.kind,
                &artifact.path_norm,
                size,
                mtime_ns,
            ));
        }
    }

    let root_raw = merkle_root_raw(&leaves);
    let mut hasher = blake3::Hasher::new();
    hasher.update(&[0x02]);
    hasher.update(b"merkle-root-v1");
    hasher.update(&[0x00]);
    hasher.update(root_raw.as_bytes());
    hasher.finalize().to_hex().to_string()
}

pub(super) fn merkle_leaf_hash_fast(
    kind: &str,
    path_norm: &str,
    size: u64,
    mtime_ns: u64,
) -> blake3::Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&[0x00]);
    hasher.update(kind.as_bytes());
    hasher.update(&[0x00]);
    hasher.update(path_norm.as_bytes());
    hasher.update(&[0x00]);
    hasher.update(&size.to_le_bytes());
    hasher.update(&[0x00]);
    hasher.update(&mtime_ns.to_le_bytes());
    hasher.finalize()
}

pub(super) fn merkle_leaf_hash_strict(
    kind: &str,
    path_norm: &str,
    content_hash: &blake3::Hash,
) -> blake3::Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&[0x00]);
    hasher.update(kind.as_bytes());
    hasher.update(&[0x00]);
    hasher.update(path_norm.as_bytes());
    hasher.update(&[0x00]);
    hasher.update(content_hash.as_bytes());
    hasher.finalize()
}

pub(super) fn merkle_root_raw(leaves: &[blake3::Hash]) -> blake3::Hash {
    let empty = [0u8; 32];
    if leaves.is_empty() {
        return blake3::Hash::from(merkle_node_hash(&empty, &empty));
    }

    let mut level: Vec<[u8; 32]> = leaves.iter().map(|hash| *hash.as_bytes()).collect();
    while level.len() > 1 {
        let mut next_level: Vec<[u8; 32]> = Vec::with_capacity(level.len().div_ceil(2));
        let mut idx = 0;
        while idx < level.len() {
            let left = level[idx];
            let right = if idx + 1 < level.len() {
                level[idx + 1]
            } else {
                left
            };
            next_level.push(merkle_node_hash(&left, &right));
            idx += 2;
        }
        level = next_level;
    }

    blake3::Hash::from(level[0])
}

pub(super) fn merkle_node_hash(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&[0x01]);
    hasher.update(left);
    hasher.update(right);
    *hasher.finalize().as_bytes()
}

pub(super) fn file_metadata_fields(path: &Path) -> (u64, u64) {
    if let Ok(metadata) = std::fs::metadata(path) {
        let size = metadata.len();
        let mtime_ns = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(duration_to_u64_nanos)
            .unwrap_or(0);
        (size, mtime_ns)
    } else {
        (0, 0)
    }
}

pub(super) fn duration_to_u64_nanos(duration: std::time::Duration) -> u64 {
    let nanos = duration.as_nanos();
    if nanos > u64::MAX as u128 {
        u64::MAX
    } else {
        nanos as u64
    }
}

pub(super) fn snapshot_is_empty(snapshot: &crate::system::IndexSnapshot) -> bool {
    snapshot.type_index.is_empty()
        && snapshot.symbol_index.is_empty()
        && snapshot.module_index.is_empty()
        && snapshot.metadata_index.is_empty()
}

pub(super) fn should_apply_warmup(
    current: &crate::system::IndexSnapshot,
    candidate: &crate::system::IndexSnapshot,
) -> bool {
    current.id == candidate.id && snapshot_is_empty(current)
}

pub(super) fn log_warmup_skip_reason(
    current: &crate::system::IndexSnapshot,
    candidate: &crate::system::IndexSnapshot,
    project_id: &str,
    config_set_id: &str,
    observability: &crate::system::BasicObservability,
) {
    if current.id != candidate.id {
        info!(
            "Warmup индекса пропущен (snapshot_id изменился) project={}, config={}",
            project_id, config_set_id
        );
        observability.record_index_warmup_skip("snapshot_changed");
        return;
    }
    if !snapshot_is_empty(current) {
        info!(
            "Warmup индекса пропущен (индекс уже заполнен) project={}, config={}",
            project_id, config_set_id
        );
        observability.record_index_warmup_skip("already_populated");
    }
}

pub(super) fn index_warmup_disabled() -> bool {
    !crate::system::runtime_config::global_runtime_config()
        .get_bool(crate::system::runtime_config::RuntimeKey::IndexWarmup)
        .unwrap_or(true)
}

pub(super) fn normalize_path(path: &Path, root: Option<&Path>) -> String {
    let relative = root
        .and_then(|base| path.strip_prefix(base).ok())
        .unwrap_or(path);
    let mut parts = Vec::new();
    for component in relative.components() {
        if let std::path::Component::Normal(value) = component {
            parts.push(value.to_string_lossy().to_string());
        }
    }
    parts.join("/")
}

pub(super) fn config_set_id_from_configs(
    configs: &[crate::data::loaders::config_metadata_parser::ConfigurationInfo],
) -> String {
    let mut base = None;
    let mut extensions = Vec::new();

    for info in configs {
        let id = info.uuid.clone().unwrap_or_else(|| {
            blake3::hash(info.path.to_string_lossy().as_bytes())
                .to_hex()
                .to_string()
        });
        if info.is_base() {
            base = Some(id);
        } else {
            extensions.push(id);
        }
    }

    extensions.sort();
    let mut parts = Vec::new();
    if let Some(base) = base {
        parts.push(base);
    }
    parts.extend(extensions);

    if parts.is_empty() {
        return String::new();
    }

    blake3::hash(parts.join("|").as_bytes())
        .to_hex()
        .to_string()
}

pub(super) fn config_set_id_from_single(
    info: &crate::data::loaders::config_metadata_parser::ConfigurationInfo,
) -> String {
    let id = info.uuid.clone().unwrap_or_else(|| {
        blake3::hash(info.path.to_string_lossy().as_bytes())
            .to_hex()
            .to_string()
    });
    blake3::hash(id.as_bytes()).to_hex().to_string()
}

pub(super) fn discover_single_config(
    discovery: &crate::data::loaders::config_metadata_parser::ConfigurationDiscovery,
    config_path: &Path,
) -> Option<crate::data::loaders::config_metadata_parser::ConfigurationInfo> {
    let config_xml =
        if config_path.file_name().and_then(|name| name.to_str()) == Some("Configuration.xml") {
            config_path.to_path_buf()
        } else {
            config_path.join("Configuration.xml")
        };

    if !config_xml.exists() {
        return None;
    }

    let configs = discovery.discover_all_configurations().ok()?;
    let config_dir = std::fs::canonicalize(config_xml.parent()?).ok()?;
    configs
        .into_iter()
        .find(|info| std::fs::canonicalize(&info.path).ok().as_ref() == Some(&config_dir))
}

#[cfg(test)]
#[path = "helpers/tests.rs"]
mod tests;
