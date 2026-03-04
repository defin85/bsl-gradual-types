use super::*;

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
            coordinator.get_domain_bundle(),
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
            coordinator.get_domain_bundle(),
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

    let bundle = match coordinator.get_domain_bundle() {
        Some(bundle) => bundle,
        None => {
            reporter
                .lock()
                .await
                .end(Some("Error: Domain bundle not available".to_string()))
                .await;
            return IncrementalUpdateResponse {
                success: false,
                message: "Domain bundle not available".to_string(),
            };
        }
    };

    let repository = bundle.repository.clone();
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

        // CPU-bound: парсинг и извлечение сигнатур. Выполняем в spawn_blocking, чтобы не подвешивать LSP.
        let modules_for_indexing = modules_for_indexing;
        let results = tokio::task::spawn_blocking(move || {
            index_configuration_bsl_modules_by_paths(
                &modules_for_indexing,
                &metadata_vec,
                None::<fn(ModuleIndexProgress)>,
            )
        })
        .await;

        match results {
            Ok(Ok(results)) => {
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
            Ok(Err(e)) => {
                warn!("Failed to reindex modules: {}", e);
            }
            Err(e) => {
                warn!("Failed to reindex modules (join): {}", e);
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
