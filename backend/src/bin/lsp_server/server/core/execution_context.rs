use super::*;

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
            return file_id;
        }

        let mut map = self.file_key_to_file_id_v2.write().await;
        if let Some(&file_id) = map.get(&key) {
            return file_id;
        }

        let raw = self.next_file_id_v2.fetch_add(1, Ordering::Relaxed);
        let file_id = V2FileId(raw);
        map.insert(key, file_id);
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
        let min_file_version = match self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied()
        {
            Some(version) => version,
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

                let file_content = match read_bsl_file(&path) {
                    Ok(content) => content,
                    Err(err) => {
                        warn!(
                            uri = %uri,
                            file_id = file_id.0,
                            operation = operation.as_str(),
                            error = %err,
                            "IntelliSense v2: failed to read file for fallback load"
                        );
                        return Err(bsl_runtime::application::SemanticOutcome::StaleVersion);
                    }
                };

                let path_string = path.to_string_lossy().to_string();
                self.analysis_v2
                    .apply_changes(vec![bsl_analysis_v2::Change::SetFile {
                        file_id,
                        text: Arc::from(file_content.clone()),
                        version: 0,
                        path: Arc::from(path_string.clone()),
                    }]);
                self.latest_received_file_versions_v2
                    .write()
                    .await
                    .insert(file_id, 0);
                self.latest_document_shadow_state_v2.write().await.insert(
                    file_id,
                    DocumentShadowStateV2 {
                        version: 0,
                        text: Arc::from(file_content),
                    },
                );
                0
            }
        };

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
            .prepare_stateful_operation(&context, Some(self.coordinator.as_ref()))
            .await?;

        Ok((context, prepared, min_file_version))
    }
}
