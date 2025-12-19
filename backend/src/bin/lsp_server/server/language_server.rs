//! LanguageServer trait implementation for BslLanguageServer
//!
//! This module contains the complete implementation of the tower_lsp::LanguageServer trait.
//! All LSP protocol methods are implemented here:
//! - Lifecycle: initialize, initialized, shutdown
//! - Configuration: did_change_configuration
//! - File management: did_open, did_change, did_close
//! - Features: completion, hover, goto_definition, signature_help
//! - Commands: execute_command

use tokio::sync::mpsc;
use tower_lsp::jsonrpc::Result as JsonRpcResult;
use tower_lsp::lsp_types::*;
use tower_lsp::LanguageServer;
use tracing::{debug, error, info, warn};

use bsl_backend::data::loaders::progress::{IndexingPhase, ProgressUpdate};
use bsl_backend::system::fs_utils::read_bsl_file;
use bsl_shared::api::semantic_dtos::{GetSemanticHtmlRequest, GetSemanticTreeRequest};

use crate::commands::{
    handle_get_all_types, handle_get_semantic_html, handle_get_semantic_tree,
    handle_get_type_repository_stats, handle_parse_configuration, handle_query_type,
    handle_search_types, GetAllTypesRequest, ParseConfigurationParams, QueryTypeParams,
    SearchTypesRequest,
};
use crate::config::{BslSettings, LspConfig};
use crate::handlers::{
    self, apply_text_edit, handle_completion, handle_goto_definition, handle_hover,
    handle_signature_help,
};
use crate::progress::log_progress_to_file;
use crate::progress_bridge::{LspWorkDoneReporter, ProgressReporter};
use crate::types::{GetCurrentContextParams, ServerStatus, ServerStatusParams};

use super::BslLanguageServer;

#[tower_lsp::async_trait]
impl LanguageServer for BslLanguageServer {
    // ========================================================================
    // LIFECYCLE METHODS
    // ========================================================================

    async fn initialize(&self, params: InitializeParams) -> JsonRpcResult<InitializeResult> {
        info!("Initializing BSL Language Server");

        // DEBUG: Log ClientCapabilities
        debug!(
            "[JSON-RPC] initialize: ClientCapabilities.window.workDoneProgress = {:?}",
            params
                .capabilities
                .window
                .as_ref()
                .and_then(|w| w.work_done_progress)
        );

        // MILESTONE 2.10: Read initializationOptions from Extension
        if let Some(options) = params.initialization_options {
            match serde_json::from_value::<LspConfig>(options.clone()) {
                Ok(config) => {
                    info!("LSP Config received: {:?}", config);
                    *self.config.write().await = Some(config.clone());
                    info!("Configuration saved, will reload types in initialized()");
                }
                Err(e) => {
                    error!("Failed to parse LSP config: {}", e);
                    error!("Raw options: {:?}", options);
                }
            }
        } else {
            info!("No initializationOptions provided - using defaults (4 basic types only)");
        }

        // Version info for LSP Protocol
        let version = env!("CARGO_PKG_VERSION");
        let build_timestamp = env!("BUILD_TIMESTAMP");
        let git_hash = env!("GIT_HASH");

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::INCREMENTAL),
                        will_save: Some(false),
                        will_save_wait_until: Some(false),
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(false),
                        })),
                    },
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string(), " ".to_string()]),
                    ..Default::default()
                }),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: Some("bsl-gradual-types".to_string()),
                        inter_file_dependencies: true,
                        workspace_diagnostics: false,
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                    },
                )),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec![
                        "bsl.getAllTypes".to_string(),
                        "bsl.getSemanticHtml".to_string(),
                        "bsl.getSemanticTree".to_string(),
                        "bsl.searchTypes".to_string(),
                        "bsl.getCurrentContext".to_string(),
                        "bsl.getTypeRepositoryStats".to_string(),
                        "bsl.parseConfiguration".to_string(),
                    ],
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                    retrigger_characters: Some(vec![")".to_string()]),
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: Some(false),
                    },
                }),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "BSL Language Server".to_string(),
                version: Some(format!(
                    "{} (build: {}, git: {})",
                    version, build_timestamp, git_hash
                )),
            }),
        })
    }

    // TODO: Consider splitting initialized() into smaller functions in future refactoring
    // This method is 278 lines but handles complex async progress reporting that's hard to split
    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "BSL Language Server initialized!")
            .await;

        // MILESTONE 2.10: Reload types with config from initializationOptions
        let config = self.config.read().await;
        if let Some(ref cfg) = *config {
            if let Some(ref platform_docs) = cfg.platform_docs_archive {
                info!(
                    "Reloading types with platformDocsArchive: {}",
                    platform_docs
                );

                // Create channels for progress and result
                let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<ProgressUpdate>();
                let (result_tx, result_rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

                // Send bsl/serverStatus (loading: true)
                info!("[LSP->Extension] Sending bsl/serverStatus: loading=true");
                let _ = self
                    .client
                    .send_notification::<ServerStatus>(ServerStatusParams::loading(
                        "Loading types...",
                    ))
                    .await;

                // Send WorkDoneProgressBegin (единый progress bridge)
                let title = if cfg.configuration_path.is_some() {
                    "Loading platform and configuration types".to_string()
                } else {
                    "Loading platform types".to_string()
                };

                let mut reporter =
                    LspWorkDoneReporter::create(self.client.clone(), "bsl-load-types").await;
                reporter.set_throttle_interval(std::time::Duration::from_millis(150));
                reporter
                    .begin(title, Some("Initializing...".to_string()))
                    .await;

                log_progress_to_file("[LSP->Extension] SEND WorkDoneProgressBegin");

                // Spawn task to handle progress
                let client_clone = self.client.clone();
                let start_time = std::time::Instant::now();
                let self_clone = self.clone();

                tokio::spawn(async move {
                    let mut reporter = reporter;

                    // PHASE 1: Process progress updates
                    while let Some(update) = progress_rx.recv().await {
                        debug!(
                            "[RECV] {:?} {:.1}% ({}/{}) - {}",
                            update.phase,
                            update.percentage,
                            update.current,
                            update.total,
                            update.message.as_deref().unwrap_or("")
                        );

                        // Calculate ETA
                        let elapsed = start_time.elapsed().as_secs_f32();
                        let eta = if update.percentage > 5.0 {
                            Some(((elapsed * 100.0 / update.percentage) - elapsed) as u32)
                        } else {
                            None
                        };

                        // Format message
                        let message = match update.phase {
                            IndexingPhase::ParsingFiles => {
                                format!(
                                    "Type {}/{}{}",
                                    update.current,
                                    update.total,
                                    update
                                        .message
                                        .as_ref()
                                        .map(|m| format!(" - {}", m))
                                        .unwrap_or_default()
                                )
                            }
                            IndexingPhase::ConfigurationParsing => {
                                update.message.clone().unwrap_or_else(|| {
                                    format!(
                                        "{} | {}/{}",
                                        update.phase.display_name(),
                                        update.current,
                                        update.total
                                    )
                                })
                            }
                            _ => {
                                format!(
                                    "{} | {}/{}",
                                    update.phase.display_name(),
                                    update.current,
                                    update.total
                                )
                            }
                        };

                        let message_with_eta = if let Some(eta_secs) = eta {
                            format!("{} - ETA: {}s", message, eta_secs)
                        } else {
                            message
                        };

                        reporter
                            .report(update.percentage as u32, Some(message_with_eta))
                            .await;
                    }

                    // PHASE 2: Channel closed, wait for result
                    match result_rx.await {
                        Ok(Ok(())) => {
                            // SUCCESS: Send WorkDoneProgressEnd
                            reporter
                                .end(Some(
                                    "Platform types loaded successfully".to_string(),
                                ))
                                .await;

                            let _ = client_clone
                                .send_notification::<ServerStatus>(ServerStatusParams::ready())
                                .await;

                            // Revalidate all open documents
                            info!("Revalidating all open documents with full platform types...");
                            let documents_to_revalidate: Vec<_> = {
                                let docs = self_clone.documents.read().await;
                                docs.iter()
                                    .map(|(uri, text)| (uri.clone(), text.clone()))
                                    .collect()
                            };

                            for (uri, text) in documents_to_revalidate {
                                if let Err(e) = self_clone.revalidate_document(&uri, &text).await {
                                    warn!("Failed to revalidate {}: {}", uri, e);
                                }
                            }

                            // Clear IR cache
                            let ir_cache = self_clone.coordinator.ir_cache();
                            ir_cache.clear().await;
                        }
                        Ok(Err(error_msg)) => {
                            // ERROR: Send WorkDoneProgressEnd with error
                            reporter
                                .end(Some(format!("Error: {}", error_msg)))
                                .await;

                            let _ = client_clone
                                .send_notification::<ServerStatus>(ServerStatusParams::ready())
                                .await;
                        }
                        Err(_) => {
                            warn!("Result channel closed unexpectedly");
                        }
                    }
                });

                // Load types
                let syntax_path = std::path::Path::new(platform_docs);
                let config_path_ref = cfg
                    .configuration_path
                    .as_ref()
                    .map(|s| std::path::Path::new(s.as_str()));

                let result = self
                    .coordinator
                    .start_with_paths(Some(syntax_path), config_path_ref, Some(progress_tx))
                    .await;

                match result {
                    Ok(()) => {
                        info!("Platform types loaded successfully");
                        let _ = result_tx.send(Ok(()));
                        self.client
                            .log_message(
                                MessageType::INFO,
                                format!("Platform documentation loaded from: {}", platform_docs),
                            )
                            .await;
                    }
                    Err(e) => {
                        error!("Failed to load platform types: {}", e);
                        let _ = result_tx.send(Err(e.to_string()));
                        self.client
                            .log_message(
                                MessageType::ERROR,
                                format!("Failed to load platform documentation: {}", e),
                            )
                            .await;
                    }
                }
            } else {
                info!("platformDocsArchive not provided - using basic types only");
            }
        }
    }

    async fn shutdown(&self) -> JsonRpcResult<()> {
        info!("Shutting down BSL Language Server");
        Ok(())
    }

    // ========================================================================
    // CONFIGURATION
    // ========================================================================

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        info!("Received didChangeConfiguration");

        if let Some(settings_value) = params.settings.as_object() {
            if let Some(bsl_value) = settings_value.get("bsl") {
                match serde_json::from_value::<BslSettings>(bsl_value.clone()) {
                    Ok(new_settings) => {
                        info!(
                            "Parsed BslSettings: hover.detailLevel={}, diagnostics.detailLevel={}",
                            new_settings.hover.detail_level, new_settings.diagnostics.detail_level
                        );
                        *self.settings.write().await = new_settings;
                    }
                    Err(e) => {
                        warn!("Failed to parse BslSettings: {}", e);
                    }
                }
            }
        }
    }

    // ========================================================================
    // FILE MANAGEMENT
    // ========================================================================

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text.clone();
        let version = params.text_document.version;

        // Cache text
        self.documents
            .write()
            .await
            .insert(uri.clone(), text.clone());

        // Preheat IR cache
        if let Some(service) = self.get_type_service() {
            match service.get_hover_info(&text, 0, 0, None).await {
                Ok(_) => info!("IR cache preheated for {}", uri),
                Err(e) => error!("Failed to preheat IR cache for {}: {}", uri, e),
            }
        }

        // Get diagnostics
        let settings = self.settings.read().await.clone();
        let diagnostics =
            handlers::handle_did_open(&uri, &text, version, self.get_type_service(), &settings)
                .await;

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, Some(version))
            .await;

        self.client
            .log_message(
                MessageType::INFO,
                format!("Opened and analyzed document: {}", uri),
            )
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        let changes = params.content_changes;

        // Apply changes
        let updated_text = if let Some(full_change) = changes.iter().find(|c| c.range.is_none()) {
            full_change.text.clone()
        } else {
            let existing_text = self
                .documents
                .read()
                .await
                .get(&uri)
                .cloned()
                .unwrap_or_default();

            let mut current_text = existing_text;
            for change in &changes {
                if let Some(range) = change.range {
                    current_text = apply_text_edit(&current_text, range, &change.text);
                }
            }
            current_text
        };

        // Cache text
        self.documents
            .write()
            .await
            .insert(uri.clone(), updated_text.clone());

        // Get diagnostics
        let settings = self.settings.read().await.clone();
        let diagnostics = handlers::handle_did_change(
            &uri,
            &updated_text,
            &changes,
            self.get_type_service(),
            &settings,
        )
        .await;

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, Some(version))
            .await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.write().await.remove(&uri);

        // Clear diagnostics
        self.client
            .publish_diagnostics(uri.clone(), vec![], None)
            .await;

        self.client
            .log_message(MessageType::INFO, format!("Closed document: {}", uri))
            .await;
    }

    // ========================================================================
    // LSP FEATURES
    // ========================================================================

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> JsonRpcResult<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let file_content = match self.documents.read().await.get(&uri) {
            Some(content) => content.clone(),
            None => match uri.to_file_path() {
                Ok(path) => match read_bsl_file(&path) {
                    Ok(content) => content,
                    Err(e) => {
                        error!("Failed to read file for completion: {}", e);
                        return Ok(Some(CompletionResponse::Array(vec![])));
                    }
                },
                Err(_) => return Ok(Some(CompletionResponse::Array(vec![]))),
            },
        };

        Ok(handle_completion(&file_content, position, self.get_type_service()).await)
    }

    async fn hover(&self, params: HoverParams) -> JsonRpcResult<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        info!(
            "Hover requested at {}:{}",
            position.line, position.character
        );

        let file_content = match self.documents.read().await.get(&uri) {
            Some(content) => content.clone(),
            None => match uri.to_file_path() {
                Ok(path) => match read_bsl_file(&path) {
                    Ok(content) => content,
                    Err(e) => {
                        error!("Failed to read file for hover: {}", e);
                        return Ok(None);
                    }
                },
                Err(_) => return Ok(None),
            },
        };

        let settings = self.settings.read().await;
        Ok(handle_hover(
            &uri,
            &file_content,
            position,
            self.get_type_service(),
            &settings.hover,
        )
        .await)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> JsonRpcResult<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let file_content = match self.documents.read().await.get(&uri) {
            Some(content) => content.clone(),
            None => match uri.to_file_path() {
                Ok(path) => match read_bsl_file(&path) {
                    Ok(content) => content,
                    Err(e) => {
                        error!("Failed to read file for definition: {}", e);
                        return Ok(None);
                    }
                },
                Err(_) => return Ok(None),
            },
        };

        let file_path_string = uri
            .to_file_path()
            .ok()
            .map(|p| p.to_string_lossy().into_owned());

        Ok(handle_goto_definition(
            &file_content,
            file_path_string.as_deref(),
            position,
            self.get_type_service(),
        )
        .await)
    }

    async fn signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> JsonRpcResult<Option<SignatureHelp>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let file_content = match self.get_document_content(&uri).await {
            Ok(content) => content,
            Err(e) => {
                warn!("Failed to get document content: {}", e);
                return Ok(None);
            }
        };

        Ok(handle_signature_help(
            &file_content,
            position,
            self.coordinator.get_analysis_engine(),
        )
        .await)
    }

    // ========================================================================
    // COMMAND EXECUTION
    // ========================================================================

    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> JsonRpcResult<Option<serde_json::Value>> {
        info!(
            "Execute command: {} with {} arguments",
            params.command,
            params.arguments.len()
        );

        match params.command.as_str() {
            "bsl.getSemanticHtml" => {
                if params.arguments.is_empty() {
                    return Err(tower_lsp::jsonrpc::Error::invalid_params(
                        "Missing request parameters",
                    ));
                }

                let request: GetSemanticHtmlRequest =
                    serde_json::from_value(params.arguments[0].clone()).map_err(|e| {
                        tower_lsp::jsonrpc::Error::invalid_params(format!(
                            "Invalid parameters: {}",
                            e
                        ))
                    })?;

                let documents = self.documents.clone();
                let result = handle_get_semantic_html(request, self.get_type_service(), |uri| {
                    // This is a sync closure, so we need to use try_read
                    documents
                        .try_read()
                        .ok()
                        .and_then(|docs| docs.get(uri).cloned())
                })
                .await
                .map_err(|_| tower_lsp::jsonrpc::Error::internal_error())?;

                Ok(Some(serde_json::to_value(result).unwrap()))
            }
            "bsl.getSemanticTree" => {
                if params.arguments.is_empty() {
                    return Err(tower_lsp::jsonrpc::Error::invalid_params(
                        "Missing request parameters",
                    ));
                }

                let request: GetSemanticTreeRequest =
                    serde_json::from_value(params.arguments[0].clone()).map_err(|e| {
                        tower_lsp::jsonrpc::Error::invalid_params(format!(
                            "Invalid parameters: {}",
                            e
                        ))
                    })?;

                let documents = self.documents.clone();
                let result = handle_get_semantic_tree(request, self.get_type_service(), |uri| {
                    documents
                        .try_read()
                        .ok()
                        .and_then(|docs| docs.get(uri).cloned())
                })
                .await
                .map_err(|_| tower_lsp::jsonrpc::Error::internal_error())?;

                Ok(Some(serde_json::to_value(result).unwrap()))
            }
            "bsl.searchTypes" => {
                if params.arguments.is_empty() {
                    return Err(tower_lsp::jsonrpc::Error::invalid_params(
                        "Missing search query",
                    ));
                }

                let request: SearchTypesRequest =
                    serde_json::from_value(params.arguments[0].clone()).map_err(|e| {
                        tower_lsp::jsonrpc::Error::invalid_params(format!(
                            "Invalid parameters: {}",
                            e
                        ))
                    })?;

                let result = handle_search_types(request, self.coordinator.get_analysis_engine());
                Ok(Some(serde_json::to_value(result).unwrap()))
            }
            "bsl.getAllTypes" => {
                // Parameters are optional - use defaults if not provided
                let request: GetAllTypesRequest = if params.arguments.is_empty() {
                    GetAllTypesRequest {
                        limit: 1000,
                        offset: 0,
                        category: None,
                    }
                } else {
                    serde_json::from_value(params.arguments[0].clone()).map_err(|e| {
                        tower_lsp::jsonrpc::Error::invalid_params(format!(
                            "Invalid parameters: {}",
                            e
                        ))
                    })?
                };

                let result = handle_get_all_types(request, self.coordinator.type_service());
                Ok(Some(serde_json::to_value(result).unwrap()))
            }
            "bsl.getCurrentContext" => {
                if params.arguments.is_empty() {
                    return Err(tower_lsp::jsonrpc::Error::invalid_params(
                        "Missing parameters",
                    ));
                }

                let request: GetCurrentContextParams =
                    serde_json::from_value(params.arguments[0].clone()).map_err(|e| {
                        tower_lsp::jsonrpc::Error::invalid_params(format!(
                            "Invalid parameters: {}",
                            e
                        ))
                    })?;

                let result = self.handle_get_current_context(request).await?;
                Ok(Some(serde_json::to_value(result).unwrap()))
            }
            "bsl.queryType" => {
                if params.arguments.is_empty() {
                    return Err(tower_lsp::jsonrpc::Error::invalid_params(
                        "Missing type name",
                    ));
                }

                let request: QueryTypeParams = serde_json::from_value(params.arguments[0].clone())
                    .map_err(|e| {
                        tower_lsp::jsonrpc::Error::invalid_params(format!(
                            "Invalid parameters: {}",
                            e
                        ))
                    })?;

                let result = handle_query_type(request, self.coordinator.get_analysis_engine());
                Ok(Some(serde_json::to_value(result).unwrap()))
            }
            "bsl.getTypeRepositoryStats" => {
                let result = handle_get_type_repository_stats(self.coordinator.clone());
                Ok(Some(serde_json::to_value(result).unwrap()))
            }
            "bsl.parseConfiguration" => {
                if params.arguments.is_empty() {
                    return Err(tower_lsp::jsonrpc::Error::invalid_params(
                        "Missing configuration path",
                    ));
                }

                let request: ParseConfigurationParams =
                    serde_json::from_value(params.arguments[0].clone()).map_err(|e| {
                        tower_lsp::jsonrpc::Error::invalid_params(format!(
                            "Invalid parameters: {}",
                            e
                        ))
                    })?;

                let result = handle_parse_configuration(
                    request,
                    self.coordinator.get_analysis_engine(),
                    self.client.clone(),
                    "parse-config",
                    "Parsing configuration",
                )
                .await;
                Ok(Some(serde_json::to_value(result).unwrap()))
            }
            _ => {
                warn!("Unknown command: {}", params.command);
                Err(tower_lsp::jsonrpc::Error::method_not_found())
            }
        }
    }
}
