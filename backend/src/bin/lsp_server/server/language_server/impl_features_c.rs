use super::*;

impl BslLanguageServer {
    pub(super) async fn lsp_signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> JsonRpcResult<Option<SignatureHelp>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        {
            let file_id = self.get_or_create_file_id_v2(&uri).await;
            self.sync_v2_globals().await;

            let include_flow_sensitive = {
                let settings = self.settings.read().await;
                settings.enable_flow_sensitive
            };
            let prepared = self
                .prepare_lsp_stateful_operation_v2(
                    &uri,
                    file_id,
                    bsl_runtime::application::SemanticOperation::SignatureHelp,
                    include_flow_sensitive,
                )
                .await;
            let (context, prepared, expected_version) = match prepared {
                Ok(values) => values,
                Err(outcome) => {
                    super::helpers::record_lsp_interactive_fail_closed_reason(
                        self.coordinator.as_ref(),
                        "signature_help",
                        super::helpers::lsp_fail_closed_reason_from_prepare_outcome(outcome),
                    );
                    debug!(
                        uri = %uri,
                        file_id = file_id.0,
                        outcome = outcome.as_str(),
                        "SignatureHelp v2: stateful operation not ready"
                    );
                    return Ok(None);
                }
            };
            if let Some(wait_elapsed) = prepared.wait_elapsed {
                if let Some(threshold) = super::super::intellisense_v2_slow_wait_warn_threshold() {
                    if wait_elapsed >= threshold {
                        warn!(
                            uri = %uri,
                            file_id = file_id.0,
                            expected_version,
                            wait_ms = wait_elapsed.as_millis(),
                            threshold_ms = threshold.as_millis(),
                            "SignatureHelp v2: wait_for_file_version is slow"
                        );
                    }
                }
            }
            if let Some(threshold) = super::super::intellisense_v2_slow_snapshot_warn_threshold() {
                if prepared.snapshot_elapsed >= threshold {
                    warn!(
                        uri = %uri,
                        file_id = file_id.0,
                        snapshot_ms = prepared.snapshot_elapsed.as_millis(),
                        threshold_ms = threshold.as_millis(),
                        "SignatureHelp v2: snapshot acquisition is slow"
                    );
                }
            }

            let (analysis, file_content, deps, ir_program) = {
                let analysis = prepared.snapshot.analysis;
                let observed_file_version = analysis.file_version(file_id).ok().flatten();
                let observed_deps_id = Some(prepared.snapshot.deps_id);
                let observed_settings_id = analysis.settings_id().ok();
                debug!(
                    "SignatureHelp v2 observed: uri={}, file_id={}, file_version={:?}, deps_id={:?}, settings_id={:?}",
                    uri,
                    file_id.0,
                    observed_file_version,
                    observed_deps_id.as_ref().map(|v| v.as_str()),
                    observed_settings_id.as_ref().map(|v| v.as_str()),
                );

                let file_content = analysis.file_text(file_id).ok().flatten();
                let deps = analysis.deps_data().ok();
                let ir_program =
                    bsl_runtime::application::IntellisenseV2Facade::run_ir_query_singleflight(
                        &context,
                        &analysis,
                        Some(self.coordinator.as_ref()),
                        file_id,
                    )
                    .ok()
                    .flatten();

                (analysis, file_content, deps, ir_program)
            };

            let started = Instant::now();
            let result = match (file_content, deps, ir_program) {
                (Some(file_content), Some(deps), Some(ir_program)) => {
                    let exact_type_index_available = bsl_runtime::application::type_system::signature_help_exact_type_index_available_at_position(
                        file_content.as_ref(),
                        position.line,
                        position.character,
                        &analysis,
                        file_id,
                    );
                    let signature_help = handle_signature_help_v2(
                        &analysis,
                        file_id,
                        file_content,
                        position,
                        ir_program,
                        deps,
                        Some(self.coordinator.as_ref()),
                    );
                    if signature_help.is_none() && !exact_type_index_available {
                        super::helpers::record_lsp_interactive_fail_closed_reason(
                            self.coordinator.as_ref(),
                            "signature_help",
                            "missing_semantic_index",
                        );
                    }
                    signature_help
                }
                (None, _, _) | (Some(_), None, _) => {
                    super::helpers::record_lsp_interactive_fail_closed_reason(
                        self.coordinator.as_ref(),
                        "signature_help",
                        "unavailable_by_contract",
                    );
                    None
                }
                (Some(_), Some(_), None) => {
                    super::helpers::record_lsp_interactive_fail_closed_reason(
                        self.coordinator.as_ref(),
                        "signature_help",
                        "missing_canonical_ir",
                    );
                    None
                }
            };
            let elapsed = started.elapsed();
            self.coordinator.record_signature_help_latency(elapsed);
            if result.is_none() {
                self.coordinator.record_signature_help_empty();
            }
            Ok(result)
        }
    }

    // ========================================================================
    // COMMAND EXECUTION
    // ========================================================================

    pub(super) async fn lsp_execute_command(
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
                let prepared = self
                    .prepare_lsp_stateful_operation_v2(
                        &uri,
                        file_id,
                        bsl_runtime::application::SemanticOperation::SymbolSearch,
                        false,
                    )
                    .await;
                let (context, prepared, _expected_version) = match prepared {
                    Ok(values) => values,
                    Err(outcome) => {
                        warn!(
                            uri = %uri,
                            file_id = file_id.0,
                            outcome = outcome.as_str(),
                            "getSemanticHtml: stateful operation not ready"
                        );
                        return Err(tower_lsp::jsonrpc::Error::internal_error());
                    }
                };
                let analysis = prepared.snapshot.analysis;
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
                let ir_program =
                    bsl_runtime::application::IntellisenseV2Facade::run_ir_query_singleflight(
                        &context,
                        &analysis,
                        Some(self.coordinator.as_ref()),
                        file_id,
                    )
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
                let enable_flow_sensitive = {
                    let settings = self.settings.read().await;
                    settings.enable_flow_sensitive
                };
                let include_flow_sensitive = effective_include_flow_sensitive(
                    request.include_flow_sensitive,
                    enable_flow_sensitive,
                );
                let prepared = self
                    .prepare_lsp_stateful_operation_v2(
                        &uri,
                        file_id,
                        bsl_runtime::application::SemanticOperation::SymbolSearch,
                        include_flow_sensitive,
                    )
                    .await;
                let (context, prepared, _expected_version) = match prepared {
                    Ok(values) => values,
                    Err(outcome) => {
                        warn!(
                            uri = %uri,
                            file_id = file_id.0,
                            outcome = outcome.as_str(),
                            "getSemanticTree: stateful operation not ready"
                        );
                        return Err(tower_lsp::jsonrpc::Error::internal_error());
                    }
                };
                let analysis = prepared.snapshot.analysis;
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
                let ir_program =
                    bsl_runtime::application::IntellisenseV2Facade::run_ir_query_singleflight(
                        &context,
                        &analysis,
                        Some(self.coordinator.as_ref()),
                        file_id,
                    )
                    .ok()
                    .flatten()
                    .ok_or_else(tower_lsp::jsonrpc::Error::internal_error)?;

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
                let request = if params.arguments.is_empty() {
                    crate::types::ObservabilityMetricsRequest::default()
                } else {
                    serde_json::from_value(params.arguments[0].clone()).map_err(|e| {
                        tower_lsp::jsonrpc::Error::invalid_params(format!(
                            "Invalid parameters: {}",
                            e
                        ))
                    })?
                };
                let result = self.handle_get_observability_metrics(request).await?;
                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
            }
            "bsl.getCompletionTimeline" => {
                let request = if params.arguments.is_empty() {
                    crate::types::CompletionTimelineRequest::default()
                } else {
                    serde_json::from_value(params.arguments[0].clone()).map_err(|e| {
                        tower_lsp::jsonrpc::Error::invalid_params(format!(
                            "Invalid parameters: {}",
                            e
                        ))
                    })?
                };
                let result = self.handle_get_completion_timeline(request).await?;
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
