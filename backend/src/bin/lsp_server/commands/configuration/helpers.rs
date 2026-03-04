use super::*;

pub(super) fn build_config_index_cache(
    config_root: &Path,
    metadata: &[UniversalMetadataObject],
    module_signatures: &[ModuleSignatureSnapshot],
) -> ConfigIndexCache {
    let discovery = ConfigurationDiscovery::new(config_root.to_path_buf(), false);
    let mut cache = ConfigIndexCache {
        config_root: config_root.to_path_buf(),
        ..Default::default()
    };

    for obj in metadata {
        let key = ObjectKey::new(&obj.object_type_raw, &obj.name);
        cache.metadata_by_key.insert(key.clone(), obj.clone());

        if let Some(xml_path) =
            resolve_object_xml_path(&discovery, config_root, &obj.object_type_raw, &obj.name)
        {
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

pub(super) fn normalize_changed_paths(
    changed_paths: &[String],
    raw_config_path: &Path,
    canonical_config_path: &Path,
) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();

    for raw in changed_paths {
        let path = PathBuf::from(raw);
        let mapped = if let Ok(rel) = path.strip_prefix(raw_config_path) {
            canonical_config_path.join(rel)
        } else if let Ok(rel) = path.strip_prefix(canonical_config_path) {
            canonical_config_path.join(rel)
        } else {
            path
        };

        if !mapped.starts_with(canonical_config_path) {
            continue;
        }
        if seen.insert(mapped.clone()) {
            out.push(mapped);
        }
    }

    out
}

pub(super) fn build_child_objects_map(
    metadata_by_key: &HashMap<ObjectKey, UniversalMetadataObject>,
) -> HashMap<String, Vec<String>> {
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
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

pub(super) fn resolve_object_xml_path(
    discovery: &ConfigurationDiscovery,
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

pub(super) fn parse_metadata_object(
    discovery: &ConfigurationDiscovery,
    xml_path: &Path,
) -> Result<UniversalMetadataObject, String> {
    let mut metadata =
        UniversalMetadataParser::parse_any_object(xml_path).map_err(|e| e.to_string())?;
    let folder_name = discovery.xml_tag_to_folder_name(&metadata.object_type_raw);

    if let Ok(forms) =
        discovery.discover_forms_for_object(&folder_name, &metadata.object_type_raw, &metadata.name)
    {
        metadata.forms = forms;
    }

    let (object_mod, manager_mod, record_set_mod) =
        discovery.discover_object_modules(&folder_name, &metadata.name);
    metadata.object_module_path = object_mod;
    metadata.manager_module_path = manager_mod;
    metadata.record_set_module_path = record_set_mod;

    Ok(metadata)
}

pub(super) fn collect_module_paths_for_metadata(
    config_root: &Path,
    metadata: &UniversalMetadataObject,
) -> Vec<PathBuf> {
    let mut out = Vec::new();

    if metadata.object_type == Some(bsl_shared::domain::types::MetadataKind::CommonModule) {
        out.push(
            config_root
                .join("CommonModules")
                .join(&metadata.name)
                .join("Ext")
                .join("Module.bsl"),
        );
    }

    if let Some(p) = metadata.object_module_path.as_ref() {
        out.push(p.clone());
    }
    if let Some(p) = metadata.manager_module_path.as_ref() {
        out.push(p.clone());
    }
    if let Some(p) = metadata.record_set_module_path.as_ref() {
        out.push(p.clone());
    }

    out.sort();
    out.dedup();
    out
}

pub(super) fn refresh_form_mappings(
    cache: &mut ConfigIndexCache,
    config_root: &Path,
    discovery: &ConfigurationDiscovery,
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

pub(super) fn remove_module_signatures(
    repository: &dyn TypeRepository,
    cache: &mut ConfigIndexCache,
    module_path: &Path,
) {
    let Some(snapshot) = cache.module_signatures.remove(module_path) else {
        return;
    };

    if let Some(owner) = snapshot.owner_type.as_ref() {
        repository.remove_config_method_signatures(owner, &snapshot.method_names);
        repository.remove_config_method_definition_locations(owner, &snapshot.method_names);
    }
    repository.remove_global_function_signatures(&snapshot.global_function_names);
    repository.remove_global_function_definition_locations(&snapshot.global_function_names);
}

pub(super) fn apply_module_index_result(
    repository: &dyn TypeRepository,
    cache: &mut ConfigIndexCache,
    result: ModuleIndexResult,
) {
    remove_module_signatures(repository, cache, &result.module_path);

    for (owner_type, sig) in result.config_methods {
        repository.add_config_method_signature(&owner_type, sig);
    }
    for (name, sig) in result.global_functions {
        repository.add_global_function_signature(&name, sig);
    }
    for (owner_type, method_name, location) in result.definition_locations {
        repository.add_config_method_definition_location(&owner_type, &method_name, location);
    }
    for (function_name, location) in result.global_definition_locations {
        repository.add_global_function_definition_location(&function_name, location);
    }

    if !result.snapshot.method_names.is_empty() || !result.snapshot.global_function_names.is_empty()
    {
        cache
            .module_signatures
            .insert(result.module_path, result.snapshot);
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_metadata_update(
    cache: &mut ConfigIndexCache,
    repository: &dyn TypeRepository,
    config_root: &Path,
    discovery: &ConfigurationDiscovery,
    xml_path: &Path,
    metadata: UniversalMetadataObject,
    modules_to_reindex: &mut HashSet<PathBuf>,
    updated_types: &mut usize,
    removed_types: &mut usize,
) {
    let new_key = ObjectKey::new(&metadata.object_type_raw, &metadata.name);
    if let Some(old_key) = cache.object_xml_map.get(xml_path).cloned() {
        if old_key != new_key {
            remove_object(cache, repository, config_root, &old_key, removed_types);
        }
    }

    let old_metadata = cache.metadata_by_key.get(&new_key).cloned();
    let old_module_paths = old_metadata
        .as_ref()
        .map(|m| collect_module_paths_for_metadata(config_root, m))
        .unwrap_or_default();

    cache
        .metadata_by_key
        .insert(new_key.clone(), metadata.clone());
    cache
        .object_xml_map
        .insert(xml_path.to_path_buf(), new_key.clone());
    refresh_form_mappings(cache, config_root, discovery, &new_key, &metadata);

    let raw_type = metadata.to_raw_type_data(None);
    if repository.upsert_types(vec![raw_type]).is_ok() {
        *updated_types += 1;
    }

    let new_module_paths = collect_module_paths_for_metadata(config_root, &metadata);
    for path in old_module_paths
        .into_iter()
        .chain(new_module_paths.into_iter())
    {
        modules_to_reindex.insert(path);
    }
}

pub(super) fn remove_object(
    cache: &mut ConfigIndexCache,
    repository: &dyn TypeRepository,
    config_root: &Path,
    object_key: &ObjectKey,
    removed_types: &mut usize,
) {
    let Some(metadata) = cache.metadata_by_key.remove(object_key) else {
        return;
    };

    let raw_type = metadata.to_raw_type_data(None);
    if let Ok(removed) = repository.remove_types(&[raw_type.name]) {
        *removed_types += removed;
    }

    for module_path in collect_module_paths_for_metadata(config_root, &metadata) {
        remove_module_signatures(repository, cache, &module_path);
    }

    cache.object_xml_map.retain(|_, key| key != object_key);
    cache.form_xml_map.retain(|_, key| key != object_key);
}

pub(super) fn resolve_object_key_for_form(
    cache: &ConfigIndexCache,
    form_xml: &Path,
) -> Option<ObjectKey> {
    if let Some(key) = cache.form_xml_map.get(form_xml) {
        return Some(key.clone());
    }

    let mut parts = Vec::new();
    for part in form_xml.iter() {
        parts.push(part.to_string_lossy().to_string());
    }

    let forms_idx = parts.iter().rposition(|p| p == "Forms")?;
    if forms_idx < 2 {
        return None;
    }

    let object_name = parts.get(forms_idx - 1)?;
    let folder_name = parts.get(forms_idx - 2)?;
    let object_type_raw = ConfigurationDiscovery::folder_name_to_xml_tag(folder_name)?.to_string();
    let key = ObjectKey::new(object_type_raw, object_name);

    cache.metadata_by_key.get(&key).map(|_| key)
}
