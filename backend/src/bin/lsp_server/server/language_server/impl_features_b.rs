use std::time::Duration;

use super::*;

struct HoverObservedSnapshotV2 {
    analysis: bsl_analysis_v2::AnalysisV2,
    file_content: Option<Arc<str>>,
    file_path: Option<Arc<str>>,
    deps: Option<Arc<bsl_analysis_v2::SemanticDeps>>,
    ir_program: Option<Arc<bsl_shared::ir::SemanticProgram>>,
    exact_type_index_available: bool,
}

struct DefinitionObservedSnapshotV2 {
    analysis: bsl_analysis_v2::AnalysisV2,
    file_content: Option<Arc<str>>,
    file_path: Option<Arc<str>>,
    deps: Option<Arc<bsl_analysis_v2::SemanticDeps>>,
    ir_program: Option<Arc<bsl_shared::ir::SemanticProgram>>,
    exact_type_index_available: bool,
}

impl BslLanguageServer {
    fn build_hover_observed_snapshot_v2(
        &self,
        context: &bsl_runtime::application::ExecutionContext,
        analysis: bsl_analysis_v2::AnalysisV2,
        file_id: bsl_analysis_v2::FileId,
        position: Position,
        uri: &Url,
        index_snapshot_id: Option<&str>,
    ) -> HoverObservedSnapshotV2 {
        let observed_deps_id = analysis.deps_id().ok();
        let observed_file_version = analysis.file_version(file_id).ok().flatten();
        let observed_settings_id = analysis.settings_id().ok();
        debug!(
            "Hover v2 observed: uri={}, file_id={}, file_version={:?}, deps_id={:?}, settings_id={:?}, index_snapshot_id={}",
            uri,
            file_id.0,
            observed_file_version,
            observed_deps_id.as_ref().map(|v| v.as_str()),
            observed_settings_id.as_ref().map(|v| v.as_str()),
            index_snapshot_id.unwrap_or("unavailable"),
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
            uri, file_id.0, position.line, position.character, observed_byte_offset, observed_point,
        );
        if let Some(offset) = observed_byte_offset {
            let offset = offset.min(u32::MAX as usize) as u32;
            if let Ok(profiled) = analysis.type_at_byte_offset_serve_only_profiled(file_id, offset)
            {
                self.coordinator
                    .record_intellisense_v2_type_index_reason(profiled.serve_reason_code.as_str());
            }
        }

        let file_content = analysis.file_text(file_id).ok().flatten();
        let file_path = analysis.file_path(file_id).ok().flatten();
        let deps = analysis.deps_data().ok();
        let ir_started = Instant::now();
        let ir_program = bsl_runtime::application::IntellisenseV2Facade::run_ir_query_singleflight(
            context,
            &analysis,
            Some(self.coordinator.as_ref()),
            file_id,
        )
        .ok()
        .flatten();
        let ir_elapsed = ir_started.elapsed();
        if let Some(threshold) = super::super::intellisense_v2_slow_query_warn_threshold() {
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

        let exact_type_index_available = file_content
            .as_ref()
            .zip(ir_program.as_ref())
            .is_some_and(|(file_content, ir_program)| {
                bsl_runtime::application::type_system::hover_exact_type_index_available_at_position(
                    &analysis,
                    file_id,
                    file_content.as_ref(),
                    position.line,
                    position.character,
                    ir_program.as_ref(),
                )
            });

        HoverObservedSnapshotV2 {
            analysis,
            file_content,
            file_path,
            deps,
            ir_program,
            exact_type_index_available,
        }
    }

    fn lsp_interactive_wait_budget_v2() -> Duration {
        Duration::from_millis(
            bsl_runtime::system::global_runtime_config()
                .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
                .unwrap_or(120),
        )
    }

    pub(super) async fn wait_for_lsp_exact_consumer_analysis_v2(
        &self,
        analysis: bsl_analysis_v2::AnalysisV2,
        file_id: bsl_analysis_v2::FileId,
        expected_version: i32,
    ) -> Option<bsl_analysis_v2::AnalysisV2> {
        let observed_version = analysis.file_version(file_id).ok().flatten();
        if observed_version == Some(expected_version)
            && analysis
                .current_type_index_serve_only_ready(file_id)
                .ok()
                .unwrap_or(false)
        {
            return Some(analysis);
        }
        self.wait_for_lsp_exact_type_index_snapshot_v2(file_id, expected_version)
            .await
    }

    pub(super) async fn wait_for_lsp_exact_type_index_snapshot_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        expected_version: i32,
    ) -> Option<bsl_analysis_v2::AnalysisV2> {
        let wait_budget = Self::lsp_interactive_wait_budget_v2();

        if !self
            .has_matching_type_index_precompute_task_v2(file_id, Some(expected_version))
            .await
        {
            self.schedule_type_index_precompute_v2(file_id, expected_version)
                .await;
        }
        let wait_trace = self
            .wait_for_current_type_index_serve_only_ready_v2(
                file_id,
                Some(expected_version),
                wait_budget,
            )
            .await;
        if wait_trace.outcome == super::super::core::ExactTypeIndexWaitOutcomeV2::Deadline {
            self.coordinator
                .record_intellisense_v2_interactive_wait_budget_exhausted();
        }
        if wait_trace.outcome != super::super::core::ExactTypeIndexWaitOutcomeV2::Ready {
            return None;
        }

        let analysis = self.analysis_v2.snapshot().await;
        let observed_version = analysis.file_version(file_id).ok().flatten();
        if observed_version != Some(expected_version) {
            return None;
        }
        analysis
            .current_type_index_serve_only_ready(file_id)
            .ok()
            .unwrap_or(false)
            .then_some(analysis)
    }

    fn build_definition_observed_snapshot_v2(
        &self,
        context: &bsl_runtime::application::ExecutionContext,
        analysis: bsl_analysis_v2::AnalysisV2,
        file_id: bsl_analysis_v2::FileId,
        position: Position,
        uri: &Url,
        index_snapshot_id: Option<&str>,
    ) -> DefinitionObservedSnapshotV2 {
        let observed_file_version = analysis.file_version(file_id).ok().flatten();
        let observed_deps_id = analysis.deps_id().ok();
        let observed_settings_id = analysis.settings_id().ok();
        debug!(
            "Definition v2 observed: uri={}, file_id={}, file_version={:?}, deps_id={:?}, settings_id={:?}, index_snapshot_id={}",
            uri,
            file_id.0,
            observed_file_version,
            observed_deps_id.as_ref().map(|value| value.as_str()),
            observed_settings_id.as_ref().map(|value| value.as_str()),
            index_snapshot_id.unwrap_or("unavailable"),
        );

        let file_content = analysis.file_text(file_id).ok().flatten();
        let file_path = analysis.file_path(file_id).ok().flatten();
        let deps = analysis.deps_data().ok();
        let ir_started = Instant::now();
        let ir_program = bsl_runtime::application::IntellisenseV2Facade::run_ir_query_singleflight(
            context,
            &analysis,
            Some(self.coordinator.as_ref()),
            file_id,
        )
        .ok()
        .flatten();
        let ir_elapsed = ir_started.elapsed();
        if let Some(threshold) = super::super::intellisense_v2_slow_query_warn_threshold() {
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

        let exact_type_index_available =
            bsl_runtime::application::type_system::definition_exact_type_index_available_at_position(
                &analysis,
                file_id,
                position.line,
                position.character,
            );

        DefinitionObservedSnapshotV2 {
            analysis,
            file_content,
            file_path,
            deps,
            ir_program,
            exact_type_index_available,
        }
    }

    pub(super) async fn lsp_completion_resolve(
        &self,
        item: CompletionItem,
    ) -> JsonRpcResult<CompletionItem> {
        let snippet_support = *self.completion_snippet_support.read().await;
        let started = Instant::now();
        let (deps, resolve_context_matches) = if let Some(resolve_context) =
            crate::handlers::parse_completion_resolve_context(&item)
        {
            let deps = if let Ok(resolve_uri) = Url::parse(&resolve_context.file_uri) {
                if let Some(file_id) = self.get_file_id_v2(&resolve_uri).await {
                    let analysis = self.analysis_v2.snapshot().await;
                    let current_file_version = self
                        .latest_received_file_versions_v2
                        .read()
                        .await
                        .get(&file_id)
                        .copied()
                        .or_else(|| analysis.file_version(file_id).ok().flatten());
                    let current_deps_id = analysis.deps_id().ok();
                    let current_settings_id = analysis.settings_id().ok();
                    let matches_context = current_file_version
                        == Some(resolve_context.file_version)
                        && current_deps_id
                            .as_ref()
                            .is_some_and(|value| value.as_str() == resolve_context.deps_id)
                        && current_settings_id.as_ref().map(|value| value.as_str())
                            == resolve_context.settings_id.as_deref();
                    (analysis.deps_data().ok(), matches_context)
                } else {
                    (None, false)
                }
            } else {
                (None, false)
            };
            deps
        } else {
            (None, false)
        };
        let resolved = if resolve_context_matches {
            handle_completion_resolve(item, deps, snippet_support).await
        } else {
            item
        };
        let elapsed = started.elapsed();
        self.coordinator.record_completion_resolve_latency(elapsed);
        Ok(resolved)
    }

    pub(super) async fn lsp_hover(&self, params: HoverParams) -> JsonRpcResult<Option<Hover>> {
        let _method_entered_at_ms = super::super::unix_timestamp_ms();
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        info!(
            "Hover requested at {}:{}",
            position.line, position.character
        );

        let _handler_entered_at_ms = super::super::unix_timestamp_ms();
        let result = {
            let file_id = self.get_or_create_file_id_v2(&uri).await;
            self.sync_v2_globals().await;

            let include_flow_sensitive = {
                let settings = self.settings.read().await;
                settings.enable_flow_sensitive
            };
            match self
                .prepare_lsp_stateful_operation_v2(
                    &uri,
                    file_id,
                    bsl_runtime::application::SemanticOperation::Hover,
                    include_flow_sensitive,
                )
                .await
            {
                Ok((context, prepared, expected_version)) => {
                    if let Some(wait_elapsed) = prepared.wait_elapsed {
                        if let Some(threshold) =
                            super::super::intellisense_v2_slow_wait_warn_threshold()
                        {
                            if wait_elapsed >= threshold {
                                warn!(
                                    uri = %uri,
                                    file_id = file_id.0,
                                    expected_version,
                                    wait_ms = wait_elapsed.as_millis(),
                                    threshold_ms = threshold.as_millis(),
                                    "Hover v2: wait_for_file_version is slow"
                                );
                            }
                        }
                    }
                    if let Some(threshold) =
                        super::super::intellisense_v2_slow_snapshot_warn_threshold()
                    {
                        if prepared.snapshot_elapsed >= threshold {
                            warn!(
                                uri = %uri,
                                file_id = file_id.0,
                                snapshot_ms = prepared.snapshot_elapsed.as_millis(),
                                threshold_ms = threshold.as_millis(),
                                "Hover v2: snapshot acquisition is slow"
                            );
                        }
                    }

                    if let Some(exact_analysis) = self
                        .wait_for_lsp_exact_consumer_analysis_v2(
                            prepared.snapshot.analysis,
                            file_id,
                            expected_version,
                        )
                        .await
                    {
                        let observed = self.build_hover_observed_snapshot_v2(
                            &context,
                            exact_analysis,
                            file_id,
                            position,
                            &uri,
                            Some(prepared.index_snapshot.id.as_str()),
                        );

                        let settings = self.settings.read().await;
                        let result = match (
                            observed.file_content,
                            observed.file_path,
                            observed.deps,
                            observed.ir_program,
                        ) {
                            (Some(file_content), Some(file_path), Some(deps), Some(ir_program)) => {
                                let result = handle_hover_v2(
                                    &observed.analysis,
                                    file_id,
                                    file_content,
                                    file_path,
                                    ir_program,
                                    deps,
                                    position,
                                    &uri,
                                    &settings.hover,
                                    include_flow_sensitive,
                                );
                                if result.is_none() && !observed.exact_type_index_available {
                                    super::helpers::record_lsp_interactive_fail_closed_reason(
                                        self.coordinator.as_ref(),
                                        "hover",
                                        "missing_semantic_index",
                                    );
                                }
                                result
                            }
                            (None, _, _, _)
                            | (Some(_), None, _, _)
                            | (Some(_), Some(_), None, _) => {
                                super::helpers::record_lsp_interactive_fail_closed_reason(
                                    self.coordinator.as_ref(),
                                    "hover",
                                    "unavailable_by_contract",
                                );
                                None
                            }
                            (Some(_), Some(_), Some(_), None) => {
                                super::helpers::record_lsp_interactive_fail_closed_reason(
                                    self.coordinator.as_ref(),
                                    "hover",
                                    "missing_canonical_ir",
                                );
                                None
                            }
                        };

                        Ok(result)
                    } else {
                        super::helpers::record_lsp_interactive_fail_closed_reason(
                            self.coordinator.as_ref(),
                            "hover",
                            "missing_semantic_index",
                        );
                        Ok(None)
                    }
                }
                Err(outcome) => {
                    super::helpers::record_lsp_interactive_fail_closed_reason(
                        self.coordinator.as_ref(),
                        "hover",
                        super::helpers::lsp_fail_closed_reason_from_prepare_outcome(outcome),
                    );
                    debug!(
                        uri = %uri,
                        file_id = file_id.0,
                        outcome = outcome.as_str(),
                        "Hover v2: stateful operation not ready"
                    );
                    Ok(None)
                }
            }
        };

        #[cfg(test)]
        super::helpers::record_current_request_server_edge_trace_for_testing(
            "textDocument/hover",
            &uri,
            _method_entered_at_ms,
            _handler_entered_at_ms,
            super::super::unix_timestamp_ms(),
        );

        result
    }

    pub(super) async fn lsp_inlay_hint(
        &self,
        params: InlayHintParams,
    ) -> JsonRpcResult<Option<Vec<InlayHint>>> {
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

        self.sync_v2_globals().await;
        let file_id = self.get_or_create_file_id_v2(&uri).await;
        let include_flow_sensitive = {
            let guard = self.settings.read().await;
            guard.enable_flow_sensitive
        };
        let prepared = self
            .prepare_lsp_stateful_operation_v2(
                &uri,
                file_id,
                bsl_runtime::application::SemanticOperation::TypeAtPosition,
                include_flow_sensitive,
            )
            .await;
        let (context, prepared, _expected_version) = match prepared {
            Ok(values) => values,
            Err(outcome) => {
                debug!(
                    uri = %uri,
                    file_id = file_id.0,
                    outcome = outcome.as_str(),
                    "Inlay hints v2: stateful operation not ready"
                );
                return Ok(None);
            }
        };
        let analysis = prepared.snapshot.analysis;
        let Some(file_content) = analysis.file_text(file_id).ok().flatten() else {
            return Ok(None);
        };
        let ir_program = bsl_runtime::application::IntellisenseV2Facade::run_ir_query_singleflight(
            &context,
            &analysis,
            Some(self.coordinator.as_ref()),
            file_id,
        )
        .ok()
        .flatten();
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

    pub(super) async fn lsp_code_action(
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

        self.sync_v2_globals().await;
        let file_id = self.get_or_create_file_id_v2(&uri).await;
        let include_flow_sensitive = {
            let guard = self.settings.read().await;
            guard.enable_flow_sensitive
        };
        let prepared = self
            .prepare_lsp_stateful_operation_v2(
                &uri,
                file_id,
                bsl_runtime::application::SemanticOperation::TypeAtPosition,
                include_flow_sensitive,
            )
            .await;
        let (context, prepared, _expected_version) = match prepared {
            Ok(values) => values,
            Err(outcome) => {
                debug!(
                    uri = %uri,
                    file_id = file_id.0,
                    outcome = outcome.as_str(),
                    "Code actions v2: stateful operation not ready"
                );
                return Ok(None);
            }
        };
        let analysis = prepared.snapshot.analysis;
        let Some(file_content) = analysis.file_text(file_id).ok().flatten() else {
            return Ok(None);
        };
        let ir_program = bsl_runtime::application::IntellisenseV2Facade::run_ir_query_singleflight(
            &context,
            &analysis,
            Some(self.coordinator.as_ref()),
            file_id,
        )
        .ok()
        .flatten();
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

    pub(super) async fn lsp_goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> JsonRpcResult<Option<GotoDefinitionResponse>> {
        let _method_entered_at_ms = super::super::unix_timestamp_ms();
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let _handler_entered_at_ms = super::super::unix_timestamp_ms();
        let result = {
            let file_id = self.get_or_create_file_id_v2(&uri).await;
            self.sync_v2_globals().await;

            let include_flow_sensitive = {
                let settings = self.settings.read().await;
                settings.enable_flow_sensitive
            };
            match self
                .prepare_lsp_stateful_operation_v2(
                    &uri,
                    file_id,
                    bsl_runtime::application::SemanticOperation::Definition,
                    include_flow_sensitive,
                )
                .await
            {
                Ok((context, prepared, expected_version)) => {
                    if let Some(wait_elapsed) = prepared.wait_elapsed {
                        if let Some(threshold) =
                            super::super::intellisense_v2_slow_wait_warn_threshold()
                        {
                            if wait_elapsed >= threshold {
                                warn!(
                                    uri = %uri,
                                    file_id = file_id.0,
                                    expected_version,
                                    wait_ms = wait_elapsed.as_millis(),
                                    threshold_ms = threshold.as_millis(),
                                    "Definition v2: wait_for_file_version is slow"
                                );
                            }
                        }
                    }
                    if let Some(threshold) =
                        super::super::intellisense_v2_slow_snapshot_warn_threshold()
                    {
                        if prepared.snapshot_elapsed >= threshold {
                            warn!(
                                uri = %uri,
                                file_id = file_id.0,
                                snapshot_ms = prepared.snapshot_elapsed.as_millis(),
                                threshold_ms = threshold.as_millis(),
                                "Definition v2: snapshot acquisition is slow"
                            );
                        }
                    }

                    if let Some(exact_analysis) = self
                        .wait_for_lsp_exact_consumer_analysis_v2(
                            prepared.snapshot.analysis,
                            file_id,
                            expected_version,
                        )
                        .await
                    {
                        let observed = self.build_definition_observed_snapshot_v2(
                            &context,
                            exact_analysis,
                            file_id,
                            position,
                            &uri,
                            Some(prepared.index_snapshot.id.as_str()),
                        );

                        let result = match (
                            observed.file_content,
                            observed.file_path,
                            observed.deps,
                            observed.ir_program,
                        ) {
                            (Some(file_content), Some(file_path), Some(deps), Some(ir_program)) => {
                                if !observed.exact_type_index_available {
                                    super::helpers::record_lsp_interactive_fail_closed_reason(
                                        self.coordinator.as_ref(),
                                        "definition",
                                        "missing_semantic_index",
                                    );
                                    None
                                } else {
                                    handle_goto_definition_v2(
                                        crate::handlers::definition::GotoDefinitionRequest {
                                            analysis: &observed.analysis,
                                            file_id,
                                            file_path,
                                            file_content,
                                            ir_program,
                                            deps,
                                            position,
                                            uri: &uri,
                                            coordinator: Some(self.coordinator.as_ref()),
                                        },
                                    )
                                }
                            }
                            (None, _, _, _)
                            | (Some(_), None, _, _)
                            | (Some(_), Some(_), None, _) => {
                                super::helpers::record_lsp_interactive_fail_closed_reason(
                                    self.coordinator.as_ref(),
                                    "definition",
                                    "unavailable_by_contract",
                                );
                                None
                            }
                            (Some(_), Some(_), Some(_), None) => {
                                super::helpers::record_lsp_interactive_fail_closed_reason(
                                    self.coordinator.as_ref(),
                                    "definition",
                                    "missing_canonical_ir",
                                );
                                None
                            }
                        };

                        Ok(result)
                    } else {
                        super::helpers::record_lsp_interactive_fail_closed_reason(
                            self.coordinator.as_ref(),
                            "definition",
                            "missing_semantic_index",
                        );
                        Ok(None)
                    }
                }
                Err(outcome) => {
                    super::helpers::record_lsp_interactive_fail_closed_reason(
                        self.coordinator.as_ref(),
                        "definition",
                        super::helpers::lsp_fail_closed_reason_from_prepare_outcome(outcome),
                    );
                    debug!(
                        uri = %uri,
                        file_id = file_id.0,
                        outcome = outcome.as_str(),
                        "Definition v2: stateful operation not ready"
                    );
                    Ok(None)
                }
            }
        };

        #[cfg(test)]
        super::helpers::record_current_request_server_edge_trace_for_testing(
            "textDocument/definition",
            &uri,
            _method_entered_at_ms,
            _handler_entered_at_ms,
            super::super::unix_timestamp_ms(),
        );

        result
    }
}
