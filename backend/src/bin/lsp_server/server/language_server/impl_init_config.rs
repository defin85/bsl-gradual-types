use super::*;

impl BslLanguageServer {
    pub(super) async fn lsp_initialize(
        &self,
        params: InitializeParams,
    ) -> JsonRpcResult<InitializeResult> {
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
                        "bsl.getCompletionTimeline".to_string(),
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

    pub(super) async fn lsp_initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "BSL Language Server initialized!")
            .await;

        self.sync_formatting_capability_registration().await;
        self.sync_inlay_hints_capability_registration().await;
        self.sync_code_actions_capability_registration().await;

        let config = self.config.read().await.clone();
        if let Some(cfg) = config {
            if let Some(platform_docs) = cfg.platform_docs_archive.clone() {
                info!(
                    "Reloading types with platformDocsArchive: {}",
                    platform_docs
                );

                let startup_operation_id = match self
                    .begin_full_index_operation(
                        super::super::FullIndexOperationKind::Startup,
                        "Loading platform and configuration types",
                    )
                    .await
                {
                    super::super::command_handlers::BeginFullIndexOutcome::Started {
                        operation_id,
                    } => operation_id,
                    super::super::command_handlers::BeginFullIndexOutcome::AlreadyRunning {
                        active_operation,
                        operation_id,
                    } => {
                        warn!(
                            active_operation = ?active_operation.map(|op| op.as_str()),
                            operation_id = ?operation_id,
                            "startup full-index already running; attaching without duplicate launch"
                        );
                        return;
                    }
                };

                let server = self.clone();
                tokio::spawn(async move {
                    server
                        .run_startup_reload_task(cfg, platform_docs, startup_operation_id)
                        .await;
                });
            } else {
                info!("platformDocsArchive not provided - using basic types only");
            }
        }
    }

    async fn run_startup_reload_task(
        &self,
        cfg: crate::config::LspConfig,
        platform_docs: String,
        startup_operation_id: String,
    ) {
        let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<ProgressUpdate>();
        let (result_tx, result_rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

        info!("[LSP->Extension] Sending bsl/serverStatus: loading=true");
        let _ = self
            .client
            .send_notification::<ServerStatus>(ServerStatusParams::loading("Loading types..."))
            .await;

        let title = if cfg.configuration_path.is_some() {
            "Loading platform and configuration types".to_string()
        } else {
            "Loading platform types".to_string()
        };

        let mut reporter = LspWorkDoneReporter::create(self.client.clone(), "bsl-load-types").await;
        reporter.set_throttle_interval(std::time::Duration::from_millis(150));
        reporter
            .begin(title, Some("Initializing...".to_string()))
            .await;

        log_progress_to_file("[LSP->Extension] SEND WorkDoneProgressBegin");

        let client_clone = self.client.clone();
        let start_time = std::time::Instant::now();
        let self_clone = self.clone();

        tokio::spawn(async move {
            let mut reporter = reporter;

            while let Some(update) = progress_rx.recv().await {
                debug!(
                    "[RECV] {:?} {:.1}% ({}/{}) - {}",
                    update.phase,
                    update.percentage,
                    update.current,
                    update.total,
                    update.message.as_deref().unwrap_or("")
                );

                let elapsed = start_time.elapsed().as_secs_f32();
                let eta = if update.percentage > 5.0 {
                    Some(((elapsed * 100.0 / update.percentage) - elapsed) as u32)
                } else {
                    None
                };

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

            match result_rx.await {
                Ok(Ok(())) => {
                    reporter
                        .end(Some("Platform types loaded successfully".to_string()))
                        .await;

                    let _ = client_clone
                        .send_notification::<ServerStatus>(ServerStatusParams::ready())
                        .await;

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
                                super::super::V2FileKey::Path(path) => {
                                    Url::from_file_path(path).ok()
                                }
                                super::super::V2FileKey::Url(raw) => Url::parse(raw).ok(),
                            }
                        });

                        if let Some(uri) = uri {
                            let diagnostics_generation =
                                self_clone.bump_diagnostics_generation_v2(file_id).await;
                            for profile in
                                bsl_runtime::application::diagnostics_profiles_for_trigger(
                                    bsl_runtime::application::DiagnosticsTrigger::DidOpen,
                                )
                            {
                                self_clone
                                    .schedule_diagnostics_profile_v2(
                                        uri.clone(),
                                        file_id,
                                        version,
                                        diagnostics_generation,
                                        bsl_runtime::application::DiagnosticsTrigger::DidOpen,
                                        *profile,
                                        true,
                                    )
                                    .await;
                            }
                        }
                    }
                }
                Ok(Err(error_msg)) => {
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

        let inputs = StartupInputs::from_lsp_settings(
            Some(&platform_docs),
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
                self.finish_full_index_operation_success(
                    &startup_operation_id,
                    "Startup index ready",
                )
                .await;
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
                self.finish_full_index_operation_failed(
                    &startup_operation_id,
                    format!("Startup index failed: {e}"),
                )
                .await;
                let _ = result_tx.send(Err(e.to_string()));
                self.client
                    .log_message(
                        MessageType::ERROR,
                        format!("Failed to load platform documentation: {}", e),
                    )
                    .await;
            }
        }
    }

    pub(super) async fn lsp_shutdown(&self) -> JsonRpcResult<()> {
        info!("Shutting down BSL Language Server");
        self.cancel_all_type_index_precompute_v2().await;
        Ok(())
    }

    // ========================================================================
    // CONFIGURATION
    // ========================================================================

    pub(super) async fn lsp_did_change_configuration(&self, params: DidChangeConfigurationParams) {
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
}
