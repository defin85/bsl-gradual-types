//! LanguageServer trait implementation for BslLanguageServer
//!
//! This module contains the complete implementation of the tower_lsp::LanguageServer trait.
//! All LSP protocol methods are implemented here:
//! - Lifecycle: initialize, initialized, shutdown
//! - Configuration: did_change_configuration
//! - File management: did_open, did_change, did_close
//! - Features: completion, hover, goto_definition, signature_help
//! - Commands: execute_command

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tower_lsp::jsonrpc::Result as JsonRpcResult;
use tower_lsp::lsp_types::*;
use tower_lsp::LanguageServer;
use tracing::{debug, error, info, warn};

use bsl_backend::data::loaders::progress::{IndexingPhase, ProgressUpdate};
use bsl_backend::system::fs_utils::read_bsl_file;
use bsl_backend::system::{startup_v2, StartupInputs};
use bsl_shared::api::semantic_dtos::{GetSemanticHtmlRequest, GetSemanticTreeRequest};

use crate::commands::{
    handle_cache_clear, handle_cache_set_enabled, handle_cache_stats, handle_get_all_types,
    handle_get_type_repository_stats, handle_parse_configuration, handle_query_type,
    handle_search_types, semantic_html_from_tree, semantic_tree_from_ir, CacheCommandParams,
    CacheToggleParams, GetAllTypesRequest, ParseConfigurationParams, QueryTypeParams,
    SearchTypesRequest,
};
use crate::config::{BslSettings, LspConfig};
use crate::handlers::{
    apply_text_edit, build_document_symbols, build_workspace_symbols, format_bsl_range_to_edits,
    format_bsl_to_edits, handle_code_actions_v2, handle_completion_resolve,
    handle_goto_definition_v2, handle_hover_v2, handle_inlay_hints_v2, handle_prepare_rename,
    handle_references, handle_rename, handle_signature_help_v2, RenameError,
};
use crate::progress::log_progress_to_file;
use crate::progress_bridge::{LspWorkDoneReporter, ProgressReporter};
use crate::types::{GetCurrentContextParams, ServerStatus, ServerStatusParams};

use super::BslLanguageServer;

fn effective_include_flow_sensitive(
    request_override: Option<bool>,
    enable_flow_sensitive_setting: bool,
) -> bool {
    request_override.unwrap_or(enable_flow_sensitive_setting)
}

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
                    info!(
                        "Feature flags: enableTypeHints={:?}, enableCodeActions={:?}",
                        config.enable_type_hints, config.enable_code_actions
                    );
                    if let Some(cache_enabled) = config.cache_enabled {
                        let result = self.coordinator.set_cache_enabled(cache_enabled).await;
                        info!(
                            "Cache enabled updated: requested={}, effective={}, env_disabled={}",
                            result.requested, result.effective, result.env_disabled
                        );
                    }
                    if let Some(strict_fingerprint) = config.strict_fingerprint {
                        self.coordinator.set_strict_fingerprint(strict_fingerprint);
                        info!("Strict fingerprint updated: {}", strict_fingerprint);
                    }
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

        let snippet_support = params
            .capabilities
            .text_document
            .as_ref()
            .and_then(|td| td.completion.as_ref())
            .and_then(|completion| completion.completion_item.as_ref())
            .and_then(|item| item.snippet_support)
            .unwrap_or(false);
        *self.completion_snippet_support.write().await = snippet_support;
        info!("Client snippet support: {}", snippet_support);

        let dynamic_document_formatting = params
            .capabilities
            .text_document
            .as_ref()
            .and_then(|td| td.formatting.as_ref())
            .and_then(|cap| cap.dynamic_registration)
            .unwrap_or(false);
        let dynamic_range_formatting = params
            .capabilities
            .text_document
            .as_ref()
            .and_then(|td| td.range_formatting.as_ref())
            .and_then(|cap| cap.dynamic_registration)
            .unwrap_or(false);

        let dynamic_inlay_hints = params
            .capabilities
            .text_document
            .as_ref()
            .and_then(|td| td.inlay_hint.as_ref())
            .and_then(|cap| cap.dynamic_registration)
            .unwrap_or(false);

        let dynamic_code_actions = params
            .capabilities
            .text_document
            .as_ref()
            .and_then(|td| td.code_action.as_ref())
            .and_then(|cap| cap.dynamic_registration)
            .unwrap_or(false);

        {
            let mut state = self.formatting_capability.write().await;
            state.dynamic_document_formatting = dynamic_document_formatting;
            state.dynamic_range_formatting = dynamic_range_formatting;
        }
        {
            let mut state = self.inlay_hints_capability.write().await;
            state.dynamic_registration = dynamic_inlay_hints;
        }
        {
            let mut state = self.code_actions_capability.write().await;
            state.dynamic_registration = dynamic_code_actions;
        }
        info!(
            "Client dynamicRegistration: formatting={}, rangeFormatting={}",
            dynamic_document_formatting, dynamic_range_formatting
        );
        info!(
            "Client dynamicRegistration: inlayHints={}, codeActions={}",
            dynamic_inlay_hints, dynamic_code_actions
        );

        // Version info for LSP Protocol
        let version = env!("CARGO_PKG_VERSION");
        let build_timestamp = env!("BUILD_TIMESTAMP");
        let git_hash = env!("GIT_HASH");

        let (enable_type_hints, enable_code_actions) = {
            let cfg = self.config.read().await;
            let enable_type_hints = cfg
                .as_ref()
                .and_then(|cfg| cfg.enable_type_hints)
                .unwrap_or(false);
            let enable_code_actions = cfg
                .as_ref()
                .and_then(|cfg| cfg.enable_code_actions)
                .unwrap_or(false);
            (enable_type_hints, enable_code_actions)
        };

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
                    resolve_provider: Some(true),
                    trigger_characters: Some(vec![".".to_string(), "(".to_string()]),
                    ..Default::default()
                }),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                })),
                diagnostic_provider: None,
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec![
                        "bsl.getAllTypes".to_string(),
                        "bsl.getSemanticHtml".to_string(),
                        "bsl.getSemanticTree".to_string(),
                        "bsl.searchTypes".to_string(),
                        "bsl.getCurrentContext".to_string(),
                        "bsl.getTypeRepositoryStats".to_string(),
                        "bsl.getWorkspaceStats".to_string(),
                        "bsl.getObservabilityMetrics".to_string(),
                        "bsl.getRuntimeConfig".to_string(),
                        "bsl.parseConfiguration".to_string(),
                        "bsl.cache.getStats".to_string(),
                        "bsl.cache.clear".to_string(),
                        "bsl.cache.setEnabled".to_string(),
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
                // Formatting is registered dynamically based on workspace settings.
                // This prevents VSCode formatOnSave from calling formatting when it's disabled.
                document_formatting_provider: None,
                document_range_formatting_provider: None,
                inlay_hint_provider: if dynamic_inlay_hints {
                    None
                } else {
                    enable_type_hints.then_some(OneOf::Left(true))
                },
                code_action_provider: if dynamic_code_actions {
                    None
                } else {
                    enable_code_actions.then_some(CodeActionProviderCapability::Simple(true))
                },
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
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

        self.sync_formatting_capability_registration().await;
        self.sync_inlay_hints_capability_registration().await;
        self.sync_code_actions_capability_registration().await;

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
                            _ => update.message.clone().unwrap_or_else(|| {
                                format!(
                                    "{} | {}/{}",
                                    update.phase.display_name(),
                                    update.current,
                                    update.total
                                )
                            }),
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
                                .end(Some("Platform types loaded successfully".to_string()))
                                .await;

                            let _ = client_clone
                                .send_notification::<ServerStatus>(ServerStatusParams::ready())
                                .await;

                            // Reschedule diagnostics for open documents so they are recomputed
                            // against the latest deps snapshot.
                            info!("Rescheduling v2 diagnostics for open documents after deps update...");
                            let open_versions: Vec<(bsl_analysis_v2::FileId, i32)> = {
                                self_clone
                                    .latest_received_file_versions_v2
                                    .read()
                                    .await
                                    .iter()
                                    .map(|(file_id, version)| (*file_id, *version))
                                    .collect()
                            };
                            let keys = self_clone.file_key_to_file_id_v2.read().await.clone();

                            for (file_id, version) in open_versions {
                                let uri = keys.iter().find_map(|(key, mapped)| {
                                    if *mapped != file_id {
                                        return None;
                                    }
                                    match key {
                                        super::V2FileKey::Path(path) => {
                                            Url::from_file_path(path).ok()
                                        }
                                        super::V2FileKey::Url(raw) => Url::parse(raw).ok(),
                                    }
                                });

                                if let Some(uri) = uri {
                                    self_clone
                                        .schedule_diagnostics_v2(uri, file_id, version, true)
                                        .await;
                                }
                            }
                        }
                        Ok(Err(error_msg)) => {
                            // ERROR: Send WorkDoneProgressEnd with error
                            reporter.end(Some(format!("Error: {}", error_msg))).await;

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
                let inputs = StartupInputs::from_lsp_settings(
                    Some(platform_docs),
                    cfg.configuration_path.as_deref(),
                    cfg.platform_version.as_deref(),
                    cfg.cache_enabled,
                    cfg.strict_fingerprint,
                );

                let result = startup_v2(self.coordinator.clone(), inputs, Some(progress_tx)).await;

                match result {
                    Ok(startup) => {
                        info!("Platform types loaded successfully");
                        self.apply_deps_bundle_v2("start_with_paths", startup.deps_bundle_v2)
                            .await;
                        self.sync_v2_globals().await;
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
            if let Some(bsl_analyzer_value) = settings_value.get("bslAnalyzer") {
                match serde_json::from_value::<LspConfig>(bsl_analyzer_value.clone()) {
                    Ok(mut new_config) => {
                        normalize_lsp_config(&mut new_config);
                        let mut guard = self.config.write().await;
                        let mut merged = guard.clone().unwrap_or(LspConfig {
                            platform_docs_archive: None,
                            configuration_path: None,
                            platform_version: None,
                            cache_enabled: None,
                            strict_fingerprint: None,
                            enable_type_hints: None,
                            enable_code_actions: None,
                        });
                        if new_config.platform_docs_archive.is_some() {
                            merged.platform_docs_archive = new_config.platform_docs_archive;
                        }
                        if new_config.configuration_path.is_some() {
                            merged.configuration_path = new_config.configuration_path;
                        }
                        if new_config.platform_version.is_some() {
                            merged.platform_version = new_config.platform_version;
                        }
                        if new_config.cache_enabled.is_some() {
                            merged.cache_enabled = new_config.cache_enabled;
                        }
                        if new_config.strict_fingerprint.is_some() {
                            merged.strict_fingerprint = new_config.strict_fingerprint;
                        }
                        if new_config.enable_type_hints.is_some() {
                            merged.enable_type_hints = new_config.enable_type_hints;
                        }
                        if new_config.enable_code_actions.is_some() {
                            merged.enable_code_actions = new_config.enable_code_actions;
                        }
                        *guard = Some(merged.clone());
                        if let Some(cache_enabled) = merged.cache_enabled {
                            let result = self.coordinator.set_cache_enabled(cache_enabled).await;
                            info!(
                                "Cache enabled updated via settings: requested={}, effective={}, env_disabled={}",
                                result.requested, result.effective, result.env_disabled
                            );
                        }
                        if let Some(strict_fingerprint) = merged.strict_fingerprint {
                            self.coordinator.set_strict_fingerprint(strict_fingerprint);
                            info!(
                                "Strict fingerprint updated via settings: {}",
                                strict_fingerprint
                            );
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse BslAnalyzer settings: {}", e);
                    }
                }
            }
            if let Some(bsl_value) = settings_value.get("bsl") {
                match serde_json::from_value::<BslSettings>(bsl_value.clone()) {
                    Ok(new_settings) => {
                        info!(
                            "Parsed BslSettings: hover.detailLevel={}, diagnostics.detailLevel={}, formatting.enabled={}, formatting.indentSize={}, typeHints.enabled={}, codeActions.enabled={}, enableFlowSensitive={}",
                            new_settings.hover.detail_level,
                            new_settings.diagnostics.detail_level,
                            new_settings.formatting.enabled,
                            new_settings.formatting.indent_size,
                            new_settings.type_hints.enabled,
                            new_settings.code_actions.enabled,
                            new_settings.enable_flow_sensitive,
                        );

                        // Apply runtime `BSL_*` overrides (stable + dev-only) without restarting the server.
                        // Stable overrides are always accepted; dev-only overrides require explicit opt-in.
                        {
                            let store = bsl_runtime::system::global_runtime_config();
                            let stable_report =
                                store.replace_stable_overrides(&new_settings.env_overrides);
                            if !stable_report.ignored_unknown_keys.is_empty()
                                || !stable_report.ignored_invalid_values.is_empty()
                                || !stable_report.ignored_wrong_tier_keys.is_empty()
                            {
                                warn!(
                                    "RuntimeConfig stable overrides: unknown={:?}, invalid={:?}, wrong_tier={:?}",
                                    stable_report.ignored_unknown_keys,
                                    stable_report.ignored_invalid_values,
                                    stable_report.ignored_wrong_tier_keys,
                                );
                            }

                            let dev_report = store.replace_dev_overrides(
                                &new_settings.dev_env_overrides,
                                new_settings.enable_dev_env_overrides(),
                            );
                            if dev_report.dev_overrides_ignored {
                                warn!(
                                    "RuntimeConfig dev-only overrides ignored (set bsl.allowDevOverrides=true or legacy bsl.dev.enableDevEnvOverrides=true to apply)."
                                );
                            } else if !dev_report.ignored_unknown_keys.is_empty()
                                || !dev_report.ignored_invalid_values.is_empty()
                                || !dev_report.ignored_wrong_tier_keys.is_empty()
                            {
                                warn!(
                                    "RuntimeConfig dev overrides: unknown={:?}, invalid={:?}, wrong_tier={:?}",
                                    dev_report.ignored_unknown_keys,
                                    dev_report.ignored_invalid_values,
                                    dev_report.ignored_wrong_tier_keys,
                                );
                            }
                        }

                        *self.settings.write().await = new_settings;

                        // Keep feature gates (initializationOptions.*) aligned with runtime settings to
                        // avoid "enabled in settings but server refuses" situations.
                        {
                            let mut guard = self.config.write().await;
                            let mut merged = guard.clone().unwrap_or(LspConfig {
                                platform_docs_archive: None,
                                configuration_path: None,
                                platform_version: None,
                                cache_enabled: None,
                                strict_fingerprint: None,
                                enable_type_hints: None,
                                enable_code_actions: None,
                            });
                            let settings = self.settings.read().await;
                            merged.enable_type_hints = Some(settings.type_hints.enabled);
                            merged.enable_code_actions = Some(settings.code_actions.enabled);
                            *guard = Some(merged);
                        }

                        self.sync_formatting_capability_registration().await;
                        self.sync_inlay_hints_capability_registration().await;
                        self.sync_code_actions_capability_registration().await;

                        // Re-sync cache/strict-fingerprint toggles via coordinator to reflect runtime-config
                        // changes (e.g., `BSL_CACHE_DISABLE`, `BSL_CACHE_STRICT_FINGERPRINT`) without restart.
                        {
                            let cache_disable = bsl_runtime::system::global_runtime_config()
                                .get_bool(bsl_runtime::system::RuntimeKey::CacheDisable)
                                .unwrap_or(false);
                            let requested_cache_enabled = self
                                .config
                                .read()
                                .await
                                .as_ref()
                                .and_then(|cfg| cfg.cache_enabled)
                                .unwrap_or(true);
                            let _ = self
                                .coordinator
                                .set_cache_enabled(requested_cache_enabled && !cache_disable)
                                .await;

                            let strict = bsl_runtime::system::global_runtime_config()
                                .get_bool(bsl_runtime::system::RuntimeKey::CacheStrictFingerprint)
                                .unwrap_or(false);
                            self.coordinator.set_strict_fingerprint(strict);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse BslSettings: {}", e);
                    }
                }
            }
        }

        self.sync_v2_globals().await;
    }

    // ========================================================================
    // FILE MANAGEMENT
    // ========================================================================

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text.clone();
        let version = params.text_document.version;

        let _sync_guard = self.text_sync_v2.lock().await;

        self.sync_v2_globals().await;
        let file_id = self.get_or_create_file_id_v2(&uri).await;
        let path = match uri.to_file_path() {
            Ok(path) => path.to_string_lossy().to_string(),
            Err(_) => uri.to_string(),
        };

        self.latest_received_file_versions_v2
            .write()
            .await
            .insert(file_id, version);

        self.analysis_v2
            .apply_changes(vec![bsl_analysis_v2::Change::SetFile {
                file_id,
                text: Arc::from(text.clone()),
                version,
                path: Arc::from(path),
            }]);

        self.schedule_diagnostics_v2(uri.clone(), file_id, version, false)
            .await;

        self.client
            .log_message(
                MessageType::INFO,
                format!("Opened document (v2 diagnostics scheduled): {}", uri),
            )
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        let changes = params.content_changes;

        let _sync_guard = self.text_sync_v2.lock().await;

        self.sync_v2_globals().await;
        let file_id = self.get_or_create_file_id_v2(&uri).await;
        let path = match uri.to_file_path() {
            Ok(path) => path.to_string_lossy().to_string(),
            Err(_) => uri.to_string(),
        };

        let prev_version = self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied();
        if let Some(prev_version) = prev_version {
            let _ = self
                .analysis_v2
                .wait_for_file_version(file_id, prev_version)
                .await;
        }

        // Apply changes
        let updated_text = if let Some(full_change) = changes.iter().find(|c| c.range.is_none()) {
            full_change.text.clone()
        } else {
            let base_text = self
                .analysis_v2
                .snapshot()
                .await
                .file_text(file_id)
                .ok()
                .flatten()
                .map(|text| text.to_string())
                .unwrap_or_default();

            let mut current_text = base_text;
            for change in &changes {
                if let Some(range) = change.range {
                    current_text = apply_text_edit(&current_text, range, &change.text);
                }
            }
            current_text
        };

        self.latest_received_file_versions_v2
            .write()
            .await
            .insert(file_id, version);

        self.analysis_v2
            .apply_changes(vec![bsl_analysis_v2::Change::SetFile {
                file_id,
                text: Arc::from(updated_text.clone()),
                version,
                path: Arc::from(path),
            }]);

        self.schedule_diagnostics_v2(uri.clone(), file_id, version, true)
            .await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;

        let _sync_guard = self.text_sync_v2.lock().await;

        if let Some(file_id) = self.get_file_id_v2(&uri).await {
            self.cancel_diagnostics_v2(file_id).await;
            self.latest_received_file_versions_v2
                .write()
                .await
                .remove(&file_id);
            self.analysis_v2
                .apply_changes(vec![bsl_analysis_v2::Change::RemoveFile { file_id }]);
        }

        // Clear diagnostics
        self.client
            .publish_diagnostics(uri.clone(), vec![], None)
            .await;
        self.update_diagnostics_count(&uri, 0).await;

        self.client
            .log_message(MessageType::INFO, format!("Closed document: {}", uri))
            .await;
    }

    // ========================================================================
    // LSP FEATURES
    // ========================================================================

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> JsonRpcResult<Option<Vec<TextEdit>>> {
        let settings = self.settings.read().await.clone();
        if !settings.formatting.enabled {
            return Ok(None);
        }

        self.sync_v2_globals().await;
        let uri = params.text_document.uri;
        let file_id = self.get_or_create_file_id_v2(&uri).await;

        let expected_version = self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied();

        if let Some(expected_version) = expected_version {
            let ok = self
                .analysis_v2
                .wait_for_file_version(file_id, expected_version)
                .await;
            if !ok {
                return Ok(None);
            }
        }

        let analysis = self.analysis_v2.snapshot().await;
        let Some(file_content) = analysis.file_text(file_id).ok().flatten() else {
            return Ok(None);
        };

        let edits = format_bsl_to_edits(&file_content, settings.formatting.indent_size)
            .map_err(|_| tower_lsp::jsonrpc::Error::internal_error())?;
        Ok(edits)
    }

    async fn range_formatting(
        &self,
        params: DocumentRangeFormattingParams,
    ) -> JsonRpcResult<Option<Vec<TextEdit>>> {
        let settings = self.settings.read().await.clone();
        if !settings.formatting.enabled {
            return Ok(None);
        }

        self.sync_v2_globals().await;
        let uri = params.text_document.uri;
        let file_id = self.get_or_create_file_id_v2(&uri).await;

        let expected_version = self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied();

        if let Some(expected_version) = expected_version {
            let ok = self
                .analysis_v2
                .wait_for_file_version(file_id, expected_version)
                .await;
            if !ok {
                return Ok(None);
            }
        }

        let analysis = self.analysis_v2.snapshot().await;
        let Some(file_content) = analysis.file_text(file_id).ok().flatten() else {
            return Ok(None);
        };

        let edits =
            format_bsl_range_to_edits(&file_content, settings.formatting.indent_size, params.range)
                .map_err(|_| tower_lsp::jsonrpc::Error::internal_error())?;
        Ok(edits)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> JsonRpcResult<Option<DocumentSymbolResponse>> {
        self.sync_v2_globals().await;

        let uri = params.text_document.uri;
        let Some(file_id) = self.get_file_id_v2(&uri).await else {
            return Ok(None);
        };

        let expected_version = self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied();
        if let Some(expected_version) = expected_version {
            let ok = self
                .analysis_v2
                .wait_for_file_version(file_id, expected_version)
                .await;
            if !ok {
                return Ok(None);
            }
        }

        let analysis = self.analysis_v2.snapshot().await;
        let Some(file_content) = analysis.file_text(file_id).ok().flatten() else {
            return Ok(None);
        };
        let Some(parse_result) = analysis.parse_result(file_id).ok().flatten() else {
            return Ok(None);
        };

        let response = build_document_symbols(&uri, &file_content, &parse_result)
            .map_err(|_| tower_lsp::jsonrpc::Error::internal_error())?;
        Ok(Some(response))
    }

    async fn references(&self, params: ReferenceParams) -> JsonRpcResult<Option<Vec<Location>>> {
        self.sync_v2_globals().await;

        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let include_declaration = params.context.include_declaration;

        let Some(file_id) = self.get_file_id_v2(&uri).await else {
            return Ok(None);
        };

        let expected_version = self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied();
        if let Some(expected_version) = expected_version {
            let ok = self
                .analysis_v2
                .wait_for_file_version(file_id, expected_version)
                .await;
            if !ok {
                return Ok(None);
            }
        }

        let analysis = self.analysis_v2.snapshot().await;
        let Some(file_content) = analysis.file_text(file_id).ok().flatten() else {
            return Ok(None);
        };
        let Some(parse_result) = analysis.parse_result(file_id).ok().flatten() else {
            return Ok(None);
        };

        Ok(handle_references(
            &file_content,
            &parse_result,
            &uri,
            position,
            include_declaration,
        ))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> JsonRpcResult<Option<PrepareRenameResponse>> {
        self.sync_v2_globals().await;

        let uri = params.text_document.uri.clone();
        let Some(file_id) = self.get_file_id_v2(&uri).await else {
            return Ok(None);
        };

        let expected_version = self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied();
        if let Some(expected_version) = expected_version {
            let ok = self
                .analysis_v2
                .wait_for_file_version(file_id, expected_version)
                .await;
            if !ok {
                return Ok(None);
            }
        }

        let analysis = self.analysis_v2.snapshot().await;
        let Some(file_content) = analysis.file_text(file_id).ok().flatten() else {
            return Ok(None);
        };
        let Some(parse_result) = analysis.parse_result(file_id).ok().flatten() else {
            return Ok(None);
        };

        Ok(handle_prepare_rename(&file_content, &parse_result, params))
    }

    async fn rename(&self, params: RenameParams) -> JsonRpcResult<Option<WorkspaceEdit>> {
        self.sync_v2_globals().await;

        let uri = params.text_document_position.text_document.uri.clone();
        let Some(file_id) = self.get_file_id_v2(&uri).await else {
            return Ok(None);
        };

        let expected_version = self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied();
        if let Some(expected_version) = expected_version {
            let ok = self
                .analysis_v2
                .wait_for_file_version(file_id, expected_version)
                .await;
            if !ok {
                return Ok(None);
            }
        }

        let analysis = self.analysis_v2.snapshot().await;
        let Some(file_content) = analysis.file_text(file_id).ok().flatten() else {
            return Ok(None);
        };
        let Some(parse_result) = analysis.parse_result(file_id).ok().flatten() else {
            return Ok(None);
        };

        match handle_rename(&file_content, &parse_result, params) {
            Ok(edit) => Ok(Some(edit)),
            Err(RenameError::InvalidNewName) => Err(tower_lsp::jsonrpc::Error::invalid_params(
                "Invalid new name",
            )),
            Err(RenameError::Unsupported) => Err(tower_lsp::jsonrpc::Error::invalid_params(
                "Rename is not supported for this symbol",
            )),
        }
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> JsonRpcResult<Option<Vec<SymbolInformation>>> {
        let query = params.query;
        if query.trim().is_empty() {
            return Ok(Some(Vec::new()));
        }

        self.sync_v2_globals().await;

        let open_versions: Vec<(bsl_analysis_v2::FileId, i32)> = self
            .latest_received_file_versions_v2
            .read()
            .await
            .iter()
            .map(|(file_id, version)| (*file_id, *version))
            .collect();

        if open_versions.is_empty() {
            return Ok(Some(Vec::new()));
        }

        let mut ready_file_ids = Vec::<bsl_analysis_v2::FileId>::new();
        for (file_id, expected_version) in &open_versions {
            let ok = self
                .analysis_v2
                .wait_for_file_version(*file_id, *expected_version)
                .await;
            if !ok {
                continue;
            }
            ready_file_ids.push(*file_id);
        }

        if ready_file_ids.is_empty() {
            return Ok(Some(Vec::new()));
        }

        let keys = self.file_key_to_file_id_v2.read().await.clone();
        let mut file_id_to_uri: std::collections::HashMap<bsl_analysis_v2::FileId, Url> =
            std::collections::HashMap::new();
        for (key, file_id) in keys {
            let uri = match key {
                super::V2FileKey::Path(path) => Url::from_file_path(path).ok(),
                super::V2FileKey::Url(raw) => Url::parse(&raw).ok(),
            };
            if let Some(uri) = uri {
                file_id_to_uri.insert(file_id, uri);
            }
        }

        let analysis = self.analysis_v2.snapshot().await;
        let mut out: Vec<SymbolInformation> = Vec::new();
        for file_id in ready_file_ids {
            let Some(uri) = file_id_to_uri.get(&file_id) else {
                continue;
            };
            let Some(file_content) = analysis.file_text(file_id).ok().flatten() else {
                continue;
            };
            let Some(parse_result) = analysis.parse_result(file_id).ok().flatten() else {
                continue;
            };
            out.extend(build_workspace_symbols(
                &query,
                uri,
                &file_content,
                &parse_result,
            ));
        }

        out.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then_with(|| a.location.uri.as_str().cmp(b.location.uri.as_str()))
                .then_with(|| {
                    a.location
                        .range
                        .start
                        .line
                        .cmp(&b.location.range.start.line)
                })
                .then_with(|| {
                    a.location
                        .range
                        .start
                        .character
                        .cmp(&b.location.range.start.character)
                })
        });

        const WORKSPACE_SYMBOL_LIMIT: usize = 200;
        if out.len() > WORKSPACE_SYMBOL_LIMIT {
            out.truncate(WORKSPACE_SYMBOL_LIMIT);
        }

        Ok(Some(out))
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> JsonRpcResult<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let started = Instant::now();
        let snippet_support = *self.completion_snippet_support.read().await;
        let mut completion_outcome: Option<&'static str> = None;
        let completion = {
            let file_id = self.get_or_create_file_id_v2(&uri).await;
            let first_completion_for_file = {
                let mut seen = self.completion_seen_files_v2.write().await;
                seen.insert(file_id)
            };
            self.coordinator
                .record_intellisense_v2_completion_temperature(if first_completion_for_file {
                    "first"
                } else {
                    "warm"
                });
            self.sync_v2_globals().await;

            let expected_version = self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied();

            let empty = || {
                Some(crate::handlers::CompletionResponseWithStats {
                    response: CompletionResponse::List(CompletionList {
                        is_incomplete: false,
                        items: vec![],
                    }),
                    stats: None,
                    had_error: false,
                })
            };

            let wait_started = Instant::now();
            let ready = if let Some(expected_version) = expected_version {
                self.analysis_v2
                    .wait_for_file_version(file_id, expected_version)
                    .await
            } else {
                let path = match uri.to_file_path() {
                    Ok(path) => path.to_string_lossy().to_string(),
                    Err(_) => uri.to_string(),
                };
                let file_content = match uri.to_file_path() {
                    Ok(path) => match read_bsl_file(&path) {
                        Ok(content) => Some(content),
                        Err(e) => {
                            error!("Failed to read file for completion: {}", e);
                            None
                        }
                    },
                    Err(_) => None,
                };
                match file_content {
                    Some(file_content) => {
                        self.analysis_v2
                            .apply_changes(vec![bsl_analysis_v2::Change::SetFile {
                                file_id,
                                text: Arc::from(file_content),
                                version: 0,
                                path: Arc::from(path),
                            }]);

                        self.analysis_v2.wait_for_file_version(file_id, 0).await
                    }
                    None => false,
                }
            };
            let wait_elapsed = wait_started.elapsed();
            self.coordinator
                .record_intellisense_v2_wait_for_file_version("completion", wait_elapsed);
            if let Some(threshold) = super::intellisense_v2_slow_wait_warn_threshold() {
                if wait_elapsed >= threshold {
                    warn!(
                        uri = %uri,
                        file_id = file_id.0,
                        expected_version = expected_version.unwrap_or(0),
                        wait_ms = wait_elapsed.as_millis(),
                        threshold_ms = threshold.as_millis(),
                        "Completion v2: wait_for_file_version is slow"
                    );
                }
            }

            if !ready {
                completion_outcome = Some("wait_not_ready");
                empty()
            } else {
                let (
                    file_content,
                    file_path,
                    parse_result,
                    member_access_owner_type_hint,
                    deps,
                    ir_program,
                    index_snapshot,
                    include_flow_sensitive,
                ) = {
                    let snapshot_started = Instant::now();
                    let (analysis, index_snapshot, deps_id) =
                        self.analysis_v2.snapshot_with_deps().await;
                    let snapshot_elapsed = snapshot_started.elapsed();
                    self.coordinator
                        .record_intellisense_v2_snapshot_latency("completion", snapshot_elapsed);
                    if let Some(threshold) = super::intellisense_v2_slow_snapshot_warn_threshold() {
                        if snapshot_elapsed >= threshold {
                            warn!(
                                uri = %uri,
                                file_id = file_id.0,
                                snapshot_ms = snapshot_elapsed.as_millis(),
                                threshold_ms = threshold.as_millis(),
                                "Completion v2: snapshot acquisition is slow"
                            );
                        }
                    }

                    let observed_file_version = analysis.file_version(file_id).ok().flatten();
                    let observed_deps_id = Some(deps_id);
                    let observed_settings_id = analysis.settings_id().ok();
                    debug!(
                        "Completion v2 observed: uri={}, file_id={}, file_version={:?}, deps_id={:?}, settings_id={:?}, index_snapshot_id={}",
                        uri,
                        file_id.0,
                        observed_file_version,
                        observed_deps_id.as_ref().map(|v| v.as_str()),
                        observed_settings_id.as_ref().map(|v| v.as_str()),
                        index_snapshot.id.as_str(),
                    );
                    match analysis.file_text_len(file_id) {
                        Ok(Some(len)) => debug!(
                            "Completion v2 (salsa) active: uri={}, file_id={}, text_len={}",
                            uri, file_id.0, len
                        ),
                        Ok(None) => debug!(
                            "Completion v2 (salsa) active: uri={}, file_id={} (file not found)",
                            uri, file_id.0
                        ),
                        Err(_) => debug!(
                            "Completion v2 (salsa) cancelled: uri={}, file_id={}",
                            uri, file_id.0
                        ),
                    }

                    let observed_byte_offset = analysis
                        .utf16_position_to_byte_offset(file_id, position.line, position.character)
                        .ok()
                        .flatten();
                    let observed_point = analysis
                        .utf16_position_to_point(file_id, position.line, position.character)
                        .ok()
                        .flatten();
                    debug!(
                        "Completion v2 positioning: uri={}, file_id={}, lsp=({}:{}) -> byte_offset={:?}, point={:?}",
                        uri,
                        file_id.0,
                        position.line,
                        position.character,
                        observed_byte_offset,
                        observed_point,
                    );

                    let file_content = analysis.file_text(file_id).ok().flatten();
                    let file_path = analysis.file_path(file_id).ok().flatten();
                    let parse_result = analysis.parse_result(file_id).ok().flatten();
                    let deps = analysis.deps_data().ok();
                    let ir_started = Instant::now();
                    let ir_program = analysis.ir(file_id).ok().flatten();
                    let ir_elapsed = ir_started.elapsed();
                    self.coordinator
                        .record_intellisense_v2_ir_query_latency("completion", ir_elapsed);
                    if let Some(threshold) = super::intellisense_v2_slow_query_warn_threshold() {
                        if ir_elapsed >= threshold {
                            warn!(
                                uri = %uri,
                                file_id = file_id.0,
                                ir_ms = ir_elapsed.as_millis(),
                                threshold_ms = threshold.as_millis(),
                                "Completion v2: ir query is slow"
                            );
                        }
                    }

                    if bsl_runtime::system::global_runtime_config()
                        .get_bool(bsl_runtime::system::RuntimeKey::IntellisenseV2P4Smoke)
                        .unwrap_or(false)
                    {
                        match ir_program.as_ref() {
                            Some(program) => debug!(
                                "Completion v2 ir: uri={}, file_id={}, deps_id={:?}, nodes={}",
                                uri,
                                file_id.0,
                                observed_deps_id.as_ref().map(|v| v.as_str()),
                                program.nodes.len()
                            ),
                            None => debug!(
                                "Completion v2 ir: uri={}, file_id={} (unavailable)",
                                uri, file_id.0
                            ),
                        }
                    }

                    if bsl_runtime::system::global_runtime_config()
                        .get_bool(bsl_runtime::system::RuntimeKey::IntellisenseV2P3Smoke)
                        .unwrap_or(false)
                    {
                        match parse_result.as_ref() {
                            Some(parsed) => debug!(
                                "Completion v2 parse_result: uri={}, file_id={}, syntax_errors={}",
                                uri,
                                file_id.0,
                                parsed.syntax_errors.len()
                            ),
                            None => debug!(
                                "Completion v2 parse_result: uri={}, file_id={} (unavailable)",
                                uri, file_id.0
                            ),
                        }
                    }

                    let include_flow_sensitive = {
                        let settings = self.settings.read().await;
                        settings.enable_flow_sensitive
                    };

                    let member_access_owner_type_hint = file_content.as_deref().and_then(|text| {
                        let line_text = text.lines().nth(position.line as usize)?;
                        let cursor_byte = bsl_backend::system::positioning::utf16_to_byte_offset(
                            line_text,
                            position.character,
                        );
                        let line_prefix = line_text.get(..cursor_byte)?;
                        let dot_in_line = line_prefix.rfind('.')?;
                        let receiver = line_prefix.get(..dot_in_line)?.trim_end();
                        let (probe_byte, _) = receiver
                            .char_indices()
                            .rev()
                            .find(|(_, ch)| !ch.is_whitespace())?;
                        let probe_utf16 = bsl_backend::system::positioning::byte_offset_to_utf16(
                            line_text, probe_byte,
                        );
                        let offset = analysis
                            .utf16_position_to_byte_offset(file_id, position.line, probe_utf16)
                            .ok()
                            .flatten()?;
                        let offset = offset.min(u32::MAX as usize) as u32;
                        if include_flow_sensitive {
                            analysis
                                .flow_type_at_byte_offset(file_id, offset)
                                .ok()
                                .flatten()
                                .or_else(|| {
                                    analysis.type_at_byte_offset(file_id, offset).ok().flatten()
                                })
                        } else {
                            analysis.type_at_byte_offset(file_id, offset).ok().flatten()
                        }
                    });

                    (
                        file_content,
                        file_path,
                        parse_result,
                        member_access_owner_type_hint,
                        deps,
                        ir_program,
                        index_snapshot,
                        include_flow_sensitive,
                    )
                };

                match (file_content, file_path, deps, ir_program) {
                    (Some(file_content), Some(file_path), Some(deps), Some(ir_program)) => {
                        crate::handlers::handle_completion_v2(
                            file_content,
                            file_path,
                            ir_program,
                            parse_result,
                            member_access_owner_type_hint,
                            deps,
                            position,
                            &uri,
                            index_snapshot.as_ref(),
                            snippet_support,
                            include_flow_sensitive,
                        )
                        .await
                    }
                    (None, _, _, _) => {
                        completion_outcome = Some("missing_file_content");
                        empty()
                    }
                    (Some(_), None, _, _) => {
                        completion_outcome = Some("missing_file_path");
                        empty()
                    }
                    (Some(_), Some(_), None, _) => {
                        completion_outcome = Some("missing_deps");
                        empty()
                    }
                    (Some(_), Some(_), Some(_), None) => {
                        completion_outcome = Some("missing_ir");
                        empty()
                    }
                }
            }
        };
        let elapsed = started.elapsed();
        self.coordinator.record_completion_latency(elapsed);

        if let Some(result) = &completion {
            if result.had_error {
                self.coordinator.record_completion_error();
            }

            let items_count = match &result.response {
                CompletionResponse::List(list) => {
                    if list.is_incomplete {
                        self.coordinator.record_completion_incomplete();
                    }
                    list.items.len()
                }
                CompletionResponse::Array(items) => items.len(),
            };
            self.coordinator
                .record_intellisense_v2_completion_items_count(items_count);

            if let Some(stats) = &result.stats {
                self.coordinator
                    .record_completion_stage_latency("snapshot_read", stats.stage_snapshot_read);
                self.coordinator
                    .record_completion_stage_latency("collect", stats.stage_collect);
                self.coordinator
                    .record_completion_stage_latency("rank", stats.stage_rank);
                self.coordinator
                    .record_completion_stage_latency("format", stats.stage_format);
            }

            if bsl_runtime::system::global_runtime_config()
                .get_bool(bsl_runtime::system::RuntimeKey::CompletionQuality)
                .unwrap_or(false)
            {
                if let Some(stats) = &result.stats {
                    self.coordinator.record_completion_quality(
                        stats.total_candidates,
                        stats.dedup_removed,
                        &stats.score_samples,
                        stats.prefix_exact,
                        stats.prefix_starts,
                        stats.prefix_contains,
                        stats.prefix_none,
                        stats.member_access,
                        stats.has_owner,
                    );
                }
            }

            if completion_outcome.is_none() {
                completion_outcome = Some(if result.had_error {
                    "handler_error"
                } else if items_count == 0 {
                    "ok_empty"
                } else {
                    "ok_non_empty"
                });
            }
        }

        if let Some(outcome) = completion_outcome {
            self.coordinator
                .record_intellisense_v2_completion_outcome(outcome);
        }

        Ok(completion.map(|result| result.response))
    }

    async fn completion_resolve(&self, item: CompletionItem) -> JsonRpcResult<CompletionItem> {
        let snippet_support = *self.completion_snippet_support.read().await;
        let started = Instant::now();
        let deps = self.analysis_v2.snapshot().await.deps_data().ok();
        let resolved = handle_completion_resolve(item, deps, snippet_support).await;
        let elapsed = started.elapsed();
        self.coordinator.record_completion_resolve_latency(elapsed);
        Ok(resolved)
    }

    async fn hover(&self, params: HoverParams) -> JsonRpcResult<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        info!(
            "Hover requested at {}:{}",
            position.line, position.character
        );

        {
            let file_id = self.get_or_create_file_id_v2(&uri).await;
            self.sync_v2_globals().await;

            let expected_version = self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied();

            let wait_started = Instant::now();
            let ok = if let Some(expected_version) = expected_version {
                self.analysis_v2
                    .wait_for_file_version(file_id, expected_version)
                    .await
            } else {
                let path = match uri.to_file_path() {
                    Ok(path) => path.to_string_lossy().to_string(),
                    Err(_) => uri.to_string(),
                };
                let file_content = match uri.to_file_path() {
                    Ok(path) => match read_bsl_file(&path) {
                        Ok(content) => Some(content),
                        Err(e) => {
                            error!("Failed to read file for hover: {}", e);
                            None
                        }
                    },
                    Err(_) => None,
                };
                match file_content {
                    Some(file_content) => {
                        self.analysis_v2
                            .apply_changes(vec![bsl_analysis_v2::Change::SetFile {
                                file_id,
                                text: Arc::from(file_content),
                                version: 0,
                                path: Arc::from(path),
                            }]);

                        self.analysis_v2.wait_for_file_version(file_id, 0).await
                    }
                    None => false,
                }
            };
            let wait_elapsed = wait_started.elapsed();
            self.coordinator
                .record_intellisense_v2_wait_for_file_version("hover", wait_elapsed);
            if let Some(threshold) = super::intellisense_v2_slow_wait_warn_threshold() {
                if wait_elapsed >= threshold {
                    warn!(
                        uri = %uri,
                        file_id = file_id.0,
                        expected_version = expected_version.unwrap_or(0),
                        wait_ms = wait_elapsed.as_millis(),
                        threshold_ms = threshold.as_millis(),
                        "Hover v2: wait_for_file_version is slow"
                    );
                }
            }
            if !ok {
                return Ok(None);
            }

            let (analysis, file_content, file_path, deps, ir_program) = {
                let snapshot_started = Instant::now();
                let (analysis, index_snapshot, deps_id) =
                    self.analysis_v2.snapshot_with_deps().await;
                let snapshot_elapsed = snapshot_started.elapsed();
                self.coordinator
                    .record_intellisense_v2_snapshot_latency("hover", snapshot_elapsed);
                if let Some(threshold) = super::intellisense_v2_slow_snapshot_warn_threshold() {
                    if snapshot_elapsed >= threshold {
                        warn!(
                            uri = %uri,
                            file_id = file_id.0,
                            snapshot_ms = snapshot_elapsed.as_millis(),
                            threshold_ms = threshold.as_millis(),
                            "Hover v2: snapshot acquisition is slow"
                        );
                    }
                }
                let observed_file_version = analysis.file_version(file_id).ok().flatten();
                let observed_deps_id = Some(deps_id);
                let observed_settings_id = analysis.settings_id().ok();
                debug!(
                    "Hover v2 observed: uri={}, file_id={}, file_version={:?}, deps_id={:?}, settings_id={:?}, index_snapshot_id={}",
                    uri,
                    file_id.0,
                    observed_file_version,
                    observed_deps_id.as_ref().map(|v| v.as_str()),
                    observed_settings_id.as_ref().map(|v| v.as_str()),
                    index_snapshot.id.as_str(),
                );

                let observed_byte_offset = analysis
                    .utf16_position_to_byte_offset(file_id, position.line, position.character)
                    .ok()
                    .flatten();
                let observed_point = analysis
                    .utf16_position_to_point(file_id, position.line, position.character)
                    .ok()
                    .flatten();
                debug!(
                    "Hover v2 positioning: uri={}, file_id={}, lsp=({}:{}) -> byte_offset={:?}, point={:?}",
                    uri,
                    file_id.0,
                    position.line,
                    position.character,
                    observed_byte_offset,
                    observed_point,
                );

                let file_content = analysis.file_text(file_id).ok().flatten();
                let file_path = analysis.file_path(file_id).ok().flatten();
                let deps = analysis.deps_data().ok();
                let ir_started = Instant::now();
                let ir_program = analysis.ir(file_id).ok().flatten();
                let ir_elapsed = ir_started.elapsed();
                self.coordinator
                    .record_intellisense_v2_ir_query_latency("hover", ir_elapsed);
                if let Some(threshold) = super::intellisense_v2_slow_query_warn_threshold() {
                    if ir_elapsed >= threshold {
                        warn!(
                            uri = %uri,
                            file_id = file_id.0,
                            ir_ms = ir_elapsed.as_millis(),
                            threshold_ms = threshold.as_millis(),
                            "Hover v2: ir query is slow"
                        );
                    }
                }

                (analysis, file_content, file_path, deps, ir_program)
            };

            let settings = self.settings.read().await;
            let include_flow_sensitive = settings.enable_flow_sensitive;
            let result = match (file_content, file_path, deps, ir_program) {
                (Some(file_content), Some(file_path), Some(deps), Some(ir_program)) => {
                    handle_hover_v2(
                        &analysis,
                        file_id,
                        file_content,
                        file_path,
                        ir_program,
                        deps,
                        position,
                        &uri,
                        &settings.hover,
                        include_flow_sensitive,
                    )
                }
                _ => None,
            };

            return Ok(result);
        }
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> JsonRpcResult<Option<Vec<InlayHint>>> {
        let uri = params.text_document.uri;

        let (feature_enabled, settings) = {
            let cfg = self.config.read().await;
            let feature_enabled = cfg
                .as_ref()
                .and_then(|cfg| cfg.enable_type_hints)
                .unwrap_or(false);
            let settings = self.settings.read().await.type_hints.clone();
            (feature_enabled, settings)
        };
        if !feature_enabled || !settings.enabled {
            return Ok(None);
        }

        let file_id = self.get_or_create_file_id_v2(&uri).await;
        self.sync_v2_globals().await;

        let expected_version = self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied();
        let ok = if let Some(expected_version) = expected_version {
            self.analysis_v2
                .wait_for_file_version(file_id, expected_version)
                .await
        } else {
            self.analysis_v2.wait_for_file_version(file_id, 0).await
        };
        if !ok {
            return Ok(None);
        }

        let (analysis, file_content, ir_program) = {
            let (analysis, _index_snapshot, _deps_id) = self.analysis_v2.snapshot_with_deps().await;
            let file_content = analysis.file_text(file_id).ok().flatten();
            let ir_program = analysis.ir(file_id).ok().flatten();
            (analysis, file_content, ir_program)
        };

        let Some(file_content) = file_content else {
            return Ok(None);
        };
        let Some(ir_program) = ir_program else {
            return Ok(None);
        };

        let range = params.range;
        let computed = timeout(std::time::Duration::from_millis(80), async move {
            handle_inlay_hints_v2(
                &analysis,
                file_id,
                file_content,
                ir_program,
                range,
                &settings,
            )
        })
        .await;

        match computed {
            Ok(hints) => Ok(Some(hints)),
            Err(_) => {
                warn!(uri = %uri, "Inlay hints: timed out");
                Ok(Some(Vec::new()))
            }
        }
    }

    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> JsonRpcResult<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;

        let (feature_enabled, code_actions_settings, type_hints_settings) = {
            let cfg = self.config.read().await;
            let feature_enabled = cfg
                .as_ref()
                .and_then(|cfg| cfg.enable_code_actions)
                .unwrap_or(false);
            let settings = self.settings.read().await;
            (
                feature_enabled,
                settings.code_actions.clone(),
                settings.type_hints.clone(),
            )
        };
        if !feature_enabled || !code_actions_settings.enabled {
            return Ok(None);
        }

        let file_id = self.get_or_create_file_id_v2(&uri).await;
        self.sync_v2_globals().await;

        let expected_version = self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied();
        let ok = if let Some(expected_version) = expected_version {
            self.analysis_v2
                .wait_for_file_version(file_id, expected_version)
                .await
        } else {
            self.analysis_v2.wait_for_file_version(file_id, 0).await
        };
        if !ok {
            return Ok(None);
        }

        let (analysis, file_content, ir_program) = {
            let (analysis, _index_snapshot, _deps_id) = self.analysis_v2.snapshot_with_deps().await;
            let file_content = analysis.file_text(file_id).ok().flatten();
            let ir_program = analysis.ir(file_id).ok().flatten();
            (analysis, file_content, ir_program)
        };

        let Some(file_content) = file_content else {
            return Ok(None);
        };
        let Some(ir_program) = ir_program else {
            return Ok(None);
        };

        let range = params.range;
        let uri_for_action = uri.clone();
        let computed = timeout(std::time::Duration::from_millis(120), async move {
            handle_code_actions_v2(
                &analysis,
                file_id,
                file_content,
                ir_program,
                &uri_for_action,
                range,
                &code_actions_settings,
                &type_hints_settings,
            )
        })
        .await;

        match computed {
            Ok(actions) => Ok(Some(actions)),
            Err(_) => {
                warn!(uri = %uri, "Code actions: timed out");
                Ok(Some(Vec::new()))
            }
        }
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> JsonRpcResult<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        {
            let file_id = self.get_or_create_file_id_v2(&uri).await;
            self.sync_v2_globals().await;

            let expected_version = self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied();

            let wait_started = Instant::now();
            let ok = if let Some(expected_version) = expected_version {
                self.analysis_v2
                    .wait_for_file_version(file_id, expected_version)
                    .await
            } else {
                let path = match uri.to_file_path() {
                    Ok(path) => path.to_string_lossy().to_string(),
                    Err(_) => uri.to_string(),
                };
                let file_content = match uri.to_file_path() {
                    Ok(path) => match read_bsl_file(&path) {
                        Ok(content) => Some(content),
                        Err(e) => {
                            error!("Failed to read file for definition: {}", e);
                            None
                        }
                    },
                    Err(_) => None,
                };
                match file_content {
                    Some(file_content) => {
                        self.analysis_v2
                            .apply_changes(vec![bsl_analysis_v2::Change::SetFile {
                                file_id,
                                text: Arc::from(file_content),
                                version: 0,
                                path: Arc::from(path),
                            }]);

                        self.analysis_v2.wait_for_file_version(file_id, 0).await
                    }
                    None => false,
                }
            };
            let wait_elapsed = wait_started.elapsed();
            self.coordinator
                .record_intellisense_v2_wait_for_file_version("definition", wait_elapsed);
            if let Some(threshold) = super::intellisense_v2_slow_wait_warn_threshold() {
                if wait_elapsed >= threshold {
                    warn!(
                        uri = %uri,
                        file_id = file_id.0,
                        expected_version = expected_version.unwrap_or(0),
                        wait_ms = wait_elapsed.as_millis(),
                        threshold_ms = threshold.as_millis(),
                        "Definition v2: wait_for_file_version is slow"
                    );
                }
            }
            if !ok {
                return Ok(None);
            }

            let (
                file_content,
                file_path,
                type_at_position_hint,
                receiver_type_hint,
                deps,
                ir_program,
            ) = {
                let snapshot_started = Instant::now();
                let (analysis, index_snapshot, deps_id) =
                    self.analysis_v2.snapshot_with_deps().await;
                let snapshot_elapsed = snapshot_started.elapsed();
                self.coordinator
                    .record_intellisense_v2_snapshot_latency("definition", snapshot_elapsed);
                if let Some(threshold) = super::intellisense_v2_slow_snapshot_warn_threshold() {
                    if snapshot_elapsed >= threshold {
                        warn!(
                            uri = %uri,
                            file_id = file_id.0,
                            snapshot_ms = snapshot_elapsed.as_millis(),
                            threshold_ms = threshold.as_millis(),
                            "Definition v2: snapshot acquisition is slow"
                        );
                    }
                }

                let observed_file_version = analysis.file_version(file_id).ok().flatten();
                let observed_deps_id = Some(deps_id);
                let observed_settings_id = analysis.settings_id().ok();
                debug!(
                    "Definition v2 observed: uri={}, file_id={}, file_version={:?}, deps_id={:?}, settings_id={:?}, index_snapshot_id={}",
                    uri,
                    file_id.0,
                    observed_file_version,
                    observed_deps_id.as_ref().map(|v| v.as_str()),
                    observed_settings_id.as_ref().map(|v| v.as_str()),
                    index_snapshot.id.as_str(),
                );

                let file_content = analysis.file_text(file_id).ok().flatten();
                let file_path = analysis.file_path(file_id).ok().flatten();
                let deps = analysis.deps_data().ok();
                let ir_started = Instant::now();
                let ir_program = analysis.ir(file_id).ok().flatten();
                let ir_elapsed = ir_started.elapsed();
                self.coordinator
                    .record_intellisense_v2_ir_query_latency("definition", ir_elapsed);
                if let Some(threshold) = super::intellisense_v2_slow_query_warn_threshold() {
                    if ir_elapsed >= threshold {
                        warn!(
                            uri = %uri,
                            file_id = file_id.0,
                            ir_ms = ir_elapsed.as_millis(),
                            threshold_ms = threshold.as_millis(),
                            "Definition v2: ir query is slow"
                        );
                    }
                }

                let include_flow_sensitive = {
                    let settings = self.settings.read().await;
                    settings.enable_flow_sensitive
                };

                let type_at_position_hint = {
                    let offset = analysis
                        .utf16_position_to_byte_offset(file_id, position.line, position.character)
                        .ok()
                        .flatten();
                    offset.and_then(|offset| {
                        let offset = offset.min(u32::MAX as usize) as u32;
                        if include_flow_sensitive {
                            analysis
                                .flow_type_at_byte_offset(file_id, offset)
                                .ok()
                                .flatten()
                                .or_else(|| {
                                    analysis.type_at_byte_offset(file_id, offset).ok().flatten()
                                })
                        } else {
                            analysis.type_at_byte_offset(file_id, offset).ok().flatten()
                        }
                    })
                };
                let receiver_type_hint = ir_program.as_ref().and_then(|program| {
                    let offset = analysis
                        .utf16_position_to_byte_offset(file_id, position.line, position.character)
                        .ok()
                        .flatten()
                        .map(|offset| offset.min(u32::MAX as usize) as u32)?;

                    let node = program.find_node_at_byte_offset(offset)?;

                    let object_span = match &node.kind {
                        bsl_shared::ir::SemanticNodeKind::MemberAccess { object_node, .. } => {
                            object_node.and_then(|idx| program.nodes.get(idx).map(|n| n.span))
                        }
                        bsl_shared::ir::SemanticNodeKind::FunctionCall { object_node, .. } => {
                            object_node.and_then(|idx| program.nodes.get(idx).map(|n| n.span))
                        }
                        _ => None,
                    }?;

                    if include_flow_sensitive {
                        analysis
                            .flow_type_at_byte_offset(file_id, object_span.start)
                            .ok()
                            .flatten()
                            .or_else(|| {
                                analysis
                                    .type_at_byte_offset(file_id, object_span.start)
                                    .ok()
                                    .flatten()
                            })
                    } else {
                        analysis
                            .type_at_byte_offset(file_id, object_span.start)
                            .ok()
                            .flatten()
                    }
                });

                (
                    file_content,
                    file_path,
                    type_at_position_hint,
                    receiver_type_hint,
                    deps,
                    ir_program,
                )
            };

            let result = match (file_content, file_path, deps, ir_program) {
                (Some(file_content), Some(file_path), Some(deps), Some(ir_program)) => {
                    handle_goto_definition_v2(
                        file_path,
                        file_content,
                        ir_program,
                        type_at_position_hint,
                        receiver_type_hint,
                        deps,
                        position,
                        &uri,
                    )
                    .await
                }
                _ => None,
            };

            return Ok(result);
        }
    }

    async fn signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> JsonRpcResult<Option<SignatureHelp>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        {
            let file_id = self.get_or_create_file_id_v2(&uri).await;
            self.sync_v2_globals().await;

            let expected_version = self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied();

            let wait_started = Instant::now();
            let ok = if let Some(expected_version) = expected_version {
                self.analysis_v2
                    .wait_for_file_version(file_id, expected_version)
                    .await
            } else {
                let path = match uri.to_file_path() {
                    Ok(path) => path.to_string_lossy().to_string(),
                    Err(_) => uri.to_string(),
                };
                let file_content = match uri.to_file_path() {
                    Ok(path) => match read_bsl_file(&path) {
                        Ok(content) => Some(content),
                        Err(e) => {
                            error!("Failed to read file for signatureHelp: {}", e);
                            None
                        }
                    },
                    Err(_) => None,
                };

                match file_content {
                    Some(file_content) => {
                        self.analysis_v2
                            .apply_changes(vec![bsl_analysis_v2::Change::SetFile {
                                file_id,
                                text: Arc::from(file_content),
                                version: 0,
                                path: Arc::from(path),
                            }]);

                        self.analysis_v2.wait_for_file_version(file_id, 0).await
                    }
                    None => false,
                }
            };
            let wait_elapsed = wait_started.elapsed();
            self.coordinator
                .record_intellisense_v2_wait_for_file_version("signature_help", wait_elapsed);
            if let Some(threshold) = super::intellisense_v2_slow_wait_warn_threshold() {
                if wait_elapsed >= threshold {
                    warn!(
                        uri = %uri,
                        file_id = file_id.0,
                        expected_version = expected_version.unwrap_or(0),
                        wait_ms = wait_elapsed.as_millis(),
                        threshold_ms = threshold.as_millis(),
                        "SignatureHelp v2: wait_for_file_version is slow"
                    );
                }
            }
            if !ok {
                return Ok(None);
            }

            let (file_content, deps, receiver_type_hint) = {
                let snapshot_started = Instant::now();
                let (analysis, index_snapshot, deps_id) =
                    self.analysis_v2.snapshot_with_deps().await;
                let snapshot_elapsed = snapshot_started.elapsed();
                self.coordinator
                    .record_intellisense_v2_snapshot_latency("signature_help", snapshot_elapsed);
                if let Some(threshold) = super::intellisense_v2_slow_snapshot_warn_threshold() {
                    if snapshot_elapsed >= threshold {
                        warn!(
                            uri = %uri,
                            file_id = file_id.0,
                            snapshot_ms = snapshot_elapsed.as_millis(),
                            threshold_ms = threshold.as_millis(),
                            "SignatureHelp v2: snapshot acquisition is slow"
                        );
                    }
                }
                let observed_file_version = analysis.file_version(file_id).ok().flatten();
                let observed_deps_id = Some(deps_id);
                let observed_settings_id = analysis.settings_id().ok();
                debug!(
                    "SignatureHelp v2 observed: uri={}, file_id={}, file_version={:?}, deps_id={:?}, settings_id={:?}, index_snapshot_id={}",
                    uri,
                    file_id.0,
                    observed_file_version,
                    observed_deps_id.as_ref().map(|v| v.as_str()),
                    observed_settings_id.as_ref().map(|v| v.as_str()),
                    index_snapshot.id.as_str(),
                );

                let observed_byte_offset = analysis
                    .utf16_position_to_byte_offset(file_id, position.line, position.character)
                    .ok()
                    .flatten();
                let observed_point = analysis
                    .utf16_position_to_point(file_id, position.line, position.character)
                    .ok()
                    .flatten();
                debug!(
                    "SignatureHelp v2 positioning: uri={}, file_id={}, lsp=({}:{}) -> byte_offset={:?}, point={:?}",
                    uri,
                    file_id.0,
                    position.line,
                    position.character,
                    observed_byte_offset,
                    observed_point,
                );

                let file_content = analysis.file_text(file_id).ok().flatten();
                let deps = analysis.deps_data().ok();

                let include_flow_sensitive = {
                    let settings = self.settings.read().await;
                    settings.enable_flow_sensitive
                };

                let receiver_type_hint = file_content.as_ref().and_then(|text| {
                    let query = bsl_backend::application::type_system::signature_help_query(
                        text.as_ref(),
                        position.line,
                        position.character,
                    )?;
                    let receiver_end_character = query.receiver_end_character?;
                    let offset = analysis
                        .utf16_position_to_byte_offset(
                            file_id,
                            query.call_start_line,
                            receiver_end_character,
                        )
                        .ok()
                        .flatten()?;
                    let offset = offset.min(u32::MAX as usize) as u32;
                    if include_flow_sensitive {
                        analysis
                            .flow_type_at_byte_offset(file_id, offset)
                            .ok()
                            .flatten()
                            .or_else(|| {
                                analysis.type_at_byte_offset(file_id, offset).ok().flatten()
                            })
                    } else {
                        analysis.type_at_byte_offset(file_id, offset).ok().flatten()
                    }
                });

                (file_content, deps, receiver_type_hint)
            };

            let started = Instant::now();
            let result = match (file_content, deps) {
                (Some(file_content), Some(deps)) => {
                    handle_signature_help_v2(file_content, position, receiver_type_hint, deps).await
                }
                _ => None,
            };
            let elapsed = started.elapsed();
            self.coordinator.record_signature_help_latency(elapsed);
            if result.is_none() {
                self.coordinator.record_signature_help_empty();
            }
            return Ok(result);
        }
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

                let uri = Url::parse(&request.uri).map_err(|e| {
                    tower_lsp::jsonrpc::Error::invalid_params(format!("Invalid URI: {}", e))
                })?;

                self.sync_v2_globals().await;
                let file_id = self.get_or_create_file_id_v2(&uri).await;

                let expected_version = self
                    .latest_received_file_versions_v2
                    .read()
                    .await
                    .get(&file_id)
                    .copied();

                let ok = if let Some(expected_version) = expected_version {
                    self.analysis_v2
                        .wait_for_file_version(file_id, expected_version)
                        .await
                } else {
                    let path = match uri.to_file_path() {
                        Ok(path) => path.to_string_lossy().to_string(),
                        Err(_) => uri.to_string(),
                    };
                    let file_content = match uri.to_file_path() {
                        Ok(path) => match read_bsl_file(&path) {
                            Ok(content) => Some(content),
                            Err(e) => {
                                error!("Failed to read file for getSemanticHtml: {}", e);
                                None
                            }
                        },
                        Err(_) => None,
                    };

                    match file_content {
                        Some(file_content) => {
                            self.analysis_v2.apply_changes(vec![
                                bsl_analysis_v2::Change::SetFile {
                                    file_id,
                                    text: Arc::from(file_content),
                                    version: 0,
                                    path: Arc::from(path),
                                },
                            ]);
                            self.analysis_v2.wait_for_file_version(file_id, 0).await
                        }
                        None => false,
                    }
                };

                if !ok {
                    return Err(tower_lsp::jsonrpc::Error::internal_error());
                }

                let analysis = self.analysis_v2.snapshot().await;
                let file_text = analysis
                    .file_text(file_id)
                    .ok()
                    .flatten()
                    .ok_or_else(tower_lsp::jsonrpc::Error::internal_error)?;
                let line_index = analysis
                    .line_index(file_id)
                    .ok()
                    .flatten()
                    .ok_or_else(tower_lsp::jsonrpc::Error::internal_error)?;
                let ir_program = analysis
                    .ir(file_id)
                    .ok()
                    .flatten()
                    .ok_or_else(tower_lsp::jsonrpc::Error::internal_error)?;

                let semantic_tree = semantic_tree_from_ir(
                    ir_program.as_ref(),
                    true,
                    true,
                    file_text.as_ref(),
                    line_index.as_ref(),
                );
                let result = semantic_html_from_tree(
                    &semantic_tree,
                    request.theme.as_deref(),
                    request.compact,
                );

                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
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

                let uri = Url::parse(&request.uri).map_err(|e| {
                    tower_lsp::jsonrpc::Error::invalid_params(format!("Invalid URI: {}", e))
                })?;

                self.sync_v2_globals().await;
                let file_id = self.get_or_create_file_id_v2(&uri).await;

                let expected_version = self
                    .latest_received_file_versions_v2
                    .read()
                    .await
                    .get(&file_id)
                    .copied();

                let ok = if let Some(expected_version) = expected_version {
                    self.analysis_v2
                        .wait_for_file_version(file_id, expected_version)
                        .await
                } else {
                    let path = match uri.to_file_path() {
                        Ok(path) => path.to_string_lossy().to_string(),
                        Err(_) => uri.to_string(),
                    };
                    let file_content = match uri.to_file_path() {
                        Ok(path) => match read_bsl_file(&path) {
                            Ok(content) => Some(content),
                            Err(e) => {
                                error!("Failed to read file for getSemanticTree: {}", e);
                                None
                            }
                        },
                        Err(_) => None,
                    };

                    match file_content {
                        Some(file_content) => {
                            self.analysis_v2.apply_changes(vec![
                                bsl_analysis_v2::Change::SetFile {
                                    file_id,
                                    text: Arc::from(file_content),
                                    version: 0,
                                    path: Arc::from(path),
                                },
                            ]);
                            self.analysis_v2.wait_for_file_version(file_id, 0).await
                        }
                        None => false,
                    }
                };

                if !ok {
                    return Err(tower_lsp::jsonrpc::Error::internal_error());
                }

                let analysis = self.analysis_v2.snapshot().await;
                let file_text = analysis
                    .file_text(file_id)
                    .ok()
                    .flatten()
                    .ok_or_else(tower_lsp::jsonrpc::Error::internal_error)?;
                let line_index = analysis
                    .line_index(file_id)
                    .ok()
                    .flatten()
                    .ok_or_else(tower_lsp::jsonrpc::Error::internal_error)?;
                let ir_program = analysis
                    .ir(file_id)
                    .ok()
                    .flatten()
                    .ok_or_else(tower_lsp::jsonrpc::Error::internal_error)?;

                let enable_flow_sensitive = {
                    let settings = self.settings.read().await;
                    settings.enable_flow_sensitive
                };
                let include_flow_sensitive = effective_include_flow_sensitive(
                    request.include_flow_sensitive,
                    enable_flow_sensitive,
                );

                let result = semantic_tree_from_ir(
                    ir_program.as_ref(),
                    request.include_call_graph,
                    include_flow_sensitive,
                    file_text.as_ref(),
                    line_index.as_ref(),
                );

                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
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

                let result = handle_search_types(request, self.coordinator.get_domain_bundle());
                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
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

                let result = handle_get_all_types(request, self.coordinator.get_domain_bundle());
                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
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
                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
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

                let result = handle_query_type(request, self.coordinator.get_domain_bundle());
                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
            }
            "bsl.getTypeRepositoryStats" => {
                let result = handle_get_type_repository_stats(self.coordinator.clone());
                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
            }
            "bsl.getWorkspaceStats" => {
                let result = self.handle_get_workspace_stats().await?;
                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
            }
            "bsl.getObservabilityMetrics" => {
                let result = self.handle_get_observability_metrics().await?;
                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
            }
            "bsl.getRuntimeConfig" => {
                let snapshot = bsl_runtime::system::global_runtime_config().snapshot();
                Ok(Some(serde_json::to_value(snapshot).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
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

                let platform_docs_root = {
                    let config = self.config.read().await;
                    config
                        .as_ref()
                        .and_then(|cfg| cfg.platform_docs_archive.as_deref())
                        .map(PathBuf::from)
                };
                let config_root = PathBuf::from(&request.config_path);

                let result = handle_parse_configuration(
                    request,
                    self.coordinator.get_domain_bundle(),
                    self.client.clone(),
                    "parse-config",
                    "Parsing configuration",
                    Some(self.coordinator.clone()),
                )
                .await;

                if result.success {
                    self.deps_update_v2(
                        "parseConfiguration",
                        platform_docs_root,
                        Some(config_root),
                    )
                    .await;
                    self.sync_v2_globals().await;
                }

                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
            }
            "bsl.cache.getStats" => {
                let config_path = resolve_cache_config_path(&params, &self.config).await?;
                let scope = self
                    .coordinator
                    .cache_scope_for_config_path(Path::new(&config_path))
                    .map_err(|e| tower_lsp::jsonrpc::Error::invalid_params(e.to_string()))?;
                let result = handle_cache_stats(self.coordinator.clone(), scope)
                    .await
                    .map_err(|_| tower_lsp::jsonrpc::Error::internal_error())?;
                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
            }
            "bsl.cache.clear" => {
                let config_path = resolve_cache_config_path(&params, &self.config).await?;
                let scope = self
                    .coordinator
                    .cache_scope_for_config_path(Path::new(&config_path))
                    .map_err(|e| tower_lsp::jsonrpc::Error::invalid_params(e.to_string()))?;
                let result = handle_cache_clear(self.coordinator.clone(), scope)
                    .await
                    .map_err(|_| tower_lsp::jsonrpc::Error::internal_error())?;
                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
            }
            "bsl.cache.setEnabled" => {
                if params.arguments.is_empty() {
                    return Err(tower_lsp::jsonrpc::Error::invalid_params(
                        "Missing enabled flag",
                    ));
                }

                let request: CacheToggleParams =
                    serde_json::from_value(params.arguments[0].clone()).map_err(|e| {
                        tower_lsp::jsonrpc::Error::invalid_params(format!(
                            "Invalid parameters: {}",
                            e
                        ))
                    })?;
                let result = handle_cache_set_enabled(self.coordinator.clone(), request.enabled)
                    .await
                    .map_err(|_| tower_lsp::jsonrpc::Error::internal_error())?;
                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
            }
            _ => {
                warn!("Unknown command: {}", params.command);
                Err(tower_lsp::jsonrpc::Error::method_not_found())
            }
        }
    }
}

async fn resolve_cache_config_path(
    params: &ExecuteCommandParams,
    config: &tokio::sync::RwLock<Option<LspConfig>>,
) -> JsonRpcResult<String> {
    if !params.arguments.is_empty() {
        let request: CacheCommandParams = serde_json::from_value(params.arguments[0].clone())
            .map_err(|e| {
                tower_lsp::jsonrpc::Error::invalid_params(format!("Invalid parameters: {}", e))
            })?;
        if let Some(path) = request.configuration_path {
            return Ok(path);
        }
    }

    let config_guard = config.read().await;
    if let Some(cfg) = config_guard.as_ref() {
        if let Some(path) = cfg.configuration_path.clone() {
            return Ok(path);
        }
    }

    Err(tower_lsp::jsonrpc::Error::invalid_params(
        "Missing configuration path",
    ))
}

fn normalize_lsp_config(config: &mut LspConfig) {
    config.platform_docs_archive = normalize_optional_string(config.platform_docs_archive.clone());
    config.configuration_path = normalize_optional_string(config.configuration_path.clone());
    config.platform_version = normalize_optional_string(config.platform_version.clone());
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}
