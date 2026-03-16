use super::*;
use std::time::Duration;

impl BslLanguageServer {
    pub(super) async fn lsp_did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        let version = params.text_document.version;

        let _sync_guard = self.text_sync_v2.lock().await;

        self.sync_v2_globals().await;
        let file_id = self.get_or_create_file_id_v2(&uri).await;
        let completion_knobs =
            bsl_runtime::application::CompletionPipelineKnobs::from_runtime_config();
        self.completion_dispatcher_v2
            .set_queue_capacity(completion_knobs.queue_capacity)
            .await;
        let completion_mode = completion_knobs.mode;
        if completion_dispatch_enabled_for_mode(completion_mode) {
            let open_ticket = self
                .completion_dispatcher_v2
                .emit_did_open(file_id, version)
                .await;
            if completion_queue_enqueue_failed(open_ticket.queue_outcome) {
                debug!(
                    uri = %uri,
                    file_id = file_id.0,
                    file_seq = open_ticket.file_seq,
                    request_epoch = open_ticket.request_epoch,
                    queue_outcome = ?open_ticket.queue_outcome,
                    "completion dispatcher dropped didOpen event"
                );
            }
        }
        let path = match uri.to_file_path() {
            Ok(path) => path.to_string_lossy().to_string(),
            Err(_) => uri.to_string(),
        };
        let text: Arc<str> = Arc::from(text);
        let path: Arc<str> = Arc::from(path);
        let parse_snapshot = self
            .coordinator
            .parser_coordinator()
            .and_then(|parser| {
                let parse_started = Instant::now();
                let report = parser
                    .parse_incremental_with_report(
                        PathBuf::from(path.as_ref()),
                        text.to_string(),
                        Vec::new(),
                    )
                    .ok()?;
                Some((report, parse_started.elapsed()))
            })
            .map(|(report, parse_elapsed)| {
                let mode = if report.incremental {
                    if report.changed_ranges.is_empty() {
                        "reused"
                    } else {
                        "incremental"
                    }
                } else {
                    "full"
                };
                let changed_ranges_count = report.changed_ranges.len();
                let changed_ranges_bytes: usize = report
                    .changed_ranges
                    .iter()
                    .map(changed_range_footprint_bytes)
                    .sum();
                self.coordinator.record_intellisense_v2_parse_snapshot(
                    bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                    mode,
                    changed_ranges_count,
                    changed_ranges_bytes,
                    report.fallback_reason.as_deref(),
                    parse_elapsed,
                );
                bsl_analysis_v2::ParseSnapshot {
                    file_id,
                    file_version: version,
                    parse_result: Arc::new(report.parse_result),
                    line_index: report.line_index,
                    backend_tree: report.backend_tree,
                    changed_ranges: Arc::new(
                        report
                            .changed_ranges
                            .into_iter()
                            .map(|range| bsl_analysis_v2::ParseChangedRange {
                                start_byte: range.start_byte,
                                old_end_byte: range.old_end_byte,
                                new_end_byte: range.new_end_byte,
                            })
                            .collect(),
                    ),
                    produced_at_millis: unix_time_millis(),
                    backend_tree_hash: report.backend_tree_hash,
                    incremental: report.incremental,
                    fallback_reason: report.fallback_reason.map(Arc::from),
                }
            });

        self.latest_received_file_versions_v2
            .write()
            .await
            .insert(file_id, version);
        self.latest_document_shadow_state_v2.write().await.insert(
            file_id,
            super::super::DocumentShadowStateV2 {
                version,
                text: text.clone(),
            },
        );
        self.latest_apply_enqueued_at_v2
            .write()
            .await
            .insert(file_id, Instant::now());

        self.analysis_v2.apply_changes_interactive(
            bsl_runtime::application::ObservabilityOrigin::Lsp,
            vec![if let Some(parse_snapshot) = parse_snapshot {
                bsl_analysis_v2::Change::SetFileWithSnapshot {
                    file_id,
                    text,
                    version,
                    path,
                    parse_snapshot,
                }
            } else {
                bsl_analysis_v2::Change::SetFile {
                    file_id,
                    text,
                    version,
                    path,
                }
            }],
        );
        self.schedule_type_index_precompute_v2(file_id, version)
            .await;

        let diagnostics_generation = self.bump_diagnostics_generation_v2(file_id).await;
        for profile in bsl_runtime::application::diagnostics_profiles_for_trigger(
            bsl_runtime::application::DiagnosticsTrigger::DidOpen,
        ) {
            self.schedule_diagnostics_profile_v2(
                uri.clone(),
                file_id,
                version,
                diagnostics_generation,
                bsl_runtime::application::DiagnosticsTrigger::DidOpen,
                *profile,
                false,
            )
            .await;
        }

        self.client
            .log_message(
                MessageType::INFO,
                format!("Opened document (v2 diagnostics scheduled): {}", uri),
            )
            .await;
    }

    pub(super) async fn lsp_did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        let changes = params.content_changes;

        let _sync_guard = self.text_sync_v2.lock().await;

        self.sync_v2_globals().await;
        let file_id = self.get_or_create_file_id_v2(&uri).await;
        let completion_knobs =
            bsl_runtime::application::CompletionPipelineKnobs::from_runtime_config();
        self.completion_dispatcher_v2
            .set_queue_capacity(completion_knobs.queue_capacity)
            .await;
        let completion_mode = completion_knobs.mode;
        if completion_dispatch_enabled_for_mode(completion_mode) {
            let change_ticket = self
                .completion_dispatcher_v2
                .emit_did_change(file_id, version)
                .await;
            if completion_queue_enqueue_failed(change_ticket.queue_outcome) {
                debug!(
                    uri = %uri,
                    file_id = file_id.0,
                    file_seq = change_ticket.file_seq,
                    request_epoch = change_ticket.request_epoch,
                    queue_outcome = ?change_ticket.queue_outcome,
                    "completion dispatcher dropped didChange event"
                );
            }
        }
        let path = match uri.to_file_path() {
            Ok(path) => path.to_string_lossy().to_string(),
            Err(_) => uri.to_string(),
        };

        // Apply changes
        let (updated_text, parser_edits) =
            if let Some(full_change) = changes.iter().find(|c| c.range.is_none()) {
                (full_change.text.clone(), Vec::new())
            } else {
                let shadow_state = {
                    let shadow = self.latest_document_shadow_state_v2.read().await;
                    shadow.get(&file_id).cloned()
                };
                if let Some(state) = shadow_state.as_ref() {
                    if version < state.version {
                        warn!(
                            uri = %uri,
                            file_id = file_id.0,
                            requested_version = version,
                            shadow_version = state.version,
                            "Skipping out-of-order didChange for older version"
                        );
                        return;
                    }
                }
                let base_text = if let Some(state) = shadow_state {
                    state.text.to_string()
                } else {
                    self.analysis_v2
                        .snapshot()
                        .await
                        .file_text(file_id)
                        .ok()
                        .flatten()
                        .map(|text| text.to_string())
                        .unwrap_or_default()
                };

                let mut current_text = base_text;
                let mut parser_edits = Vec::new();
                for change in &changes {
                    if let Some(range) = change.range {
                        if let Some(edit) = lsp_range_change_to_parser_edit(change) {
                            parser_edits.push(edit);
                        }
                        current_text = apply_text_edit(&current_text, range, &change.text);
                    }
                }
                (current_text, parser_edits)
            };

        let scale_aware_knobs =
            bsl_runtime::application::ScaleAwareDiagnosticsKnobs::from_runtime_config();
        let mut large_churn_active = false;
        if scale_aware_knobs.enabled {
            let is_large_document = bsl_runtime::application::scale_aware_document_is_large(
                &updated_text,
                scale_aware_knobs,
            );
            let now = Instant::now();
            let transition = {
                let mut churn_state = self.scale_aware_churn_state_v2.write().await;
                let state =
                    churn_state
                        .entry(file_id)
                        .or_insert(super::super::ScaleAwareChurnStateV2 {
                            window_started_at: now,
                            changes_in_window: 0,
                            large_churn_active: false,
                        });
                let transition =
                    advance_large_churn_state(state, now, is_large_document, scale_aware_knobs);
                large_churn_active = state.large_churn_active;
                transition
            };
            match transition {
                LargeChurnTransition::Entered => self
                    .coordinator
                    .record_intellisense_v2_large_churn_transition(
                        bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                        "enter",
                    ),
                LargeChurnTransition::Exited => self
                    .coordinator
                    .record_intellisense_v2_large_churn_transition(
                        bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                        "exit",
                    ),
                LargeChurnTransition::None => {}
            }
        } else {
            let was_active = self
                .scale_aware_churn_state_v2
                .write()
                .await
                .remove(&file_id)
                .is_some_and(|state| state.large_churn_active);
            if was_active {
                self.coordinator
                    .record_intellisense_v2_large_churn_transition(
                        bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                        "exit",
                    );
            }
        }

        self.latest_received_file_versions_v2
            .write()
            .await
            .insert(file_id, version);
        let updated_text: Arc<str> = Arc::from(updated_text);
        let path: Arc<str> = Arc::from(path);
        self.latest_document_shadow_state_v2.write().await.insert(
            file_id,
            super::super::DocumentShadowStateV2 {
                version,
                text: updated_text.clone(),
            },
        );
        let parse_snapshot = if large_churn_active {
            // Under large+churn we must not keep synchronous parse work on the didChange path.
            self.coordinator.record_intellisense_v2_parse_snapshot(
                bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                "other",
                0,
                0,
                Some("other"),
                Duration::default(),
            );
            None
        } else {
            self.coordinator
                .parser_coordinator()
                .and_then(|parser| {
                    let parse_started = Instant::now();
                    let report = parser
                        .parse_incremental_with_report(
                            PathBuf::from(path.as_ref()),
                            updated_text.to_string(),
                            parser_edits,
                        )
                        .ok()?;
                    Some((report, parse_started.elapsed()))
                })
                .map(|(report, parse_elapsed)| {
                    let mode = if report.incremental {
                        if report.changed_ranges.is_empty() {
                            "reused"
                        } else {
                            "incremental"
                        }
                    } else {
                        "full"
                    };
                    let changed_ranges_count = report.changed_ranges.len();
                    let changed_ranges_bytes: usize = report
                        .changed_ranges
                        .iter()
                        .map(changed_range_footprint_bytes)
                        .sum();
                    self.coordinator.record_intellisense_v2_parse_snapshot(
                        bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                        mode,
                        changed_ranges_count,
                        changed_ranges_bytes,
                        report.fallback_reason.as_deref(),
                        parse_elapsed,
                    );
                    bsl_analysis_v2::ParseSnapshot {
                        file_id,
                        file_version: version,
                        parse_result: Arc::new(report.parse_result),
                        line_index: report.line_index,
                        backend_tree: report.backend_tree,
                        changed_ranges: Arc::new(
                            report
                                .changed_ranges
                                .into_iter()
                                .map(|range| bsl_analysis_v2::ParseChangedRange {
                                    start_byte: range.start_byte,
                                    old_end_byte: range.old_end_byte,
                                    new_end_byte: range.new_end_byte,
                                })
                                .collect(),
                        ),
                        produced_at_millis: unix_time_millis(),
                        backend_tree_hash: report.backend_tree_hash,
                        incremental: report.incremental,
                        fallback_reason: report.fallback_reason.map(Arc::from),
                    }
                })
        };
        self.latest_apply_enqueued_at_v2
            .write()
            .await
            .insert(file_id, Instant::now());
        self.analysis_v2.apply_changes_interactive(
            bsl_runtime::application::ObservabilityOrigin::Lsp,
            vec![if let Some(parse_snapshot) = parse_snapshot {
                bsl_analysis_v2::Change::SetFileWithSnapshot {
                    file_id,
                    text: updated_text,
                    version,
                    path,
                    parse_snapshot,
                }
            } else {
                bsl_analysis_v2::Change::SetFile {
                    file_id,
                    text: updated_text,
                    version,
                    path,
                }
            }],
        );
        self.schedule_type_index_precompute_v2(file_id, version)
            .await;

        let flow_sensitive_enabled = {
            let settings = self.settings.read().await;
            settings.enable_flow_sensitive
        };
        let diagnostics_generation = self.bump_diagnostics_generation_v2(file_id).await;
        for profile in bsl_runtime::application::diagnostics_profiles_for_trigger(
            bsl_runtime::application::DiagnosticsTrigger::DidChange,
        ) {
            if !should_schedule_profile(
                bsl_runtime::application::DiagnosticsTrigger::DidChange,
                *profile,
                flow_sensitive_enabled,
            ) {
                continue;
            }
            if should_defer_heavy_diagnostics_for_large_churn(
                bsl_runtime::application::DiagnosticsTrigger::DidChange,
                *profile,
                large_churn_active,
            ) {
                self.coordinator
                    .record_intellisense_v2_heavy_diagnostics_deferred(
                        bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                        profile.as_str(),
                        bsl_runtime::application::DeferredHeavyDiagnosticsReason::LargeAndChurn
                            .as_str(),
                    );
                self.schedule_diagnostics_profile_v2(
                    uri.clone(),
                    file_id,
                    version,
                    diagnostics_generation,
                    bsl_runtime::application::DiagnosticsTrigger::Idle,
                    *profile,
                    true,
                )
                .await;
                continue;
            }
            match profile {
                bsl_runtime::application::DiagnosticsProfile::Fast => {
                    self.run_diagnostics_profile_immediate_v2(
                        uri.clone(),
                        file_id,
                        version,
                        diagnostics_generation,
                        bsl_runtime::application::DiagnosticsTrigger::DidChange,
                        *profile,
                    )
                    .await;
                }
                _ => {
                    let trigger = match profile {
                        bsl_runtime::application::DiagnosticsProfile::IdleHeavy => {
                            bsl_runtime::application::DiagnosticsTrigger::Idle
                        }
                        _ => bsl_runtime::application::DiagnosticsTrigger::DidChange,
                    };
                    self.schedule_diagnostics_profile_v2(
                        uri.clone(),
                        file_id,
                        version,
                        diagnostics_generation,
                        trigger,
                        *profile,
                        true,
                    )
                    .await;
                }
            }
        }
    }

    pub(super) async fn lsp_did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        let Some(file_id) = self.get_file_id_v2(&uri).await else {
            return;
        };
        let Some(version) = self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied()
        else {
            return;
        };

        let flow_sensitive_enabled = {
            let settings = self.settings.read().await;
            settings.enable_flow_sensitive
        };
        let diagnostics_generation = self.bump_diagnostics_generation_v2(file_id).await;
        for profile in bsl_runtime::application::diagnostics_profiles_for_trigger(
            bsl_runtime::application::DiagnosticsTrigger::DidSave,
        ) {
            if !should_schedule_profile(
                bsl_runtime::application::DiagnosticsTrigger::DidSave,
                *profile,
                flow_sensitive_enabled,
            ) {
                continue;
            }
            self.schedule_diagnostics_profile_v2(
                uri.clone(),
                file_id,
                version,
                diagnostics_generation,
                bsl_runtime::application::DiagnosticsTrigger::DidSave,
                *profile,
                false,
            )
            .await;
        }
    }

    pub(super) async fn lsp_did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;

        let _sync_guard = self.text_sync_v2.lock().await;

        if let Some(file_id) = self.get_file_id_v2(&uri).await {
            let close_ticket = self
                .completion_dispatcher_v2
                .close_file_dispatcher(file_id)
                .await;
            if close_ticket
                .map(|ticket| completion_queue_enqueue_failed(ticket.queue_outcome))
                .unwrap_or(false)
            {
                debug!(
                    uri = %uri,
                    file_id = file_id.0,
                    file_seq = ?close_ticket.map(|ticket| ticket.file_seq),
                    request_epoch = ?close_ticket.map(|ticket| ticket.request_epoch),
                    queue_outcome = ?close_ticket.map(|ticket| ticket.queue_outcome),
                    "completion dispatcher dropped didClose event"
                );
            }
            let removed_completion_cancellations = self
                .completion_cancellation_registry_v2
                .remove_file(file_id);
            if removed_completion_cancellations > 0 {
                debug!(
                    uri = %uri,
                    file_id = file_id.0,
                    removed_completion_cancellations,
                    "completion cancellation registry cleanup on didClose"
                );
            }
            self.cancel_diagnostics_v2(file_id).await;
            self.cancel_type_index_precompute_v2(file_id).await;
            self.latest_received_file_versions_v2
                .write()
                .await
                .remove(&file_id);
            self.latest_document_shadow_state_v2
                .write()
                .await
                .remove(&file_id);
            self.latest_apply_enqueued_at_v2
                .write()
                .await
                .remove(&file_id);
            self.completion_parity_state_v2
                .write()
                .await
                .retain(|(tracked_file_id, _, _, _), _| *tracked_file_id != file_id);
            self.diagnostics_generation_v2
                .write()
                .await
                .remove(&file_id);
            let had_large_churn = self
                .scale_aware_churn_state_v2
                .write()
                .await
                .remove(&file_id)
                .is_some_and(|state| state.large_churn_active);
            if had_large_churn {
                self.coordinator
                    .record_intellisense_v2_large_churn_transition(
                        bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                        "exit",
                    );
            }
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
}
