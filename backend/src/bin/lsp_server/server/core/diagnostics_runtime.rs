use super::*;

impl BslLanguageServer {
    pub(crate) async fn cancel_diagnostics_v2(&self, file_id: V2FileId) {
        let mut tasks = self.diagnostics_tasks_v2.lock().await;
        let keys: Vec<super::super::DiagnosticsTaskKeyV2> = tasks
            .keys()
            .copied()
            .filter(|key| key.file_id == file_id)
            .collect();
        for key in keys {
            if let Some(task) = tasks.remove(&key) {
                task.cancel_token
                    .cancel(super::super::DiagnosticsCancellationReasonV2::ClientCancel);
                self.record_diagnostics_pipeline_event_v2(
                    task.trigger,
                    task.supersession_key.profile,
                    bsl_runtime::application::DiagnosticsDisposition::ClientCancel,
                );
                task.handle.abort();
            }
        }
    }

    pub(crate) async fn bump_diagnostics_generation_v2(&self, file_id: V2FileId) -> u64 {
        let mut generations = self.diagnostics_generation_v2.write().await;
        let next = generations
            .get(&file_id)
            .copied()
            .unwrap_or(0)
            .saturating_add(1);
        generations.insert(file_id, next);
        next
    }

    async fn current_diagnostics_generation_v2(&self, file_id: V2FileId) -> Option<u64> {
        self.diagnostics_generation_v2
            .read()
            .await
            .get(&file_id)
            .copied()
    }

    fn record_diagnostics_pipeline_event_v2(
        &self,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        profile: bsl_runtime::application::DiagnosticsProfile,
        reason: bsl_runtime::application::DiagnosticsDisposition,
    ) {
        self.coordinator
            .record_intellisense_v2_diagnostics_pipeline_event(
                bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                trigger.as_str(),
                profile.as_str(),
                reason.as_str(),
            );
    }

    fn diagnostics_cancelled_disposition_v2(
        &self,
        cancel_token: Option<&super::super::DiagnosticsCancellationTokenV2>,
        current_generation: Option<u64>,
        expected_generation: u64,
        current_version: Option<i32>,
        expected_version: i32,
    ) -> bsl_runtime::application::DiagnosticsDisposition {
        if let Some(token) = cancel_token {
            if token.is_cancelled() {
                return token.reason().to_disposition();
            }
        }
        if current_generation != Some(expected_generation) {
            return bsl_runtime::application::DiagnosticsDisposition::SupersededGeneration;
        }
        if current_version != Some(expected_version) {
            return bsl_runtime::application::DiagnosticsDisposition::SupersededVersion;
        }
        bsl_runtime::application::DiagnosticsDisposition::OtherCancel
    }

    async fn diagnostics_checkpoint_v2(
        &self,
        key: &super::super::DiagnosticsSupersessionKeyV2,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        cancel_token: Option<&super::super::DiagnosticsCancellationTokenV2>,
    ) -> Option<bsl_runtime::application::DiagnosticsDisposition> {
        let current_generation = self.current_diagnostics_generation_v2(key.file_id).await;
        let current_version = self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&key.file_id)
            .copied();
        let disposition = if let Some(token) = cancel_token {
            if token.is_cancelled() {
                token.reason().to_disposition()
            } else {
                self.diagnostics_cancelled_disposition_v2(
                    None,
                    current_generation,
                    key.diagnostics_generation,
                    current_version,
                    key.requested_version,
                )
            }
        } else {
            self.diagnostics_cancelled_disposition_v2(
                None,
                current_generation,
                key.diagnostics_generation,
                current_version,
                key.requested_version,
            )
        };
        if matches!(
            disposition,
            bsl_runtime::application::DiagnosticsDisposition::OtherCancel
        ) && current_generation == Some(key.diagnostics_generation)
            && current_version == Some(key.requested_version)
        {
            return None;
        }
        self.record_diagnostics_pipeline_event_v2(trigger, key.profile, disposition);
        Some(disposition)
    }

    async fn diagnostics_publish_checkpoint_v2(
        &self,
        key: &super::super::DiagnosticsSupersessionKeyV2,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        cancel_token: Option<&super::super::DiagnosticsCancellationTokenV2>,
        observed_deps_id: Option<&str>,
        observed_settings_id: Option<&str>,
    ) -> Option<bsl_runtime::application::DiagnosticsDisposition> {
        if let Some(disposition) = self
            .diagnostics_checkpoint_v2(key, trigger, cancel_token)
            .await
        {
            return Some(disposition);
        }

        let current_deps_id = self
            .last_deps_id_v2
            .read()
            .await
            .as_ref()
            .map(|id| id.as_str().to_string());
        let current_settings_id = self
            .last_settings_id_v2
            .read()
            .await
            .as_ref()
            .map(|id| id.as_str().to_string());

        if current_deps_id.as_deref() != observed_deps_id
            || current_settings_id.as_deref() != observed_settings_id
        {
            let disposition =
                bsl_runtime::application::DiagnosticsDisposition::SupersededGeneration;
            self.record_diagnostics_pipeline_event_v2(trigger, key.profile, disposition);
            return Some(disposition);
        }

        None
    }

    pub(crate) async fn run_diagnostics_profile_immediate_v2(
        &self,
        uri: Url,
        file_id: V2FileId,
        expected_version: i32,
        diagnostics_generation: u64,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        profile: bsl_runtime::application::DiagnosticsProfile,
    ) {
        let supersession_key = super::super::DiagnosticsSupersessionKeyV2 {
            file_id,
            profile,
            diagnostics_generation,
            requested_version: expected_version,
        };
        let _ = self
            .execute_diagnostics_profile_once_v2(&uri, supersession_key, trigger, None)
            .await;
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn schedule_diagnostics_profile_v2(
        &self,
        uri: Url,
        file_id: V2FileId,
        expected_version: i32,
        diagnostics_generation: u64,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        profile: bsl_runtime::application::DiagnosticsProfile,
        debounce: bool,
    ) {
        let slot_key = super::super::DiagnosticsTaskKeyV2 { file_id, profile };
        let supersession_key = super::super::DiagnosticsSupersessionKeyV2 {
            file_id,
            profile,
            diagnostics_generation,
            requested_version: expected_version,
        };
        let mut tasks = self.diagnostics_tasks_v2.lock().await;
        if let Some(task) = tasks.get_mut(&slot_key) {
            if task.supersession_key != supersession_key {
                let reason = super::super::DiagnosticsCancellationReasonV2::for_supersession(
                    task.supersession_key,
                    diagnostics_generation,
                    expected_version,
                );
                task.cancel_token.cancel(reason);
                self.record_diagnostics_pipeline_event_v2(
                    task.trigger,
                    task.supersession_key.profile,
                    reason.to_disposition(),
                );
                task.cancel_token = super::super::DiagnosticsCancellationTokenV2::new();
                task.supersession_key = supersession_key;
            }
            task.trigger = trigger;
            task.debounce = debounce;
            return;
        }

        let server = self.clone();
        let uri_for_task = uri.clone();
        let initial_cancel_token = super::super::DiagnosticsCancellationTokenV2::new();
        let handle = tokio::spawn(async move {
            loop {
                // If the document is already closed, stop.
                let file_still_open = server
                    .latest_received_file_versions_v2
                    .read()
                    .await
                    .contains_key(&file_id);
                if !file_still_open {
                    break;
                }

                let (supersession_key, trigger, debounce, cancel_token) = {
                    let tasks = server.diagnostics_tasks_v2.lock().await;
                    let Some(task) = tasks.get(&slot_key) else {
                        break;
                    };
                    (
                        task.supersession_key,
                        task.trigger,
                        task.debounce,
                        task.cancel_token.clone(),
                    )
                };

                // Coalesce rapid edits: while user is typing, keep moving the target forward.
                if debounce {
                    let delay = diagnostics_debounce_duration();
                    if delay != Duration::from_millis(0) {
                        tokio::time::sleep(delay).await;
                    }

                    let current_requested = {
                        let tasks = server.diagnostics_tasks_v2.lock().await;
                        let Some(task) = tasks.get(&slot_key) else {
                            break;
                        };
                        task.supersession_key
                    };
                    if current_requested != supersession_key {
                        continue;
                    }
                }

                if server
                    .diagnostics_checkpoint_v2(&supersession_key, trigger, Some(&cancel_token))
                    .await
                    .is_some()
                {
                    continue;
                }

                let _ = server
                    .execute_diagnostics_profile_once_v2(
                        &uri_for_task,
                        supersession_key,
                        trigger,
                        Some(&cancel_token),
                    )
                    .await;

                let mut tasks = server.diagnostics_tasks_v2.lock().await;
                let Some(task) = tasks.get(&slot_key) else {
                    break;
                };
                if task.supersession_key == supersession_key
                    && task.cancel_token.same_inner(&cancel_token)
                {
                    tasks.remove(&slot_key);
                    break;
                }
            }
        });

        tasks.insert(
            slot_key,
            super::super::DiagnosticsTaskV2 {
                supersession_key,
                cancel_token: initial_cancel_token,
                trigger,
                debounce,
                handle,
            },
        );
    }

    async fn execute_diagnostics_profile_once_v2(
        &self,
        uri: &Url,
        supersession_key: super::super::DiagnosticsSupersessionKeyV2,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        cancel_token: Option<&super::super::DiagnosticsCancellationTokenV2>,
    ) -> bsl_runtime::application::DiagnosticsDisposition {
        if let Some(disposition) = self
            .diagnostics_checkpoint_v2(&supersession_key, trigger, cancel_token)
            .await
        {
            return disposition;
        }
        let file_id = supersession_key.file_id;
        let requested_version = supersession_key.requested_version;
        let requested_generation = supersession_key.diagnostics_generation;
        let profile = supersession_key.profile;

        let (show_hints, flow_sensitive_enabled) = {
            let settings = self.settings.read().await;
            (
                settings.diagnostics.show_hints,
                settings.enable_flow_sensitive,
            )
        };
        let plan =
            bsl_runtime::application::diagnostics_execution_plan(profile, flow_sensitive_enabled);
        if !plan.run_syntax && !plan.run_semantic {
            self.record_diagnostics_pipeline_event_v2(
                trigger,
                profile,
                bsl_runtime::application::DiagnosticsDisposition::Published,
            );
            return bsl_runtime::application::DiagnosticsDisposition::Published;
        }

        let context = self
            .build_execution_context_v2(
                bsl_runtime::application::SemanticOperation::Diagnostics,
                file_id,
                Some(requested_version),
                plan.flow_sensitive_semantic,
            )
            .await;
        let prepared = match self
            .analysis_v2
            .prepare_stateful_operation(&context, Some(self.coordinator.as_ref()))
            .await
        {
            Ok(prepared) => prepared,
            Err(outcome) => {
                let disposition = if matches!(
                    outcome,
                    bsl_runtime::application::SemanticOutcome::StaleVersion
                ) {
                    bsl_runtime::application::DiagnosticsDisposition::SupersededVersion
                } else {
                    let current_generation = self.current_diagnostics_generation_v2(file_id).await;
                    let current_version = self
                        .latest_received_file_versions_v2
                        .read()
                        .await
                        .get(&file_id)
                        .copied();
                    self.diagnostics_cancelled_disposition_v2(
                        cancel_token,
                        current_generation,
                        requested_generation,
                        current_version,
                        requested_version,
                    )
                };
                debug!(
                    uri = %uri,
                    file_id = file_id.0,
                    expected_version = requested_version,
                    expected_generation = requested_generation,
                    profile = profile.as_str(),
                    trigger = trigger.as_str(),
                    outcome = outcome.as_str(),
                    "diagnostics_v2: skip publish (stateful operation not ready)"
                );
                self.record_diagnostics_pipeline_event_v2(trigger, profile, disposition);
                return disposition;
            }
        };

        let wait_elapsed = prepared.wait_elapsed.unwrap_or(Duration::ZERO);
        if wait_elapsed > Duration::ZERO {
            if let Some(threshold) = super::super::intellisense_v2_slow_client_log_threshold() {
                if wait_elapsed >= threshold {
                    self.client
                        .log_message(
                            MessageType::INFO,
                            format!(
                                "[perf] diagnostics_v2 wait_for_file_version: wait_ms={} uri={} file_id={} expected_version={} profile={}",
                                wait_elapsed.as_millis(),
                                uri,
                                file_id.0,
                                requested_version,
                                profile.as_str(),
                            ),
                        )
                        .await;
                }
            }
            if let Some(threshold) = super::super::intellisense_v2_slow_wait_warn_threshold() {
                if wait_elapsed >= threshold {
                    warn!(
                        uri = %uri,
                        file_id = file_id.0,
                        expected_version = requested_version,
                        expected_generation = requested_generation,
                        profile = profile.as_str(),
                        wait_ms = wait_elapsed.as_millis(),
                        threshold_ms = threshold.as_millis(),
                        "diagnostics_v2: wait_for_file_version is slow"
                    );
                }
            }
        }

        if let Some(disposition) = self
            .diagnostics_checkpoint_v2(&supersession_key, trigger, cancel_token)
            .await
        {
            return disposition;
        }

        let mut analysis = Some(prepared.snapshot.analysis);
        let index_snapshot_id = prepared.snapshot.index_snapshot.id.as_str().to_string();
        let observed_deps_id = Some(prepared.snapshot.deps_id.as_str().to_string());
        let observed_settings_id = analysis
            .as_ref()
            .and_then(|value| value.settings_id().ok().map(|id| id.as_str().to_string()));
        let file_text = analysis
            .as_ref()
            .and_then(|value| value.file_text(file_id).ok().flatten());
        let line_index = analysis
            .as_ref()
            .and_then(|value| value.line_index(file_id).ok().flatten());
        let (file_bytes, file_lines) = file_text
            .as_deref()
            .map(|text| (text.len(), text.lines().count()))
            .unwrap_or((0, 0));

        let snapshot_elapsed = prepared.snapshot_elapsed;
        if let Some(threshold) = super::super::intellisense_v2_slow_snapshot_warn_threshold() {
            if snapshot_elapsed >= threshold {
                warn!(
                    uri = %uri,
                    file_id = file_id.0,
                    expected_version = requested_version,
                    expected_generation = requested_generation,
                    profile = profile.as_str(),
                    snapshot_ms = snapshot_elapsed.as_millis(),
                    threshold_ms = threshold.as_millis(),
                    "diagnostics_v2: snapshot acquisition is slow"
                );
            }
        }

        let mut diagnostics = Vec::new();
        let mut was_cancelled = false;

        if plan.run_syntax {
            self.coordinator
                .record_intellisense_v2_payload_shape_with_origin(
                    context.origin.as_str(),
                    context.operation.as_str(),
                    bsl_runtime::application::ObservabilityStage::SyntaxDiagnosticsQuery.as_str(),
                    file_bytes,
                    file_lines,
                );
            let analysis_for_blocking = analysis
                .take()
                .expect("analysis snapshot must be available for syntax stage");
            let context_for_blocking = context.clone();
            let coordinator_for_blocking = self.coordinator.clone();
            let uri_for_blocking = uri.clone();
            let file_text_for_blocking = file_text.clone();
            let line_index_for_blocking = line_index.clone();
            let syntax_result = bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
                plan.cpu_class,
                context_for_blocking.origin.as_str(),
                Some(self.coordinator.as_ref()),
                move || {
                    let started = Instant::now();
                    let syntax_query =
                        bsl_runtime::application::IntellisenseV2Facade::run_syntax_diagnostics_query_singleflight(
                            &context_for_blocking,
                            &analysis_for_blocking,
                            Some(coordinator_for_blocking.as_ref()),
                            file_id,
                        );
                    let elapsed = started.elapsed();
                    match syntax_query {
                        Ok(Some(syntax_errors)) => {
                            let mut diagnostics = Vec::new();
                            if let (Some(text), Some(index)) =
                                (file_text_for_blocking.as_deref(), line_index_for_blocking.as_deref())
                            {
                                diagnostics.extend(syntax_errors_to_diagnostics(
                                    &syntax_errors,
                                    &uri_for_blocking,
                                    text,
                                    index,
                                ));
                            }
                            (diagnostics, false, elapsed, analysis_for_blocking)
                        }
                        Ok(None) => (Vec::new(), false, elapsed, analysis_for_blocking),
                        Err(_) => (Vec::new(), true, elapsed, analysis_for_blocking),
                    }
                },
            )
            .await;
            match syntax_result {
                Ok((syntax_diagnostics, syntax_cancelled, syntax_elapsed, next_analysis)) => {
                    analysis = Some(next_analysis);
                    diagnostics.extend(syntax_diagnostics);
                    was_cancelled |= syntax_cancelled;
                    if let Some(threshold) =
                        super::super::intellisense_v2_slow_query_warn_threshold()
                    {
                        if syntax_elapsed >= threshold {
                            warn!(
                                uri = %uri,
                                file_id = file_id.0,
                                expected_version = requested_version,
                                expected_generation = requested_generation,
                                profile = profile.as_str(),
                                syntax_diagnostics_ms = syntax_elapsed.as_millis(),
                                file_bytes,
                                file_lines,
                                threshold_ms = threshold.as_millis(),
                                "diagnostics_v2: syntax_diagnostics query is slow"
                            );
                        }
                    }
                }
                Err(err) => {
                    warn!(
                        uri = %uri,
                        file_id = file_id.0,
                        expected_version = requested_version,
                        expected_generation = requested_generation,
                        profile = profile.as_str(),
                        error = ?err,
                        "diagnostics_v2: syntax spawn_blocking failed"
                    );
                    was_cancelled = true;
                }
            }
        }

        if was_cancelled {
            let current_generation = self.current_diagnostics_generation_v2(file_id).await;
            let current_version = self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied();
            let disposition = self.diagnostics_cancelled_disposition_v2(
                cancel_token,
                current_generation,
                requested_generation,
                current_version,
                requested_version,
            );
            self.record_diagnostics_pipeline_event_v2(trigger, profile, disposition);
            return disposition;
        }

        if plan.run_semantic {
            self.coordinator
                .record_intellisense_v2_payload_shape_with_origin(
                    context.origin.as_str(),
                    context.operation.as_str(),
                    bsl_runtime::application::ObservabilityStage::SemanticDiagnosticsQuery.as_str(),
                    file_bytes,
                    file_lines,
                );
            if let Some(disposition) = self
                .diagnostics_checkpoint_v2(&supersession_key, trigger, cancel_token)
                .await
            {
                return disposition;
            }

            let analysis_for_blocking = analysis
                .take()
                .expect("analysis snapshot must be available for semantic stage");
            let context_for_blocking = context.clone();
            let coordinator_for_blocking = self.coordinator.clone();
            let file_text_for_blocking = file_text.clone();
            let line_index_for_blocking = line_index.clone();
            let semantic_flow_sensitive = plan.flow_sensitive_semantic;
            let semantic_show_hints = show_hints;
            let semantic_result = bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
                plan.cpu_class,
                context_for_blocking.origin.as_str(),
                Some(self.coordinator.as_ref()),
                move || {
                    let started = Instant::now();
                    let query = bsl_runtime::application::IntellisenseV2Facade::run_optional_query(
                        &context_for_blocking,
                        bsl_runtime::application::ObservabilityStage::SemanticDiagnosticsQuery,
                        &analysis_for_blocking,
                        Some(coordinator_for_blocking.as_ref()),
                        |analysis| {
                            if semantic_flow_sensitive {
                                analysis.semantic_diagnostics_flow_sensitive(file_id)
                            } else {
                                analysis.semantic_diagnostics(file_id)
                            }
                        },
                    );
                    let elapsed = started.elapsed();
                    match query {
                        Ok(Some(semantic_errors)) => {
                            let mut diagnostics = Vec::new();
                            for error in semantic_errors.iter() {
                                if !semantic_show_hints
                                    && matches!(
                                        error.severity,
                                        bsl_shared::domain::types::DiagnosticSeverity::Hint
                                    )
                                {
                                    continue;
                                }
                                if let (Some(text), Some(index)) =
                                    (file_text_for_blocking.as_deref(), line_index_for_blocking.as_deref())
                                {
                                    diagnostics.push(semantic_error_to_diagnostic(error, text, index));
                                }
                            }
                            (diagnostics, false, elapsed)
                        }
                        Ok(None) => (Vec::new(), false, elapsed),
                        Err(_) => (Vec::new(), true, elapsed),
                    }
                },
            )
            .await;
            match semantic_result {
                Ok((semantic_diagnostics, semantic_cancelled, semantic_elapsed)) => {
                    diagnostics.extend(semantic_diagnostics);
                    was_cancelled |= semantic_cancelled;
                    if let Some(threshold) =
                        super::super::intellisense_v2_slow_query_warn_threshold()
                    {
                        if semantic_elapsed >= threshold {
                            warn!(
                                uri = %uri,
                                file_id = file_id.0,
                                expected_version = requested_version,
                                expected_generation = requested_generation,
                                profile = profile.as_str(),
                                semantic_diagnostics_ms = semantic_elapsed.as_millis(),
                                file_bytes,
                                file_lines,
                                threshold_ms = threshold.as_millis(),
                                "diagnostics_v2: semantic_diagnostics query is slow"
                            );
                        }
                    }
                }
                Err(err) => {
                    warn!(
                        uri = %uri,
                        file_id = file_id.0,
                        expected_version = requested_version,
                        expected_generation = requested_generation,
                        profile = profile.as_str(),
                        error = ?err,
                        "diagnostics_v2: semantic spawn_blocking failed"
                    );
                    was_cancelled = true;
                }
            }
        }

        let diagnostics_len = diagnostics.len();
        let (is_current, current_version, current_deps_id, current_settings_id, current_generation) = {
            let current_version = self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied();
            let current_deps_id = self
                .last_deps_id_v2
                .read()
                .await
                .as_ref()
                .map(|id| id.as_str().to_string());
            let current_settings_id = self
                .last_settings_id_v2
                .read()
                .await
                .as_ref()
                .map(|id| id.as_str().to_string());
            let current_generation = self.current_diagnostics_generation_v2(file_id).await;
            let is_current = !was_cancelled
                && current_version == Some(requested_version)
                && current_deps_id == observed_deps_id
                && current_settings_id == observed_settings_id
                && current_generation == Some(requested_generation);
            (
                is_current,
                current_version,
                current_deps_id,
                current_settings_id,
                current_generation,
            )
        };

        if !is_current {
            let disposition = if was_cancelled {
                self.diagnostics_cancelled_disposition_v2(
                    cancel_token,
                    current_generation,
                    requested_generation,
                    current_version,
                    requested_version,
                )
            } else if current_generation != Some(requested_generation) {
                bsl_runtime::application::DiagnosticsDisposition::SupersededGeneration
            } else if current_version != Some(requested_version) {
                bsl_runtime::application::DiagnosticsDisposition::SupersededVersion
            } else {
                bsl_runtime::application::DiagnosticsDisposition::OtherCancel
            };
            debug!(
                uri = %uri,
                file_id = file_id.0,
                expected_version = requested_version,
                expected_generation = requested_generation,
                current_version = ?current_version,
                current_generation = ?current_generation,
                observed_deps_id = ?observed_deps_id,
                current_deps_id = ?current_deps_id,
                observed_settings_id = ?observed_settings_id,
                current_settings_id = ?current_settings_id,
                profile = profile.as_str(),
                trigger = trigger.as_str(),
                disposition = disposition.as_str(),
                "diagnostics_v2: skip publish (stale)"
            );
            self.record_diagnostics_pipeline_event_v2(trigger, profile, disposition);
            return disposition;
        }

        if let Some(disposition) = self
            .diagnostics_publish_checkpoint_v2(
                &supersession_key,
                trigger,
                cancel_token,
                observed_deps_id.as_deref(),
                observed_settings_id.as_deref(),
            )
            .await
        {
            debug!(
                uri = %uri,
                file_id = file_id.0,
                expected_version = requested_version,
                expected_generation = requested_generation,
                profile = profile.as_str(),
                trigger = trigger.as_str(),
                disposition = disposition.as_str(),
                "diagnostics_v2: skip publish (final checkpoint)"
            );
            return disposition;
        }

        debug!(
            uri = %uri,
            file_id = file_id.0,
            expected_version = requested_version,
            expected_generation = requested_generation,
            deps_id = observed_deps_id.as_deref().unwrap_or_default(),
            settings_id = observed_settings_id.as_deref().unwrap_or_default(),
            index_snapshot_id = index_snapshot_id,
            profile = profile.as_str(),
            trigger = trigger.as_str(),
            diagnostics_len,
            "diagnostics_v2: publish diagnostics"
        );

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, Some(requested_version))
            .await;
        self.update_diagnostics_count(uri, diagnostics_len).await;
        self.record_diagnostics_pipeline_event_v2(
            trigger,
            profile,
            bsl_runtime::application::DiagnosticsDisposition::Published,
        );
        bsl_runtime::application::DiagnosticsDisposition::Published
    }
}
