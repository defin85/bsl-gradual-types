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
use bsl_backend::system::{ConfigIndexCache, DomainBundle, ObjectKey, SystemCoordinator};
use bsl_shared::domain::repository::TypeRepository;

use crate::progress_bridge::{LspWorkDoneReporter, ProgressPlan, ProgressReporter};
use crate::types::{IncrementalUpdateParams, IncrementalUpdateResponse};

#[path = "configuration/helpers.rs"]
mod helpers;
#[path = "configuration/incremental_update.rs"]
mod incremental_update;

use self::helpers::*;
pub use self::incremental_update::handle_incremental_update;

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
    domain_bundle: Option<Arc<DomainBundle>>,
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

    let bundle = match domain_bundle {
        Some(bundle) => bundle,
        None => {
            error!("Domain bundle not available");
            reporter
                .lock()
                .await
                .end(Some("Error: Domain bundle not available".to_string()))
                .await;
            return ParseConfigurationResponse {
                success: false,
                loaded_types: 0,
                message: Some("Domain bundle not available".to_string()),
            };
        }
    };

    let repo = bundle.repository.clone();

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
        Ok(data) => Arc::new(data),
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
    //
    // Важно: индексация CPU-bound (rayon + парсинг) и может занимать секунды.
    // Если выполнять её на tokio worker thread, LSP "замирает" (completion/hover не отвечают).
    // Поэтому уводим работу в spawn_blocking.
    let canonical_path_for_index = canonical_path.clone();
    let metadata_for_index = metadata.clone();
    let modules_tx_clone = modules_tx.clone();
    let indexed = tokio::task::spawn_blocking(move || {
        index_configuration_bsl_modules_with_progress_parallel(
            &canonical_path_for_index,
            metadata_for_index.as_slice(),
            Some(move |p| {
                let _ = modules_tx_clone.send(p);
            }),
        )
    })
    .await;

    // Завершаем consumer для modules progress.
    drop(modules_tx);
    let _ = modules_task.await;

    match indexed {
        Ok(Ok(indexed)) => {
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
        Ok(Err(e)) => {
            warn!("Failed to index configuration module methods: {}", e);
            reporter
                .lock()
                .await
                .report(
                    index_modules_range.end,
                    Some(format!("Indexing skipped: {}", e)),
                )
                .await;
        }
        Err(e) => {
            warn!("Failed to index configuration module methods (join): {}", e);
            reporter
                .lock()
                .await
                .report(
                    index_modules_range.end,
                    Some(format!("Indexing skipped (join error): {}", e)),
                )
                .await;
        }
    }

    if let Some(coordinator) = coordinator {
        let cache =
            build_config_index_cache(&canonical_path, metadata.as_slice(), &module_signatures);
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
