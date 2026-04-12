use super::super::{DocumentShadowStateV2, ReadyParseSnapshotStateV2};
use super::*;
#[cfg(test)]
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;

struct BuildParseSnapshotRequest {
    file_id: bsl_analysis_v2::FileId,
    version: i32,
    path: Arc<str>,
    text: Arc<str>,
    parser_edits: Vec<bsl_runtime::system::parser_coordinator::TextEdit>,
    blocking_delay_env_key: Option<&'static str>,
    requested_version_state: Option<Arc<std::sync::atomic::AtomicI32>>,
    did_change_attribution: Option<DidChangeParseSnapshotAttributionV2>,
}

#[derive(Debug, Clone)]
struct DidChangeParseSnapshotAttributionV2 {
    uri: Url,
    base_text_source: &'static str,
    change_shape: &'static str,
}

#[cfg(test)]
static DID_CHANGE_PARSE_DELAY_ACTIVE: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
static DID_SAVE_PARSE_DELAY_ACTIVE: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
async fn maybe_inject_parse_delay(env_key: &'static str, active_counter: &'static AtomicUsize) {
    if let Some(delay_ms) = std::env::var(env_key)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
    {
        active_counter.fetch_add(1, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        active_counter.fetch_sub(1, Ordering::SeqCst);
    }
}

#[cfg(test)]
async fn maybe_inject_did_change_parse_delay() {
    maybe_inject_parse_delay(
        "BSL_TEST_DID_CHANGE_PARSE_DELAY_MS",
        &DID_CHANGE_PARSE_DELAY_ACTIVE,
    )
    .await;
}

#[cfg(not(test))]
async fn maybe_inject_did_change_parse_delay() {}

#[cfg(test)]
async fn maybe_inject_did_save_parse_delay() {
    maybe_inject_parse_delay(
        "BSL_TEST_DID_SAVE_PARSE_DELAY_MS",
        &DID_SAVE_PARSE_DELAY_ACTIVE,
    )
    .await;
}

#[cfg(not(test))]
async fn maybe_inject_did_save_parse_delay() {}

#[cfg(test)]
fn maybe_inject_blocking_parse_delay_for_test(env_key: &'static str) {
    if let Some(delay_ms) = std::env::var(env_key)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
    {
        std::thread::sleep(Duration::from_millis(delay_ms));
    }
}

#[cfg(not(test))]
fn maybe_inject_blocking_parse_delay_for_test(_env_key: &'static str) {}

#[cfg(test)]
fn maybe_inject_current_revision_head_precompute_delay_for_test() {
    if let Some(delay_ms) = std::env::var("BSL_TEST_CURRENT_REVISION_HEAD_PRECOMPUTE_DELAY_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
    {
        static DELAY_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        let _delay_lock = DELAY_LOCK
            .get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        std::thread::sleep(Duration::from_millis(delay_ms));
    }
}

#[cfg(not(test))]
fn maybe_inject_current_revision_head_precompute_delay_for_test() {}

#[cfg(test)]
pub(super) fn did_change_inline_parse_delay_active_for_test() -> bool {
    DID_CHANGE_PARSE_DELAY_ACTIVE.load(Ordering::SeqCst) > 0
}

#[cfg(test)]
pub(super) fn did_save_inline_parse_delay_active_for_test() -> bool {
    DID_SAVE_PARSE_DELAY_ACTIVE.load(Ordering::SeqCst) > 0
}

fn parse_snapshot_from_report(
    file_id: bsl_analysis_v2::FileId,
    version: i32,
    report: bsl_runtime::system::parser_coordinator::ParseSnapshotReport,
) -> bsl_analysis_v2::ParseSnapshot {
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
}

fn parse_snapshot_mode_from_report(
    report: &bsl_runtime::system::parser_coordinator::ParseSnapshotReport,
) -> &'static str {
    if report.incremental {
        if report.changed_ranges.is_empty() {
            "reused"
        } else {
            "incremental"
        }
    } else {
        "full"
    }
}

fn did_change_parse_snapshot_change_shape(
    changes: &[TextDocumentContentChangeEvent],
) -> &'static str {
    let has_full_replace = changes.iter().any(|change| change.range.is_none());
    let has_ranged = changes.iter().any(|change| change.range.is_some());
    match (has_full_replace, has_ranged) {
        (true, true) => "mixed",
        (true, false) => "full_replace",
        (false, true) => "ranged",
        (false, false) => "other",
    }
}

enum ParseSnapshotAsyncDelayMode {
    None,
    DidChangeTestOnly,
    DidSaveTestOnly,
}

fn parse_snapshot_apply_debounce_duration() -> Duration {
    Duration::from_millis(25)
}

fn parse_snapshot_text_hash(text: &str) -> [u8; 32] {
    *blake3::hash(text.as_bytes()).as_bytes()
}

struct BackgroundParseSnapshotApplyArgs {
    file_id: bsl_analysis_v2::FileId,
    requested_version: i32,
    path: Arc<str>,
    text: Arc<str>,
    parser_edits: Vec<bsl_runtime::system::parser_coordinator::TextEdit>,
    async_delay_mode: ParseSnapshotAsyncDelayMode,
    blocking_delay_env_key: Option<&'static str>,
    force_reschedule_same_version: bool,
    source: super::super::BackgroundParseSnapshotApplyTaskSourceV2,
    did_change_attribution: Option<DidChangeParseSnapshotAttributionV2>,
}

impl BslLanguageServer {
    async fn record_ready_parse_snapshot_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        text: Arc<str>,
        parse_snapshot: &bsl_analysis_v2::ParseSnapshot,
    ) {
        self.latest_ready_parse_snapshots_v2.write().await.insert(
            file_id,
            ReadyParseSnapshotStateV2 {
                text,
                parse_snapshot: parse_snapshot.clone(),
            },
        );
    }

    fn record_parse_snapshot_report_v2(
        &self,
        report: &bsl_runtime::system::parser_coordinator::ParseSnapshotReport,
        parse_elapsed: Duration,
    ) {
        let mode = parse_snapshot_mode_from_report(report);
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
    }

    pub(crate) async fn run_completion_exact_ir_singleflight_prewarm_v2(
        &self,
        analysis: bsl_analysis_v2::AnalysisV2,
        file_id: bsl_analysis_v2::FileId,
        cpu_class: bsl_runtime::application::CpuWorkClass,
        inject_test_delay: bool,
    ) {
        let context = self
            .build_execution_context_v2(
                bsl_runtime::application::SemanticOperation::Completion,
                file_id,
                None,
                false,
            )
            .await;
        let coordinator = self.coordinator.clone();
        let observed_coordinator = coordinator.clone();
        let _ = bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
            cpu_class,
            bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
            Some(observed_coordinator.as_ref()),
            move || {
                if inject_test_delay {
                    maybe_inject_current_revision_head_precompute_delay_for_test();
                }
                let _ = bsl_runtime::application::IntellisenseV2Facade::run_ir_query_singleflight(
                    &context,
                    &analysis,
                    Some(coordinator.as_ref()),
                    file_id,
                );
            },
        )
        .await;
    }

    async fn build_parse_snapshot_v2(
        &self,
        request: BuildParseSnapshotRequest,
    ) -> Option<bsl_analysis_v2::ParseSnapshot> {
        let coordinator = self.coordinator.clone();
        let path_for_parse = request.path.clone();
        let text_for_parse = request.text.clone();
        let parse_started = Instant::now();
        let blocking_delay_env_key_for_parse = request.blocking_delay_env_key;
        let requested_version_state_for_parse = request.requested_version_state;
        let did_change_attribution = request.did_change_attribution.clone();
        let version = request.version;
        let file_id = request.file_id;
        let parser_edits = request.parser_edits;
        let report = bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
            bsl_runtime::application::CpuWorkClass::Background,
            bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
            Some(self.coordinator.as_ref()),
            move || {
                if let Some(env_key) = blocking_delay_env_key_for_parse {
                    maybe_inject_blocking_parse_delay_for_test(env_key);
                }
                if requested_version_state_for_parse
                    .as_ref()
                    .is_some_and(|state| state.load(Ordering::Relaxed) != version)
                {
                    return None;
                }
                coordinator.parser_coordinator().and_then(|parser| {
                    parser
                        .parse_incremental_with_report(
                            PathBuf::from(path_for_parse.as_ref()),
                            text_for_parse.to_string(),
                            parser_edits,
                        )
                        .ok()
                })
            },
        )
        .await
        .ok()
        .flatten()?;
        self.record_parse_snapshot_report_v2(&report, parse_started.elapsed());
        if let Some(attribution) = did_change_attribution.as_ref() {
            self.record_did_change_parse_snapshot_evidence(
                &attribution.uri,
                super::super::DidChangeParseSnapshotEvidenceKey {
                    file_id,
                    requested_version: version,
                },
                parse_snapshot_mode_from_report(&report),
                attribution.base_text_source,
                attribution.change_shape,
                report.changed_ranges.len(),
                report.fallback_reason.as_deref(),
            );
        }
        Some(parse_snapshot_from_report(file_id, version, report))
    }

    async fn schedule_background_parse_snapshot_apply_v2(
        &self,
        args: BackgroundParseSnapshotApplyArgs,
    ) {
        let mut tasks = self.background_parse_snapshot_apply_tasks_v2.lock().await;
        let file_id = args.file_id;
        let text_hash = parse_snapshot_text_hash(args.text.as_ref());
        if let Some(task) = tasks.get(&file_id) {
            let same_version =
                task.requested_version.load(Ordering::Relaxed) == args.requested_version;
            if same_version && task.text_hash == text_hash {
                return;
            }
            if !args.force_reschedule_same_version && same_version {
                return;
            }
        }
        if let Some(previous) = tasks.remove(&file_id) {
            previous.requested_version.store(0, Ordering::Relaxed);
            previous.handle.abort();
        }

        let requested_version_state =
            Arc::new(std::sync::atomic::AtomicI32::new(args.requested_version));
        let task_source = args.source;
        let server = self.clone();
        let worker_requested_version_state = Arc::clone(&requested_version_state);
        let handle = tokio::spawn(async move {
            server
                .run_background_parse_snapshot_apply_worker_v2(args, worker_requested_version_state)
                .await;
        });
        tasks.insert(
            file_id,
            super::super::BackgroundParseSnapshotApplyTaskV2 {
                requested_version: requested_version_state,
                text_hash,
                source: task_source,
                handle,
            },
        );
    }

    async fn run_background_parse_snapshot_apply_worker_v2(
        &self,
        args: BackgroundParseSnapshotApplyArgs,
        requested_version_state: Arc<std::sync::atomic::AtomicI32>,
    ) {
        let still_requested = |state: &Arc<std::sync::atomic::AtomicI32>, version: i32| {
            state.load(Ordering::Relaxed) == version
        };
        let file_id = args.file_id;

        let run = async {
            let debounce = parse_snapshot_apply_debounce_duration();
            if debounce > Duration::ZERO {
                tokio::time::sleep(debounce).await;
            }
            if !still_requested(&requested_version_state, args.requested_version) {
                return;
            }
            if self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&args.file_id)
                .copied()
                != Some(args.requested_version)
            {
                return;
            }
            match args.async_delay_mode {
                ParseSnapshotAsyncDelayMode::None => {}
                ParseSnapshotAsyncDelayMode::DidChangeTestOnly => {
                    maybe_inject_did_change_parse_delay().await;
                }
                ParseSnapshotAsyncDelayMode::DidSaveTestOnly => {
                    maybe_inject_did_save_parse_delay().await;
                }
            }
            if !still_requested(&requested_version_state, args.requested_version) {
                return;
            }
            if self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&args.file_id)
                .copied()
                != Some(args.requested_version)
            {
                return;
            }

            let Some(parse_snapshot) = self
                .build_parse_snapshot_v2(BuildParseSnapshotRequest {
                    file_id: args.file_id,
                    version: args.requested_version,
                    path: args.path.clone(),
                    text: args.text.clone(),
                    parser_edits: args.parser_edits,
                    blocking_delay_env_key: args.blocking_delay_env_key,
                    requested_version_state: Some(Arc::clone(&requested_version_state)),
                    did_change_attribution: args.did_change_attribution.clone(),
                })
                .await
            else {
                return;
            };
            self.record_ready_parse_snapshot_v2(args.file_id, args.text.clone(), &parse_snapshot)
                .await;
            let text_for_symbols = args.text.clone();
            let parse_result_for_symbols = Arc::clone(&parse_snapshot.parse_result);
            match bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
                bsl_runtime::application::CpuWorkClass::Background,
                bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                Some(self.coordinator.as_ref()),
                move || {
                    build_document_symbols(
                        text_for_symbols.as_ref(),
                        parse_result_for_symbols.as_ref(),
                    )
                    .map_err(|err| err.to_string())
                },
            )
            .await
            {
                Ok(Ok(response)) => {
                    self.record_document_symbol_ready_v2(
                        args.file_id,
                        args.requested_version,
                        response,
                    )
                    .await;
                }
                Ok(Err(err)) => {
                    warn!(
                        file_id = args.file_id.0,
                        file_version = args.requested_version,
                        error = %err,
                        "failed to build documentSymbol ready cache from parse snapshot"
                    );
                }
                Err(err) => {
                    warn!(
                        file_id = args.file_id.0,
                        file_version = args.requested_version,
                        error = %err,
                        "documentSymbol ready-cache task failed after parse snapshot"
                    );
                }
            }

            if !still_requested(&requested_version_state, args.requested_version) {
                return;
            }
            if self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&args.file_id)
                .copied()
                != Some(args.requested_version)
            {
                return;
            }
            if !self
                .analysis_v2
                .wait_for_file_version(args.file_id, args.requested_version)
                .await
            {
                return;
            }
            if !still_requested(&requested_version_state, args.requested_version) {
                return;
            }
            if self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&args.file_id)
                .copied()
                != Some(args.requested_version)
            {
                return;
            }

            // Snapshot-backed apply is enrichment for the already published current revision.
            // Keep it off the interactive writer queue so completion wait_for_file_version
            // is not blocked by slow snapshot installs on large modules.
            self.analysis_v2
                .apply_changes(vec![bsl_analysis_v2::Change::SetFileWithSnapshot {
                    file_id: args.file_id,
                    text: args.text,
                    version: args.requested_version,
                    path: args.path,
                    parse_snapshot,
                }]);
            self.spawn_completion_head_precompute_from_snapshot_v2(
                args.file_id,
                args.requested_version,
            );
        };
        run.await;

        let mut tasks = self.background_parse_snapshot_apply_tasks_v2.lock().await;
        if tasks
            .get(&file_id)
            .is_some_and(|task| Arc::ptr_eq(&task.requested_version, &requested_version_state))
        {
            tasks.remove(&file_id);
        }
    }

    fn spawn_completion_head_reuse_from_previous_version_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        requested_version: i32,
        previous_version: i32,
        previous_analysis: bsl_analysis_v2::AnalysisV2,
    ) {
        let server = self.clone();
        tokio::spawn(async move {
            let _ = bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
                bsl_runtime::application::CpuWorkClass::Interactive,
                bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                Some(server.coordinator.as_ref()),
                move || previous_analysis.ir(file_id),
            )
            .await;
            if !server
                .analysis_v2
                .wait_for_file_version_for_operation(
                    bsl_runtime::application::ObservabilityOrigin::Lsp,
                    bsl_runtime::application::SemanticOperation::Completion,
                    file_id,
                    requested_version,
                )
                .await
            {
                return;
            }
            if server
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                != Some(requested_version)
            {
                return;
            }

            let analysis = server
                .analysis_v2
                .completion_current_revision_snapshot_for_origin_and_operation(
                    bsl_runtime::application::ObservabilityOrigin::Lsp,
                    bsl_runtime::application::SemanticOperation::Completion,
                )
                .await
                .analysis;
            if analysis.file_version(file_id).ok().flatten() != Some(requested_version) {
                return;
            }
            let reused = analysis
                .try_publish_completion_head_from_previous_ir_reuse(
                    file_id,
                    requested_version,
                    previous_version,
                )
                .ok()
                .unwrap_or(false);
            if !reused {
                server
                    .run_completion_exact_ir_singleflight_prewarm_v2(
                        analysis,
                        file_id,
                        bsl_runtime::application::CpuWorkClass::Interactive,
                        false,
                    )
                    .await;
            }
        });
    }

    fn spawn_completion_head_version_alias_from_previous_version_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        requested_version: i32,
        previous_version: i32,
    ) {
        let server = self.clone();
        tokio::spawn(async move {
            if server
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                != Some(requested_version)
            {
                return;
            }
            if !server
                .analysis_v2
                .wait_for_file_version_for_operation(
                    bsl_runtime::application::ObservabilityOrigin::Lsp,
                    bsl_runtime::application::SemanticOperation::Completion,
                    file_id,
                    requested_version,
                )
                .await
            {
                return;
            }
            if server
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                != Some(requested_version)
            {
                return;
            }

            let analysis = server
                .analysis_v2
                .completion_current_revision_snapshot_for_origin_and_operation(
                    bsl_runtime::application::ObservabilityOrigin::Lsp,
                    bsl_runtime::application::SemanticOperation::Completion,
                )
                .await
                .analysis;
            if analysis.file_version(file_id).ok().flatten() != Some(requested_version) {
                return;
            }
            let _ = analysis.try_publish_completion_head_from_previous_ir_reuse_for_version(
                file_id,
                requested_version,
                previous_version,
            );
            if !analysis
                .current_type_index_serve_only_ready(file_id)
                .ok()
                .unwrap_or(false)
            {
                server
                    .run_completion_exact_ir_singleflight_prewarm_v2(
                        analysis,
                        file_id,
                        bsl_runtime::application::CpuWorkClass::Interactive,
                        false,
                    )
                    .await;
            }
        });
    }

    fn spawn_completion_head_precompute_from_snapshot_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        requested_version: i32,
    ) {
        let server = self.clone();
        tokio::spawn(async move {
            if server
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                != Some(requested_version)
            {
                return;
            }

            let analysis = server.analysis_v2.snapshot().await;
            if analysis.file_version(file_id).ok().flatten() != Some(requested_version) {
                return;
            }
            if analysis
                .current_completion_head_ready(file_id)
                .ok()
                .unwrap_or(false)
            {
                return;
            }

            server
                .run_completion_exact_ir_singleflight_prewarm_v2(
                    analysis,
                    file_id,
                    bsl_runtime::application::CpuWorkClass::Background,
                    false,
                )
                .await;
        });
    }

    async fn schedule_completion_head_precompute_from_current_revision_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        requested_version: i32,
    ) {
        let mut tasks = self.current_revision_head_precompute_tasks_v2.lock().await;
        if let Some(task) = tasks.get(&file_id) {
            if task.requested_version.load(Ordering::Relaxed) == requested_version {
                return;
            }
        }
        if let Some(previous) = tasks.remove(&file_id) {
            previous.requested_version.store(0, Ordering::Relaxed);
            previous.handle.abort();
        }

        let requested_version_state =
            Arc::new(std::sync::atomic::AtomicI32::new(requested_version));
        let server = self.clone();
        let worker_requested_version_state = Arc::clone(&requested_version_state);
        let handle = tokio::spawn(async move {
            server
                .run_current_revision_head_precompute_worker_v2(
                    file_id,
                    worker_requested_version_state,
                )
                .await;
        });
        tasks.insert(
            file_id,
            super::super::CurrentRevisionHeadPrecomputeTaskV2 {
                requested_version: requested_version_state,
                handle,
            },
        );
    }

    async fn run_current_revision_head_precompute_worker_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        requested_version_state: Arc<std::sync::atomic::AtomicI32>,
    ) {
        tokio::task::yield_now().await;
        loop {
            let requested_version = requested_version_state.load(Ordering::Relaxed);
            if requested_version <= 0 {
                break;
            }
            let latest_received_version = self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                .unwrap_or(0);
            if latest_received_version <= 0 {
                break;
            }
            if latest_received_version != requested_version {
                break;
            }
            if !self
                .analysis_v2
                .wait_for_file_version_for_operation(
                    bsl_runtime::application::ObservabilityOrigin::Lsp,
                    bsl_runtime::application::SemanticOperation::Completion,
                    file_id,
                    requested_version,
                )
                .await
            {
                break;
            }
            let latest_received_version = self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                .unwrap_or(0);
            if latest_received_version != requested_version {
                break;
            }

            let analysis = self
                .analysis_v2
                .completion_current_revision_snapshot_for_origin_and_operation(
                    bsl_runtime::application::ObservabilityOrigin::Lsp,
                    bsl_runtime::application::SemanticOperation::Completion,
                )
                .await
                .analysis;
            if analysis.file_version(file_id).ok().flatten() != Some(requested_version) {
                continue;
            }
            if analysis
                .current_completion_head_ready(file_id)
                .ok()
                .unwrap_or(false)
            {
                if requested_version_state.load(Ordering::Relaxed) == requested_version
                    && self
                        .try_finish_current_revision_head_precompute_v2(
                            file_id,
                            requested_version,
                            &requested_version_state,
                        )
                        .await
                {
                    return;
                }
                continue;
            }
            if requested_version_state.load(Ordering::Relaxed) != requested_version {
                break;
            }
            let latest_received_version = self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                .unwrap_or(0);
            if latest_received_version != requested_version {
                break;
            }

            self.run_completion_exact_ir_singleflight_prewarm_v2(
                analysis,
                file_id,
                // Current-revision completion head is the readiness fast lane for strict-latest
                // completion. Keep it on interactive CPU permits so background exact/index work
                // cannot starve head publication after didOpen/didChange already handed off.
                bsl_runtime::application::CpuWorkClass::Interactive,
                true,
            )
            .await;

            if requested_version_state.load(Ordering::Relaxed) == requested_version
                && self
                    .try_finish_current_revision_head_precompute_v2(
                        file_id,
                        requested_version,
                        &requested_version_state,
                    )
                    .await
            {
                return;
            }
        }

        let mut tasks = self.current_revision_head_precompute_tasks_v2.lock().await;
        if tasks
            .get(&file_id)
            .is_some_and(|task| Arc::ptr_eq(&task.requested_version, &requested_version_state))
        {
            tasks.remove(&file_id);
        }
    }

    async fn try_finish_current_revision_head_precompute_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        requested_version: i32,
        requested_version_state: &Arc<std::sync::atomic::AtomicI32>,
    ) -> bool {
        let mut tasks = self.current_revision_head_precompute_tasks_v2.lock().await;
        let Some(task) = tasks.get(&file_id) else {
            return true;
        };
        if !Arc::ptr_eq(&task.requested_version, requested_version_state) {
            return false;
        }
        if task.requested_version.load(Ordering::Relaxed) != requested_version {
            return false;
        }
        tasks.remove(&file_id);
        true
    }

    pub(crate) async fn cancel_current_revision_head_precompute_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
    ) {
        let task = self
            .current_revision_head_precompute_tasks_v2
            .lock()
            .await
            .remove(&file_id);
        if let Some(task) = task {
            task.requested_version.store(0, Ordering::Relaxed);
            task.handle.abort();
        }
    }

    async fn cancel_background_parse_snapshot_apply_v2(&self, file_id: bsl_analysis_v2::FileId) {
        let task = self
            .background_parse_snapshot_apply_tasks_v2
            .lock()
            .await
            .remove(&file_id);
        if let Some(task) = task {
            task.requested_version.store(0, Ordering::Relaxed);
            task.handle.abort();
        }
    }

    pub(crate) async fn schedule_document_symbol_bootstrap_from_request_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        requested_version: i32,
    ) {
        let mut tasks = self.document_symbol_bootstrap_tasks_v2.lock().await;
        if tasks
            .get(&file_id)
            .is_some_and(|task| task.requested_version.load(Ordering::Relaxed) == requested_version)
        {
            return;
        }
        if let Some(previous) = tasks.remove(&file_id) {
            previous.requested_version.store(0, Ordering::Relaxed);
            previous.handle.abort();
        }

        let requested_version_state =
            Arc::new(std::sync::atomic::AtomicI32::new(requested_version));
        let server = self.clone();
        let worker_requested_version_state = Arc::clone(&requested_version_state);
        let handle = tokio::spawn(async move {
            server
                .run_document_symbol_request_bootstrap_worker_v2(
                    file_id,
                    worker_requested_version_state,
                )
                .await;
        });
        tasks.insert(
            file_id,
            super::super::DocumentSymbolBootstrapTaskV2 {
                requested_version: requested_version_state,
                handle,
            },
        );
    }

    async fn build_document_symbol_response_from_shadow_state_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        requested_version: i32,
    ) -> Option<DocumentSymbolResponse> {
        if self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied()
            != Some(requested_version)
        {
            return None;
        }
        let shadow_state = self
            .latest_document_shadow_state_v2
            .read()
            .await
            .get(&file_id)
            .cloned()?;
        if shadow_state.version != requested_version {
            return None;
        }
        let text = shadow_state.text;
        bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
            bsl_runtime::application::CpuWorkClass::Background,
            bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
            Some(self.coordinator.as_ref()),
            move || {
                let parse_result = bsl_syntax::parse_fast(text.as_ref()).ok()?;
                build_document_symbols(text.as_ref(), &parse_result).ok()
            },
        )
        .await
        .ok()
        .flatten()
    }

    async fn run_document_symbol_request_bootstrap_worker_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        requested_version_state: Arc<std::sync::atomic::AtomicI32>,
    ) {
        let requested_version = requested_version_state.load(Ordering::Relaxed);
        let run = async {
            tokio::task::yield_now().await;
            if requested_version <= 0 {
                return;
            }
            if self
                .latest_document_symbol_ready_v2(file_id)
                .await
                .is_some_and(|ready| ready.file_version >= requested_version)
            {
                return;
            }
            if self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                != Some(requested_version)
            {
                return;
            }
            if requested_version_state.load(Ordering::Relaxed) != requested_version {
                return;
            }
            if self
                .latest_document_symbol_ready_v2(file_id)
                .await
                .is_some_and(|ready| ready.file_version >= requested_version)
            {
                return;
            }
            let response = self
                .build_document_symbol_response_from_shadow_state_v2(file_id, requested_version)
                .await;
            let Some(response) = response else {
                return;
            };
            if requested_version_state.load(Ordering::Relaxed) != requested_version {
                return;
            }
            if self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                .is_none()
            {
                return;
            }
            self.record_document_symbol_ready_v2(file_id, requested_version, response)
                .await;
        };
        run.await;

        let mut tasks = self.document_symbol_bootstrap_tasks_v2.lock().await;
        if tasks
            .get(&file_id)
            .is_some_and(|task| Arc::ptr_eq(&task.requested_version, &requested_version_state))
        {
            tasks.remove(&file_id);
        }
    }

    async fn cancel_document_symbol_bootstrap_v2(&self, file_id: bsl_analysis_v2::FileId) {
        let task = self
            .document_symbol_bootstrap_tasks_v2
            .lock()
            .await
            .remove(&file_id);
        if let Some(task) = task {
            task.requested_version.store(0, Ordering::Relaxed);
            task.handle.abort();
        }
    }

    fn spawn_large_churn_completion_head_reuse_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        requested_version: i32,
        path: Arc<str>,
        text: Arc<str>,
        parser_edits: Vec<bsl_runtime::system::parser_coordinator::TextEdit>,
    ) {
        let server = self.clone();
        tokio::spawn(async move {
            let coordinator = server.coordinator.clone();
            let path_for_parse = path.clone();
            let text_for_parse = text.clone();
            let report =
                bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
                    bsl_runtime::application::CpuWorkClass::Background,
                    bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                    Some(server.coordinator.as_ref()),
                    move || {
                        coordinator.parser_coordinator().and_then(|parser| {
                            parser
                                .parse_incremental_with_report(
                                    PathBuf::from(path_for_parse.as_ref()),
                                    text_for_parse.to_string(),
                                    parser_edits,
                                )
                                .ok()
                        })
                    },
                )
                .await
                .ok()
                .flatten();
            let Some(report) = report else {
                return;
            };

            if server
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                != Some(requested_version)
            {
                return;
            }

            let parse_snapshot = parse_snapshot_from_report(file_id, requested_version, report);
            server
                .record_ready_parse_snapshot_v2(file_id, text.clone(), &parse_snapshot)
                .await;
            let analysis = server.analysis_v2.snapshot().await;
            let _ = analysis.try_publish_completion_head_from_parse_snapshot_reuse(
                file_id,
                requested_version,
                &parse_snapshot,
                text.as_ref(),
            );
        });
    }

    pub(super) async fn lsp_did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        let version = params.text_document.version;

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
        {
            let _sync_guard = self.text_sync_v2.lock().await;
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
            self.analysis_v2.apply_changes_interactive(
                bsl_runtime::application::ObservabilityOrigin::Lsp,
                vec![bsl_analysis_v2::Change::SetFile {
                    file_id,
                    text: text.clone(),
                    version,
                    path: path.clone(),
                }],
            );
            let handoff_registered_at = Instant::now();
            self.latest_current_revision_handoff_versions_v2
                .write()
                .await
                .insert(file_id, version);
            self.latest_apply_enqueued_at_v2
                .write()
                .await
                .insert(file_id, handoff_registered_at);
        }
        self.schedule_completion_head_precompute_from_current_revision_v2(file_id, version)
            .await;
        self.schedule_background_parse_snapshot_apply_v2(BackgroundParseSnapshotApplyArgs {
            file_id,
            requested_version: version,
            path: path.clone(),
            text: text.clone(),
            parser_edits: Vec::new(),
            async_delay_mode: ParseSnapshotAsyncDelayMode::None,
            blocking_delay_env_key: Some("BSL_TEST_DID_OPEN_BLOCKING_PARSE_DELAY_MS"),
            force_reschedule_same_version: false,
            source: super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidOpen,
            did_change_attribution: None,
        })
        .await;
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
                None,
                bsl_runtime::application::DiagnosticsTrigger::DidOpen,
                *profile,
                false,
            )
            .await;
        }
        let client = self.client.clone();
        tokio::spawn(async move {
            client
                .log_message(
                    MessageType::INFO,
                    format!("Opened document (v2 diagnostics scheduled): {}", uri),
                )
                .await;
        });
    }

    pub(super) async fn lsp_did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        let changes = params.content_changes;
        let parse_snapshot_change_shape = did_change_parse_snapshot_change_shape(&changes);

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
        let (
            updated_text,
            path,
            parser_edits,
            large_churn_active,
            identical_text_previous_version,
            tail_whitespace_append_previous_version,
            previous_analysis_for_identical_text_reuse,
            parse_snapshot_base_text_source,
        ) = {
            let _sync_guard = self.text_sync_v2.lock().await;
            let previous_shadow_state = {
                let shadow = self.latest_document_shadow_state_v2.read().await;
                shadow.get(&file_id).cloned()
            };

            let (updated_text, parser_edits, parse_snapshot_base_text_source) =
                if let Some(full_change) = changes.iter().find(|c| c.range.is_none()) {
                    (full_change.text.clone(), Vec::new(), "not_applicable")
                } else {
                    if let Some(state) = previous_shadow_state.as_ref() {
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
                    let (base_text, parse_snapshot_base_text_source) =
                        if let Some(state) = previous_shadow_state.clone() {
                            (state.text.to_string(), "shadow_state")
                        } else {
                            (
                                self.analysis_v2
                                    .snapshot()
                                    .await
                                    .file_text(file_id)
                                    .ok()
                                    .flatten()
                                    .map(|text| text.to_string())
                                    .unwrap_or_default(),
                                "analysis_snapshot",
                            )
                        };

                    let replay_plan = canonicalize_ranged_did_change_replay_plan(&changes);
                    let mut current_text = base_text;
                    let mut parser_edits = Vec::with_capacity(replay_plan.len());
                    for step in replay_plan {
                        current_text = apply_text_edit(&current_text, step.range, &step.new_text);
                        parser_edits.push(step.parser_edit);
                    }
                    (current_text, parser_edits, parse_snapshot_base_text_source)
                };
            let identical_text_previous_version =
                previous_shadow_state.as_ref().and_then(|state| {
                    (state.text.as_ref() == updated_text.as_str()).then_some(state.version)
                });
            let tail_whitespace_append_previous_version =
                previous_shadow_state.as_ref().and_then(|state| {
                    let previous_text = state.text.as_ref();
                    if !updated_text.starts_with(previous_text)
                        || updated_text.len() <= previous_text.len()
                    {
                        return None;
                    }
                    let suffix = &updated_text[previous_text.len()..];
                    (!suffix.is_empty() && suffix.chars().all(char::is_whitespace))
                        .then_some(state.version)
                });

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
                    let state = churn_state.entry(file_id).or_insert(
                        super::super::ScaleAwareChurnStateV2 {
                            window_started_at: now,
                            changes_in_window: 0,
                            large_churn_active: false,
                        },
                    );
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
            let previous_analysis_for_identical_text_reuse =
                if identical_text_previous_version.is_some() {
                    Some(self.analysis_v2.snapshot().await)
                } else {
                    None
                };

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
            let mut current_revision_changes = vec![bsl_analysis_v2::Change::SetFile {
                file_id,
                text: updated_text.clone(),
                version,
                path: path.clone(),
            }];
            if let Some(previous_version) = tail_whitespace_append_previous_version {
                current_revision_changes.push(
                    bsl_analysis_v2::Change::ReuseCompletionHeadFromPreviousVersion {
                        file_id,
                        expected_version: version,
                        previous_version,
                    },
                );
            }
            // This handoff advances transport-visible freshness immediately, but runtime
            // applied_version may still lag until the interactive writer path catches up.
            self.analysis_v2.apply_changes_interactive(
                bsl_runtime::application::ObservabilityOrigin::Lsp,
                current_revision_changes,
            );
            let handoff_registered_at = Instant::now();
            self.latest_current_revision_handoff_versions_v2
                .write()
                .await
                .insert(file_id, version);
            self.latest_apply_enqueued_at_v2
                .write()
                .await
                .insert(file_id, handoff_registered_at);
            (
                updated_text,
                path,
                parser_edits,
                large_churn_active,
                identical_text_previous_version,
                tail_whitespace_append_previous_version,
                previous_analysis_for_identical_text_reuse,
                parse_snapshot_base_text_source,
            )
        };

        // Publish current-revision text/version immediately so completion waiters do not sit
        // behind slow parse work on the didChange path.
        if identical_text_previous_version.is_none()
            && tail_whitespace_append_previous_version.is_none()
        {
            self.schedule_completion_head_precompute_from_current_revision_v2(file_id, version)
                .await;
        }
        if let Some(previous_version) = identical_text_previous_version {
            self.spawn_completion_head_reuse_from_previous_version_v2(
                file_id,
                version,
                previous_version,
                previous_analysis_for_identical_text_reuse
                    .expect("previous analysis snapshot for identical-text head reuse"),
            );
        }
        if let Some(previous_version) = tail_whitespace_append_previous_version {
            self.spawn_completion_head_version_alias_from_previous_version_v2(
                file_id,
                version,
                previous_version,
            );
        }
        if large_churn_active {
            self.coordinator.record_intellisense_v2_parse_snapshot(
                bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                "other",
                0,
                0,
                Some("other"),
                Duration::default(),
            );
            self.spawn_large_churn_completion_head_reuse_v2(
                file_id,
                version,
                path.clone(),
                updated_text.clone(),
                parser_edits,
            );
        } else {
            self.schedule_background_parse_snapshot_apply_v2(BackgroundParseSnapshotApplyArgs {
                file_id,
                requested_version: version,
                path: path.clone(),
                text: updated_text.clone(),
                parser_edits,
                async_delay_mode: ParseSnapshotAsyncDelayMode::DidChangeTestOnly,
                blocking_delay_env_key: Some("BSL_TEST_DID_CHANGE_BLOCKING_PARSE_DELAY_MS"),
                force_reschedule_same_version: false,
                source: super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidChange,
                did_change_attribution: Some(DidChangeParseSnapshotAttributionV2 {
                    uri: uri.clone(),
                    base_text_source: parse_snapshot_base_text_source,
                    change_shape: parse_snapshot_change_shape,
                }),
            })
            .await;
        }
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
                    None,
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
                        None,
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
        let shadow_state = self
            .latest_document_shadow_state_v2
            .read()
            .await
            .get(&file_id)
            .cloned();
        let save_text = params.text.map(Arc::<str>::from);
        let save_text = match shadow_state.as_ref() {
            Some(shadow_state) if shadow_state.version == version => {
                Some(shadow_state.text.clone())
            }
            _ => save_text,
        };
        if let Some(text) = save_text {
            self.latest_document_shadow_state_v2.write().await.insert(
                file_id,
                DocumentShadowStateV2 {
                    version,
                    text: text.clone(),
                },
            );
            let path = match uri.to_file_path() {
                Ok(path) => path.to_string_lossy().to_string(),
                Err(_) => uri.to_string(),
            };
            // Save can be followed by an immediate outline refresh without a new version bump.
            // Coalesce identical same-version refresh behind the existing worker so save does
            // not restart the same cold/full parse for unchanged shadow text.
            self.schedule_background_parse_snapshot_apply_v2(BackgroundParseSnapshotApplyArgs {
                file_id,
                requested_version: version,
                path: Arc::from(path),
                text,
                parser_edits: Vec::new(),
                async_delay_mode: ParseSnapshotAsyncDelayMode::DidSaveTestOnly,
                blocking_delay_env_key: Some("BSL_TEST_DID_SAVE_BLOCKING_PARSE_DELAY_MS"),
                force_reschedule_same_version: true,
                source: super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidSave,
                did_change_attribution: None,
            })
            .await;
        }

        let flow_sensitive_enabled = {
            let settings = self.settings.read().await;
            settings.enable_flow_sensitive
        };
        let diagnostics_generation = self.bump_diagnostics_generation_v2(file_id).await;
        let save_cycle_sequence = self.bump_diagnostics_save_cycle_sequence_v2(file_id).await;
        self.begin_diagnostics_save_timeline_cycle(
            &uri,
            super::super::DiagnosticsSaveTimelineCycleKey {
                file_id,
                diagnostics_generation,
                save_cycle_sequence,
                requested_version: version,
            },
        );
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
                Some(save_cycle_sequence),
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
            self.cancel_current_revision_head_precompute_v2(file_id)
                .await;
            self.cancel_background_parse_snapshot_apply_v2(file_id)
                .await;
            self.cancel_document_symbol_bootstrap_v2(file_id).await;
            self.latest_received_file_versions_v2
                .write()
                .await
                .remove(&file_id);
            self.latest_current_revision_handoff_versions_v2
                .write()
                .await
                .remove(&file_id);
            self.latest_document_shadow_state_v2
                .write()
                .await
                .remove(&file_id);
            self.latest_ready_parse_snapshots_v2
                .write()
                .await
                .remove(&file_id);
            self.latest_save_fastlane_syntax_artifacts_v2
                .write()
                .await
                .remove(&file_id);
            self.latest_apply_enqueued_at_v2
                .write()
                .await
                .remove(&file_id);
            self.latest_diagnostics_publish_state_v2
                .write()
                .await
                .remove(&file_id);
            self.clear_active_diagnostics_save_timeline_cycles_for_file(file_id);
            self.clear_diagnostics_save_timeline_terminal_keys_for_file(file_id);
            self.document_symbol_ready_cache_v2
                .write()
                .await
                .remove(&file_id);
            self.document_symbol_request_epochs_v2
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
            self.diagnostics_save_cycle_sequence_v2
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
