use super::*;

#[cfg(debug_assertions)]
fn maybe_inject_save_fastlane_shadow_parse_delay_for_test() {
    let delay_ms = std::env::var("BSL_TEST_SAVE_FASTLANE_PARSE_DELAY_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0);
    let Some(delay_ms) = delay_ms else {
        return;
    };
    std::thread::sleep(Duration::from_millis(delay_ms));
}

#[cfg(not(debug_assertions))]
fn maybe_inject_save_fastlane_shadow_parse_delay_for_test() {}

enum SaveFastlaneFirstPublishWaitOutcome {
    Published,
    NotPublished,
}

const SAVE_FOLLOWUP_READY_PARSE_SNAPSHOT_WAIT_BUDGET: Duration = Duration::from_millis(3_500);

struct SaveFollowupReadyArtifactsReply {
    diagnostics: Vec<tower_lsp::lsp_types::Diagnostic>,
    observed_deps_id: String,
    observed_settings_id: String,
    syntax_elapsed: Option<Duration>,
    semantic_elapsed: Option<Duration>,
    syntax_work_mode: Option<&'static str>,
}

impl BslLanguageServer {
    fn diagnostics_save_timeline_cycle_key_for_supersession_key(
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
    ) -> Option<super::super::DiagnosticsSaveTimelineCycleKey> {
        supersession_key
            .save_cycle_sequence
            .map(
                |save_cycle_sequence| super::super::DiagnosticsSaveTimelineCycleKey {
                    file_id: supersession_key.file_id,
                    diagnostics_generation: supersession_key.diagnostics_generation,
                    save_cycle_sequence,
                    requested_version: supersession_key.requested_version,
                },
            )
    }

    async fn finalize_diagnostics_save_profile_result_v2(
        &self,
        uri: &Url,
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        disposition: bsl_runtime::application::DiagnosticsDisposition,
        publish_kind: Option<&'static str>,
        blocking_queue_wait_ms: Option<Duration>,
        wait_for_file_version_ms: Option<Duration>,
        snapshot_with_deps_ms: Option<Duration>,
        syntax_diagnostics_query_ms: Option<Duration>,
        semantic_diagnostics_query_ms: Option<Duration>,
        publish_wait_ms: Option<Duration>,
        syntax_work_mode: Option<&'static str>,
        pipeline_started: Instant,
    ) -> bsl_runtime::application::DiagnosticsDisposition {
        if !matches!(
            trigger,
            bsl_runtime::application::DiagnosticsTrigger::DidSave
        ) {
            return disposition;
        }

        let publish = (matches!(
            disposition,
            bsl_runtime::application::DiagnosticsDisposition::Published
        ) || publish_kind.is_some()
            || syntax_work_mode.is_some()
            || blocking_queue_wait_ms.is_some()
            || wait_for_file_version_ms.is_some()
            || snapshot_with_deps_ms.is_some()
            || syntax_diagnostics_query_ms.is_some()
            || semantic_diagnostics_query_ms.is_some()
            || publish_wait_ms.is_some())
        .then(|| crate::types::DiagnosticsSaveTimelinePublishTrace {
            profile: supersession_key.profile.as_str().to_string(),
            publish_kind: publish_kind.unwrap_or("unknown").to_string(),
            outcome: disposition.as_str().to_string(),
            elapsed_ms: pipeline_started.elapsed().as_millis().min(u64::MAX as u128) as u64,
            syntax_work_mode: syntax_work_mode.map(str::to_string),
            blocking_queue_wait_ms: blocking_queue_wait_ms
                .map(|value| value.as_millis().min(u64::MAX as u128) as u64),
            wait_for_file_version_ms: wait_for_file_version_ms
                .map(|value| value.as_millis().min(u64::MAX as u128) as u64),
            snapshot_with_deps_ms: snapshot_with_deps_ms
                .map(|value| value.as_millis().min(u64::MAX as u128) as u64),
            syntax_diagnostics_query_ms: syntax_diagnostics_query_ms
                .map(|value| value.as_millis().min(u64::MAX as u128) as u64),
            semantic_diagnostics_query_ms: semantic_diagnostics_query_ms
                .map(|value| value.as_millis().min(u64::MAX as u128) as u64),
            publish_wait_ms: publish_wait_ms
                .map(|value| value.as_millis().min(u64::MAX as u128) as u64),
        });

        let Some(cycle_key) =
            Self::diagnostics_save_timeline_cycle_key_for_supersession_key(supersession_key)
        else {
            return disposition;
        };

        self.record_diagnostics_save_timeline_profile_result(
            uri,
            cycle_key,
            super::super::DiagnosticsSaveTimelineProfileResult {
                profile: supersession_key.profile,
                disposition,
                publish,
            },
        );
        disposition
    }

    fn diagnostics_publish_rank(profile: bsl_runtime::application::DiagnosticsProfile) -> u8 {
        match profile {
            bsl_runtime::application::DiagnosticsProfile::SaveFastlane => 1,
            _ => 2,
        }
    }

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

    pub(crate) async fn bump_diagnostics_save_cycle_sequence_v2(&self, file_id: V2FileId) -> u64 {
        let mut sequences = self.diagnostics_save_cycle_sequence_v2.write().await;
        let next = sequences
            .get(&file_id)
            .copied()
            .unwrap_or(0)
            .saturating_add(1);
        sequences.insert(file_id, next);
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

    fn record_diagnostics_pipeline_publish_latency_v2(
        &self,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        profile: bsl_runtime::application::DiagnosticsProfile,
        duration: Duration,
    ) {
        self.coordinator
            .record_intellisense_v2_diagnostics_pipeline_publish_latency(
                bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                trigger.as_str(),
                profile.as_str(),
                duration,
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

        let deps_mismatch =
            observed_deps_id.is_some() && current_deps_id.as_deref() != observed_deps_id;
        let settings_mismatch = observed_settings_id.is_some()
            && current_settings_id.as_deref() != observed_settings_id;
        if deps_mismatch || settings_mismatch {
            let disposition =
                bsl_runtime::application::DiagnosticsDisposition::SupersededGeneration;
            self.record_diagnostics_pipeline_event_v2(trigger, key.profile, disposition);
            return Some(disposition);
        }

        None
    }

    async fn publish_diagnostics_v2(
        &self,
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
        uri: &Url,
        diagnostics: Vec<tower_lsp::lsp_types::Diagnostic>,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        profile: bsl_runtime::application::DiagnosticsProfile,
        pipeline_started: Instant,
    ) -> bsl_runtime::application::DiagnosticsDisposition {
        let publish_rank = Self::diagnostics_publish_rank(profile);
        let file_id = supersession_key.file_id;
        let requested_version = supersession_key.requested_version;
        let diagnostics_generation = supersession_key.diagnostics_generation;
        {
            let publish_state = self.latest_diagnostics_publish_state_v2.read().await;
            if publish_state.get(&file_id).is_some_and(|state| {
                state.requested_version == requested_version
                    && state.diagnostics_generation == diagnostics_generation
                    && state.publish_rank > publish_rank
            }) {
                self.record_diagnostics_pipeline_event_v2(
                    trigger,
                    profile,
                    bsl_runtime::application::DiagnosticsDisposition::OtherCancel,
                );
                return bsl_runtime::application::DiagnosticsDisposition::OtherCancel;
            }
        }

        let diagnostics_len = diagnostics.len();
        self.client
            .publish_diagnostics(uri.clone(), diagnostics, Some(requested_version))
            .await;
        self.update_diagnostics_count(uri, diagnostics_len).await;
        self.latest_diagnostics_publish_state_v2
            .write()
            .await
            .insert(
                file_id,
                super::super::DiagnosticsPublishedStateV2 {
                    requested_version,
                    diagnostics_generation,
                    publish_rank,
                },
            );
        self.record_diagnostics_pipeline_publish_latency_v2(
            trigger,
            profile,
            pipeline_started.elapsed(),
        );
        self.record_diagnostics_pipeline_event_v2(
            trigger,
            profile,
            bsl_runtime::application::DiagnosticsDisposition::Published,
        );
        bsl_runtime::application::DiagnosticsDisposition::Published
    }

    async fn try_collect_save_fastlane_diagnostics_from_applied_analysis_v2(
        &self,
        uri: &Url,
        file_id: V2FileId,
        requested_version: i32,
    ) -> Option<(
        Vec<tower_lsp::lsp_types::Diagnostic>,
        Vec<bsl_shared::domain::types::ParseError>,
        &'static str,
        Duration,
    )> {
        tokio::time::timeout(Duration::from_millis(20), async {
            let started = Instant::now();
            let revision_state = self.analysis_v2.file_revision_state(file_id).await?;
            if revision_state.version != requested_version {
                return None;
            }

            let analysis = self.analysis_v2.snapshot().await;
            if analysis.file_version(file_id).ok().flatten() != Some(requested_version) {
                return None;
            }
            let syntax_errors = analysis.syntax_diagnostics(file_id).ok().flatten()?;
            let line_index = analysis.line_index(file_id).ok().flatten()?;
            let mode = analysis
                .syntax_diagnostics_observability_mode(file_id)
                .ok()
                .flatten()
                .unwrap_or("other");
            Some((
                syntax_errors_to_diagnostics(
                    syntax_errors.as_ref(),
                    uri,
                    analysis.file_text(file_id).ok().flatten()?.as_ref(),
                    line_index.as_ref(),
                ),
                syntax_errors.iter().cloned().collect(),
                mode,
                started.elapsed(),
            ))
        })
        .await
        .ok()
        .flatten()
    }

    fn parse_snapshot_observability_mode_v2(
        parse_snapshot: &bsl_analysis_v2::ParseSnapshot,
    ) -> &'static str {
        if parse_snapshot.incremental {
            if parse_snapshot.changed_ranges.is_empty() {
                "reused"
            } else {
                "incremental"
            }
        } else {
            "full"
        }
    }

    async fn try_collect_save_fastlane_diagnostics_from_ready_parse_snapshot_v2(
        &self,
        uri: &Url,
        file_id: V2FileId,
        requested_version: i32,
    ) -> Option<(
        Vec<tower_lsp::lsp_types::Diagnostic>,
        Vec<bsl_shared::domain::types::ParseError>,
        &'static str,
        Duration,
    )> {
        let started = Instant::now();
        let ready_state = self
            .latest_ready_parse_snapshots_v2
            .read()
            .await
            .get(&file_id)
            .cloned()?;
        let parse_snapshot = ready_state.parse_snapshot;
        if parse_snapshot.file_version != requested_version {
            return None;
        }
        Some((
            syntax_errors_to_diagnostics(
                &parse_snapshot.parse_result.syntax_errors,
                uri,
                ready_state.text.as_ref(),
                parse_snapshot.line_index.as_ref(),
            ),
            parse_snapshot
                .parse_result
                .syntax_errors
                .iter()
                .cloned()
                .collect(),
            Self::parse_snapshot_observability_mode_v2(&parse_snapshot),
            started.elapsed(),
        ))
    }

    async fn ready_parse_snapshot_state_for_version_v2(
        &self,
        file_id: V2FileId,
        requested_version: i32,
    ) -> Option<super::super::ReadyParseSnapshotStateV2> {
        self.latest_ready_parse_snapshots_v2
            .read()
            .await
            .get(&file_id)
            .cloned()
            .filter(|state| state.parse_snapshot.file_version == requested_version)
    }

    async fn save_fastlane_syntax_artifacts_for_version_v2(
        &self,
        file_id: V2FileId,
        requested_version: i32,
    ) -> Option<Arc<Vec<bsl_shared::domain::types::ParseError>>> {
        self.latest_save_fastlane_syntax_artifacts_v2
            .read()
            .await
            .get(&file_id)
            .cloned()
            .filter(|state| state.version == requested_version)
            .map(|state| state.syntax_errors)
    }

    async fn record_save_fastlane_syntax_artifacts_v2(
        &self,
        file_id: V2FileId,
        requested_version: i32,
        syntax_errors: Vec<bsl_shared::domain::types::ParseError>,
    ) {
        self.latest_save_fastlane_syntax_artifacts_v2
            .write()
            .await
            .insert(
                file_id,
                super::super::SaveFastlaneSyntaxArtifactsV2 {
                    version: requested_version,
                    syntax_errors: Arc::new(syntax_errors),
                },
            );
    }

    async fn wait_for_ready_parse_snapshot_state_for_version_v2(
        &self,
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
        cancel_token: Option<&super::super::DiagnosticsCancellationTokenV2>,
        wait_budget: Duration,
    ) -> Option<super::super::ReadyParseSnapshotStateV2> {
        let wait_started = Instant::now();
        loop {
            if let Some(state) = self
                .ready_parse_snapshot_state_for_version_v2(
                    supersession_key.file_id,
                    supersession_key.requested_version,
                )
                .await
            {
                return Some(state);
            }
            if wait_started.elapsed() >= wait_budget {
                return None;
            }
            if cancel_token.is_some_and(|token| token.is_cancelled()) {
                return None;
            }
            if self
                .current_diagnostics_generation_v2(supersession_key.file_id)
                .await
                != Some(supersession_key.diagnostics_generation)
            {
                return None;
            }
            if self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&supersession_key.file_id)
                .copied()
                != Some(supersession_key.requested_version)
            {
                return None;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    fn diagnostics_save_cycle_key_from_supersession_key_v2(
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
    ) -> Option<super::super::DiagnosticsSaveTimelineCycleKey> {
        supersession_key
            .save_cycle_sequence
            .map(
                |save_cycle_sequence| super::super::DiagnosticsSaveTimelineCycleKey {
                    file_id: supersession_key.file_id,
                    diagnostics_generation: supersession_key.diagnostics_generation,
                    save_cycle_sequence,
                    requested_version: supersession_key.requested_version,
                },
            )
    }

    fn record_diagnostics_save_followup_wait_state_v2(
        &self,
        uri: &Url,
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
        reason: &'static str,
        wait_for_file_version_ms: Option<Duration>,
        snapshot_with_deps_ms: Option<Duration>,
        syntax_work_mode: Option<&'static str>,
    ) {
        let Some(cycle_key) =
            Self::diagnostics_save_cycle_key_from_supersession_key_v2(supersession_key)
        else {
            return;
        };
        self.record_diagnostics_save_timeline_followup_wait_state(
            uri,
            cycle_key,
            reason,
            wait_for_file_version_ms,
            snapshot_with_deps_ms,
            syntax_work_mode,
        );
    }

    async fn wait_for_save_fastlane_first_publish_v2(
        &self,
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
        cancel_token: Option<&super::super::DiagnosticsCancellationTokenV2>,
    ) -> SaveFastlaneFirstPublishWaitOutcome {
        let Some(cycle_key) =
            Self::diagnostics_save_timeline_cycle_key_for_supersession_key(supersession_key)
        else {
            return SaveFastlaneFirstPublishWaitOutcome::NotPublished;
        };
        loop {
            match self.diagnostics_save_timeline_fastlane_progress(cycle_key) {
                super::DiagnosticsSaveTimelineFastlaneProgress::SuccessfulFirstPublish => {
                    return SaveFastlaneFirstPublishWaitOutcome::Published;
                }
                super::DiagnosticsSaveTimelineFastlaneProgress::TerminalWithoutPublish => {
                    return SaveFastlaneFirstPublishWaitOutcome::NotPublished;
                }
                super::DiagnosticsSaveTimelineFastlaneProgress::Pending => {}
            }
            if cancel_token.is_some_and(|token| token.is_cancelled()) {
                return SaveFastlaneFirstPublishWaitOutcome::NotPublished;
            }
            if self
                .current_diagnostics_generation_v2(supersession_key.file_id)
                .await
                != Some(supersession_key.diagnostics_generation)
            {
                return SaveFastlaneFirstPublishWaitOutcome::NotPublished;
            }
            if self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&supersession_key.file_id)
                .copied()
                != Some(supersession_key.requested_version)
            {
                return SaveFastlaneFirstPublishWaitOutcome::NotPublished;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    async fn try_execute_save_followup_from_applied_state_v2(
        &self,
        uri: &Url,
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        cancel_token: Option<&super::super::DiagnosticsCancellationTokenV2>,
        pipeline_started: Instant,
        show_hints: bool,
        flow_sensitive_semantic: bool,
    ) -> Option<bsl_runtime::application::DiagnosticsDisposition> {
        if !matches!(
            (trigger, supersession_key.profile),
            (
                bsl_runtime::application::DiagnosticsTrigger::DidSave,
                bsl_runtime::application::DiagnosticsProfile::IdleHeavy
            )
        ) {
            return None;
        }

        self.analysis_v2
            .cached_file_revision_state(supersession_key.file_id)
            .filter(|state| state.version == supersession_key.requested_version)?;

        let context = self
            .build_execution_context_v2(
                bsl_runtime::application::SemanticOperation::Diagnostics,
                supersession_key.file_id,
                None,
                flow_sensitive_semantic,
            )
            .await;
        let (analysis, _index_snapshot, deps_id) = self.analysis_v2.snapshot_with_deps().await;
        if analysis
            .file_version(supersession_key.file_id)
            .ok()
            .flatten()
            != Some(supersession_key.requested_version)
        {
            return None;
        }

        let file_text = analysis
            .file_text(supersession_key.file_id)
            .ok()
            .flatten()?;
        let line_index = analysis
            .line_index(supersession_key.file_id)
            .ok()
            .flatten()
            .unwrap_or_else(|| Arc::new(bsl_line_index::LineIndex::new(file_text.as_ref())));
        let save_fastlane_syntax_artifacts = self
            .save_fastlane_syntax_artifacts_for_version_v2(
                supersession_key.file_id,
                supersession_key.requested_version,
            )
            .await;
        let syntax_work_mode = if save_fastlane_syntax_artifacts.is_some() {
            Some("reused")
        } else {
            Some("recomputed")
        };

        self.record_diagnostics_save_followup_wait_state_v2(
            uri,
            supersession_key,
            "semantic_work",
            None,
            None,
            syntax_work_mode,
        );

        let mut diagnostics = Vec::new();
        let syntax_elapsed = if let Some(syntax_errors) = save_fastlane_syntax_artifacts {
            diagnostics.extend(syntax_errors_to_diagnostics(
                syntax_errors.as_ref(),
                uri,
                file_text.as_ref(),
                line_index.as_ref(),
            ));
            None
        } else {
            let syntax_started = Instant::now();
            let syntax_errors = bsl_runtime::application::IntellisenseV2Facade::run_syntax_diagnostics_query_singleflight(
                &context,
                &analysis,
                Some(self.coordinator.as_ref()),
                supersession_key.file_id,
            )
            .ok()?;
            let syntax_elapsed = syntax_started.elapsed();
            if let Some(syntax_errors) = syntax_errors {
                diagnostics.extend(syntax_errors_to_diagnostics(
                    syntax_errors.as_ref(),
                    uri,
                    file_text.as_ref(),
                    line_index.as_ref(),
                ));
            }
            Some(syntax_elapsed)
        };

        let semantic_started = Instant::now();
        let query = bsl_runtime::application::IntellisenseV2Facade::run_optional_query(
            &context,
            bsl_runtime::application::ObservabilityStage::SemanticDiagnosticsQuery,
            &analysis,
            Some(self.coordinator.as_ref()),
            |analysis| {
                if flow_sensitive_semantic {
                    analysis.semantic_diagnostics_flow_sensitive_profiled(supersession_key.file_id)
                } else {
                    analysis.semantic_diagnostics_profiled(supersession_key.file_id)
                }
            },
        )
        .ok()?;
        let semantic_elapsed = semantic_started.elapsed();
        let duration_from_profile_ms =
            |value: u128| Duration::from_millis(value.min(u64::MAX as u128) as u64);
        if let Some(profiled) = query {
            self.coordinator
                .record_intellisense_v2_semantic_diagnostics_query_breakdown(
                    duration_from_profile_ms(profiled.profile.inputs_ms),
                    duration_from_profile_ms(profiled.profile.parse_result_ms),
                    duration_from_profile_ms(profiled.profile.ir_ms),
                    duration_from_profile_ms(profiled.profile.collect_ms),
                    (profiled.profile.flow_sensitive_ms > 0)
                        .then(|| duration_from_profile_ms(profiled.profile.flow_sensitive_ms)),
                );
            for error in profiled.diagnostics.iter() {
                if !show_hints
                    && matches!(
                        error.severity,
                        bsl_shared::domain::types::DiagnosticSeverity::Hint
                    )
                {
                    continue;
                }
                diagnostics.push(semantic_error_to_diagnostic(
                    error,
                    file_text.as_ref(),
                    line_index.as_ref(),
                ));
            }
        }

        if let Some(disposition) = self
            .diagnostics_publish_checkpoint_v2(
                supersession_key,
                trigger,
                cancel_token,
                Some(deps_id.as_str()),
                Some(context.settings.settings_id.as_str()),
            )
            .await
        {
            return Some(
                self.finalize_diagnostics_save_profile_result_v2(
                    uri,
                    supersession_key,
                    trigger,
                    disposition,
                    None,
                    None,
                    None,
                    None,
                    syntax_elapsed,
                    Some(semantic_elapsed),
                    None,
                    syntax_work_mode,
                    pipeline_started,
                )
                .await,
            );
        }

        let publish_started = Instant::now();
        let disposition = self
            .publish_diagnostics_v2(
                supersession_key,
                uri,
                diagnostics,
                trigger,
                supersession_key.profile,
                pipeline_started,
            )
            .await;
        Some(
            self.finalize_diagnostics_save_profile_result_v2(
                uri,
                supersession_key,
                trigger,
                disposition,
                Some("full"),
                None,
                None,
                None,
                syntax_elapsed,
                Some(semantic_elapsed),
                Some(publish_started.elapsed()),
                syntax_work_mode,
                pipeline_started,
            )
            .await,
        )
    }

    async fn try_execute_save_followup_from_ready_artifacts_v2(
        &self,
        uri: &Url,
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        cancel_token: Option<&super::super::DiagnosticsCancellationTokenV2>,
        pipeline_started: Instant,
        show_hints: bool,
        flow_sensitive_semantic: bool,
    ) -> Option<bsl_runtime::application::DiagnosticsDisposition> {
        if !matches!(
            (trigger, supersession_key.profile),
            (
                bsl_runtime::application::DiagnosticsTrigger::DidSave,
                bsl_runtime::application::DiagnosticsProfile::IdleHeavy
            )
        ) {
            return None;
        }

        let ready_state = self
            .wait_for_ready_parse_snapshot_state_for_version_v2(
                supersession_key,
                cancel_token,
                SAVE_FOLLOWUP_READY_PARSE_SNAPSHOT_WAIT_BUDGET,
            )
            .await?;
        let context = self
            .build_execution_context_v2(
                bsl_runtime::application::SemanticOperation::Diagnostics,
                supersession_key.file_id,
                None,
                flow_sensitive_semantic,
            )
            .await;
        let support_bundle = self.analysis_v2.completion_support_bundle();
        let path = match uri.to_file_path() {
            Ok(path) => path.to_string_lossy().to_string(),
            Err(_) => uri.to_string(),
        };
        let profile = supersession_key.profile;
        let uri_for_blocking = uri.clone();
        let coordinator_for_blocking = self.coordinator.clone();
        let file_id = supersession_key.file_id;
        let requested_version = supersession_key.requested_version;
        let ready_text = ready_state.text.clone();
        let parse_snapshot = ready_state.parse_snapshot.clone();
        let ready_line_index = parse_snapshot.line_index.clone();
        let ready_syntax_errors = parse_snapshot.parse_result.syntax_errors.clone();
        let deps_id = support_bundle.deps_id.clone();
        let deps = support_bundle.deps.clone();
        let settings_id = context.settings.settings_id.clone();
        let diagnostics_detail_level = context.settings.diagnostics_detail_level;
        let context_for_blocking = context.clone();
        let path_for_blocking: Arc<str> = Arc::from(path);
        self.record_diagnostics_save_followup_wait_state_v2(
            uri,
            supersession_key,
            "semantic_work",
            None,
            None,
            Some("reused"),
        );
        let followup_result =
            bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
                bsl_runtime::application::CpuWorkClass::Background,
                context.origin.as_str(),
                Some(self.coordinator.as_ref()),
                move || {
                    let mut host = bsl_analysis_v2::AnalysisHostV2::default();
                    host.apply_change(bsl_analysis_v2::Change::SetDepsSnapshot {
                        deps_id: deps_id.clone(),
                        deps,
                    });
                    host.apply_change(bsl_analysis_v2::Change::SetSettingsSnapshot {
                        settings_id: settings_id.clone(),
                        diagnostics_detail_level,
                    });
                    host.apply_change(bsl_analysis_v2::Change::SetFileWithSnapshot {
                        file_id,
                        text: ready_text.clone(),
                        version: requested_version,
                        path: path_for_blocking,
                        parse_snapshot,
                    });

                    let analysis = host.snapshot();
                    let file_text = ready_text.clone();
                    let line_index = ready_line_index.clone();

                    let mut diagnostics = Vec::new();
                    diagnostics.extend(syntax_errors_to_diagnostics(
                        &ready_syntax_errors,
                        &uri_for_blocking,
                        file_text.as_ref(),
                        line_index.as_ref(),
                    ));

                    let semantic_started = Instant::now();
                    let query = bsl_runtime::application::IntellisenseV2Facade::run_optional_query(
                        &context_for_blocking,
                        bsl_runtime::application::ObservabilityStage::SemanticDiagnosticsQuery,
                        &analysis,
                        Some(coordinator_for_blocking.as_ref()),
                        |analysis| {
                            if flow_sensitive_semantic {
                                analysis.semantic_diagnostics_flow_sensitive_profiled(file_id)
                            } else {
                                analysis.semantic_diagnostics_profiled(file_id)
                            }
                        },
                    )
                    .map_err(|_| ())?;
                    let semantic_elapsed = semantic_started.elapsed();
                    let duration_from_profile_ms =
                        |value: u128| Duration::from_millis(value.min(u64::MAX as u128) as u64);
                    if let Some(profiled) = query {
                        coordinator_for_blocking
                            .record_intellisense_v2_semantic_diagnostics_query_breakdown(
                                duration_from_profile_ms(profiled.profile.inputs_ms),
                                duration_from_profile_ms(profiled.profile.parse_result_ms),
                                duration_from_profile_ms(profiled.profile.ir_ms),
                                duration_from_profile_ms(profiled.profile.collect_ms),
                                (profiled.profile.flow_sensitive_ms > 0).then(|| {
                                    duration_from_profile_ms(profiled.profile.flow_sensitive_ms)
                                }),
                            );
                        for error in profiled.diagnostics.iter() {
                            if !show_hints
                                && matches!(
                                    error.severity,
                                    bsl_shared::domain::types::DiagnosticSeverity::Hint
                                )
                            {
                                continue;
                            }
                            diagnostics.push(semantic_error_to_diagnostic(
                                error,
                                file_text.as_ref(),
                                line_index.as_ref(),
                            ));
                        }
                    }

                    Ok::<SaveFollowupReadyArtifactsReply, ()>(SaveFollowupReadyArtifactsReply {
                        diagnostics,
                        observed_deps_id: deps_id.as_str().to_string(),
                        observed_settings_id: settings_id.as_str().to_string(),
                        syntax_elapsed: None,
                        semantic_elapsed: Some(semantic_elapsed),
                        syntax_work_mode: Some("reused"),
                    })
                },
            )
            .await;

        let reply = match followup_result {
            Ok(Ok(reply)) => reply,
            Ok(Err(())) | Err(_) => {
                return Some(
                    self.finalize_diagnostics_save_profile_result_v2(
                        uri,
                        supersession_key,
                        trigger,
                        self.diagnostics_cancelled_disposition_v2(
                            cancel_token,
                            self.current_diagnostics_generation_v2(supersession_key.file_id)
                                .await,
                            supersession_key.diagnostics_generation,
                            self.latest_received_file_versions_v2
                                .read()
                                .await
                                .get(&supersession_key.file_id)
                                .copied(),
                            supersession_key.requested_version,
                        ),
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        pipeline_started,
                    )
                    .await,
                );
            }
        };

        if let Some(disposition) = self
            .diagnostics_publish_checkpoint_v2(
                supersession_key,
                trigger,
                cancel_token,
                Some(reply.observed_deps_id.as_str()),
                Some(reply.observed_settings_id.as_str()),
            )
            .await
        {
            return Some(
                self.finalize_diagnostics_save_profile_result_v2(
                    uri,
                    supersession_key,
                    trigger,
                    disposition,
                    None,
                    None,
                    None,
                    None,
                    reply.syntax_elapsed,
                    reply.semantic_elapsed,
                    None,
                    reply.syntax_work_mode,
                    pipeline_started,
                )
                .await,
            );
        }

        self.record_diagnostics_save_followup_wait_state_v2(
            uri,
            supersession_key,
            "pending_publish",
            None,
            None,
            reply.syntax_work_mode,
        );
        let publish_started = Instant::now();
        let disposition = self
            .publish_diagnostics_v2(
                supersession_key,
                uri,
                reply.diagnostics,
                trigger,
                profile,
                pipeline_started,
            )
            .await;
        Some(
            self.finalize_diagnostics_save_profile_result_v2(
                uri,
                supersession_key,
                trigger,
                disposition,
                Some("full"),
                None,
                None,
                None,
                reply.syntax_elapsed,
                reply.semantic_elapsed,
                Some(publish_started.elapsed()),
                reply.syntax_work_mode,
                pipeline_started,
            )
            .await,
        )
    }

    async fn execute_save_fastlane_profile_once_v2(
        &self,
        uri: &Url,
        supersession_key: super::super::DiagnosticsSupersessionKeyV2,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        cancel_token: Option<&super::super::DiagnosticsCancellationTokenV2>,
        pipeline_started: Instant,
    ) -> bsl_runtime::application::DiagnosticsDisposition {
        let file_id = supersession_key.file_id;
        let requested_version = supersession_key.requested_version;
        let requested_generation = supersession_key.diagnostics_generation;
        let profile = supersession_key.profile;
        let blocking_queue_wait_elapsed = None;
        let wait_for_file_version_elapsed = None;
        let snapshot_with_deps_elapsed = None;

        let (diagnostics, syntax_errors, syntax_mode, syntax_elapsed) = if let Some(result) = self
            .try_collect_save_fastlane_diagnostics_from_applied_analysis_v2(
                uri,
                file_id,
                requested_version,
            )
            .await
        {
            result
        } else if let Some(result) = self
            .try_collect_save_fastlane_diagnostics_from_ready_parse_snapshot_v2(
                uri,
                file_id,
                requested_version,
            )
            .await
        {
            result
        } else {
            let Some(shadow_state) = self
                .latest_document_shadow_state_v2
                .read()
                .await
                .get(&file_id)
                .cloned()
            else {
                let disposition = self.diagnostics_cancelled_disposition_v2(
                    cancel_token,
                    self.current_diagnostics_generation_v2(file_id).await,
                    requested_generation,
                    self.latest_received_file_versions_v2
                        .read()
                        .await
                        .get(&file_id)
                        .copied(),
                    requested_version,
                );
                self.record_diagnostics_pipeline_event_v2(trigger, profile, disposition);
                return self
                    .finalize_diagnostics_save_profile_result_v2(
                        uri,
                        &supersession_key,
                        trigger,
                        disposition,
                        None,
                        blocking_queue_wait_elapsed,
                        wait_for_file_version_elapsed,
                        snapshot_with_deps_elapsed,
                        None,
                        None,
                        None,
                        None,
                        pipeline_started,
                    )
                    .await;
            };

            if shadow_state.version != requested_version {
                let disposition =
                    bsl_runtime::application::DiagnosticsDisposition::SupersededVersion;
                self.record_diagnostics_pipeline_event_v2(trigger, profile, disposition);
                return self
                    .finalize_diagnostics_save_profile_result_v2(
                        uri,
                        &supersession_key,
                        trigger,
                        disposition,
                        None,
                        blocking_queue_wait_elapsed,
                        wait_for_file_version_elapsed,
                        snapshot_with_deps_elapsed,
                        None,
                        None,
                        None,
                        None,
                        pipeline_started,
                    )
                    .await;
            }

            let uri_for_parse = uri.clone();
            let shadow_text = shadow_state.text;
            // Save freshness must not sit behind unrelated shared interactive blocking work.
            let syntax_result = tokio::task::spawn_blocking(move || {
                maybe_inject_save_fastlane_shadow_parse_delay_for_test();
                let started = Instant::now();
                let syntax_errors = bsl_syntax::syntax_errors_only(shadow_text.as_ref());
                let elapsed = started.elapsed();
                match syntax_errors {
                    Ok(syntax_errors) => {
                        let line_index = bsl_line_index::LineIndex::new(shadow_text.as_ref());
                        let diagnostics = syntax_errors_to_diagnostics(
                            &syntax_errors,
                            &uri_for_parse,
                            shadow_text.as_ref(),
                            &line_index,
                        );
                        Ok((syntax_errors, diagnostics, elapsed))
                    }
                    Err(err) => Err((err, elapsed)),
                }
            })
            .await;

            match syntax_result {
                Ok(Ok((syntax_errors, diagnostics, syntax_elapsed))) => {
                    (diagnostics, syntax_errors, "other", syntax_elapsed)
                }
                Ok(Err((err, syntax_elapsed))) => {
                    self.coordinator
                        .record_intellisense_v2_syntax_diagnostics_query_latency_with_origin_and_mode(
                            bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                            "other",
                            syntax_elapsed,
                        );
                    warn!(
                        uri = %uri,
                        file_id = file_id.0,
                        expected_version = requested_version,
                        expected_generation = requested_generation,
                        profile = profile.as_str(),
                        error = ?err,
                        "diagnostics_v2: save_fastlane syntax parse failed"
                    );
                    let disposition = bsl_runtime::application::DiagnosticsDisposition::OtherCancel;
                    self.record_diagnostics_pipeline_event_v2(trigger, profile, disposition);
                    return self
                        .finalize_diagnostics_save_profile_result_v2(
                            uri,
                            &supersession_key,
                            trigger,
                            disposition,
                            None,
                            blocking_queue_wait_elapsed,
                            wait_for_file_version_elapsed,
                            snapshot_with_deps_elapsed,
                            Some(syntax_elapsed),
                            None,
                            None,
                            Some("recomputed"),
                            pipeline_started,
                        )
                        .await;
                }
                Err(err) => {
                    warn!(
                        uri = %uri,
                        file_id = file_id.0,
                        expected_version = requested_version,
                        expected_generation = requested_generation,
                        profile = profile.as_str(),
                        error = ?err,
                        "diagnostics_v2: save_fastlane spawn_blocking failed"
                    );
                    let disposition = self.diagnostics_cancelled_disposition_v2(
                        cancel_token,
                        self.current_diagnostics_generation_v2(file_id).await,
                        requested_generation,
                        self.latest_received_file_versions_v2
                            .read()
                            .await
                            .get(&file_id)
                            .copied(),
                        requested_version,
                    );
                    self.record_diagnostics_pipeline_event_v2(trigger, profile, disposition);
                    return self
                        .finalize_diagnostics_save_profile_result_v2(
                            uri,
                            &supersession_key,
                            trigger,
                            disposition,
                            None,
                            blocking_queue_wait_elapsed,
                            wait_for_file_version_elapsed,
                            snapshot_with_deps_elapsed,
                            None,
                            None,
                            None,
                            None,
                            pipeline_started,
                        )
                        .await;
                }
            }
        };

        self.record_save_fastlane_syntax_artifacts_v2(file_id, requested_version, syntax_errors)
            .await;

        self.coordinator
            .record_intellisense_v2_syntax_diagnostics_query_latency_with_origin_and_mode(
                bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                syntax_mode,
                syntax_elapsed,
            );

        if let Some(disposition) = self
            .diagnostics_publish_checkpoint_v2(&supersession_key, trigger, cancel_token, None, None)
            .await
        {
            return self
                .finalize_diagnostics_save_profile_result_v2(
                    uri,
                    &supersession_key,
                    trigger,
                    disposition,
                    None,
                    blocking_queue_wait_elapsed,
                    wait_for_file_version_elapsed,
                    snapshot_with_deps_elapsed,
                    Some(syntax_elapsed),
                    None,
                    None,
                    Some("recomputed"),
                    pipeline_started,
                )
                .await;
        }

        let publish_started = Instant::now();
        let disposition = self
            .publish_diagnostics_v2(
                &supersession_key,
                uri,
                diagnostics,
                trigger,
                profile,
                pipeline_started,
            )
            .await;
        self.finalize_diagnostics_save_profile_result_v2(
            uri,
            &supersession_key,
            trigger,
            disposition,
            Some("syntax_only"),
            blocking_queue_wait_elapsed,
            wait_for_file_version_elapsed,
            snapshot_with_deps_elapsed,
            Some(syntax_elapsed),
            None,
            Some(publish_started.elapsed()),
            Some("recomputed"),
            pipeline_started,
        )
        .await
    }

    fn configured_workspace_root_for_semantic_v2(
        config: Option<&crate::config::LspConfig>,
    ) -> Option<std::path::PathBuf> {
        let path = config
            .and_then(|cfg| cfg.configuration_path.as_ref())
            .map(std::path::PathBuf::from)?;
        if path.is_dir() {
            return Some(path);
        }
        if path.file_name().and_then(|name| name.to_str()) == Some("Configuration.xml") {
            return path.parent().map(std::path::Path::to_path_buf);
        }
        None
    }

    fn resolve_path_scope_for_semantic_v2(path: &std::path::Path) -> Option<std::path::PathBuf> {
        if path.exists() {
            path.canonicalize().ok()
        } else if path.is_absolute() {
            Some(path.to_path_buf())
        } else {
            None
        }
    }

    async fn should_run_semantic_diagnostics_for_uri_v2(&self, uri: &Url) -> bool {
        let config = self.config.read().await.clone();
        let Some(config_root) = Self::configured_workspace_root_for_semantic_v2(config.as_ref())
        else {
            return true;
        };
        let Ok(document_path) = uri.to_file_path() else {
            return true;
        };
        let Some(config_root) = Self::resolve_path_scope_for_semantic_v2(&config_root) else {
            return true;
        };
        let Some(document_path) = Self::resolve_path_scope_for_semantic_v2(&document_path) else {
            return true;
        };
        let in_config_root = document_path.starts_with(&config_root);
        if !in_config_root {
            debug!(
                uri = %uri,
                document_path = %document_path.display(),
                config_root = %config_root.display(),
                "diagnostics_v2: skip semantic stage for document outside configured configurationPath"
            );
        }
        in_config_root
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
            save_cycle_sequence: None,
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
        save_cycle_sequence: Option<u64>,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        profile: bsl_runtime::application::DiagnosticsProfile,
        debounce: bool,
    ) {
        let slot_key = super::super::DiagnosticsTaskKeyV2 { file_id, profile };
        let supersession_key = super::super::DiagnosticsSupersessionKeyV2 {
            file_id,
            profile,
            diagnostics_generation,
            save_cycle_sequence,
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
                if let Some(save_cycle_sequence) = task.supersession_key.save_cycle_sequence {
                    self.record_diagnostics_save_timeline_profile_disposition(
                        &uri,
                        super::super::DiagnosticsSaveTimelineCycleKey {
                            file_id: task.supersession_key.file_id,
                            diagnostics_generation: task.supersession_key.diagnostics_generation,
                            save_cycle_sequence,
                            requested_version: task.supersession_key.requested_version,
                        },
                        task.supersession_key.profile,
                        reason.to_disposition(),
                    );
                }
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
        let pipeline_started = Instant::now();
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
        if matches!(
            profile,
            bsl_runtime::application::DiagnosticsProfile::SaveFastlane
        ) {
            return self
                .execute_save_fastlane_profile_once_v2(
                    uri,
                    supersession_key,
                    trigger,
                    cancel_token,
                    pipeline_started,
                )
                .await;
        }

        let (show_hints, flow_sensitive_enabled) = {
            let settings = self.settings.read().await;
            (
                settings.diagnostics.show_hints,
                settings.enable_flow_sensitive,
            )
        };
        let plan =
            bsl_runtime::application::diagnostics_execution_plan(profile, flow_sensitive_enabled);
        let run_semantic =
            plan.run_semantic && self.should_run_semantic_diagnostics_for_uri_v2(uri).await;
        let save_followup_from_did_save = matches!(
            (trigger, profile),
            (
                bsl_runtime::application::DiagnosticsTrigger::DidSave,
                bsl_runtime::application::DiagnosticsProfile::IdleHeavy
            )
        );
        if !plan.run_syntax && !run_semantic {
            self.record_diagnostics_pipeline_event_v2(
                trigger,
                profile,
                bsl_runtime::application::DiagnosticsDisposition::Published,
            );
            return self
                .finalize_diagnostics_save_profile_result_v2(
                    uri,
                    &supersession_key,
                    trigger,
                    bsl_runtime::application::DiagnosticsDisposition::Published,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    pipeline_started,
                )
                .await;
        }

        let mut followup_syntax_artifact_reuse_allowed = false;
        if save_followup_from_did_save && run_semantic {
            let applied_revision_matches_requested = || {
                self.analysis_v2
                    .cached_file_revision_state(file_id)
                    .is_some_and(|state| state.version == requested_version)
            };
            self.record_diagnostics_save_followup_wait_state_v2(
                uri,
                &supersession_key,
                "pending_publish",
                None,
                None,
                None,
            );
            if matches!(
                self.wait_for_save_fastlane_first_publish_v2(&supersession_key, cancel_token)
                    .await,
                SaveFastlaneFirstPublishWaitOutcome::Published
            ) {
                followup_syntax_artifact_reuse_allowed = plan.run_syntax
                    && self
                        .save_fastlane_syntax_artifacts_for_version_v2(file_id, requested_version)
                        .await
                        .is_some();
                if applied_revision_matches_requested() {
                    if let Some(disposition) = self
                        .try_execute_save_followup_from_applied_state_v2(
                            uri,
                            &supersession_key,
                            trigger,
                            cancel_token,
                            pipeline_started,
                            show_hints,
                            plan.flow_sensitive_semantic,
                        )
                        .await
                    {
                        return disposition;
                    }
                }
                if let Some(disposition) = self
                    .try_execute_save_followup_from_ready_artifacts_v2(
                        uri,
                        &supersession_key,
                        trigger,
                        cancel_token,
                        pipeline_started,
                        show_hints,
                        plan.flow_sensitive_semantic,
                    )
                    .await
                {
                    return disposition;
                }
                let wait_reason = if applied_revision_matches_requested() {
                    "semantic_work"
                } else {
                    "apply_lag"
                };
                self.record_diagnostics_save_followup_wait_state_v2(
                    uri,
                    &supersession_key,
                    wait_reason,
                    None,
                    None,
                    None,
                );
            } else {
                let wait_reason = if applied_revision_matches_requested() {
                    "semantic_work"
                } else {
                    "apply_lag"
                };
                self.record_diagnostics_save_followup_wait_state_v2(
                    uri,
                    &supersession_key,
                    wait_reason,
                    None,
                    None,
                    None,
                );
            }
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
                return self
                    .finalize_diagnostics_save_profile_result_v2(
                        uri,
                        &supersession_key,
                        trigger,
                        disposition,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        pipeline_started,
                    )
                    .await;
            }
        };

        let wait_elapsed = prepared.wait_elapsed.unwrap_or(Duration::ZERO);
        let mut syntax_stage_elapsed = None;
        let mut semantic_stage_elapsed = None;
        let save_fastlane_syntax_artifacts = if save_followup_from_did_save
            && run_semantic
            && plan.run_syntax
            && followup_syntax_artifact_reuse_allowed
        {
            self.save_fastlane_syntax_artifacts_for_version_v2(file_id, requested_version)
                .await
        } else {
            None
        };
        let followup_syntax_work_mode =
            if save_followup_from_did_save && run_semantic && plan.run_syntax {
                Some(if save_fastlane_syntax_artifacts.is_some() {
                    "reused"
                } else {
                    "recomputed"
                })
            } else {
                None
            };
        if save_followup_from_did_save && run_semantic {
            self.record_diagnostics_save_followup_wait_state_v2(
                uri,
                &supersession_key,
                "semantic_work",
                Some(wait_elapsed),
                Some(prepared.snapshot_elapsed),
                followup_syntax_work_mode,
            );
        }
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
        let index_snapshot_id = prepared.index_snapshot.id.as_str().to_string();
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
            if let Some(syntax_errors) = save_fastlane_syntax_artifacts {
                if let (Some(text), Some(index)) = (file_text.as_deref(), line_index.as_deref()) {
                    diagnostics.extend(syntax_errors_to_diagnostics(
                        syntax_errors.as_ref(),
                        uri,
                        text,
                        index,
                    ));
                }
            } else {
                self.coordinator
                    .record_intellisense_v2_payload_shape_with_origin(
                        context.origin.as_str(),
                        context.operation.as_str(),
                        bsl_runtime::application::ObservabilityStage::SyntaxDiagnosticsQuery
                            .as_str(),
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
                        syntax_stage_elapsed = Some(syntax_elapsed);
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
            return self
                .finalize_diagnostics_save_profile_result_v2(
                    uri,
                    &supersession_key,
                    trigger,
                    disposition,
                    None,
                    None,
                    Some(wait_elapsed),
                    Some(snapshot_elapsed),
                    syntax_stage_elapsed,
                    semantic_stage_elapsed,
                    None,
                    None,
                    pipeline_started,
                )
                .await;
        }

        if run_semantic {
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
                return self
                    .finalize_diagnostics_save_profile_result_v2(
                        uri,
                        &supersession_key,
                        trigger,
                        disposition,
                        None,
                        None,
                        Some(wait_elapsed),
                        Some(snapshot_elapsed),
                        syntax_stage_elapsed,
                        semantic_stage_elapsed,
                        None,
                        followup_syntax_work_mode,
                        pipeline_started,
                    )
                    .await;
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
                                analysis.semantic_diagnostics_flow_sensitive_profiled(file_id)
                            } else {
                                analysis.semantic_diagnostics_profiled(file_id)
                            }
                        },
                    );
                    let elapsed = started.elapsed();
                    let duration_from_profile_ms = |value: u128| {
                        Duration::from_millis(value.min(u64::MAX as u128) as u64)
                    };
                    match query {
                        Ok(Some(profiled)) => {
                            coordinator_for_blocking
                                .record_intellisense_v2_semantic_diagnostics_query_breakdown(
                                    duration_from_profile_ms(profiled.profile.inputs_ms),
                                    duration_from_profile_ms(profiled.profile.parse_result_ms),
                                    duration_from_profile_ms(profiled.profile.ir_ms),
                                    duration_from_profile_ms(profiled.profile.collect_ms),
                                    (profiled.profile.flow_sensitive_ms > 0).then(|| {
                                        duration_from_profile_ms(profiled.profile.flow_sensitive_ms)
                                    }),
                                );
                            let mut diagnostics = Vec::new();
                            for error in profiled.diagnostics.iter() {
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
                    semantic_stage_elapsed = Some(semantic_elapsed);
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
            return self
                .finalize_diagnostics_save_profile_result_v2(
                    uri,
                    &supersession_key,
                    trigger,
                    disposition,
                    None,
                    None,
                    Some(wait_elapsed),
                    Some(snapshot_elapsed),
                    syntax_stage_elapsed,
                    semantic_stage_elapsed,
                    None,
                    followup_syntax_work_mode,
                    pipeline_started,
                )
                .await;
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
            return self
                .finalize_diagnostics_save_profile_result_v2(
                    uri,
                    &supersession_key,
                    trigger,
                    disposition,
                    None,
                    None,
                    Some(wait_elapsed),
                    Some(snapshot_elapsed),
                    syntax_stage_elapsed,
                    semantic_stage_elapsed,
                    None,
                    followup_syntax_work_mode,
                    pipeline_started,
                )
                .await;
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

        if save_followup_from_did_save && run_semantic {
            self.record_diagnostics_save_followup_wait_state_v2(
                uri,
                &supersession_key,
                "pending_publish",
                Some(wait_elapsed),
                Some(snapshot_elapsed),
                followup_syntax_work_mode,
            );
        }
        let publish_kind = if run_semantic { "full" } else { "syntax_only" };
        let publish_started = Instant::now();
        let disposition = self
            .publish_diagnostics_v2(
                &supersession_key,
                uri,
                diagnostics,
                trigger,
                profile,
                pipeline_started,
            )
            .await;
        self.finalize_diagnostics_save_profile_result_v2(
            uri,
            &supersession_key,
            trigger,
            disposition,
            Some(publish_kind),
            None,
            Some(wait_elapsed),
            Some(snapshot_elapsed),
            syntax_stage_elapsed,
            semantic_stage_elapsed,
            Some(publish_started.elapsed()),
            followup_syntax_work_mode,
            pipeline_started,
        )
        .await
    }
}
