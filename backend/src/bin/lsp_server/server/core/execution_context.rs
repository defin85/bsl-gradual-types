use super::*;
use tower_lsp::lsp_types::Position;

impl BslLanguageServer {
    pub async fn update_diagnostics_count(&self, uri: &Url, count: usize) {
        let mut counts = self.diagnostics_counts.write().await;
        if count == 0 {
            counts.remove(uri);
        } else {
            counts.insert(uri.clone(), count);
        }
    }

    pub(crate) async fn get_or_create_file_id_v2(&self, uri: &Url) -> V2FileId {
        let key = match uri.to_file_path() {
            Ok(path) => V2FileKey::Path(path),
            Err(_) => V2FileKey::Url(uri.to_string()),
        };

        if let Some(&file_id) = self.file_key_to_file_id_v2.read().await.get(&key) {
            self.file_id_to_uri_v2
                .write()
                .await
                .insert(file_id, uri.clone());
            return file_id;
        }

        let mut map = self.file_key_to_file_id_v2.write().await;
        if let Some(&file_id) = map.get(&key) {
            drop(map);
            self.file_id_to_uri_v2
                .write()
                .await
                .insert(file_id, uri.clone());
            return file_id;
        }

        let raw = self.next_file_id_v2.fetch_add(1, Ordering::Relaxed);
        let file_id = V2FileId(raw);
        map.insert(key, file_id);
        drop(map);
        self.file_id_to_uri_v2
            .write()
            .await
            .insert(file_id, uri.clone());
        file_id
    }

    pub(crate) async fn get_file_id_v2(&self, uri: &Url) -> Option<V2FileId> {
        let key = match uri.to_file_path() {
            Ok(path) => V2FileKey::Path(path),
            Err(_) => V2FileKey::Url(uri.to_string()),
        };
        self.file_key_to_file_id_v2.read().await.get(&key).copied()
    }

    pub(crate) async fn build_execution_context_v2(
        &self,
        operation: bsl_runtime::application::SemanticOperation,
        file_id: V2FileId,
        min_file_version: Option<i32>,
        flow_sensitive: bool,
    ) -> bsl_runtime::application::ExecutionContext {
        self.build_execution_context_v2_with_completion_mode(
            operation,
            file_id,
            min_file_version,
            flow_sensitive,
            None,
        )
        .await
    }

    pub(crate) async fn build_execution_context_v2_with_completion_mode(
        &self,
        operation: bsl_runtime::application::SemanticOperation,
        file_id: V2FileId,
        min_file_version: Option<i32>,
        flow_sensitive: bool,
        completion_mode: Option<&'static str>,
    ) -> bsl_runtime::application::ExecutionContext {
        let settings = self.settings.read().await.clone();
        let settings_id = self
            .last_settings_id_v2
            .read()
            .await
            .clone()
            .unwrap_or_else(|| compute_settings_id_v2(&settings));
        let cancellation = match operation {
            bsl_runtime::application::SemanticOperation::Diagnostics => {
                bsl_runtime::application::CancellationPolicy::BestEffort
            }
            _ => bsl_runtime::application::CancellationPolicy::RespectClientAbort,
        };
        let completion_large_churn_active = matches!(
            operation,
            bsl_runtime::application::SemanticOperation::Completion
        ) && self
            .scale_aware_churn_state_v2
            .read()
            .await
            .get(&file_id)
            .is_some_and(|state| state.large_churn_active);
        let expected_deps_id = self.last_deps_id_v2.read().await.clone();

        bsl_runtime::application::ExecutionContext {
            origin: bsl_runtime::application::ObservabilityOrigin::Lsp,
            operation,
            completion_mode,
            completion_large_churn_active,
            file_id,
            min_file_version,
            expected_deps_id,
            flow_sensitive,
            settings: bsl_runtime::application::ExecutionSettings {
                settings_id,
                diagnostics_detail_level: bsl_shared::formatting::DetailLevel::parse(
                    &settings.diagnostics.detail_level,
                ),
            },
            cancellation,
        }
    }

    pub(crate) async fn prepare_lsp_stateful_operation_v2(
        &self,
        uri: &Url,
        file_id: V2FileId,
        operation: bsl_runtime::application::SemanticOperation,
        flow_sensitive: bool,
    ) -> Result<
        (
            bsl_runtime::application::ExecutionContext,
            bsl_runtime::application::PreparedOperationSnapshot,
            i32,
        ),
        bsl_runtime::application::SemanticOutcome,
    > {
        self.prepare_lsp_stateful_operation_v2_with_completion_mode(
            uri,
            file_id,
            operation,
            flow_sensitive,
            None,
        )
        .await
    }

    pub(crate) async fn resolve_or_seed_min_file_version_v2(
        &self,
        uri: &Url,
        file_id: V2FileId,
        operation: bsl_runtime::application::SemanticOperation,
    ) -> Result<i32, bsl_runtime::application::SemanticOutcome> {
        match self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied()
        {
            Some(version) => Ok(version),
            None => {
                let path = match uri.to_file_path() {
                    Ok(path) => path,
                    Err(_) => {
                        warn!(
                            uri = %uri,
                            file_id = file_id.0,
                            operation = operation.as_str(),
                            "IntelliSense v2: missing local file path for fallback load"
                        );
                        return Err(bsl_runtime::application::SemanticOutcome::StaleVersion);
                    }
                };

                let path_for_read = path.clone();
                let file_content = match tokio::task::spawn_blocking(move || {
                    read_bsl_file(&path_for_read)
                })
                .await
                {
                    Ok(Ok(content)) => content,
                    Ok(Err(err)) => {
                        warn!(
                            uri = %uri,
                            file_id = file_id.0,
                            operation = operation.as_str(),
                            error = %err,
                            "IntelliSense v2: failed to read file for fallback load"
                        );
                        return Err(bsl_runtime::application::SemanticOutcome::StaleVersion);
                    }
                    Err(err) => {
                        warn!(
                            uri = %uri,
                            file_id = file_id.0,
                            operation = operation.as_str(),
                            error = %err,
                            "IntelliSense v2: fallback file-read task join failed"
                        );
                        return Err(bsl_runtime::application::SemanticOutcome::StaleVersion);
                    }
                };

                let path_string = path.to_string_lossy().to_string();
                self.analysis_v2.apply_changes_interactive(
                    bsl_runtime::application::ObservabilityOrigin::Lsp,
                    vec![bsl_analysis_v2::Change::SetFile {
                        file_id,
                        text: Arc::from(file_content.clone()),
                        version: 0,
                        path: Arc::from(path_string.clone()),
                    }],
                );
                let handoff_registered_at = Instant::now();
                self.latest_current_revision_handoff_versions_v2
                    .write()
                    .await
                    .insert(file_id, 0);
                self.latest_received_file_versions_v2
                    .write()
                    .await
                    .insert(file_id, 0);
                self.latest_apply_enqueued_at_v2
                    .write()
                    .await
                    .insert(file_id, handoff_registered_at);
                self.latest_document_shadow_state_v2.write().await.insert(
                    file_id,
                    DocumentShadowStateV2 {
                        version: 0,
                        text: Arc::from(file_content),
                    },
                );
                self.publish_same_file_ingress_token_v2(
                    file_id,
                    0,
                    super::super::SameFileIngressTokenSourceV2::Other,
                )
                .await;
                Ok(0)
            }
        }
    }

    pub(crate) async fn prepare_lsp_completion_first_response_v2_with_completion_mode_and_progress(
        &self,
        uri: &Url,
        file_id: V2FileId,
        position: Position,
        flow_sensitive: bool,
        completion_mode: Option<&'static str>,
        progress: Option<&bsl_runtime::application::PrepareStatefulProgress>,
    ) -> Result<
        (
            bsl_runtime::application::ExecutionContext,
            bsl_runtime::application::PreparedCompletionFirstResponse,
            i32,
        ),
        bsl_runtime::application::SemanticOutcome,
    > {
        let min_file_version = self
            .resolve_or_seed_min_file_version_v2(
                uri,
                file_id,
                bsl_runtime::application::SemanticOperation::Completion,
            )
            .await?;
        let context = self
            .build_execution_context_v2_with_completion_mode(
                bsl_runtime::application::SemanticOperation::Completion,
                file_id,
                Some(min_file_version),
                flow_sensitive,
                completion_mode,
            )
            .await;
        let prepared = self
            .analysis_v2
            .prepare_completion_first_response_with_progress(
                &context,
                Some(self.coordinator.as_ref()),
                progress,
                position.line,
                position.character,
            )
            .await?;

        Ok((context, prepared, min_file_version))
    }

    pub(crate) async fn prepare_lsp_stateful_operation_v2_with_completion_mode_and_progress(
        &self,
        uri: &Url,
        file_id: V2FileId,
        operation: bsl_runtime::application::SemanticOperation,
        flow_sensitive: bool,
        completion_mode: Option<&'static str>,
        progress: Option<&bsl_runtime::application::PrepareStatefulProgress>,
    ) -> Result<
        (
            bsl_runtime::application::ExecutionContext,
            bsl_runtime::application::PreparedOperationSnapshot,
            i32,
        ),
        bsl_runtime::application::SemanticOutcome,
    > {
        // latest_received tracks the freshest transport revision. prepare_stateful_operation
        // still waits for runtime applied_version to reach this bound before treating the
        // semantic snapshot as ready.
        let min_file_version = self
            .resolve_or_seed_min_file_version_v2(uri, file_id, operation)
            .await?;

        let context = self
            .build_execution_context_v2_with_completion_mode(
                operation,
                file_id,
                Some(min_file_version),
                flow_sensitive,
                completion_mode,
            )
            .await;
        let prepared = self
            .analysis_v2
            .prepare_stateful_operation_with_progress(
                &context,
                Some(self.coordinator.as_ref()),
                progress,
            )
            .await?;

        Ok((context, prepared, min_file_version))
    }

    pub(crate) async fn prepare_lsp_stateful_operation_v2_with_completion_mode(
        &self,
        uri: &Url,
        file_id: V2FileId,
        operation: bsl_runtime::application::SemanticOperation,
        flow_sensitive: bool,
        completion_mode: Option<&'static str>,
    ) -> Result<
        (
            bsl_runtime::application::ExecutionContext,
            bsl_runtime::application::PreparedOperationSnapshot,
            i32,
        ),
        bsl_runtime::application::SemanticOutcome,
    > {
        self.prepare_lsp_stateful_operation_v2_with_completion_mode_and_progress(
            uri,
            file_id,
            operation,
            flow_sensitive,
            completion_mode,
            None,
        )
        .await
    }
}
