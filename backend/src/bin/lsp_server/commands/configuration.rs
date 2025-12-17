//! Configuration parsing command handler
//!
//! MILESTONE 2.17: Handles bsl.parseConfiguration command.

use std::sync::Arc;
use tower_lsp::lsp_types::*;
use tower_lsp::Client;
use tracing::{debug, error, info, warn};

use bsl_backend::data::loaders::config_metadata_parser::ConfigurationDiscovery;
use bsl_backend::data::loaders::index_configuration_bsl_modules;
use bsl_shared::engine::AnalysisEngine;

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

    // Create progress token
    let token = ProgressToken::String(format!(
        "parse-config-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));

    // Create progress token
    if let Err(e) = client
        .send_request::<tower_lsp::lsp_types::request::WorkDoneProgressCreate>(
            WorkDoneProgressCreateParams {
                token: token.clone(),
            },
        )
        .await
    {
        error!("Failed to create work done progress token: {}", e);
    }

    // Send progress begin
    let _ = client
        .send_notification::<tower_lsp::lsp_types::notification::Progress>(ProgressParams {
            token: token.clone(),
            value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(WorkDoneProgressBegin {
                title: "Parsing configuration".to_string(),
                message: Some("Initializing...".to_string()),
                percentage: Some(0),
                cancellable: Some(false),
            })),
        })
        .await;

    // Get AnalysisEngine
    let engine = match analysis_engine {
        Some(e) => e,
        None => {
            error!("AnalysisEngine not available");
            return ParseConfigurationResponse {
                success: false,
                loaded_types: 0,
                message: Some("AnalysisEngine not available".to_string()),
            };
        }
    };

    let repo = engine.get_repository();

    // Progress: discovering metadata (10%)
    let _ = client
        .send_notification::<tower_lsp::lsp_types::notification::Progress>(ProgressParams {
            token: token.clone(),
            value: ProgressParamsValue::WorkDone(WorkDoneProgress::Report(
                WorkDoneProgressReport {
                    message: Some("Discovering metadata objects...".to_string()),
                    percentage: Some(10),
                    cancellable: Some(false),
                },
            )),
        })
        .await;

    let discovery = ConfigurationDiscovery::new(canonical_path.clone(), false);

    // Create progress callback
    let client_clone = client.clone();
    let token_clone = token.clone();
    let progress_callback = move |update: bsl_backend::data::loaders::progress::ProgressUpdate| {
        let client = client_clone.clone();
        let token = token_clone.clone();

        let percentage = update.percentage as u32;
        let message = update.message.unwrap_or_else(|| {
            format!(
                "{}: {}/{}",
                update.phase.display_name(),
                update.current,
                update.total
            )
        });

        tokio::spawn(async move {
            let _ = client
                .send_notification::<tower_lsp::lsp_types::notification::Progress>(ProgressParams {
                    token,
                    value: ProgressParamsValue::WorkDone(WorkDoneProgress::Report(
                        WorkDoneProgressReport {
                            message: Some(message),
                            percentage: Some(percentage),
                            cancellable: Some(false),
                        },
                    )),
                })
                .await;
        });
    };

    // Convert Result to avoid Send issues
    let discovery_result = discovery
        .discover_all_metadata(Some(progress_callback))
        .map_err(|e| e.to_string());

    let metadata = match discovery_result {
        Ok(data) => data,
        Err(error_msg) => {
            error!("Failed to discover metadata: {}", error_msg);

            let _ = client
                .send_notification::<tower_lsp::lsp_types::notification::Progress>(ProgressParams {
                    token: token.clone(),
                    value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(
                        WorkDoneProgressEnd {
                            message: Some(format!("Error: {}", error_msg)),
                        },
                    )),
                })
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

                let _ = client
                    .send_notification::<tower_lsp::lsp_types::notification::Progress>(
                        ProgressParams {
                            token: token.clone(),
                            value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(
                                WorkDoneProgressEnd {
                                    message: Some(format!("Error: {}", error_msg)),
                                },
                            )),
                        },
                    )
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
            let load_progress = ((loaded * 80) / total_objects) as u32;
            let total_progress = 10 + load_progress;

            let _ = client
                .send_notification::<tower_lsp::lsp_types::notification::Progress>(ProgressParams {
                    token: token.clone(),
                    value: ProgressParamsValue::WorkDone(WorkDoneProgress::Report(
                        WorkDoneProgressReport {
                            message: Some(format!("Loaded {}/{} types", loaded, total_objects)),
                            percentage: Some(total_progress),
                            cancellable: Some(false),
                        },
                    )),
                })
                .await;
        }
    }

    // Progress end (100%)
    let _ = client
        .send_notification::<tower_lsp::lsp_types::notification::Progress>(ProgressParams {
            token: token.clone(),
            value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd {
                message: Some(format!("Loaded {} types", loaded)),
            })),
        })
        .await;

    info!("Configuration parsed successfully: {} types loaded", loaded);

    // Индексация экспортных методов из модулей конфигурации (*.bsl)
    match index_configuration_bsl_modules(&canonical_path, &metadata) {
        Ok(indexed) => {
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
        }
        Err(e) => {
            warn!("Failed to index configuration module methods: {}", e);
        }
    }

    ParseConfigurationResponse {
        success: true,
        loaded_types: loaded,
        message: Some(format!(
            "Configuration loaded successfully: {} types",
            loaded
        )),
    }
}
