//! Configuration parsing command handler
//!
//! MILESTONE 2.17: Handles bsl.parseConfiguration command.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tower_lsp::Client;
use tracing::{debug, error, info, warn};

use bsl_backend::data::loaders::config_metadata_parser::{
    ConfigurationDiscovery, UniversalMetadataObject, UniversalMetadataParser,
};
use bsl_backend::data::loaders::{
    index_configuration_bsl_modules_by_paths,
    index_configuration_bsl_modules_with_progress_parallel, ModuleIndexProgress, ModuleIndexResult,
    ModuleSignatureSnapshot,
};
use bsl_backend::system::{ConfigIndexCache, ObjectKey, SystemCoordinator};
use bsl_shared::domain::repository::TypeRepository;
use bsl_shared::engine::AnalysisEngine;

use crate::progress_bridge::{LspWorkDoneReporter, ProgressPlan, ProgressReporter};
use crate::types::{IncrementalUpdateParams, IncrementalUpdateResponse};

/// Request for bsl.parseConfiguration
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseConfigurationParams {
    pub config_path: String,
}

/// Response for bsl.parseConfiguration
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseConfigurationResponse {
    pub success: bool,
    pub loaded_types: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Handle bsl.parseConfiguration command
pub async fn handle_parse_configuration(
    params: ParseConfigurationParams,
    analysis_engine: Option<Arc<AnalysisEngine>>,
    client: Client,
    progress_token_prefix: &str,
    progress_title: &str,
    coordinator: Option<Arc<SystemCoordinator>>,
) -> ParseConfigurationResponse {
    info!(
        "Custom command: bsl.parseConfiguration - {}",
        params.config_path
    );

    let config_path = std::path::PathBuf::from(&params.config_path);

    // Validate path
    if !config_path.exists() {
        warn!("Configuration path does not exist: {:?}", config_path);
        return ParseConfigurationResponse {
            success: false,
            loaded_types: 0,
            message: Some(format!("Path does not exist: {}", config_path.display())),
        };
    }

    if !config_path.is_dir() {
        warn!("Configuration path is not a directory: {:?}", config_path);
        return ParseConfigurationResponse {
            success: false,
            loaded_types: 0,
            message: Some("Path must be a directory".to_string()),
        };
    }

    let config_xml = config_path.join("Configuration.xml");
    if !config_xml.exists() {
        warn!("Configuration.xml not found in {:?}", config_path);
        return ParseConfigurationResponse {
            success: false,
            loaded_types: 0,
            message: Some("Configuration.xml not found in directory".to_string()),
        };
    }

    // Canonicalize path (protection from path traversal)
    let canonical_path = match config_path.canonicalize() {
        Ok(path) => path,
        Err(e) => {
            error!("Failed to canonicalize path {:?}: {}", config_path, e);
            return ParseConfigurationResponse {
                success: false,
                loaded_types: 0,
                message: Some(format!("Invalid path: {}", e)),
            };
        }
    };

    debug!("Validated configuration path: {:?}", canonical_path);

    // План зависит от операции: parseConfiguration/buildIndex/incrementalUpdate.
    // (Нужно, чтобы стадийные веса можно было корректировать независимо, без копипасты.)
    let plan = if progress_token_prefix.starts_with("bsl-incremental-update") {
        ProgressPlan::incremental_update()
    } else if progress_token_prefix.starts_with("bsl-build-index") {
        ProgressPlan::build_index()
    } else {
        ProgressPlan::parse_configuration()
    };
    let discovery_range = plan.discovery;
    let load_types_range = plan.load_types;
    let index_modules_range = plan.index_bsl_modules;
    let reporter = tokio::sync::Mutex::new(
        LspWorkDoneReporter::create(client.clone(), progress_token_prefix).await,
    );
    let reporter = Arc::new(reporter);

    {
        let mut reporter = reporter.lock().await;
        reporter
            .begin(
                progress_title.to_string(),
                Some("Initializing...".to_string()),
            )
            .await;
    }

    // Валидация завершена — фиксируем конец validation стадии (0..10).
    reporter
        .lock()
        .await
        .report(plan.validation.end, Some("Validation OK".to_string()))
        .await;

    // Get AnalysisEngine
    let engine = match analysis_engine {
        Some(e) => e,
        None => {
            error!("AnalysisEngine not available");
            reporter
                .lock()
                .await
                .end(Some("Error: AnalysisEngine not available".to_string()))
                .await;
            return ParseConfigurationResponse {
                success: false,
                loaded_types: 0,
                message: Some("AnalysisEngine not available".to_string()),
            };
        }
    };

    let repo = engine.get_repository();

    reporter
        .lock()
        .await
        .report(
            discovery_range.start,
            Some("Discovering metadata objects...".to_string()),
        )
        .await;

    let discovery = ConfigurationDiscovery::new(canonical_path.clone(), false);

    let (discovery_tx, mut discovery_rx) = tokio::sync::mpsc::unbounded_channel::<(u32, String)>();
    let reporter_for_discovery = reporter.clone();
    let discovery_task = tokio::spawn(async move {
        while let Some((percentage, message)) = discovery_rx.recv().await {
            reporter_for_discovery
                .lock()
                .await
                .report(percentage, Some(message))
                .await;
        }
    });

    // Create progress callback (sync -> async via channel)
    let progress_callback = {
        let discovery_tx = discovery_tx.clone();
        move |update: bsl_backend::data::loaders::progress::ProgressUpdate| {
            let percentage = discovery_range.map_percent_0_100(update.percentage as u32);
            let message = update.message.unwrap_or_else(|| {
                format!(
                    "{}: {}/{}",
                    update.phase.display_name(),
                    update.current,
                    update.total
                )
            });
            let _ = discovery_tx.send((percentage, message));
        }
    };

    // Convert Result to avoid Send issues
    let discovery_result = discovery
        .discover_all_metadata(Some(progress_callback))
        .map_err(|e| e.to_string());

    // Завершаем consumer для discovery (flush последнего репорта на token).
    drop(discovery_tx);
    let _ = discovery_task.await;

    let metadata = match discovery_result {
        Ok(data) => data,
        Err(error_msg) => {
            error!("Failed to discover metadata: {}", error_msg);

            reporter
                .lock()
                .await
                .end(Some(format!("Error: {}", error_msg)))
                .await;

            return ParseConfigurationResponse {
                success: false,
                loaded_types: 0,
                message: Some(format!("Metadata discovery error: {}", error_msg)),
            };
        }
    };

    let total_objects = metadata.len();
    info!(
        "Discovered {} metadata objects from configuration",
        total_objects
    );

    const BATCH_SIZE: usize = 100;
    const PROGRESS_REPORT_INTERVAL: usize = 10;

    let mut loaded = 0;
    let mut batch = Vec::with_capacity(BATCH_SIZE);

    for (index, obj) in metadata.iter().enumerate() {
        let raw_type = obj.to_raw_type_data(None);
        batch.push(raw_type);

        // Load batch when size reached or at end
        if batch.len() >= BATCH_SIZE || index == total_objects - 1 {
            if let Err(e) = repo.load_types(batch.clone()) {
                let error_msg = e.to_string();
                error!("Failed to load types batch: {}", error_msg);

                reporter
                    .lock()
                    .await
                    .end(Some(format!("Error: {}", error_msg)))
                    .await;

                return ParseConfigurationResponse {
                    success: false,
                    loaded_types: loaded,
                    message: Some(format!("Type loading error: {}", error_msg)),
                };
            }

            loaded += batch.len();
            batch.clear();
        }

        // Report progress every 10 types
        if (index + 1) % PROGRESS_REPORT_INTERVAL == 0 || index == total_objects - 1 {
            let total_progress = load_types_range.map_current_total(loaded, total_objects);
            reporter
                .lock()
                .await
                .report(
                    total_progress,
                    Some(format!("Loaded {}/{} types", loaded, total_objects)),
                )
                .await;
        }
    }

    info!("Configuration parsed successfully: {} types loaded", loaded);

    reporter
        .lock()
        .await
        .report(
            index_modules_range.start,
            Some("Indexing configuration module methods (*.bsl)...".to_string()),
        )
        .await;

    let mut module_signatures: Vec<ModuleSignatureSnapshot> = Vec::new();

    let (modules_tx, mut modules_rx) =
        tokio::sync::mpsc::unbounded_channel::<ModuleIndexProgress>();
    let reporter_for_modules = reporter.clone();
    let modules_task = tokio::spawn(async move {
        while let Some(p) = modules_rx.recv().await {
            let percentage = index_modules_range.map_current_total(p.current, p.total);
            let message = format!(
                "Indexed {}/{}: {}",
                p.current,
                p.total,
                p.module_path.display()
            );
            reporter_for_modules
                .lock()
                .await
                .report(percentage, Some(message))
                .await;
        }
    });

    // Индексация экспортных методов из модулей конфигурации (*.bsl)
    let modules_tx_clone = modules_tx.clone();
    match index_configuration_bsl_modules_with_progress_parallel(
        &canonical_path,
        &metadata,
        Some(move |p| {
            let _ = modules_tx_clone.send(p);
        }),
    ) {
        Ok(indexed) => {
            module_signatures = indexed.module_signatures.clone();
            let config_methods_count = indexed.config_methods.len();
            let global_functions_count = indexed.global_functions.len();

            for (owner_type, sig) in indexed.config_methods {
                repo.add_config_method_signature(&owner_type, sig);
            }
            for (name, sig) in indexed.global_functions {
                repo.add_global_function_signature(&name, sig);
            }
            for (owner_type, method_name, location) in indexed.definition_locations {
                repo.add_config_method_definition_location(&owner_type, &method_name, location);
            }
            for (function_name, location) in indexed.global_definition_locations {
                repo.add_global_function_definition_location(&function_name, location);
            }
            info!("Configuration module methods indexed successfully");

            drop(modules_tx);
            let _ = modules_task.await;

            reporter
                .lock()
                .await
                .report(
                    index_modules_range.end,
                    Some(format!(
                        "Indexed {} methods, {} global functions",
                        config_methods_count, global_functions_count
                    )),
                )
                .await;
        }
        Err(e) => {
            warn!("Failed to index configuration module methods: {}", e);

            drop(modules_tx);
            let _ = modules_task.await;

            reporter
                .lock()
                .await
                .report(
                    index_modules_range.end,
                    Some(format!("Indexing skipped: {}", e)),
                )
                .await;
        }
    }

    if let Some(coordinator) = coordinator {
        let cache = build_config_index_cache(&canonical_path, &metadata, &module_signatures);
        let cache_lock = coordinator.config_index_cache();
        let mut guard = cache_lock.write().unwrap_or_else(|poisoned| {
            warn!("Config index cache RwLock poisoned (write), recovering");
            poisoned.into_inner()
        });
        *guard = Some(cache);
    }

    reporter
        .lock()
        .await
        .end(Some(format!("Loaded {} types", loaded)))
        .await;

    ParseConfigurationResponse {
        success: true,
        loaded_types: loaded,
        message: Some(format!(
            "Configuration loaded successfully: {} types",
            loaded
        )),
    }
}

/// Handle bsl/incrementalUpdate command
pub async fn handle_incremental_update(
    params: IncrementalUpdateParams,
    coordinator: Arc<SystemCoordinator>,
    client: Client,
) -> IncrementalUpdateResponse {
    info!(
        "Custom command: bsl.incrementalUpdate - {}",
        params.config_path
    );

    let config_path = PathBuf::from(&params.config_path);
    if !config_path.exists() {
        return IncrementalUpdateResponse {
            success: false,
            message: format!("Path does not exist: {}", config_path.display()),
        };
    }

    if !config_path.is_dir() {
        return IncrementalUpdateResponse {
            success: false,
            message: "Path must be a directory".to_string(),
        };
    }

    let config_xml = config_path.join("Configuration.xml");
    if !config_xml.exists() {
        return IncrementalUpdateResponse {
            success: false,
            message: "Configuration.xml not found in directory".to_string(),
        };
    }

    let canonical_path = match config_path.canonicalize() {
        Ok(path) => path,
        Err(e) => {
            return IncrementalUpdateResponse {
                success: false,
                message: format!("Invalid path: {}", e),
            };
        }
    };

    let changed_paths =
        normalize_changed_paths(&params.changed_paths, &config_path, &canonical_path);

    if changed_paths.is_empty() {
        let resp = handle_parse_configuration(
            ParseConfigurationParams {
                config_path: params.config_path,
            },
            coordinator.get_analysis_engine(),
            client,
            "bsl-incremental-update",
            "Incremental index update",
            Some(coordinator),
        )
        .await;
        return IncrementalUpdateResponse {
            success: resp.success,
            message: resp
                .message
                .unwrap_or_else(|| "Incremental update completed".to_string()),
        };
    }

    let cache_lock = coordinator.config_index_cache();
    let cache_ready = {
        let guard = cache_lock.read().unwrap_or_else(|poisoned| {
            warn!("Config index cache RwLock poisoned (read), recovering");
            poisoned.into_inner()
        });
        guard
            .as_ref()
            .is_some_and(|cache| cache.config_root == canonical_path)
    };

    if !cache_ready {
        let resp = handle_parse_configuration(
            ParseConfigurationParams {
                config_path: params.config_path,
            },
            coordinator.get_analysis_engine(),
            client,
            "bsl-incremental-update",
            "Incremental index update",
            Some(coordinator),
        )
        .await;
        return IncrementalUpdateResponse {
            success: resp.success,
            message: resp
                .message
                .unwrap_or_else(|| "Incremental update completed".to_string()),
        };
    }

    let plan = ProgressPlan::incremental_update();
    let reporter = tokio::sync::Mutex::new(
        LspWorkDoneReporter::create(client.clone(), "bsl-incremental-update").await,
    );
    let reporter = Arc::new(reporter);

    {
        let mut reporter = reporter.lock().await;
        reporter
            .begin(
                "Incremental index update".to_string(),
                Some("Initializing...".to_string()),
            )
            .await;
    }

    reporter
        .lock()
        .await
        .report(plan.validation.end, Some("Validation OK".to_string()))
        .await;

    let engine = match coordinator.get_analysis_engine() {
        Some(engine) => engine,
        None => {
            reporter
                .lock()
                .await
                .end(Some("Error: AnalysisEngine not available".to_string()))
                .await;
            return IncrementalUpdateResponse {
                success: false,
                message: "AnalysisEngine not available".to_string(),
            };
        }
    };

    let repository = engine.get_repository();
    let cache_taken = {
        let mut guard = cache_lock.write().unwrap_or_else(|poisoned| {
            warn!("Config index cache RwLock poisoned (write), recovering");
            poisoned.into_inner()
        });
        guard.take()
    };

    let mut cache = match cache_taken {
        Some(cache) => cache,
        None => {
            reporter
                .lock()
                .await
                .end(Some("Error: config cache not initialized".to_string()))
                .await;
            return IncrementalUpdateResponse {
                success: false,
                message: "Config cache not initialized".to_string(),
            };
        }
    };

    let mut changed_bsl = Vec::new();
    let mut changed_xml = Vec::new();
    for path in changed_paths {
        match path.extension().and_then(|s| s.to_str()) {
            Some(ext) if ext.eq_ignore_ascii_case("bsl") => changed_bsl.push(path),
            Some(ext) if ext.eq_ignore_ascii_case("xml") => changed_xml.push(path),
            _ => {}
        }
    }

    let discovery = ConfigurationDiscovery::new(canonical_path.clone(), false);
    let discovery_range = plan.discovery;
    let load_types_range = plan.load_types;
    let index_modules_range = plan.index_bsl_modules;

    let xml_total = changed_xml.len().max(1);
    let mut xml_processed = 0;
    let mut updated_types = 0usize;
    let mut removed_types = 0usize;
    let mut modules_to_reindex: HashSet<PathBuf> = HashSet::new();

    let mut processed_config_xml = false;
    for xml_path in &changed_xml {
        xml_processed += 1;
        reporter
            .lock()
            .await
            .report(
                discovery_range.map_current_total(xml_processed, xml_total),
                Some(format!("XML update: {}", xml_path.display())),
            )
            .await;

        if xml_path.file_name().and_then(|n| n.to_str()) == Some("Configuration.xml") {
            if processed_config_xml {
                continue;
            }
            processed_config_xml = true;

            match discovery.parse_child_objects_list(&canonical_path.join("Configuration.xml")) {
                Ok(child_objects) => {
                    let mut new_keys = HashSet::new();
                    for (object_type_raw, names) in &child_objects {
                        for name in names {
                            new_keys.insert(ObjectKey::new(object_type_raw, name));
                        }
                    }
                    let old_keys: HashSet<ObjectKey> =
                        cache.metadata_by_key.keys().cloned().collect();

                    for removed in old_keys.difference(&new_keys) {
                        remove_object(
                            &mut cache,
                            repository.as_ref(),
                            &canonical_path,
                            removed,
                            &mut removed_types,
                        );
                    }

                    for added in new_keys.difference(&old_keys) {
                        if let Some(xml_path) = resolve_object_xml_path(
                            &discovery,
                            &canonical_path,
                            &added.object_type_raw,
                            &added.name,
                        ) {
                            if let Ok(metadata) = parse_metadata_object(&discovery, &xml_path) {
                                apply_metadata_update(
                                    &mut cache,
                                    repository.as_ref(),
                                    &canonical_path,
                                    &discovery,
                                    &xml_path,
                                    metadata,
                                    &mut modules_to_reindex,
                                    &mut updated_types,
                                    &mut removed_types,
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to parse Configuration.xml: {}", e);
                }
            }

            continue;
        }

        if xml_path.file_name().and_then(|n| n.to_str()) == Some("Form.xml") {
            if let Some(object_key) = resolve_object_key_for_form(&cache, xml_path) {
                let updated = cache.metadata_by_key.get(&object_key).and_then(|metadata| {
                    let folder_name = discovery.xml_tag_to_folder_name(&metadata.object_type_raw);
                    discovery
                        .discover_forms_for_object(
                            &folder_name,
                            &metadata.object_type_raw,
                            &metadata.name,
                        )
                        .ok()
                        .map(|forms| {
                            let mut clone = metadata.clone();
                            clone.forms = forms;
                            clone
                        })
                });

                if let Some(metadata) = updated {
                    cache
                        .metadata_by_key
                        .insert(object_key.clone(), metadata.clone());
                    refresh_form_mappings(
                        &mut cache,
                        &canonical_path,
                        &discovery,
                        &object_key,
                        &metadata,
                    );
                }
            }
            continue;
        }

        if xml_path.exists() {
            match parse_metadata_object(&discovery, xml_path) {
                Ok(metadata) => {
                    apply_metadata_update(
                        &mut cache,
                        repository.as_ref(),
                        &canonical_path,
                        &discovery,
                        xml_path,
                        metadata,
                        &mut modules_to_reindex,
                        &mut updated_types,
                        &mut removed_types,
                    );
                }
                Err(e) => {
                    warn!("Failed to parse metadata XML {:?}: {}", xml_path, e);
                }
            }
        } else if let Some(old_key) = cache.object_xml_map.get(xml_path).cloned() {
            remove_object(
                &mut cache,
                repository.as_ref(),
                &canonical_path,
                &old_key,
                &mut removed_types,
            );
        }
    }

    for bsl_path in changed_bsl {
        modules_to_reindex.insert(bsl_path);
    }

    cache.child_objects = build_child_objects_map(&cache.metadata_by_key);

    reporter
        .lock()
        .await
        .report(
            load_types_range.end,
            Some(format!(
                "Types updated: {}, removed: {}",
                updated_types, removed_types
            )),
        )
        .await;

    let mut modules_for_indexing: Vec<PathBuf> = Vec::new();
    let mut removed_modules = 0usize;
    for module_path in modules_to_reindex {
        if !module_path.exists() {
            remove_module_signatures(repository.as_ref(), &mut cache, &module_path);
            removed_modules += 1;
            continue;
        }
        modules_for_indexing.push(module_path);
    }

    let mut module_processed = 0usize;
    let mut reindexed_modules = 0usize;

    if !modules_for_indexing.is_empty() {
        let metadata_vec: Vec<UniversalMetadataObject> =
            cache.metadata_by_key.values().cloned().collect();
        let results = index_configuration_bsl_modules_by_paths(
            &modules_for_indexing,
            &metadata_vec,
            None::<fn(ModuleIndexProgress)>,
        );

        match results {
            Ok(results) => {
                let results_total = results.len().max(1);
                for result in results {
                    module_processed += 1;
                    reporter
                        .lock()
                        .await
                        .report(
                            index_modules_range.map_current_total(module_processed, results_total),
                            Some(format!(
                                "Indexed {}/{}: {}",
                                module_processed,
                                results_total,
                                result.module_path.display()
                            )),
                        )
                        .await;

                    apply_module_index_result(repository.as_ref(), &mut cache, result);
                    reindexed_modules += 1;
                }
            }
            Err(e) => {
                warn!("Failed to reindex modules: {}", e);
            }
        }
    }

    {
        let mut guard = cache_lock.write().unwrap_or_else(|poisoned| {
            warn!("Config index cache RwLock poisoned (write), recovering");
            poisoned.into_inner()
        });
        *guard = Some(cache);
    }

    reporter
        .lock()
        .await
        .end(Some(format!(
            "Updated types: {}, modules: {}, removed modules: {}",
            updated_types, reindexed_modules, removed_modules
        )))
        .await;

    IncrementalUpdateResponse {
        success: true,
        message: format!(
            "Incremental update completed: types={}, modules={}, removed_modules={}",
            updated_types, reindexed_modules, removed_modules
        ),
    }
}

fn build_config_index_cache(
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

fn normalize_changed_paths(
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

fn build_child_objects_map(
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

fn resolve_object_xml_path(
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

fn parse_metadata_object(
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

fn collect_module_paths_for_metadata(
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

fn refresh_form_mappings(
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

fn remove_module_signatures(
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

fn apply_module_index_result(
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
fn apply_metadata_update(
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

fn remove_object(
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

fn resolve_object_key_for_form(cache: &ConfigIndexCache, form_xml: &Path) -> Option<ObjectKey> {
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
