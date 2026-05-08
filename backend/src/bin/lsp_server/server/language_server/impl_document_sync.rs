use super::super::{
    DidChangeParseSnapshotAttributionV2, DidChangeStaleParserBaseAttributionV2,
    DocumentShadowStateV2, ParseSnapshotAsyncDelayMode, ReadyParseSnapshotStateV2,
};
use super::*;
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

struct BuildParseSnapshotRequest {
    file_id: bsl_analysis_v2::FileId,
    version: i32,
    path: Arc<str>,
    text: Arc<str>,
    cpu_work_class: bsl_runtime::application::CpuWorkClass,
    reused_prefix_parse_result: Option<Arc<bsl_syntax::ast::ParseResult>>,
    parser_base_recovery_text: Option<Arc<str>>,
    parser_base_recovery_reuse_parse_result: Option<Arc<bsl_syntax::ast::ParseResult>>,
    parser_edits: Vec<bsl_runtime::system::parser_coordinator::TextEdit>,
    forced_full_parse_reason: Option<&'static str>,
    blocking_delay_env_key: Option<&'static str>,
    requested_target_epoch_state: Option<Arc<std::sync::atomic::AtomicU64>>,
    requested_target_epoch: Option<u64>,
    task_control: Option<Arc<super::super::BackgroundParseSnapshotApplyTaskControlV2>>,
    admission_lane: Option<bsl_runtime::application::AdmissionLane>,
    did_change_attribution: Option<super::super::DidChangeParseSnapshotAttributionV2>,
}

#[derive(Clone)]
struct ParseSnapshotAstReuseSeedV2 {
    source_text: Arc<str>,
    parse_result: Arc<bsl_syntax::ast::ParseResult>,
}

struct DidChangePostHandoffWorkV2 {
    uri: Url,
    file_id: bsl_analysis_v2::FileId,
    version: i32,
    diagnostics_save_cycle_sequence_at_handoff: u64,
    path: Arc<str>,
    updated_text: Arc<str>,
    parser_edits: Vec<bsl_runtime::system::parser_coordinator::TextEdit>,
    previous_shadow_state: Option<DocumentShadowStateV2>,
    identical_text_previous_version: Option<i32>,
    tail_whitespace_append_previous_version: Option<i32>,
    previous_analysis_for_identical_text_reuse: Option<bsl_analysis_v2::AnalysisV2>,
    parse_snapshot_change_shape: &'static str,
    parse_snapshot_replay_order: &'static str,
    parse_snapshot_content_changes_count: usize,
    parse_snapshot_base_text_source: &'static str,
    parse_snapshot_base_document_version: Option<i32>,
}

enum BuildParseSnapshotAbortReasonV2 {
    Superseded,
    RetargetedDuringParse,
    BuildSnapshotAborted,
}

#[derive(Debug, Clone, Copy, Default)]
struct DeferredParseSnapshotWorkV2 {
    optional_cache_enrichment: bool,
    tree_cache_install: bool,
    syntax_error_assembly: bool,
}

enum BuildParseSnapshotOutcomeV2 {
    Ready(
        bsl_analysis_v2::ParseSnapshot,
        DeferredParseSnapshotWorkV2,
        Box<bsl_runtime::system::parser_coordinator::ParseSnapshotProgramLoweringSummary>,
    ),
    Aborted(BuildParseSnapshotAbortReasonV2),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExactTypeIndexBeforeReadyInstallOutcomeV2 {
    Ready,
    Deadline,
    Retargeted,
    Superseded,
    LatestVersionMismatch,
}

impl ExactTypeIndexBeforeReadyInstallOutcomeV2 {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Deadline => "deadline",
            Self::Retargeted => "retargeted",
            Self::Superseded => "superseded",
            Self::LatestVersionMismatch => "latest_version_mismatch",
        }
    }
}

#[cfg(test)]
static DID_CHANGE_PARSE_DELAY_ACTIVE: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
static DID_SAVE_PARSE_DELAY_ACTIVE: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
static DID_CHANGE_PRE_MATERIALIZATION_DELAY_ACTIVE: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
static DID_CHANGE_POST_HANDOFF_DELAY_ACTIVE: AtomicUsize = AtomicUsize::new(0);

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
async fn maybe_inject_did_change_pre_materialization_delay() {
    maybe_inject_parse_delay(
        "BSL_TEST_DID_CHANGE_PRE_MATERIALIZATION_DELAY_MS",
        &DID_CHANGE_PRE_MATERIALIZATION_DELAY_ACTIVE,
    )
    .await;
}

#[cfg(not(test))]
async fn maybe_inject_did_change_pre_materialization_delay() {}

#[cfg(test)]
async fn maybe_inject_did_change_post_handoff_delay() {
    maybe_inject_parse_delay(
        "BSL_TEST_DID_CHANGE_POST_HANDOFF_DELAY_MS",
        &DID_CHANGE_POST_HANDOFF_DELAY_ACTIVE,
    )
    .await;
}

#[cfg(not(test))]
async fn maybe_inject_did_change_post_handoff_delay() {}

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

#[cfg(test)]
pub(super) fn did_change_pre_materialization_delay_active_for_test() -> bool {
    DID_CHANGE_PRE_MATERIALIZATION_DELAY_ACTIVE.load(Ordering::SeqCst) > 0
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

fn did_change_parse_snapshot_replay_order(
    changes: &[TextDocumentContentChangeEvent],
) -> &'static str {
    if changes.iter().any(|change| change.range.is_none()) {
        "not_applicable"
    } else {
        "receive_order"
    }
}

fn parse_snapshot_apply_debounce_duration() -> Duration {
    Duration::from_millis(25)
}

fn ready_install_exact_type_index_wait_max_duration() -> Duration {
    const DEFAULT_MAX_MS: u64 = 5_000;
    #[cfg(test)]
    {
        if let Some(value) = std::env::var("BSL_TEST_READY_INSTALL_EXACT_TYPE_INDEX_WAIT_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
        {
            return Duration::from_millis(value);
        }
    }
    Duration::from_millis(DEFAULT_MAX_MS)
}

fn ready_install_exact_type_index_wait_probe_max_duration() -> Duration {
    Duration::from_millis(100)
}

fn duration_from_millis_u128_for_metrics(value_ms: u128) -> Duration {
    Duration::from_millis(value_ms.min(u64::MAX as u128) as u64)
}

#[derive(Debug, Clone)]
struct ShadowStateParserBaseInspectionV2 {
    forced_full_parse_reason: Option<&'static str>,
    stale_parser_base: Option<DidChangeStaleParserBaseAttributionV2>,
}

fn derive_reused_prefix_parse_result_from_ready_state(
    ready_state: &ReadyParseSnapshotStateV2,
    requested_version: i32,
    requested_text: &Arc<str>,
) -> Option<Arc<bsl_syntax::ast::ParseResult>> {
    if !ready_state.syntax_errors_complete
        || ready_state.parse_snapshot.parse_result.has_errors()
        || ready_state.parse_snapshot.file_version >= requested_version
        || ready_state.text.as_ref() == requested_text.as_ref()
        || !requested_text.starts_with(ready_state.text.as_ref())
        || ready_state
            .parse_snapshot
            .parse_result
            .program
            .statements
            .is_empty()
    {
        return None;
    }
    Some(ready_state.parse_snapshot.parse_result.clone())
}

fn derive_same_version_rebuild_previous_ready_seed_v2(
    ready_state: &ReadyParseSnapshotStateV2,
    requested_version: i32,
    requested_text: &Arc<str>,
) -> Option<ParseSnapshotAstReuseSeedV2> {
    let ready_version = ready_state.parse_snapshot.file_version;
    let is_previous_version_seed =
        ready_version < requested_version && ready_state.text.as_ref() != requested_text.as_ref();
    let is_same_version_same_text_seed =
        ready_version == requested_version && ready_state.text.as_ref() == requested_text.as_ref();
    if !ready_state.syntax_errors_complete
        || ready_state.parse_snapshot.parse_result.has_errors()
        || !(is_previous_version_seed || is_same_version_same_text_seed)
        || ready_state
            .parse_snapshot
            .parse_result
            .program
            .statements
            .is_empty()
    {
        return None;
    }
    Some(ParseSnapshotAstReuseSeedV2 {
        source_text: Arc::clone(&ready_state.text),
        parse_result: ready_state.parse_snapshot.parse_result.clone(),
    })
}

async fn derive_parser_base_recovery_reuse_parse_result_from_shadow_state_v2(
    server: &BslLanguageServer,
    file_id: bsl_analysis_v2::FileId,
    shadow_state: &DocumentShadowStateV2,
) -> Option<Arc<bsl_syntax::ast::ParseResult>> {
    let analysis = server.analysis_v2.snapshot().await;
    if analysis.file_version(file_id).ok().flatten() != Some(shadow_state.version) {
        return None;
    }
    if analysis.file_text(file_id).ok().flatten().as_deref() != Some(shadow_state.text.as_ref()) {
        return None;
    }
    let parse_result = analysis.parse_result(file_id).ok().flatten()?;
    (!parse_result.has_errors()).then_some(parse_result)
}

async fn derive_same_version_rebuild_reuse_parse_result_from_current_state_v2(
    server: &BslLanguageServer,
    file_id: bsl_analysis_v2::FileId,
    version: i32,
    text: &str,
) -> Option<Arc<bsl_syntax::ast::ParseResult>> {
    let analysis = server.analysis_v2.snapshot().await;
    if analysis.file_version(file_id).ok().flatten() != Some(version) {
        return None;
    }
    if analysis.file_text(file_id).ok().flatten().as_deref() != Some(text) {
        return None;
    }
    let parse_result = analysis.parse_result(file_id).ok().flatten()?;
    (!parse_result.has_errors()).then_some(parse_result)
}

fn classify_stale_parser_base_root_cause(
    shadow_document_version: i32,
    latest_ready_document_version: Option<i32>,
    matching_ready_snapshot_for_shadow_state: bool,
    ready_snapshot_prime_attempted: bool,
    tree_cache_matches_shadow_text_after_prime: Option<bool>,
) -> &'static str {
    if matching_ready_snapshot_for_shadow_state {
        if ready_snapshot_prime_attempted
            && matches!(tree_cache_matches_shadow_text_after_prime, Some(false))
        {
            "tree_cache_mismatch_after_prime"
        } else {
            "other_internal_reason"
        }
    } else if latest_ready_document_version
        .is_some_and(|ready_version| ready_version < shadow_document_version)
    {
        "ready_snapshot_lags_shadow_state"
    } else {
        "no_matching_ready_snapshot_for_shadow_state"
    }
}

#[cfg(test)]
fn maybe_poison_tree_cache_after_prime_for_test(
    parser: &bsl_runtime::system::parser_coordinator::ParserCoordinator,
    path: &str,
    shadow_text: &str,
) {
    if std::env::var_os("BSL_TEST_DID_CHANGE_POISON_TREE_CACHE_AFTER_PRIME").is_none() {
        return;
    }
    let poisoned_text = format!("{shadow_text}\n// stale parser base post-prime poison\n");
    let _ = parser.parse_incremental_with_report(PathBuf::from(path), poisoned_text, Vec::new());
}

#[cfg(test)]
fn maybe_poison_tree_cache_after_recovery_for_test(
    parser: &bsl_runtime::system::parser_coordinator::ParserCoordinator,
    path: &str,
    shadow_text: &str,
) {
    if std::env::var_os("BSL_TEST_DID_CHANGE_POISON_TREE_CACHE_AFTER_RECOVERY").is_none() {
        return;
    }
    let poisoned_text = format!("{shadow_text}\n// stale parser base post-recovery poison\n");
    let _ = parser.parse_incremental_with_report(PathBuf::from(path), poisoned_text, Vec::new());
}

#[cfg(not(test))]
fn maybe_poison_tree_cache_after_prime_for_test(
    _parser: &bsl_runtime::system::parser_coordinator::ParserCoordinator,
    _path: &str,
    _shadow_text: &str,
) {
}

#[cfg(not(test))]
fn maybe_poison_tree_cache_after_recovery_for_test(
    _parser: &bsl_runtime::system::parser_coordinator::ParserCoordinator,
    _path: &str,
    _shadow_text: &str,
) {
}

fn parse_snapshot_text_hash(text: &str) -> [u8; 32] {
    *blake3::hash(text.as_bytes()).as_bytes()
}

fn lowering_reuse_seed_eviction_reason_for_did_save_lifecycle_state_v2(
    state: super::super::DidSaveExactProducerLifecycleStateV2,
) -> Option<
    bsl_runtime::system::parser_coordinator::ParseSnapshotProgramLoweringReuseSeedEvictionReason,
> {
    use super::super::DidSaveExactProducerLifecycleStateV2 as State;
    use bsl_runtime::system::parser_coordinator::ParseSnapshotProgramLoweringReuseSeedEvictionReason as Reason;

    match state {
        State::Admitted | State::Started => None,
        State::DetachedDiagnosticsReadyPublished
        | State::FullyMaterialized
        | State::ExactTypeIndexDeadline => Some(Reason::TerminalCleanup),
        State::Superseded => Some(Reason::Superseded),
        State::Cancelled => Some(Reason::Cancelled),
        State::Failed => Some(Reason::Failed),
        State::ContinuityLost => Some(Reason::ContinuityLost),
    }
}

fn late_ranged_did_change_parser_base_preservation_allowed(
    did_change_attribution: Option<&DidChangeParseSnapshotAttributionV2>,
    control: &super::super::BackgroundParseSnapshotApplyTaskControlV2,
) -> bool {
    if did_change_attribution.is_none_or(|attribution| attribution.change_shape != "ranged") {
        return false;
    }
    let snapshot = control.phase_attribution_snapshot();
    matches!(
        (
            snapshot.current_phase,
            snapshot.current_parse_exec_subphase,
            snapshot.current_core_build_checkpoint,
            snapshot.current_assembly_checkpoint,
        ),
        (
            Some(super::super::ReadyParseSnapshotAttributionPhaseV2::ParseExec),
            Some(super::super::ReadyParseSnapshotParseExecSubphaseV2::CoreParseBuild),
            Some(super::super::ReadyParseSnapshotCoreBuildCheckpointV2::ExactReadySnapshotAssembly),
            Some(
                super::super::ReadyParseSnapshotAssemblyCheckpointV2::ProgramLowering
                    | super::super::ReadyParseSnapshotAssemblyCheckpointV2::PublishableArtifactPackaging
                    | super::super::ReadyParseSnapshotAssemblyCheckpointV2::SyntaxErrorCollection
            ),
        )
    )
}

fn background_parse_snapshot_apply_target_from_args(
    args: &BackgroundParseSnapshotApplyArgs,
    text_hash: [u8; 32],
    epoch: u64,
) -> super::super::BackgroundParseSnapshotApplyTargetV2 {
    super::super::BackgroundParseSnapshotApplyTargetV2 {
        requested_version: args.requested_version,
        text_hash,
        save_cycle_sequence: args.save_cycle_sequence,
        source: args.source,
        path: args.path.clone(),
        text: args.text.clone(),
        parser_base_recovery_text: args.parser_base_recovery_text.clone(),
        parser_base_recovery_reuse_parse_result: args
            .parser_base_recovery_reuse_parse_result
            .clone(),
        parser_edits: args.parser_edits.clone(),
        forced_full_parse_reason: args.forced_full_parse_reason,
        async_delay_mode: args.async_delay_mode,
        blocking_delay_env_key: args.blocking_delay_env_key,
        did_change_attribution: args.did_change_attribution.clone(),
        epoch,
    }
}

fn background_parse_snapshot_task_target_v2(
    task: &super::super::BackgroundParseSnapshotApplyTaskV2,
) -> super::super::BackgroundParseSnapshotApplyTargetV2 {
    task.target
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

fn background_parse_snapshot_task_matches_v2(
    task: &super::super::BackgroundParseSnapshotApplyTaskV2,
    requested_version: i32,
    expected_text_hash: Option<[u8; 32]>,
) -> bool {
    let target = background_parse_snapshot_task_target_v2(task);
    target.requested_version == requested_version
        && expected_text_hash.is_none_or(|text_hash| target.text_hash == text_hash)
}

fn background_parse_snapshot_apply_source_label(
    source: super::super::BackgroundParseSnapshotApplyTaskSourceV2,
) -> &'static str {
    match source {
        super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidOpen => "did_open",
        super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidChange => "did_change",
        super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidSave => "did_save",
    }
}

fn ready_install_exact_type_index_wait_blocker_class(
    requested_version: i32,
    observed_file_version: Option<i32>,
    exact_ready: bool,
    parse_snapshot_serve_only_blocked: Option<bool>,
    matching_task_state: Option<&'static str>,
    task_phase: Option<&'static str>,
) -> &'static str {
    if observed_file_version == Some(requested_version) && exact_ready {
        return "ready";
    }
    if observed_file_version != Some(requested_version) {
        return "observed_version_mismatch";
    }
    if parse_snapshot_serve_only_blocked == Some(true) {
        return "serve_only_blocked";
    }
    match matching_task_state {
        Some("missing") => "no_matching_task",
        Some("wrong_version") => "task_present_wrong_version",
        Some("matching") => match task_phase {
            Some("waiting_for_version") => "type_index_waiting_for_version",
            Some("snapshotting") => "type_index_snapshotting",
            Some("waiting_cpu_permit") => "type_index_waiting_cpu_permit",
            Some("computing") => "type_index_computing",
            Some("completed") => "type_index_completed",
            _ => "type_index_not_ready",
        },
        _ => "metadata_missing",
    }
}

fn record_ready_parse_snapshot_phase_metrics(
    coordinator: &Arc<bsl_runtime::system::SystemCoordinator>,
    origin: &'static str,
    source: &'static str,
    phase_attribution: &super::super::ReadyParseSnapshotPhaseAttributionV2,
) {
    for (phase, duration_ms) in [
        ("parse_exec", phase_attribution.parse_exec_ms),
        (
            "post_parse_pre_materialization",
            phase_attribution.post_parse_pre_materialization_ms,
        ),
        ("ready_install", phase_attribution.ready_install_ms),
        (
            "document_symbol_side_work",
            phase_attribution.document_symbol_side_work_ms,
        ),
    ] {
        let Some(duration_ms) = duration_ms else {
            continue;
        };
        coordinator.record_intellisense_v2_ready_parse_snapshot_phase_latency(
            origin,
            source,
            phase,
            Duration::from_millis(duration_ms),
        );
    }
}

struct ReadyParseSnapshotWorkerLifecycleGuard {
    coordinator: Arc<bsl_runtime::system::SystemCoordinator>,
    origin: &'static str,
    source: &'static str,
    started: Instant,
    materialized: bool,
    terminal_reason: Option<&'static str>,
}

impl ReadyParseSnapshotWorkerLifecycleGuard {
    fn new(
        coordinator: Arc<bsl_runtime::system::SystemCoordinator>,
        origin: &'static str,
        source: &'static str,
    ) -> Self {
        coordinator.record_intellisense_v2_ready_parse_snapshot_worker_started(origin, source);
        Self {
            coordinator,
            origin,
            source,
            started: Instant::now(),
            materialized: false,
            terminal_reason: None,
        }
    }

    fn mark_materialized(&mut self) {
        self.materialized = true;
    }

    fn set_source(&mut self, source: &'static str) {
        self.source = source;
    }

    fn set_terminal_reason(&mut self, reason: &'static str) {
        self.terminal_reason = Some(reason);
    }
}

impl Drop for ReadyParseSnapshotWorkerLifecycleGuard {
    fn drop(&mut self) {
        if self.materialized {
            return;
        }
        self.coordinator
            .record_intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization(
                self.origin,
                self.source,
                self.terminal_reason.unwrap_or("aborted"),
                self.started.elapsed(),
            );
    }
}

struct BackgroundParseSnapshotApplyArgs {
    file_id: bsl_analysis_v2::FileId,
    requested_version: i32,
    save_cycle_sequence: Option<u64>,
    path: Arc<str>,
    text: Arc<str>,
    cpu_work_class: bsl_runtime::application::CpuWorkClass,
    parser_base_recovery_text: Option<Arc<str>>,
    parser_base_recovery_reuse_parse_result: Option<Arc<bsl_syntax::ast::ParseResult>>,
    parser_edits: Vec<bsl_runtime::system::parser_coordinator::TextEdit>,
    forced_full_parse_reason: Option<&'static str>,
    async_delay_mode: ParseSnapshotAsyncDelayMode,
    blocking_delay_env_key: Option<&'static str>,
    force_reschedule_same_version: bool,
    source: super::super::BackgroundParseSnapshotApplyTaskSourceV2,
    did_change_attribution: Option<DidChangeParseSnapshotAttributionV2>,
}

struct ReadyInstallExactTypeIndexWaitArgs<'a> {
    file_id: bsl_analysis_v2::FileId,
    requested_version: i32,
    target_epoch_state: &'a Arc<std::sync::atomic::AtomicU64>,
    target_epoch: u64,
    task_control: &'a Arc<super::super::BackgroundParseSnapshotApplyTaskControlV2>,
    max_wait: Option<Duration>,
    allow_type_index_precompute: bool,
}

struct ReadyParseSnapshotRecordArgs<'a> {
    file_id: bsl_analysis_v2::FileId,
    path: &'a Arc<str>,
    text: Arc<str>,
    parse_snapshot: &'a bsl_analysis_v2::ParseSnapshot,
    source: super::super::BackgroundParseSnapshotApplyTaskSourceV2,
    syntax_errors_complete: bool,
    program_lowering_summary:
        bsl_runtime::system::parser_coordinator::ParseSnapshotProgramLoweringSummary,
}

impl BslLanguageServer {
    async fn ready_install_exact_type_index_wait_probe_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        requested_version: i32,
        waiter_action: super::super::core::TypeIndexPrecomputeWaiterActionV2,
    ) -> super::super::ReadyInstallExactTypeIndexWaitProbeV2 {
        let analysis = tokio::time::timeout(
            ready_install_exact_type_index_wait_probe_max_duration(),
            self.analysis_v2
                .current_revision_analysis_snapshot_for_origin_and_operation(
                    bsl_runtime::application::ObservabilityOrigin::Lsp,
                    bsl_runtime::application::SemanticOperation::Completion,
                ),
        )
        .await
        .ok();
        let observed_file_version = if let Some(analysis) = analysis.as_ref() {
            analysis.file_version(file_id).ok().flatten()
        } else {
            self.latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
        };
        let exact_ready = analysis.as_ref().is_some_and(|analysis| {
            observed_file_version == Some(requested_version)
                && analysis
                    .current_type_index_serve_only_ready(file_id)
                    .ok()
                    .unwrap_or(false)
        });
        let parse_snapshot_meta = analysis.as_ref().and_then(|analysis| {
            analysis
                .current_type_index_parse_snapshot_meta(file_id)
                .ok()
                .flatten()
        });

        let ready_snapshot_version = self
            .latest_ready_parse_snapshots_v2
            .read()
            .await
            .get(&file_id)
            .map(|state| state.parse_snapshot.file_version);

        let (
            matching_task_state,
            task_phase,
            task_requested_version,
            task_active_requested_version,
        ) = {
            let tasks = self.type_index_precompute_tasks_v2.lock().await;
            match tasks.get(&file_id) {
                Some(task) => {
                    let matching_task_state =
                        if task.supersession_key.requested_version == requested_version {
                            "matching"
                        } else {
                            "wrong_version"
                        };
                    let task_phase = super::super::core::TypeIndexPrecomputePhaseV2::from_atomic(
                        task.phase.load(Ordering::Relaxed),
                    )
                    .as_str();
                    (
                        Some(matching_task_state),
                        Some(task_phase),
                        Some(task.supersession_key.requested_version),
                        Some(task.active_requested_version.load(Ordering::Relaxed)),
                    )
                }
                None => (Some("missing"), None, None, None),
            }
        };

        let (
            parse_snapshot_incremental,
            parse_snapshot_changed_ranges_count,
            parse_snapshot_serve_only_blocked,
        ) = parse_snapshot_meta
            .map(|(incremental, changed_ranges_count, serve_only_blocked)| {
                (
                    Some(incremental),
                    Some(changed_ranges_count),
                    Some(serve_only_blocked),
                )
            })
            .unwrap_or((None, None, None));

        super::super::ReadyInstallExactTypeIndexWaitProbeV2 {
            waiter_action: Some(waiter_action.as_str()),
            matching_task_state,
            task_phase,
            task_requested_version,
            task_active_requested_version,
            observed_file_version,
            exact_ready: Some(exact_ready),
            ready_snapshot_version,
            parse_snapshot_incremental,
            parse_snapshot_changed_ranges_count,
            parse_snapshot_serve_only_blocked,
            blocker_class: Some(ready_install_exact_type_index_wait_blocker_class(
                requested_version,
                observed_file_version,
                exact_ready,
                parse_snapshot_serve_only_blocked,
                matching_task_state,
                task_phase,
            )),
        }
    }

    async fn wait_for_exact_type_index_before_ready_install_v2(
        &self,
        args: ReadyInstallExactTypeIndexWaitArgs<'_>,
    ) -> ExactTypeIndexBeforeReadyInstallOutcomeV2 {
        let ReadyInstallExactTypeIndexWaitArgs {
            file_id,
            requested_version,
            target_epoch_state,
            target_epoch,
            task_control,
            max_wait,
            allow_type_index_precompute,
        } = args;
        let started = Instant::now();
        let mut waiter_action = super::super::core::TypeIndexPrecomputeWaiterActionV2::None;
        task_control.start_ready_install_exact_type_index_wait(max_wait);
        loop {
            if target_epoch_state.load(Ordering::SeqCst) != target_epoch {
                let probe = self
                    .ready_install_exact_type_index_wait_probe_v2(
                        file_id,
                        requested_version,
                        waiter_action,
                    )
                    .await;
                task_control.finish_ready_install_exact_type_index_wait(
                    ExactTypeIndexBeforeReadyInstallOutcomeV2::Retargeted.as_str(),
                    probe,
                );
                return ExactTypeIndexBeforeReadyInstallOutcomeV2::Retargeted;
            }
            if task_control.cancel_requested.load(Ordering::SeqCst) {
                let probe = self
                    .ready_install_exact_type_index_wait_probe_v2(
                        file_id,
                        requested_version,
                        waiter_action,
                    )
                    .await;
                task_control.finish_ready_install_exact_type_index_wait(
                    ExactTypeIndexBeforeReadyInstallOutcomeV2::Superseded.as_str(),
                    probe,
                );
                return ExactTypeIndexBeforeReadyInstallOutcomeV2::Superseded;
            }
            if self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                != Some(requested_version)
            {
                let probe = self
                    .ready_install_exact_type_index_wait_probe_v2(
                        file_id,
                        requested_version,
                        waiter_action,
                    )
                    .await;
                task_control.finish_ready_install_exact_type_index_wait(
                    ExactTypeIndexBeforeReadyInstallOutcomeV2::LatestVersionMismatch.as_str(),
                    probe,
                );
                return ExactTypeIndexBeforeReadyInstallOutcomeV2::LatestVersionMismatch;
            }

            if max_wait.is_some_and(|max_wait| started.elapsed() >= max_wait) {
                let probe = self
                    .ready_install_exact_type_index_wait_probe_v2(
                        file_id,
                        requested_version,
                        waiter_action,
                    )
                    .await;
                task_control.finish_ready_install_exact_type_index_wait(
                    ExactTypeIndexBeforeReadyInstallOutcomeV2::Deadline.as_str(),
                    probe,
                );
                return ExactTypeIndexBeforeReadyInstallOutcomeV2::Deadline;
            }

            let probe = self
                .ready_install_exact_type_index_wait_probe_v2(
                    file_id,
                    requested_version,
                    waiter_action,
                )
                .await;
            task_control.update_ready_install_exact_type_index_wait(probe.clone());
            if probe.observed_file_version == Some(requested_version)
                && probe.exact_ready == Some(true)
            {
                self.cleanup_completed_type_index_precompute_task_v2(
                    file_id,
                    Some(requested_version),
                )
                .await;
                task_control.finish_ready_install_exact_type_index_wait(
                    ExactTypeIndexBeforeReadyInstallOutcomeV2::Ready.as_str(),
                    probe,
                );
                return ExactTypeIndexBeforeReadyInstallOutcomeV2::Ready;
            }

            if allow_type_index_precompute {
                if !self
                    .has_matching_type_index_precompute_task_v2(file_id, Some(requested_version))
                    .await
                {
                    self.schedule_type_index_precompute_v2(file_id, requested_version)
                        .await;
                }
                if matches!(
                    waiter_action,
                    super::super::core::TypeIndexPrecomputeWaiterActionV2::None
                ) {
                    waiter_action = self
                        .promote_type_index_precompute_for_waiter_v2(
                            file_id,
                            Some(requested_version),
                        )
                        .await;
                }
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    fn classify_parse_snapshot_cancellation_abort_reason_v2(
        requested_target_epoch_state: Option<&Arc<std::sync::atomic::AtomicU64>>,
        requested_target_epoch: Option<u64>,
        task_control: Option<&Arc<super::super::BackgroundParseSnapshotApplyTaskControlV2>>,
    ) -> BuildParseSnapshotAbortReasonV2 {
        let retargeted = requested_target_epoch_state
            .zip(requested_target_epoch)
            .is_some_and(|(state, epoch)| state.load(Ordering::Relaxed) != epoch)
            || task_control
                .is_some_and(|control| control.retarget_requested.load(Ordering::SeqCst));
        if retargeted {
            BuildParseSnapshotAbortReasonV2::RetargetedDuringParse
        } else {
            BuildParseSnapshotAbortReasonV2::Superseded
        }
    }

    async fn record_did_save_exact_producer_lifecycle_state_v2(
        &self,
        key: super::super::DidSaveExactProducerKeyV2,
        state: super::super::DidSaveExactProducerLifecycleStateV2,
    ) {
        {
            self.did_save_exact_producer_lifecycle_v2
                .write()
                .await
                .insert(
                    key,
                    super::super::DidSaveExactProducerLifecycleEntryV2::new(state),
                );
        }
        self.release_lowering_reuse_save_family_seed_for_did_save_lifecycle_v2(key, state);
    }

    fn release_lowering_reuse_save_family_seed_for_did_save_lifecycle_v2(
        &self,
        key: super::super::DidSaveExactProducerKeyV2,
        state: super::super::DidSaveExactProducerLifecycleStateV2,
    ) {
        let Some(reason) =
            lowering_reuse_seed_eviction_reason_for_did_save_lifecycle_state_v2(state)
        else {
            return;
        };
        if let Some(parser) = self.coordinator.parser_coordinator() {
            parser.release_lowering_reuse_save_family_seed(key.text_hash, reason);
        }
    }

    async fn record_did_save_exact_producer_lifecycle_events_v2(
        &self,
        events: Vec<(
            super::super::DidSaveExactProducerKeyV2,
            super::super::DidSaveExactProducerLifecycleStateV2,
        )>,
    ) {
        if events.is_empty() {
            return;
        }
        let mut seed_releases = Vec::with_capacity(events.len());
        {
            let mut lifecycles = self.did_save_exact_producer_lifecycle_v2.write().await;
            for (key, state) in events {
                lifecycles.insert(
                    key,
                    super::super::DidSaveExactProducerLifecycleEntryV2::new(state),
                );
                seed_releases.push((key, state));
            }
        }
        for (key, state) in seed_releases {
            self.release_lowering_reuse_save_family_seed_for_did_save_lifecycle_v2(key, state);
        }
    }

    async fn record_did_save_exact_producer_lifecycle_for_target_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        target: &super::super::BackgroundParseSnapshotApplyTargetV2,
        state: super::super::DidSaveExactProducerLifecycleStateV2,
    ) {
        let Some(key) = super::super::DidSaveExactProducerKeyV2::from_target(file_id, target)
        else {
            return;
        };
        self.record_did_save_exact_producer_lifecycle_state_v2(key, state)
            .await;
    }

    async fn record_ready_parse_snapshot_v2(&self, args: ReadyParseSnapshotRecordArgs<'_>) {
        let ReadyParseSnapshotRecordArgs {
            file_id,
            path,
            text,
            parse_snapshot,
            source,
            syntax_errors_complete,
            program_lowering_summary,
        } = args;
        let cache_text = Arc::clone(&text);
        self.latest_snapshot_failures_v2
            .write()
            .await
            .remove(&file_id);
        self.latest_ready_parse_snapshots_v2.write().await.insert(
            file_id,
            ReadyParseSnapshotStateV2 {
                text,
                parse_snapshot: parse_snapshot.clone(),
                source,
                syntax_errors_complete,
                phase_attribution: super::super::ReadyParseSnapshotPhaseAttributionV2::default(),
                program_lowering_summary: Some(program_lowering_summary),
            },
        );
        if let Some(parser) = self.coordinator.parser_coordinator() {
            parser.prime_ast_cache_for_source(
                cache_text.as_ref(),
                Arc::clone(&parse_snapshot.parse_result),
            );
            parser.prime_tree_cache_for_file(
                PathBuf::from(path.as_ref()),
                cache_text.as_ref().to_string(),
                Arc::clone(&parse_snapshot.backend_tree),
                parse_snapshot.backend_tree_hash,
            );
        }
        self.refresh_snapshot_status_v2(file_id).await;
    }

    #[allow(clippy::too_many_arguments)]
    async fn record_detached_diagnostics_ready_artifact_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        requested_version: i32,
        text_hash: [u8; 32],
        save_cycle_sequence: Option<u64>,
        task_control: Option<&super::super::BackgroundParseSnapshotApplyTaskControlV2>,
        path: &Arc<str>,
        text: Arc<str>,
        parse_snapshot: &bsl_analysis_v2::ParseSnapshot,
        syntax_errors_complete: bool,
    ) {
        let Some(save_cycle_sequence) = save_cycle_sequence else {
            return;
        };
        let cache_text = Arc::clone(&text);
        let producer_key = super::super::DidSaveExactProducerKeyV2 {
            file_id,
            requested_version,
            text_hash,
            save_cycle_sequence,
        };
        self.latest_detached_diagnostics_ready_artifacts_v2
            .write()
            .await
            .insert(
                file_id,
                super::super::DetachedDiagnosticsReadyArtifactV2 {
                    requested_version,
                    text_hash,
                    save_cycle_sequence,
                    text,
                    parse_snapshot: parse_snapshot.clone(),
                    syntax_errors_complete,
                },
            );
        if let Some(parser) = self.coordinator.parser_coordinator() {
            parser.prime_ast_cache_for_source(
                cache_text.as_ref(),
                Arc::clone(&parse_snapshot.parse_result),
            );
            parser.prime_tree_cache_for_file(
                PathBuf::from(path.as_ref()),
                cache_text.as_ref().to_string(),
                Arc::clone(&parse_snapshot.backend_tree),
                parse_snapshot.backend_tree_hash,
            );
        }
        self.record_did_save_exact_producer_lifecycle_state_v2(
            producer_key,
            super::super::DidSaveExactProducerLifecycleStateV2::DetachedDiagnosticsReadyPublished,
        )
        .await;
        if let Some(task_control) = task_control {
            task_control.note_detached_ready_artifact_published();
        }
    }

    async fn update_ready_parse_snapshot_phase_attribution_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        expected_text: &Arc<str>,
        source: super::super::BackgroundParseSnapshotApplyTaskSourceV2,
        phase_attribution: &super::super::ReadyParseSnapshotPhaseAttributionV2,
    ) {
        let mut ready_states = self.latest_ready_parse_snapshots_v2.write().await;
        let Some(state) = ready_states.get_mut(&file_id) else {
            return;
        };
        if state.source != source || state.text.as_ref() != expected_text.as_ref() {
            return;
        }
        state.phase_attribution = phase_attribution.clone();
    }

    async fn record_snapshot_build_failure_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        requested_version: i32,
        reason: &'static str,
    ) {
        self.latest_snapshot_failures_v2.write().await.insert(
            file_id,
            super::super::SnapshotBuildFailureStateV2 {
                requested_version,
                reason: Arc::from(reason),
            },
        );
    }

    pub(crate) async fn update_ready_parse_snapshot_after_deferred_syntax_error_assembly_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        requested_version: i32,
        expected_text: &Arc<str>,
        parse_result: Arc<bsl_runtime::parsing::ParseResult>,
    ) {
        let mut ready_states = self.latest_ready_parse_snapshots_v2.write().await;
        let Some(state) = ready_states.get_mut(&file_id) else {
            return;
        };
        if state.parse_snapshot.file_version != requested_version
            || state.text.as_ref() != expected_text.as_ref()
        {
            return;
        }
        state.parse_snapshot.parse_result = parse_result;
        state.syntax_errors_complete = true;
    }

    fn spawn_snapshot_status_refresh_v2(&self, file_id: bsl_analysis_v2::FileId) {
        let server = self.clone();
        tokio::spawn(async move {
            server.refresh_snapshot_status_v2(file_id).await;
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn_deferred_parse_snapshot_post_publish_enrichment_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        requested_version: i32,
        path: Arc<str>,
        text: Arc<str>,
        parse_snapshot: bsl_analysis_v2::ParseSnapshot,
        complete_syntax_error_assembly: bool,
        complete_cache_enrichment: bool,
    ) {
        let server = self.clone();
        tokio::spawn(async move {
            let update_symbol_index = server
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                == Some(requested_version);
            let Some(parser) = server.coordinator.parser_coordinator() else {
                return;
            };
            let path_buf = PathBuf::from(path.as_ref());
            let text_for_enrichment = text;
            let text_for_blocking = Arc::clone(&text_for_enrichment);
            let parse_result_for_enrichment = Arc::clone(&parse_snapshot.parse_result);
            let backend_tree_for_enrichment = Arc::clone(&parse_snapshot.backend_tree);
            let enriched_result =
                bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
                    bsl_runtime::application::CpuWorkClass::Background,
                    bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                    Some(server.coordinator.as_ref()),
                    move || {
                        let parse_result = if complete_syntax_error_assembly {
                            parser.complete_deferred_parse_snapshot_syntax_error_assembly(
                                backend_tree_for_enrichment.as_ref(),
                                text_for_blocking.as_ref(),
                                parse_result_for_enrichment.as_ref(),
                            )
                        } else {
                            parse_result_for_enrichment.as_ref().clone()
                        };
                        if complete_cache_enrichment {
                            parser.complete_deferred_parse_snapshot_cache_enrichment(
                                path_buf.as_path(),
                                text_for_blocking.as_ref(),
                                &parse_result,
                                update_symbol_index,
                            );
                        }
                        parse_result
                    },
                )
                .await
                .ok();
            let Some(enriched_result) = enriched_result else {
                return;
            };
            if complete_syntax_error_assembly {
                server
                    .update_ready_parse_snapshot_after_deferred_syntax_error_assembly_v2(
                        file_id,
                        requested_version,
                        &text_for_enrichment,
                        Arc::new(enriched_result),
                    )
                    .await;
            }
        });
    }

    fn spawn_deferred_parse_snapshot_tree_cache_install_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        requested_version: i32,
        path: Arc<str>,
        text: Arc<str>,
    ) {
        let server = self.clone();
        tokio::spawn(async move {
            let shadow_state = super::super::DocumentShadowStateV2 {
                version: requested_version,
                text,
            };
            server
                .prime_parser_tree_cache_from_matching_ready_snapshot_v2(
                    file_id,
                    path.as_ref(),
                    &shadow_state,
                )
                .await;
        });
    }

    async fn prime_parser_tree_cache_from_matching_ready_snapshot_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        path: &str,
        shadow_state: &super::super::DocumentShadowStateV2,
    ) {
        let ready_state = self
            .latest_ready_parse_snapshots_v2
            .read()
            .await
            .get(&file_id)
            .cloned()
            .filter(|state| {
                state.parse_snapshot.file_version == shadow_state.version
                    && state.text.as_ref() == shadow_state.text.as_ref()
            });
        let Some(ready_state) = ready_state else {
            return;
        };
        let Some(parser) = self.coordinator.parser_coordinator() else {
            return;
        };
        parser.prime_tree_cache_for_file(
            PathBuf::from(path),
            ready_state.text.as_ref().to_string(),
            Arc::clone(&ready_state.parse_snapshot.backend_tree),
            ready_state.parse_snapshot.backend_tree_hash,
        );
    }

    async fn inspect_shadow_state_parser_base_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        path: &str,
        shadow_state: &super::super::DocumentShadowStateV2,
    ) -> ShadowStateParserBaseInspectionV2 {
        let latest_ready_state = self
            .latest_ready_parse_snapshots_v2
            .read()
            .await
            .get(&file_id)
            .cloned();
        let latest_ready_document_version = latest_ready_state
            .as_ref()
            .map(|state| state.parse_snapshot.file_version);
        let matching_ready_snapshot_for_shadow_state =
            latest_ready_state.as_ref().is_some_and(|state| {
                state.parse_snapshot.file_version == shadow_state.version
                    && state.text.as_ref() == shadow_state.text.as_ref()
            });
        let Some(parser) = self.coordinator.parser_coordinator() else {
            return ShadowStateParserBaseInspectionV2 {
                forced_full_parse_reason: None,
                stale_parser_base: None,
            };
        };
        let mut ready_snapshot_prime_attempted = false;
        if matching_ready_snapshot_for_shadow_state {
            ready_snapshot_prime_attempted = true;
            self.prime_parser_tree_cache_from_matching_ready_snapshot_v2(
                file_id,
                path,
                shadow_state,
            )
            .await;
            maybe_poison_tree_cache_after_prime_for_test(&parser, path, shadow_state.text.as_ref());
        }
        let tree_cache_matches_shadow_text_after_prime =
            parser.tree_cache_matches_source_for_file(Path::new(path), shadow_state.text.as_ref());
        if tree_cache_matches_shadow_text_after_prime {
            return ShadowStateParserBaseInspectionV2 {
                forced_full_parse_reason: None,
                stale_parser_base: None,
            };
        }
        let stale_parser_base = DidChangeStaleParserBaseAttributionV2 {
            root_cause: classify_stale_parser_base_root_cause(
                shadow_state.version,
                latest_ready_document_version,
                matching_ready_snapshot_for_shadow_state,
                ready_snapshot_prime_attempted,
                Some(tree_cache_matches_shadow_text_after_prime),
            ),
            shadow_document_version: shadow_state.version,
            latest_ready_document_version,
            matching_ready_snapshot_for_shadow_state,
            ready_snapshot_prime_attempted,
            tree_cache_matches_shadow_text_after_prime: if ready_snapshot_prime_attempted {
                Some(tree_cache_matches_shadow_text_after_prime)
            } else {
                None
            },
        };
        ShadowStateParserBaseInspectionV2 {
            forced_full_parse_reason: Some(
                bsl_runtime::system::parser_coordinator::ParserCoordinator::parse_snapshot_fallback_stale_parser_base_reason(),
            ),
            stale_parser_base: Some(stale_parser_base),
        }
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
        let expected_version = analysis.file_version(file_id).ok().flatten();
        let context = self
            .build_execution_context_v2(
                bsl_runtime::application::SemanticOperation::Completion,
                file_id,
                expected_version,
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
                let ir_program =
                    bsl_runtime::application::IntellisenseV2Facade::run_ir_query_singleflight_attach_or_direct(
                        &context,
                        &analysis,
                        Some(coordinator.as_ref()),
                        file_id,
                    )?;
                let Some(ir_program) = ir_program else {
                    return Ok(());
                };
                let Some(expected_version) = expected_version else {
                    return Ok(());
                };
                if analysis
                    .current_type_index_serve_only_ready(file_id)
                    .ok()
                    .unwrap_or(false)
                {
                    return Ok(());
                }
                let precompute_started = Instant::now();
                let precompute = analysis.precompute_type_index_for_file_from_program(
                    file_id,
                    Some(expected_version),
                    0,
                    ir_program,
                ).map_err(|_| bsl_runtime::application::SingleflightQueryError::Cancelled)?;
                coordinator.record_intellisense_v2_type_index_reason(
                    precompute.reason_code.as_str(),
                );
                if precompute.stats.evicted_per_file_window_total > 0 {
                    coordinator.record_intellisense_v2_type_index_reason(
                        bsl_analysis_v2::TypeIndexArtifactReasonCode::TypeIndexArtifactEvictedPerFileWindow
                            .as_str(),
                    );
                }
                if precompute.stats.evicted_global_guard_total > 0 {
                    coordinator.record_intellisense_v2_type_index_reason(
                        bsl_analysis_v2::TypeIndexArtifactReasonCode::TypeIndexArtifactEvictedGlobalGuard
                            .as_str(),
                    );
                }
                coordinator.record_intellisense_v2_runtime_exec_latency_with_origin(
                    bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                    "type_index_precompute",
                    precompute_started.elapsed(),
                );
                coordinator.record_intellisense_v2_runtime_exec_latency_with_origin(
                    bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                    "type_index_precompute_build",
                    duration_from_millis_u128_for_metrics(precompute.stats.build_ms),
                );
                coordinator.record_intellisense_v2_runtime_exec_latency_with_origin(
                    bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                    "type_index_precompute_ir",
                    duration_from_millis_u128_for_metrics(precompute.stats.ir_ms),
                );
                coordinator.record_intellisense_v2_runtime_exec_latency_with_origin(
                    bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                    "type_index_precompute_ast_to_ir",
                    duration_from_millis_u128_for_metrics(precompute.stats.ast_to_ir_convert_ms),
                );
                coordinator.record_intellisense_v2_runtime_exec_latency_with_origin(
                    bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                    "type_index_precompute_semantic_facts",
                    duration_from_millis_u128_for_metrics(precompute.stats.semantic_facts_materialize_ms),
                );
                coordinator.record_intellisense_v2_runtime_exec_latency_with_origin(
                    bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                    "type_index_precompute_semantic_facts_local_function_summaries",
                    duration_from_millis_u128_for_metrics(
                        precompute.stats.semantic_facts_local_function_summaries_ms,
                    ),
                );
                Ok::<(), bsl_runtime::application::SingleflightQueryError>(())
            },
        )
        .await;
    }

    async fn build_parse_snapshot_v2(
        &self,
        request: BuildParseSnapshotRequest,
    ) -> BuildParseSnapshotOutcomeV2 {
        let coordinator = self.coordinator.clone();
        let path_for_parse = request.path.clone();
        let text_for_parse = request.text.clone();
        let reused_prefix_parse_result_for_parse = request.reused_prefix_parse_result.clone();
        let parser_base_recovery_text_for_parse = request.parser_base_recovery_text.clone();
        let parser_base_recovery_reuse_parse_result_for_parse =
            request.parser_base_recovery_reuse_parse_result.clone();
        let parse_started = Instant::now();
        let blocking_delay_env_key_for_parse = request.blocking_delay_env_key;
        let requested_target_epoch_state_for_parse = request.requested_target_epoch_state;
        let requested_target_epoch_for_parse = request.requested_target_epoch;
        let task_control_for_parse = request.task_control;
        let did_change_attribution = request.did_change_attribution.clone();
        let did_change_attribution_for_task = did_change_attribution.clone();
        let version = request.version;
        let file_id = request.file_id;
        let parser_edits = request.parser_edits;
        let forced_full_parse_reason = request.forced_full_parse_reason;
        let initial_admission_lane = request.admission_lane;
        let promoted_to_did_save_followup = task_control_for_parse
            .as_ref()
            .is_some_and(|control| control.promotion_requested.load(Ordering::SeqCst));
        let same_version_did_save_followup = matches!(
            initial_admission_lane,
            Some(bsl_runtime::application::AdmissionLane::DidSaveFollowup)
        ) || promoted_to_did_save_followup;
        let same_version_previous_ready_seed_for_parse = if task_control_for_parse.is_some() {
            self.latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .and_then(|state| {
                    derive_same_version_rebuild_previous_ready_seed_v2(
                        state,
                        version,
                        &text_for_parse,
                    )
                })
        } else {
            None
        };
        let same_version_rebuild_reuse_parse_result_for_parse =
            if same_version_did_save_followup && parser_edits.is_empty() {
                derive_same_version_rebuild_reuse_parse_result_from_current_state_v2(
                    self,
                    file_id,
                    version,
                    text_for_parse.as_ref(),
                )
                .await
            } else {
                None
            };
        let parse_call = if let Some(task_control) = task_control_for_parse.clone() {
            let task_control_for_lane = Arc::clone(&task_control);
            let task_control_for_exec = Some(Arc::clone(&task_control));
            let task_control_for_exec_started = Arc::clone(&task_control);
            let same_version_previous_ready_seed_for_exec =
                same_version_previous_ready_seed_for_parse.clone();
            bsl_runtime::application::spawn_bounded_blocking_with_class_observed_call_origin_dynamic_lane_hooks(
                request.cpu_work_class,
                bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                move || {
                    if task_control_for_lane
                        .promotion_requested
                        .load(Ordering::SeqCst)
                    {
                        Some(bsl_runtime::application::AdmissionLane::DidSaveFollowup)
                    } else {
                        initial_admission_lane
                    }
                },
                &task_control.control_notify,
                Some(self.coordinator.as_ref()),
                Option::<fn()>::None,
                Some(move |_queue_wait_elapsed| {
                    task_control_for_exec_started.transition_phase_attribution(
                        super::super::ReadyParseSnapshotAttributionPhaseV2::ParseExec,
                    );
                }),
                move || {
                    let preserve_late_ranged_parser_base = || {
                        task_control_for_exec.as_ref().is_some_and(|control| {
                            control.retarget_requested.load(Ordering::SeqCst)
                                && late_ranged_did_change_parser_base_preservation_allowed(
                                    did_change_attribution_for_task.as_ref(),
                                    control.as_ref(),
                                )
                        })
                    };
                    let progress_control = task_control_for_exec
                        .as_ref()
                        .expect("task control for parse progress");
                    progress_control.transition_parse_exec_subphase_attribution(
                        super::super::ReadyParseSnapshotParseExecSubphaseV2::CoreParseBuild,
                    );
                    progress_control.transition_core_build_checkpoint_attribution(
                        super::super::ReadyParseSnapshotCoreBuildCheckpointV2::PreParseSetup,
                    );
                    if requested_target_epoch_state_for_parse
                        .as_ref()
                        .zip(requested_target_epoch_for_parse)
                        .is_some_and(|(state, epoch)| state.load(Ordering::Relaxed) != epoch)
                        && !preserve_late_ranged_parser_base()
                    {
                        return Err(BuildParseSnapshotAbortReasonV2::Superseded);
                    }
                    if task_control_for_exec
                        .as_ref()
                        .is_some_and(|control| {
                            control.cancel_requested.load(Ordering::SeqCst)
                                || control.retarget_requested.load(Ordering::SeqCst)
                        })
                        && !preserve_late_ranged_parser_base()
                    {
                        return Err(BuildParseSnapshotAbortReasonV2::Superseded);
                    }
                    let Some(parser) = coordinator.parser_coordinator() else {
                        return Err(BuildParseSnapshotAbortReasonV2::BuildSnapshotAborted);
                    };
                    if let Some(seed) = same_version_previous_ready_seed_for_exec.as_ref() {
                        parser.prime_ast_cache_for_source(
                            seed.source_text.as_ref(),
                            Arc::clone(&seed.parse_result),
                        );
                    }
                    if let Some(parse_result) =
                        same_version_rebuild_reuse_parse_result_for_parse.as_ref()
                    {
                        parser.prime_ast_cache_for_source(
                            text_for_parse.as_ref(),
                            Arc::clone(parse_result),
                        );
                    }
                    let parse_exec_progress = |subphase: bsl_runtime::system::parser_coordinator::ParseSnapshotExecSubphase| {
                        let mapped = match subphase {
                            bsl_runtime::system::parser_coordinator::ParseSnapshotExecSubphase::CoreParseBuild => {
                                super::super::ReadyParseSnapshotParseExecSubphaseV2::CoreParseBuild
                            }
                            bsl_runtime::system::parser_coordinator::ParseSnapshotExecSubphase::OptionalCacheEnrichment => {
                                super::super::ReadyParseSnapshotParseExecSubphaseV2::OptionalCacheEnrichment
                            }
                        };
                        progress_control.transition_parse_exec_subphase_attribution(mapped);
                    };
                    let core_build_progress = |checkpoint: bsl_runtime::system::parser_coordinator::ParseSnapshotCoreBuildCheckpoint| {
                        let mapped = match checkpoint {
                            bsl_runtime::system::parser_coordinator::ParseSnapshotCoreBuildCheckpoint::ParserTreeBuild => {
                                super::super::ReadyParseSnapshotCoreBuildCheckpointV2::ParserTreeBuild
                            }
                            bsl_runtime::system::parser_coordinator::ParseSnapshotCoreBuildCheckpoint::ExactReadySnapshotAssembly => {
                                super::super::ReadyParseSnapshotCoreBuildCheckpointV2::ExactReadySnapshotAssembly
                            }
                            bsl_runtime::system::parser_coordinator::ParseSnapshotCoreBuildCheckpoint::TreeCacheInstall => {
                                super::super::ReadyParseSnapshotCoreBuildCheckpointV2::TreeCacheInstall
                            }
                        };
                        progress_control.transition_core_build_checkpoint_attribution(mapped);
                    };
                    let assembly_progress = |checkpoint: bsl_runtime::system::parser_coordinator::ParseSnapshotAssemblyCheckpoint| {
                        let mapped = match checkpoint {
                            bsl_runtime::system::parser_coordinator::ParseSnapshotAssemblyCheckpoint::ProgramLowering => {
                                super::super::ReadyParseSnapshotAssemblyCheckpointV2::ProgramLowering
                            }
                            bsl_runtime::system::parser_coordinator::ParseSnapshotAssemblyCheckpoint::PublishableArtifactPackaging => {
                                super::super::ReadyParseSnapshotAssemblyCheckpointV2::PublishableArtifactPackaging
                            }
                            bsl_runtime::system::parser_coordinator::ParseSnapshotAssemblyCheckpoint::SyntaxErrorCollection => {
                                super::super::ReadyParseSnapshotAssemblyCheckpointV2::SyntaxErrorCollection
                            }
                        };
                        progress_control.transition_assembly_checkpoint_attribution(mapped);
                    };
                    let exact_ready_snapshot_control = || {
                        let superseded = requested_target_epoch_state_for_parse
                            .as_ref()
                            .zip(requested_target_epoch_for_parse)
                            .is_some_and(|(state, epoch)| state.load(Ordering::Relaxed) != epoch)
                            || progress_control.cancel_requested.load(Ordering::SeqCst)
                            || progress_control.retarget_requested.load(Ordering::SeqCst);
                        if superseded
                            && !late_ranged_did_change_parser_base_preservation_allowed(
                                did_change_attribution_for_task.as_ref(),
                                progress_control.as_ref(),
                            )
                        {
                            bsl_runtime::system::parser_coordinator::ParseSnapshotExactReadyControl::Cancel
                        } else if progress_control
                            .promotion_requested
                            .load(Ordering::SeqCst)
                        {
                            bsl_runtime::system::parser_coordinator::ParseSnapshotExactReadyControl::SaveCritical
                        } else {
                            bsl_runtime::system::parser_coordinator::ParseSnapshotExactReadyControl::Continue
                        }
                    };
                    let mut parse_options =
                        bsl_runtime::system::parser_coordinator::ParseSnapshotExecutionOptions::default();
                    parse_options.save_critical_initial = matches!(
                        initial_admission_lane,
                        Some(bsl_runtime::application::AdmissionLane::DidSaveFollowup)
                    );
                    parse_options.save_critical_requested =
                        Some(&progress_control.promotion_requested);
                    parse_options.reused_program_prefix = reused_prefix_parse_result_for_parse
                        .as_ref()
                        .map(|parse_result| parse_result.program.statements.as_slice());
                    parse_options.lowering_reuse_plan = None;
                    parse_options.exact_ready_snapshot_control_callback =
                        Some(&exact_ready_snapshot_control);
                    parse_options.progress_callback = Some(&parse_exec_progress);
                    parse_options.core_build_progress_callback = Some(&core_build_progress);
                    parse_options.assembly_progress_callback = Some(&assembly_progress);
                    let mut blocking_delay_injected = false;
                    let mut inject_blocking_delay_at_checkpoint = |
                        checkpoint: super::super::ReadyParseSnapshotCoreBuildCheckpointV2| {
                        progress_control.transition_core_build_checkpoint_attribution(checkpoint);
                        if blocking_delay_injected {
                            return;
                        }
                        if let Some(env_key) = blocking_delay_env_key_for_parse {
                            maybe_inject_blocking_parse_delay_for_test(env_key);
                        }
                        blocking_delay_injected = true;
                    };
                    let mut effective_forced_full_parse_reason = forced_full_parse_reason;
                    if effective_forced_full_parse_reason
                        == Some(
                            bsl_runtime::system::parser_coordinator::ParserCoordinator::parse_snapshot_fallback_stale_parser_base_reason(),
                        )
                    {
                        if let Some(recovery_text) = parser_base_recovery_text_for_parse.as_ref() {
                            let recovery_path = PathBuf::from(path_for_parse.as_ref());
                            let recovery_reuse_seeded =
                                parser_base_recovery_reuse_parse_result_for_parse
                                    .as_ref()
                                    .is_some_and(|parse_result| {
                                        parser.prime_ast_cache_for_source(
                                            recovery_text.as_ref(),
                                            Arc::clone(parse_result),
                                        );
                                        true
                                    });
                            let prime_options =
                                bsl_runtime::system::parser_coordinator::PrimeTreeCacheFromSourceOptions {
                                    skip_optional_ast_priming_initial: recovery_reuse_seeded,
                                    skip_optional_ast_priming_requested: None,
                                };
                            let recovery_matched = if parser.tree_cache_matches_source_for_file(
                                recovery_path.as_path(),
                                recovery_text.as_ref(),
                            ) {
                                true
                            } else {
                                inject_blocking_delay_at_checkpoint(
                                    super::super::ReadyParseSnapshotCoreBuildCheckpointV2::ParserBaseRecovery,
                                );
                                match parser
                                    .prime_tree_cache_from_source_with_cancellation_and_options(
                                        recovery_path.clone(),
                                        recovery_text.to_string(),
                                        &task_control_for_exec
                                            .as_ref()
                                            .expect("task control for parser-base recovery")
                                            .cancel_requested,
                                        prime_options,
                                    )
                                {
                                    Ok(()) => {
                                        maybe_poison_tree_cache_after_recovery_for_test(
                                            &parser,
                                            path_for_parse.as_ref(),
                                            recovery_text.as_ref(),
                                        );
                                        parser.tree_cache_matches_source_for_file(
                                            recovery_path.as_path(),
                                            recovery_text.as_ref(),
                                        )
                                    }
                                    Err(error)
                                        if bsl_runtime::system::parser_coordinator::is_parse_cancelled_error(&error) =>
                                    {
                                        return Err(Self::classify_parse_snapshot_cancellation_abort_reason_v2(
                                            requested_target_epoch_state_for_parse.as_ref(),
                                            requested_target_epoch_for_parse,
                                            task_control_for_exec.as_ref(),
                                        ));
                                    }
                                    Err(error) => {
                                        warn!(
                                            file_id = file_id.0,
                                            file_version = version,
                                            error = %error,
                                            "failed to recover lagging parser base from shadow state"
                                        );
                                        false
                                    }
                                }
                            };
                            if recovery_matched {
                                effective_forced_full_parse_reason = None;
                            }
                        }
                    }
                    inject_blocking_delay_at_checkpoint(
                        super::super::ReadyParseSnapshotCoreBuildCheckpointV2::ParserTreeBuild,
                    );
                    let parse_result = if let Some(reason) = effective_forced_full_parse_reason {
                        parser.parse_full_with_report_with_cancellation_and_options(
                            PathBuf::from(path_for_parse.as_ref()),
                            text_for_parse.to_string(),
                            reason,
                            &progress_control.cancel_requested,
                            parse_options,
                        )
                    } else {
                        parser.parse_incremental_with_report_with_cancellation_and_options(
                            PathBuf::from(path_for_parse.as_ref()),
                            text_for_parse.to_string(),
                            parser_edits,
                            &progress_control.cancel_requested,
                            parse_options,
                        )
                    };
                    parse_result.map_err(|error| {
                        let cancelled_by_current_target_control =
                            requested_target_epoch_state_for_parse
                                .as_ref()
                                .zip(requested_target_epoch_for_parse)
                                .is_some_and(|(state, epoch)| {
                                    state.load(Ordering::Relaxed) != epoch
                                })
                                || task_control_for_exec.as_ref().is_some_and(|control| {
                                    control.cancel_requested.load(Ordering::SeqCst)
                                        || control.retarget_requested.load(Ordering::SeqCst)
                                });
                        if cancelled_by_current_target_control
                            && bsl_runtime::system::parser_coordinator::is_parse_cancelled_error(&error)
                        {
                            Self::classify_parse_snapshot_cancellation_abort_reason_v2(
                                requested_target_epoch_state_for_parse.as_ref(),
                                requested_target_epoch_for_parse,
                                task_control_for_exec.as_ref(),
                            )
                        } else {
                            warn!(
                                file_id = file_id.0,
                                file_version = version,
                                error = %error,
                                "background parse snapshot build failed"
                            );
                            BuildParseSnapshotAbortReasonV2::BuildSnapshotAborted
                        }
                    })
                },
            )
            .await
        } else {
            let same_version_previous_ready_seed_for_exec =
                same_version_previous_ready_seed_for_parse.clone();
            bsl_runtime::application::spawn_bounded_blocking_with_class_observed_call_origin_lane_hooks(
                request.cpu_work_class,
                bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                request.admission_lane,
                Some(self.coordinator.as_ref()),
                Option::<fn()>::None,
                Option::<fn(Duration)>::None,
                move || {
                    if let Some(env_key) = blocking_delay_env_key_for_parse {
                        maybe_inject_blocking_parse_delay_for_test(env_key);
                    }
                    if requested_target_epoch_state_for_parse
                        .as_ref()
                        .zip(requested_target_epoch_for_parse)
                        .is_some_and(|(state, epoch)| state.load(Ordering::Relaxed) != epoch)
                    {
                        return Err(BuildParseSnapshotAbortReasonV2::Superseded);
                    }
                    if task_control_for_parse
                        .as_ref()
                        .is_some_and(|control| {
                            control.cancel_requested.load(Ordering::SeqCst)
                                || control.retarget_requested.load(Ordering::SeqCst)
                        })
                    {
                        return Err(BuildParseSnapshotAbortReasonV2::Superseded);
                    }
                    let Some(parser) = coordinator.parser_coordinator() else {
                        return Err(BuildParseSnapshotAbortReasonV2::BuildSnapshotAborted);
                    };
                    if let Some(seed) = same_version_previous_ready_seed_for_exec.as_ref() {
                        parser.prime_ast_cache_for_source(
                            seed.source_text.as_ref(),
                            Arc::clone(&seed.parse_result),
                        );
                    }
                    if let Some(parse_result) =
                        same_version_rebuild_reuse_parse_result_for_parse.as_ref()
                    {
                        parser.prime_ast_cache_for_source(
                            text_for_parse.as_ref(),
                            Arc::clone(parse_result),
                        );
                    }
                    let mut effective_forced_full_parse_reason = forced_full_parse_reason;
                    if effective_forced_full_parse_reason
                        == Some(
                            bsl_runtime::system::parser_coordinator::ParserCoordinator::parse_snapshot_fallback_stale_parser_base_reason(),
                        )
                    {
                        if let Some(recovery_text) = parser_base_recovery_text_for_parse.as_ref() {
                            let recovery_path = PathBuf::from(path_for_parse.as_ref());
                            let recovery_reuse_seeded =
                                parser_base_recovery_reuse_parse_result_for_parse
                                    .as_ref()
                                    .is_some_and(|parse_result| {
                                        parser.prime_ast_cache_for_source(
                                            recovery_text.as_ref(),
                                            Arc::clone(parse_result),
                                        );
                                        true
                                    });
                            let recovery_matched = if parser.tree_cache_matches_source_for_file(
                                recovery_path.as_path(),
                                recovery_text.as_ref(),
                            ) {
                                true
                            } else {
                                let recovery_cancel = std::sync::atomic::AtomicBool::new(false);
                                match parser
                                    .prime_tree_cache_from_source_with_cancellation_and_options(
                                        recovery_path.clone(),
                                        recovery_text.to_string(),
                                        &recovery_cancel,
                                        bsl_runtime::system::parser_coordinator::PrimeTreeCacheFromSourceOptions {
                                            skip_optional_ast_priming_initial: recovery_reuse_seeded,
                                            skip_optional_ast_priming_requested: None,
                                        },
                                    )
                                {
                                    Ok(()) => {
                                        maybe_poison_tree_cache_after_recovery_for_test(
                                            &parser,
                                            path_for_parse.as_ref(),
                                            recovery_text.as_ref(),
                                        );
                                        parser.tree_cache_matches_source_for_file(
                                            recovery_path.as_path(),
                                            recovery_text.as_ref(),
                                        )
                                    }
                                    Err(error) => {
                                        warn!(
                                            file_id = file_id.0,
                                            file_version = version,
                                            error = %error,
                                            "failed to recover lagging parser base from shadow state"
                                        );
                                        false
                                    }
                                }
                            };
                            if recovery_matched {
                                effective_forced_full_parse_reason = None;
                            }
                        }
                    }
                    let parse_result = if let Some(reason) = effective_forced_full_parse_reason {
                        parser.parse_full_with_report(
                            PathBuf::from(path_for_parse.as_ref()),
                            text_for_parse.to_string(),
                            reason,
                        )
                    } else {
                        parser.parse_incremental_with_report(
                            PathBuf::from(path_for_parse.as_ref()),
                            text_for_parse.to_string(),
                            parser_edits,
                        )
                    };
                    parse_result.map_err(|error| {
                        if task_control_for_parse
                            .as_ref()
                            .is_some_and(|control| {
                                (control.cancel_requested.load(Ordering::SeqCst)
                                    || control.retarget_requested.load(Ordering::SeqCst))
                                    && bsl_runtime::system::parser_coordinator::is_parse_cancelled_error(&error)
                            })
                        {
                            Self::classify_parse_snapshot_cancellation_abort_reason_v2(
                                requested_target_epoch_state_for_parse.as_ref(),
                                requested_target_epoch_for_parse,
                                task_control_for_parse.as_ref(),
                            )
                        } else {
                            warn!(
                                file_id = file_id.0,
                                file_version = version,
                                error = %error,
                                "background parse snapshot build failed"
                            );
                            BuildParseSnapshotAbortReasonV2::BuildSnapshotAborted
                        }
                    })
                },
            )
            .await
        };
        let report = match parse_call.join_result {
            Ok(Ok(report)) => report,
            Ok(Err(reason)) => return BuildParseSnapshotOutcomeV2::Aborted(reason),
            Err(_) => {
                return BuildParseSnapshotOutcomeV2::Aborted(
                    BuildParseSnapshotAbortReasonV2::BuildSnapshotAborted,
                );
            }
        };
        self.record_parse_snapshot_report_v2(&report, parse_started.elapsed());
        let deferred_work = DeferredParseSnapshotWorkV2 {
            optional_cache_enrichment: report
                .parse_exec_subphases
                .deferred_optional_cache_enrichment,
            tree_cache_install: report.parse_exec_subphases.deferred_tree_cache_install,
            syntax_error_assembly: report.parse_exec_subphases.deferred_syntax_error_assembly,
        };
        let mut effective_did_change_attribution = did_change_attribution;
        if report.fallback_reason.as_deref()
            != Some(
                bsl_runtime::system::parser_coordinator::ParserCoordinator::parse_snapshot_fallback_stale_parser_base_reason(),
            )
        {
            if let Some(attribution) = effective_did_change_attribution.as_mut() {
                attribution.stale_parser_base = None;
            }
        }
        if let Some(attribution) = effective_did_change_attribution.as_ref() {
            self.record_did_change_parse_snapshot_evidence(
                &attribution.uri,
                super::super::DidChangeParseSnapshotEvidenceKey {
                    file_id,
                    requested_version: version,
                },
                parse_snapshot_mode_from_report(&report),
                attribution.base_text_source,
                attribution.change_shape,
                attribution.content_changes_count,
                attribution.replay_order,
                attribution.base_document_version,
                report.changed_ranges.len(),
                report.fallback_reason.as_deref(),
                attribution,
            );
        }
        let program_lowering_summary = report.program_lowering_summary;
        BuildParseSnapshotOutcomeV2::Ready(
            parse_snapshot_from_report(file_id, version, report),
            deferred_work,
            Box::new(program_lowering_summary),
        )
    }

    pub(crate) async fn matching_background_parse_snapshot_task_control_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        requested_version: i32,
        expected_text_hash: Option<[u8; 32]>,
    ) -> Option<Arc<super::super::BackgroundParseSnapshotApplyTaskControlV2>> {
        let tasks = self.background_parse_snapshot_apply_tasks_v2.lock().await;
        tasks
            .get(&file_id)
            .filter(|task| {
                background_parse_snapshot_task_matches_v2(
                    task,
                    requested_version,
                    expected_text_hash,
                )
            })
            .map(|task| Arc::clone(&task.control))
    }

    pub(crate) async fn current_shadow_text_hash_for_version_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        requested_version: i32,
    ) -> Option<[u8; 32]> {
        self.latest_document_shadow_state_v2
            .read()
            .await
            .get(&file_id)
            .filter(|state| state.version == requested_version)
            .map(|state| *blake3::hash(state.text.as_bytes()).as_bytes())
    }

    pub(crate) async fn promote_matching_background_parse_snapshot_apply_task_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        requested_version: i32,
        expected_text_hash: Option<[u8; 32]>,
    ) -> bool {
        let Some(task_control) = self
            .matching_background_parse_snapshot_task_control_v2(
                file_id,
                requested_version,
                expected_text_hash,
            )
            .await
        else {
            return false;
        };
        task_control
            .promotion_requested
            .store(true, Ordering::SeqCst);
        task_control.control_notify.notify_waiters();
        true
    }

    pub(crate) async fn background_parse_snapshot_task_retargeted_away_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        requested_version: i32,
        expected_text_hash: Option<[u8; 32]>,
    ) -> bool {
        let tasks = self.background_parse_snapshot_apply_tasks_v2.lock().await;
        tasks.get(&file_id).is_some_and(|task| {
            !background_parse_snapshot_task_matches_v2(task, requested_version, expected_text_hash)
        })
    }

    async fn background_parse_snapshot_apply_task_is_current_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        target_epoch_state: &Arc<std::sync::atomic::AtomicU64>,
    ) -> bool {
        let tasks = self.background_parse_snapshot_apply_tasks_v2.lock().await;
        tasks
            .get(&file_id)
            .is_some_and(|task| Arc::ptr_eq(&task.target_epoch, target_epoch_state))
    }

    pub(crate) async fn promote_background_parse_snapshot_apply_task_for_did_save_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        requested_version: i32,
        expected_text_hash: Option<[u8; 32]>,
    ) -> bool {
        let tasks = self.background_parse_snapshot_apply_tasks_v2.lock().await;
        let Some(task) = tasks.get(&file_id) else {
            return false;
        };
        let task_target = background_parse_snapshot_task_target_v2(task);
        if task_target.requested_version != requested_version
            || expected_text_hash.is_some_and(|text_hash| task_target.text_hash != text_hash)
            || matches!(
                task_target.source,
                super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidSave
            )
        {
            return false;
        }
        task.control
            .promotion_requested
            .store(true, Ordering::SeqCst);
        task.control.control_notify.notify_waiters();
        true
    }

    async fn schedule_background_parse_snapshot_apply_v2(
        &self,
        mut args: BackgroundParseSnapshotApplyArgs,
    ) {
        let file_id = args.file_id;
        let text_hash = parse_snapshot_text_hash(args.text.as_ref());
        if let Some(producer_key) = super::super::DidSaveExactProducerKeyV2::from_parts(
            file_id,
            args.requested_version,
            text_hash,
            args.save_cycle_sequence,
        ) {
            self.record_did_save_exact_producer_lifecycle_state_v2(
                producer_key,
                super::super::DidSaveExactProducerLifecycleStateV2::Admitted,
            )
            .await;
        }
        let mut tasks = self.background_parse_snapshot_apply_tasks_v2.lock().await;
        let mut lifecycle_events = Vec::new();
        if let Some(task) = tasks.get(&file_id) {
            let current_target = background_parse_snapshot_task_target_v2(task);
            let same_version = current_target.requested_version == args.requested_version;
            if same_version && current_target.text_hash == text_hash {
                let did_save_parser_edit_upgrade = matches!(
                    (current_target.source, args.source),
                    (
                        super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidSave,
                        super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidSave
                    )
                ) && current_target.parser_edits.is_empty()
                    && !args.parser_edits.is_empty();
                if matches!(
                    args.source,
                    super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidSave
                ) {
                    let pre_exec_waiting = task.control.phase_attribution_snapshot().current_phase
                        == Some(super::super::ReadyParseSnapshotAttributionPhaseV2::Waiting);
                    let waiting_did_change_needs_respawn = pre_exec_waiting
                        && !matches!(
                            current_target.source,
                            super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidSave
                        );
                    if waiting_did_change_needs_respawn {
                        args = BackgroundParseSnapshotApplyArgs {
                            file_id: args.file_id,
                            requested_version: args.requested_version,
                            save_cycle_sequence: args.save_cycle_sequence,
                            path: current_target.path.clone(),
                            text: current_target.text.clone(),
                            cpu_work_class: args.cpu_work_class,
                            parser_base_recovery_text: current_target
                                .parser_base_recovery_text
                                .clone(),
                            parser_base_recovery_reuse_parse_result: current_target
                                .parser_base_recovery_reuse_parse_result
                                .clone(),
                            parser_edits: current_target.parser_edits.clone(),
                            forced_full_parse_reason: current_target.forced_full_parse_reason,
                            async_delay_mode: args.async_delay_mode,
                            blocking_delay_env_key: args.blocking_delay_env_key,
                            force_reschedule_same_version: args.force_reschedule_same_version,
                            source: super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidSave,
                            did_change_attribution: current_target.did_change_attribution.clone(),
                        };
                    }
                    if !waiting_did_change_needs_respawn {
                        let mut target = task
                            .target
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                        let previous_producer_key =
                            super::super::DidSaveExactProducerKeyV2::from_target(file_id, &target);
                        target.save_cycle_sequence = args.save_cycle_sequence;
                        let next_producer_key =
                            super::super::DidSaveExactProducerKeyV2::from_target(file_id, &target);
                        if previous_producer_key.is_some()
                            && previous_producer_key != next_producer_key
                        {
                            if let Some(previous_producer_key) = previous_producer_key {
                                lifecycle_events.push((
                                    previous_producer_key,
                                    super::super::DidSaveExactProducerLifecycleStateV2::Superseded,
                                ));
                            }
                        }
                        if !matches!(
                            target.source,
                            super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidSave
                        ) {
                            target.source =
                                super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidSave;
                        }
                        task.control.set_cpu_work_class(args.cpu_work_class);
                    }
                    if !waiting_did_change_needs_respawn
                        && !matches!(
                            current_target.source,
                            super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidSave
                        )
                    {
                        task.control
                            .promotion_requested
                            .store(true, Ordering::SeqCst);
                        task.control.control_notify.notify_waiters();
                    }
                    if !waiting_did_change_needs_respawn
                        && matches!(
                            current_target.source,
                            super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidChange
                        )
                        && !pre_exec_waiting
                    {
                        if let Some(producer_key) =
                            super::super::DidSaveExactProducerKeyV2::from_parts(
                                file_id,
                                args.requested_version,
                                text_hash,
                                args.save_cycle_sequence,
                            )
                        {
                            lifecycle_events.push((
                                producer_key,
                                super::super::DidSaveExactProducerLifecycleStateV2::Started,
                            ));
                        }
                    }
                    if !waiting_did_change_needs_respawn
                        && !did_save_parser_edit_upgrade
                        && (pre_exec_waiting
                            || matches!(
                                current_target.source,
                                super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidSave
                            )
                            || matches!(
                                current_target.source,
                                super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidChange
                            )
                            || !args.force_reschedule_same_version)
                    {
                        drop(tasks);
                        self.record_did_save_exact_producer_lifecycle_events_v2(lifecycle_events)
                            .await;
                        return;
                    }
                } else {
                    return;
                }
            }
            if !args.force_reschedule_same_version && same_version {
                return;
            }
            if matches!(
                (current_target.source, args.source),
                (
                    super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidChange,
                    super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidChange
                )
            ) {
                let preserve_late_ranged_parser_base =
                    late_ranged_did_change_parser_base_preservation_allowed(
                        args.did_change_attribution.as_ref(),
                        task.control.as_ref(),
                    );
                let next_epoch = task.target_epoch.fetch_add(1, Ordering::SeqCst) + 1;
                let next_target =
                    background_parse_snapshot_apply_target_from_args(&args, text_hash, next_epoch);
                {
                    let mut target = task
                        .target
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    *target = next_target;
                }
                task.control
                    .cancel_requested
                    .store(!preserve_late_ranged_parser_base, Ordering::SeqCst);
                task.control
                    .retarget_requested
                    .store(true, Ordering::SeqCst);
                task.control
                    .promotion_requested
                    .store(false, Ordering::SeqCst);
                task.control.materialized.store(false, Ordering::SeqCst);
                task.control.phase.store(
                    super::super::BackgroundParseSnapshotApplyTaskPhaseV2::Waiting as u8,
                    Ordering::SeqCst,
                );
                task.control.materialized_notify.notify_waiters();
                task.control.control_notify.notify_waiters();
                drop(tasks);
                self.spawn_snapshot_status_refresh_v2(file_id);
                return;
            }
        }
        if let Some(previous) = tasks.remove(&file_id) {
            if let Some(previous_producer_key) =
                super::super::DidSaveExactProducerKeyV2::from_target(
                    file_id,
                    &background_parse_snapshot_task_target_v2(&previous),
                )
            {
                lifecycle_events.push((
                    previous_producer_key,
                    super::super::DidSaveExactProducerLifecycleStateV2::Superseded,
                ));
            }
            previous
                .control
                .cancel_requested
                .store(true, Ordering::SeqCst);
            previous.control.control_notify.notify_waiters();
        }

        let target_epoch = Arc::new(std::sync::atomic::AtomicU64::new(1));
        let target = Arc::new(std::sync::Mutex::new(
            background_parse_snapshot_apply_target_from_args(&args, text_hash, 1),
        ));
        let task_control = Arc::new(
            super::super::BackgroundParseSnapshotApplyTaskControlV2::new_with_work_class(
                args.cpu_work_class,
            ),
        );
        let server = self.clone();
        let worker_target_epoch = Arc::clone(&target_epoch);
        let worker_target = Arc::clone(&target);
        let worker_task_control = Arc::clone(&task_control);
        let handle = tokio::spawn(async move {
            server
                .run_background_parse_snapshot_apply_worker_v2(
                    file_id,
                    worker_target,
                    worker_target_epoch,
                    worker_task_control,
                )
                .await;
        });
        tasks.insert(
            file_id,
            super::super::BackgroundParseSnapshotApplyTaskV2 {
                target_epoch,
                target,
                control: task_control,
                handle,
            },
        );
        drop(tasks);
        self.record_did_save_exact_producer_lifecycle_events_v2(lifecycle_events)
            .await;
        self.spawn_snapshot_status_refresh_v2(file_id);
    }

    async fn run_background_parse_snapshot_apply_worker_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        target_state: Arc<std::sync::Mutex<super::super::BackgroundParseSnapshotApplyTargetV2>>,
        target_epoch_state: Arc<std::sync::atomic::AtomicU64>,
        task_control: Arc<super::super::BackgroundParseSnapshotApplyTaskControlV2>,
    ) {
        let origin_label = bsl_runtime::application::ObservabilityOrigin::Lsp.as_str();
        loop {
            if !self
                .background_parse_snapshot_apply_task_is_current_v2(file_id, &target_epoch_state)
                .await
            {
                break;
            }
            let mut target = target_state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone();
            task_control.cancel_requested.store(false, Ordering::SeqCst);
            task_control
                .retarget_requested
                .store(false, Ordering::SeqCst);
            let mut source_label = background_parse_snapshot_apply_source_label(target.source);
            let mut lifecycle_guard = ReadyParseSnapshotWorkerLifecycleGuard::new(
                self.coordinator.clone(),
                origin_label,
                source_label,
            );
            task_control.reset_phase_attribution();
            task_control.reset_ready_install_exact_type_index_wait();
            task_control.phase.store(
                super::super::BackgroundParseSnapshotApplyTaskPhaseV2::Waiting as u8,
                Ordering::SeqCst,
            );
            task_control.transition_phase_attribution(
                super::super::ReadyParseSnapshotAttributionPhaseV2::Waiting,
            );
            self.refresh_snapshot_status_v2(file_id).await;
            let debounce = if matches!(
                target.source,
                super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidSave
            ) {
                Duration::ZERO
            } else {
                parse_snapshot_apply_debounce_duration()
            };
            if debounce > Duration::ZERO
                && !task_control.cancel_requested.load(Ordering::SeqCst)
                && !task_control.promotion_requested.load(Ordering::SeqCst)
            {
                let notified = task_control.control_notify.notified();
                tokio::pin!(notified);
                tokio::select! {
                    _ = tokio::time::sleep(debounce) => {}
                    _ = &mut notified => {}
                }
            }
            let refreshed_target = target_state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone();
            if refreshed_target.epoch != target.epoch {
                self.record_did_save_exact_producer_lifecycle_for_target_v2(
                    file_id,
                    &target,
                    super::super::DidSaveExactProducerLifecycleStateV2::Superseded,
                )
                .await;
                lifecycle_guard.set_terminal_reason("retargeted_before_parse");
                task_control.control_notify.notify_waiters();
                continue;
            }
            target = refreshed_target;
            source_label = background_parse_snapshot_apply_source_label(target.source);
            lifecycle_guard.set_source(source_label);
            if target_epoch_state.load(Ordering::SeqCst) != target.epoch {
                self.record_did_save_exact_producer_lifecycle_for_target_v2(
                    file_id,
                    &target,
                    super::super::DidSaveExactProducerLifecycleStateV2::Superseded,
                )
                .await;
                lifecycle_guard.set_terminal_reason("retargeted_before_parse");
                task_control.control_notify.notify_waiters();
                continue;
            }
            if task_control.cancel_requested.load(Ordering::SeqCst) {
                self.record_did_save_exact_producer_lifecycle_for_target_v2(
                    file_id,
                    &target,
                    super::super::DidSaveExactProducerLifecycleStateV2::Cancelled,
                )
                .await;
                lifecycle_guard.set_terminal_reason("superseded");
                task_control.control_notify.notify_waiters();
                break;
            }
            if self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                != Some(target.requested_version)
            {
                self.record_did_save_exact_producer_lifecycle_for_target_v2(
                    file_id,
                    &target,
                    super::super::DidSaveExactProducerLifecycleStateV2::Superseded,
                )
                .await;
                lifecycle_guard.set_terminal_reason("latest_version_mismatch");
                task_control.control_notify.notify_waiters();
                break;
            }
            match target.async_delay_mode {
                ParseSnapshotAsyncDelayMode::None => {}
                ParseSnapshotAsyncDelayMode::DidChangeTestOnly => {
                    maybe_inject_did_change_parse_delay().await;
                }
                ParseSnapshotAsyncDelayMode::DidSaveTestOnly => {
                    maybe_inject_did_save_parse_delay().await;
                }
            }
            let refreshed_target = target_state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone();
            if refreshed_target.epoch != target.epoch {
                self.record_did_save_exact_producer_lifecycle_for_target_v2(
                    file_id,
                    &target,
                    super::super::DidSaveExactProducerLifecycleStateV2::Superseded,
                )
                .await;
                lifecycle_guard.set_terminal_reason("retargeted_before_parse");
                task_control.control_notify.notify_waiters();
                continue;
            }
            target = refreshed_target;
            source_label = background_parse_snapshot_apply_source_label(target.source);
            lifecycle_guard.set_source(source_label);
            if target_epoch_state.load(Ordering::SeqCst) != target.epoch {
                self.record_did_save_exact_producer_lifecycle_for_target_v2(
                    file_id,
                    &target,
                    super::super::DidSaveExactProducerLifecycleStateV2::Superseded,
                )
                .await;
                lifecycle_guard.set_terminal_reason("retargeted_before_parse");
                task_control.control_notify.notify_waiters();
                continue;
            }
            if task_control.cancel_requested.load(Ordering::SeqCst) {
                self.record_did_save_exact_producer_lifecycle_for_target_v2(
                    file_id,
                    &target,
                    super::super::DidSaveExactProducerLifecycleStateV2::Cancelled,
                )
                .await;
                lifecycle_guard.set_terminal_reason("superseded");
                task_control.control_notify.notify_waiters();
                break;
            }
            if self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                != Some(target.requested_version)
            {
                self.record_did_save_exact_producer_lifecycle_for_target_v2(
                    file_id,
                    &target,
                    super::super::DidSaveExactProducerLifecycleStateV2::Superseded,
                )
                .await;
                lifecycle_guard.set_terminal_reason("latest_version_mismatch");
                task_control.control_notify.notify_waiters();
                break;
            }

            self.record_did_save_exact_producer_lifecycle_for_target_v2(
                file_id,
                &target,
                super::super::DidSaveExactProducerLifecycleStateV2::Started,
            )
            .await;
            task_control.phase.store(
                super::super::BackgroundParseSnapshotApplyTaskPhaseV2::Parsing as u8,
                Ordering::SeqCst,
            );
            self.refresh_snapshot_status_v2(file_id).await;
            let (parse_snapshot, deferred_work, program_lowering_summary) = match self
                .build_parse_snapshot_v2(BuildParseSnapshotRequest {
                    file_id,
                    version: target.requested_version,
                    path: target.path.clone(),
                    text: target.text.clone(),
                    cpu_work_class: task_control.cpu_work_class(),
                    reused_prefix_parse_result: self
                        .latest_ready_parse_snapshots_v2
                        .read()
                        .await
                        .get(&file_id)
                        .and_then(|state| {
                            derive_reused_prefix_parse_result_from_ready_state(
                                state,
                                target.requested_version,
                                &target.text,
                            )
                        }),
                    parser_base_recovery_text: target.parser_base_recovery_text.clone(),
                    parser_base_recovery_reuse_parse_result: target
                        .parser_base_recovery_reuse_parse_result
                        .clone(),
                    parser_edits: target.parser_edits.clone(),
                    forced_full_parse_reason: target.forced_full_parse_reason,
                    blocking_delay_env_key: target.blocking_delay_env_key,
                    requested_target_epoch_state: Some(Arc::clone(&target_epoch_state)),
                    requested_target_epoch: Some(target.epoch),
                    task_control: Some(Arc::clone(&task_control)),
                    admission_lane: matches!(
                        target.source,
                        super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidSave
                    )
                    .then_some(bsl_runtime::application::AdmissionLane::DidSaveFollowup)
                    .or_else(|| {
                        task_control
                            .promotion_requested
                            .load(Ordering::SeqCst)
                            .then_some(bsl_runtime::application::AdmissionLane::DidSaveFollowup)
                    }),
                    did_change_attribution: target.did_change_attribution.clone(),
                })
                .await
            {
                BuildParseSnapshotOutcomeV2::Ready(
                    parse_snapshot,
                    deferred_work,
                    program_lowering_summary,
                ) => (parse_snapshot, deferred_work, *program_lowering_summary),
                BuildParseSnapshotOutcomeV2::Aborted(
                    BuildParseSnapshotAbortReasonV2::RetargetedDuringParse,
                ) => {
                    self.record_did_save_exact_producer_lifecycle_for_target_v2(
                        file_id,
                        &target,
                        super::super::DidSaveExactProducerLifecycleStateV2::Superseded,
                    )
                    .await;
                    lifecycle_guard.set_terminal_reason("retargeted_during_parse");
                    task_control.control_notify.notify_waiters();
                    if target_epoch_state.load(Ordering::SeqCst) != target.epoch
                        && self
                            .background_parse_snapshot_apply_task_is_current_v2(
                                file_id,
                                &target_epoch_state,
                            )
                            .await
                    {
                        continue;
                    }
                    break;
                }
                BuildParseSnapshotOutcomeV2::Aborted(
                    BuildParseSnapshotAbortReasonV2::Superseded,
                ) => {
                    self.record_did_save_exact_producer_lifecycle_for_target_v2(
                        file_id,
                        &target,
                        if target_epoch_state.load(Ordering::SeqCst) != target.epoch {
                            super::super::DidSaveExactProducerLifecycleStateV2::Superseded
                        } else {
                            super::super::DidSaveExactProducerLifecycleStateV2::Cancelled
                        },
                    )
                    .await;
                    lifecycle_guard.set_terminal_reason(
                        if target_epoch_state.load(Ordering::SeqCst) != target.epoch {
                            "retargeted_before_materialization"
                        } else {
                            "superseded"
                        },
                    );
                    task_control.control_notify.notify_waiters();
                    if target_epoch_state.load(Ordering::SeqCst) != target.epoch
                        && self
                            .background_parse_snapshot_apply_task_is_current_v2(
                                file_id,
                                &target_epoch_state,
                            )
                            .await
                    {
                        continue;
                    }
                    break;
                }
                BuildParseSnapshotOutcomeV2::Aborted(
                    BuildParseSnapshotAbortReasonV2::BuildSnapshotAborted,
                ) => {
                    self.record_did_save_exact_producer_lifecycle_for_target_v2(
                        file_id,
                        &target,
                        super::super::DidSaveExactProducerLifecycleStateV2::Failed,
                    )
                    .await;
                    self.record_snapshot_build_failure_v2(
                        file_id,
                        target.requested_version,
                        "build_snapshot_aborted",
                    )
                    .await;
                    lifecycle_guard.set_terminal_reason("build_snapshot_aborted");
                    task_control.control_notify.notify_waiters();
                    break;
                }
            };
            task_control.set_program_lowering_summary(program_lowering_summary);
            task_control.transition_phase_attribution(
                super::super::ReadyParseSnapshotAttributionPhaseV2::PostParsePreMaterialization,
            );
            if matches!(
                target.async_delay_mode,
                ParseSnapshotAsyncDelayMode::DidChangeTestOnly
            ) {
                maybe_inject_did_change_pre_materialization_delay().await;
            }
            if target_epoch_state.load(Ordering::SeqCst) != target.epoch {
                self.record_did_save_exact_producer_lifecycle_for_target_v2(
                    file_id,
                    &target,
                    super::super::DidSaveExactProducerLifecycleStateV2::Superseded,
                )
                .await;
                lifecycle_guard.set_terminal_reason("retargeted_before_materialization");
                task_control.control_notify.notify_waiters();
                continue;
            }
            if task_control.cancel_requested.load(Ordering::SeqCst) {
                self.record_did_save_exact_producer_lifecycle_for_target_v2(
                    file_id,
                    &target,
                    super::super::DidSaveExactProducerLifecycleStateV2::Cancelled,
                )
                .await;
                lifecycle_guard.set_terminal_reason("superseded");
                task_control.control_notify.notify_waiters();
                break;
            }
            if self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                != Some(target.requested_version)
            {
                self.record_did_save_exact_producer_lifecycle_for_target_v2(
                    file_id,
                    &target,
                    super::super::DidSaveExactProducerLifecycleStateV2::Superseded,
                )
                .await;
                lifecycle_guard.set_terminal_reason("latest_version_mismatch");
                task_control.control_notify.notify_waiters();
                break;
            }
            let refreshed_target = target_state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone();
            if refreshed_target.epoch != target.epoch {
                self.record_did_save_exact_producer_lifecycle_for_target_v2(
                    file_id,
                    &target,
                    super::super::DidSaveExactProducerLifecycleStateV2::Superseded,
                )
                .await;
                lifecycle_guard.set_terminal_reason("retargeted_before_materialization");
                task_control.control_notify.notify_waiters();
                continue;
            }
            target = refreshed_target;
            source_label = background_parse_snapshot_apply_source_label(target.source);
            lifecycle_guard.set_source(source_label);
            task_control.phase.store(
                super::super::BackgroundParseSnapshotApplyTaskPhaseV2::Materializing as u8,
                Ordering::SeqCst,
            );
            task_control.transition_phase_attribution(
                super::super::ReadyParseSnapshotAttributionPhaseV2::ReadyInstall,
            );
            self.refresh_snapshot_status_v2(file_id).await;
            let detached_save_cycle_sequence = target.save_cycle_sequence;
            self.record_detached_diagnostics_ready_artifact_v2(
                file_id,
                target.requested_version,
                target.text_hash,
                detached_save_cycle_sequence,
                Some(task_control.as_ref()),
                &target.path,
                target.text.clone(),
                &parse_snapshot,
                !deferred_work.syntax_error_assembly,
            )
            .await;

            // Install the already-built parse snapshot before the exact type-index wait.
            // Canonical ready publication still waits below, but type-index precompute can now
            // reuse the same snapshot-backed current revision instead of falling back to the
            // much slower raw Salsa parse/IR path on large modules.
            self.analysis_v2.apply_changes_interactive(
                bsl_runtime::application::ObservabilityOrigin::Lsp,
                vec![bsl_analysis_v2::Change::SetFileWithSnapshot {
                    file_id,
                    text: target.text.clone(),
                    version: target.requested_version,
                    path: target.path.clone(),
                    parse_snapshot: parse_snapshot.clone(),
                }],
            );
            self.spawn_completion_head_precompute_from_snapshot_v2(
                file_id,
                target.requested_version,
            );
            let allow_type_index_precompute = !matches!(
                target.source,
                super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidSave
            );

            match self
                .wait_for_exact_type_index_before_ready_install_v2(
                    ReadyInstallExactTypeIndexWaitArgs {
                        file_id,
                        requested_version: target.requested_version,
                        target_epoch_state: &target_epoch_state,
                        target_epoch: target.epoch,
                        task_control: &task_control,
                        max_wait: Some(ready_install_exact_type_index_wait_max_duration()),
                        allow_type_index_precompute,
                    },
                )
                .await
            {
                ExactTypeIndexBeforeReadyInstallOutcomeV2::Ready => {}
                ExactTypeIndexBeforeReadyInstallOutcomeV2::Deadline => {
                    self.record_did_save_exact_producer_lifecycle_for_target_v2(
                        file_id,
                        &target,
                        super::super::DidSaveExactProducerLifecycleStateV2::ExactTypeIndexDeadline,
                    )
                    .await;
                    self.record_snapshot_build_failure_v2(
                        file_id,
                        target.requested_version,
                        "exact_type_index_deadline_before_ready_install",
                    )
                    .await;
                    lifecycle_guard
                        .set_terminal_reason("exact_type_index_deadline_before_ready_install");
                    task_control.control_notify.notify_waiters();
                    break;
                }
                ExactTypeIndexBeforeReadyInstallOutcomeV2::Retargeted => {
                    self.record_did_save_exact_producer_lifecycle_for_target_v2(
                        file_id,
                        &target,
                        super::super::DidSaveExactProducerLifecycleStateV2::Superseded,
                    )
                    .await;
                    lifecycle_guard.set_terminal_reason("retargeted_before_exact_ready_install");
                    task_control.control_notify.notify_waiters();
                    continue;
                }
                ExactTypeIndexBeforeReadyInstallOutcomeV2::Superseded => {
                    self.record_did_save_exact_producer_lifecycle_for_target_v2(
                        file_id,
                        &target,
                        super::super::DidSaveExactProducerLifecycleStateV2::Cancelled,
                    )
                    .await;
                    lifecycle_guard.set_terminal_reason("superseded_before_exact_ready_install");
                    task_control.control_notify.notify_waiters();
                    break;
                }
                ExactTypeIndexBeforeReadyInstallOutcomeV2::LatestVersionMismatch => {
                    self.record_did_save_exact_producer_lifecycle_for_target_v2(
                        file_id,
                        &target,
                        super::super::DidSaveExactProducerLifecycleStateV2::Superseded,
                    )
                    .await;
                    lifecycle_guard
                        .set_terminal_reason("latest_version_mismatch_before_exact_ready_install");
                    task_control.control_notify.notify_waiters();
                    break;
                }
            }
            self.record_ready_parse_snapshot_v2(ReadyParseSnapshotRecordArgs {
                file_id,
                path: &target.path,
                text: target.text.clone(),
                parse_snapshot: &parse_snapshot,
                source: target.source,
                syntax_errors_complete: !deferred_work.syntax_error_assembly,
                program_lowering_summary,
            })
            .await;
            if let Some(producer_key) = super::super::DidSaveExactProducerKeyV2::from_parts(
                file_id,
                target.requested_version,
                target.text_hash,
                detached_save_cycle_sequence,
            ) {
                self.record_did_save_exact_producer_lifecycle_state_v2(
                    producer_key,
                    super::super::DidSaveExactProducerLifecycleStateV2::FullyMaterialized,
                )
                .await;
            }
            let ready_phase_snapshot = task_control.finish_phase_attribution();
            self.update_ready_parse_snapshot_phase_attribution_v2(
                file_id,
                &target.text,
                target.source,
                &ready_phase_snapshot.completed,
            )
            .await;
            lifecycle_guard.mark_materialized();
            task_control.materialized.store(true, Ordering::SeqCst);
            task_control.materialized_notify.notify_waiters();
            task_control.control_notify.notify_waiters();
            if deferred_work.tree_cache_install {
                self.spawn_deferred_parse_snapshot_tree_cache_install_v2(
                    file_id,
                    target.requested_version,
                    Arc::clone(&target.path),
                    Arc::clone(&target.text),
                );
            }
            if deferred_work.syntax_error_assembly || deferred_work.optional_cache_enrichment {
                self.spawn_deferred_parse_snapshot_post_publish_enrichment_v2(
                    file_id,
                    target.requested_version,
                    Arc::clone(&target.path),
                    Arc::clone(&target.text),
                    parse_snapshot.clone(),
                    deferred_work.syntax_error_assembly,
                    deferred_work.optional_cache_enrichment,
                );
            }
            self.coordinator
                .record_intellisense_v2_ready_parse_snapshot_materialization(
                    origin_label,
                    source_label,
                    lifecycle_guard.started.elapsed(),
                );
            record_ready_parse_snapshot_phase_metrics(
                &self.coordinator,
                origin_label,
                source_label,
                &ready_phase_snapshot.completed,
            );
            task_control.transition_phase_attribution(
                super::super::ReadyParseSnapshotAttributionPhaseV2::DocumentSymbolSideWork,
            );
            let text_for_symbols = target.text.clone();
            let parse_result_for_symbols = Arc::clone(&parse_snapshot.parse_result);
            match bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
                bsl_runtime::application::CpuWorkClass::Background,
                origin_label,
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
                        file_id,
                        target.requested_version,
                        response,
                    )
                    .await;
                }
                Ok(Err(err)) => {
                    warn!(
                        file_id = file_id.0,
                        file_version = target.requested_version,
                        error = %err,
                        "failed to build documentSymbol ready cache from parse snapshot"
                    );
                }
                Err(err) => {
                    warn!(
                        file_id = file_id.0,
                        file_version = target.requested_version,
                        error = %err,
                        "documentSymbol ready-cache task failed after parse snapshot"
                    );
                }
            }
            let symbol_phase_snapshot = task_control.finish_phase_attribution();
            self.update_ready_parse_snapshot_phase_attribution_v2(
                file_id,
                &target.text,
                target.source,
                &symbol_phase_snapshot.completed,
            )
            .await;
            if let Some(document_symbol_side_work_ms) =
                symbol_phase_snapshot.completed.document_symbol_side_work_ms
            {
                self.coordinator
                    .record_intellisense_v2_ready_parse_snapshot_phase_latency(
                        origin_label,
                        source_label,
                        "document_symbol_side_work",
                        Duration::from_millis(document_symbol_side_work_ms),
                    );
            }

            if target_epoch_state.load(Ordering::SeqCst) != target.epoch {
                task_control.control_notify.notify_waiters();
                continue;
            }
            if task_control.cancel_requested.load(Ordering::SeqCst) {
                task_control.control_notify.notify_waiters();
                break;
            }
            if self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                != Some(target.requested_version)
            {
                task_control.control_notify.notify_waiters();
                break;
            }
            if !self
                .analysis_v2
                .wait_for_file_version(file_id, target.requested_version)
                .await
            {
                task_control.control_notify.notify_waiters();
                break;
            }
            if target_epoch_state.load(Ordering::SeqCst) != target.epoch {
                task_control.control_notify.notify_waiters();
                continue;
            }
            if task_control.cancel_requested.load(Ordering::SeqCst) {
                task_control.control_notify.notify_waiters();
                break;
            }
            if self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                != Some(target.requested_version)
            {
                task_control.control_notify.notify_waiters();
                break;
            }
            task_control.control_notify.notify_waiters();
            if target_epoch_state.load(Ordering::SeqCst) == target.epoch {
                break;
            }
        }

        let mut tasks = self.background_parse_snapshot_apply_tasks_v2.lock().await;
        if tasks
            .get(&file_id)
            .is_some_and(|task| Arc::ptr_eq(&task.target_epoch, &target_epoch_state))
        {
            tasks.remove(&file_id);
        }
        drop(tasks);
        self.refresh_snapshot_status_v2(file_id).await;
        task_control.control_notify.notify_waiters();
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
            let completion_head_ready = analysis
                .current_completion_head_ready(file_id)
                .ok()
                .unwrap_or(false);
            let exact_type_index_ready = analysis
                .current_type_index_serve_only_ready(file_id)
                .ok()
                .unwrap_or(false);
            if completion_head_ready && exact_type_index_ready {
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
            let expected_text_hash = self
                .current_shadow_text_hash_for_version_v2(file_id, requested_version)
                .await;
            let _ = self
                .promote_matching_background_parse_snapshot_apply_task_v2(
                    file_id,
                    requested_version,
                    expected_text_hash,
                )
                .await;

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
            let completion_head_ready = analysis
                .current_completion_head_ready(file_id)
                .ok()
                .unwrap_or(false);
            let exact_type_index_ready = analysis
                .current_type_index_serve_only_ready(file_id)
                .ok()
                .unwrap_or(false);
            if completion_head_ready && exact_type_index_ready {
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

    pub(crate) async fn cancel_background_parse_snapshot_apply_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
    ) {
        let task = self
            .background_parse_snapshot_apply_tasks_v2
            .lock()
            .await
            .remove(&file_id);
        if let Some(task) = task {
            task.control.cancel_requested.store(true, Ordering::SeqCst);
            task.control.control_notify.notify_waiters();
            task.handle.abort();
            self.refresh_snapshot_status_v2(file_id).await;
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

            let program_lowering_summary = report.program_lowering_summary;
            let parse_snapshot = parse_snapshot_from_report(file_id, requested_version, report);
            server
                .record_ready_parse_snapshot_v2(ReadyParseSnapshotRecordArgs {
                    file_id,
                    path: &path,
                    text: text.clone(),
                    parse_snapshot: &parse_snapshot,
                    source: super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidChange,
                    syntax_errors_complete: true,
                    program_lowering_summary,
                })
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

    #[allow(clippy::too_many_arguments)]
    async fn apply_did_change_current_revision_fast_lane_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        uri: &Url,
        version: i32,
        changes: &[TextDocumentContentChangeEvent],
        path: Arc<str>,
        parse_snapshot_change_shape: &'static str,
        parse_snapshot_replay_order: &'static str,
        parse_snapshot_content_changes_count: usize,
    ) -> Option<DidChangePostHandoffWorkV2> {
        let _sync_guard = self.text_sync_v2.lock().await;
        let previous_shadow_state = {
            let shadow = self.latest_document_shadow_state_v2.read().await;
            shadow.get(&file_id).cloned()
        };

        let (
            updated_text,
            parser_edits,
            parse_snapshot_base_text_source,
            parse_snapshot_base_document_version,
        ) = if let Some(full_change) = changes.iter().find(|c| c.range.is_none()) {
            if let Some(state) = previous_shadow_state.as_ref() {
                if version < state.version {
                    warn!(
                        uri = %uri,
                        file_id = file_id.0,
                        requested_version = version,
                        shadow_version = state.version,
                        "Skipping out-of-order didChange for older version"
                    );
                    return None;
                }
            }
            if let Some(state) = previous_shadow_state.as_ref() {
                (
                    full_change.text.clone(),
                    whole_text_change_to_parser_edit(state.text.as_ref(), &full_change.text)
                        .into_iter()
                        .collect(),
                    "shadow_state",
                    Some(state.version),
                )
            } else {
                (full_change.text.clone(), Vec::new(), "not_applicable", None)
            }
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
                    return None;
                }
            }
            let (base_text, base_text_source, base_document_version) =
                if let Some(state) = previous_shadow_state.as_ref() {
                    (state.text.to_string(), "shadow_state", Some(state.version))
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
                        None,
                    )
                };

            let replay_plan = canonicalize_ranged_did_change_replay_plan(changes);
            let mut current_text = base_text;
            let mut parser_edits = Vec::with_capacity(replay_plan.len());
            for step in replay_plan {
                current_text = apply_text_edit(&current_text, step.range, &step.new_text);
                parser_edits.push(step.parser_edit);
            }
            (
                current_text,
                parser_edits,
                base_text_source,
                base_document_version,
            )
        };

        let identical_text_previous_version = previous_shadow_state.as_ref().and_then(|state| {
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
        self.cancel_stale_type_index_precompute_v2(file_id, version)
            .await;
        self.cleanup_stale_completed_type_index_precompute_task_v2(file_id, version)
            .await;
        let updated_text: Arc<str> = Arc::from(updated_text);
        self.latest_document_shadow_state_v2.write().await.insert(
            file_id,
            super::super::DocumentShadowStateV2 {
                version,
                text: updated_text.clone(),
            },
        );
        self.analysis_v2.apply_changes_interactive(
            bsl_runtime::application::ObservabilityOrigin::Lsp,
            vec![bsl_analysis_v2::Change::SetFile {
                file_id,
                text: updated_text.clone(),
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
        let diagnostics_save_cycle_sequence_at_handoff = self
            .diagnostics_save_cycle_sequence_v2
            .read()
            .await
            .get(&file_id)
            .copied()
            .unwrap_or(0);

        Some(DidChangePostHandoffWorkV2 {
            uri: uri.clone(),
            file_id,
            version,
            diagnostics_save_cycle_sequence_at_handoff,
            path,
            updated_text,
            parser_edits,
            previous_shadow_state,
            identical_text_previous_version,
            tail_whitespace_append_previous_version,
            previous_analysis_for_identical_text_reuse,
            parse_snapshot_change_shape,
            parse_snapshot_replay_order,
            parse_snapshot_content_changes_count,
            parse_snapshot_base_text_source,
            parse_snapshot_base_document_version,
        })
    }

    fn spawn_did_change_post_handoff_v2(&self, work: DidChangePostHandoffWorkV2) {
        let server = self.clone();
        tokio::spawn(async move {
            maybe_inject_did_change_post_handoff_delay().await;
            server.run_did_change_post_handoff_v2(work).await;
        });
    }

    async fn did_change_diagnostics_are_stale_after_same_version_save_v2(
        &self,
        file_id: bsl_analysis_v2::FileId,
        requested_version: i32,
        diagnostics_save_cycle_sequence_at_handoff: u64,
        updated_text: &Arc<str>,
    ) -> bool {
        let latest_save_cycle_sequence = self
            .diagnostics_save_cycle_sequence_v2
            .read()
            .await
            .get(&file_id)
            .copied()
            .unwrap_or(0);
        if latest_save_cycle_sequence <= diagnostics_save_cycle_sequence_at_handoff {
            return false;
        }

        let latest_received_version = self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied();
        if latest_received_version != Some(requested_version) {
            return false;
        }

        self.latest_document_shadow_state_v2
            .read()
            .await
            .get(&file_id)
            .is_some_and(|state| {
                state.version == requested_version && state.text.as_ref() == updated_text.as_ref()
            })
    }

    async fn run_did_change_post_handoff_v2(&self, work: DidChangePostHandoffWorkV2) {
        if self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&work.file_id)
            .copied()
            != Some(work.version)
        {
            return;
        }

        let scale_aware_knobs =
            bsl_runtime::application::ScaleAwareDiagnosticsKnobs::from_runtime_config();
        let mut large_churn_active = false;
        if scale_aware_knobs.enabled {
            let is_large_document = bsl_runtime::application::scale_aware_document_is_large(
                &work.updated_text,
                scale_aware_knobs,
            );
            let now = Instant::now();
            let transition = {
                let mut churn_state = self.scale_aware_churn_state_v2.write().await;
                let state = churn_state.entry(work.file_id).or_insert(
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
                .remove(&work.file_id)
                .is_some_and(|state| state.large_churn_active);
            if was_active {
                self.coordinator
                    .record_intellisense_v2_large_churn_transition(
                        bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                        "exit",
                    );
            }
        }

        if self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&work.file_id)
            .copied()
            != Some(work.version)
        {
            return;
        }

        let (
            forced_full_parse_reason,
            parse_snapshot_stale_parser_base,
            parser_base_recovery_text,
            parser_base_recovery_reuse_parse_result,
        ) = if let Some(state) = work.previous_shadow_state.clone() {
            let shadow_state_parser_base = self
                .inspect_shadow_state_parser_base_v2(work.file_id, work.path.as_ref(), &state)
                .await;
            let parser_base_recovery_text = shadow_state_parser_base
                .stale_parser_base
                .as_ref()
                .filter(|stale| stale.root_cause == "ready_snapshot_lags_shadow_state")
                .map(|_| state.text.clone());
            let parser_base_recovery_reuse_parse_result = if parser_base_recovery_text.is_some() {
                derive_parser_base_recovery_reuse_parse_result_from_shadow_state_v2(
                    self,
                    work.file_id,
                    &state,
                )
                .await
            } else {
                None
            };
            (
                shadow_state_parser_base.forced_full_parse_reason,
                shadow_state_parser_base.stale_parser_base,
                parser_base_recovery_text,
                parser_base_recovery_reuse_parse_result,
            )
        } else {
            (None, None, None, None)
        };

        if self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&work.file_id)
            .copied()
            != Some(work.version)
        {
            return;
        }

        if self
            .did_change_diagnostics_are_stale_after_same_version_save_v2(
                work.file_id,
                work.version,
                work.diagnostics_save_cycle_sequence_at_handoff,
                &work.updated_text,
            )
            .await
        {
            let latest_save_cycle_sequence = self
                .diagnostics_save_cycle_sequence_v2
                .read()
                .await
                .get(&work.file_id)
                .copied()
                .unwrap_or(0);
            debug!(
                uri = %work.uri,
                file_id = work.file_id.0,
                requested_version = work.version,
                save_cycle_sequence_at_handoff = work.diagnostics_save_cycle_sequence_at_handoff,
                latest_save_cycle_sequence,
                "skip stale didChange follow-up scheduling after same-version didSave"
            );
            self.schedule_background_parse_snapshot_apply_v2(BackgroundParseSnapshotApplyArgs {
                file_id: work.file_id,
                requested_version: work.version,
                save_cycle_sequence: Some(latest_save_cycle_sequence),
                path: work.path.clone(),
                text: work.updated_text.clone(),
                cpu_work_class: bsl_runtime::application::CpuWorkClass::Interactive,
                parser_base_recovery_text,
                parser_base_recovery_reuse_parse_result,
                parser_edits: work.parser_edits,
                forced_full_parse_reason,
                async_delay_mode: ParseSnapshotAsyncDelayMode::DidSaveTestOnly,
                blocking_delay_env_key: Some("BSL_TEST_DID_SAVE_BLOCKING_PARSE_DELAY_MS"),
                force_reschedule_same_version: true,
                source: super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidSave,
                did_change_attribution: Some(DidChangeParseSnapshotAttributionV2 {
                    uri: work.uri.clone(),
                    base_text_source: work.parse_snapshot_base_text_source,
                    change_shape: work.parse_snapshot_change_shape,
                    content_changes_count: work.parse_snapshot_content_changes_count,
                    replay_order: work.parse_snapshot_replay_order,
                    base_document_version: work.parse_snapshot_base_document_version,
                    stale_parser_base: parse_snapshot_stale_parser_base,
                }),
            })
            .await;
            return;
        }

        if work.identical_text_previous_version.is_none()
            && work.tail_whitespace_append_previous_version.is_none()
        {
            self.schedule_completion_head_precompute_from_current_revision_v2(
                work.file_id,
                work.version,
            )
            .await;
        }
        if let Some(previous_version) = work.identical_text_previous_version {
            self.spawn_completion_head_reuse_from_previous_version_v2(
                work.file_id,
                work.version,
                previous_version,
                work.previous_analysis_for_identical_text_reuse
                    .expect("previous analysis snapshot for identical-text head reuse"),
            );
        }
        if let Some(previous_version) = work.tail_whitespace_append_previous_version {
            self.spawn_completion_head_version_alias_from_previous_version_v2(
                work.file_id,
                work.version,
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
                work.file_id,
                work.version,
                work.path.clone(),
                work.updated_text.clone(),
                work.parser_edits,
            );
        } else {
            self.schedule_background_parse_snapshot_apply_v2(BackgroundParseSnapshotApplyArgs {
                file_id: work.file_id,
                requested_version: work.version,
                save_cycle_sequence: None,
                path: work.path.clone(),
                text: work.updated_text.clone(),
                cpu_work_class: bsl_runtime::application::CpuWorkClass::Background,
                parser_base_recovery_text,
                parser_base_recovery_reuse_parse_result,
                parser_edits: work.parser_edits,
                forced_full_parse_reason,
                async_delay_mode: ParseSnapshotAsyncDelayMode::DidChangeTestOnly,
                blocking_delay_env_key: Some("BSL_TEST_DID_CHANGE_BLOCKING_PARSE_DELAY_MS"),
                force_reschedule_same_version: false,
                source: super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidChange,
                did_change_attribution: Some(DidChangeParseSnapshotAttributionV2 {
                    uri: work.uri.clone(),
                    base_text_source: work.parse_snapshot_base_text_source,
                    change_shape: work.parse_snapshot_change_shape,
                    content_changes_count: work.parse_snapshot_content_changes_count,
                    replay_order: work.parse_snapshot_replay_order,
                    base_document_version: work.parse_snapshot_base_document_version,
                    stale_parser_base: parse_snapshot_stale_parser_base,
                }),
            })
            .await;
        }
        if self
            .did_change_diagnostics_are_stale_after_same_version_save_v2(
                work.file_id,
                work.version,
                work.diagnostics_save_cycle_sequence_at_handoff,
                &work.updated_text,
            )
            .await
        {
            let latest_save_cycle_sequence = self
                .diagnostics_save_cycle_sequence_v2
                .read()
                .await
                .get(&work.file_id)
                .copied()
                .unwrap_or(0);
            debug!(
                uri = %work.uri,
                file_id = work.file_id.0,
                requested_version = work.version,
                save_cycle_sequence_at_handoff = work.diagnostics_save_cycle_sequence_at_handoff,
                latest_save_cycle_sequence,
                "skip stale didChange diagnostics scheduling after same-version didSave"
            );
            return;
        }
        self.schedule_type_index_precompute_v2(work.file_id, work.version)
            .await;
        if self
            .did_change_diagnostics_are_stale_after_same_version_save_v2(
                work.file_id,
                work.version,
                work.diagnostics_save_cycle_sequence_at_handoff,
                &work.updated_text,
            )
            .await
        {
            let latest_save_cycle_sequence = self
                .diagnostics_save_cycle_sequence_v2
                .read()
                .await
                .get(&work.file_id)
                .copied()
                .unwrap_or(0);
            debug!(
                uri = %work.uri,
                file_id = work.file_id.0,
                requested_version = work.version,
                save_cycle_sequence_at_handoff = work.diagnostics_save_cycle_sequence_at_handoff,
                latest_save_cycle_sequence,
                "skip stale didChange diagnostics scheduling after same-version didSave"
            );
            return;
        }

        let flow_sensitive_enabled = {
            let settings = self.settings.read().await;
            settings.enable_flow_sensitive
        };
        let diagnostics_generation = self.bump_diagnostics_generation_v2(work.file_id).await;
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
                    work.uri.clone(),
                    work.file_id,
                    work.version,
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
                        work.uri.clone(),
                        work.file_id,
                        work.version,
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
                        work.uri.clone(),
                        work.file_id,
                        work.version,
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
        self.publish_same_file_ingress_token_v2(
            file_id,
            version,
            super::super::SameFileIngressTokenSourceV2::DidOpen,
        )
        .await;
        self.schedule_completion_head_precompute_from_current_revision_v2(file_id, version)
            .await;
        self.schedule_background_parse_snapshot_apply_v2(BackgroundParseSnapshotApplyArgs {
            file_id,
            requested_version: version,
            save_cycle_sequence: None,
            path: path.clone(),
            text: text.clone(),
            cpu_work_class: bsl_runtime::application::CpuWorkClass::Background,
            parser_base_recovery_text: None,
            parser_base_recovery_reuse_parse_result: None,
            parser_edits: Vec::new(),
            forced_full_parse_reason: None,
            async_delay_mode: ParseSnapshotAsyncDelayMode::None,
            blocking_delay_env_key: Some("BSL_TEST_DID_OPEN_BLOCKING_PARSE_DELAY_MS"),
            force_reschedule_same_version: false,
            source: super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidOpen,
            did_change_attribution: None,
        })
        .await;
        let _ = self
            .promote_matching_background_parse_snapshot_apply_task_v2(
                file_id,
                version,
                Some(parse_snapshot_text_hash(text.as_ref())),
            )
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
        let parse_snapshot_replay_order = did_change_parse_snapshot_replay_order(&changes);
        let parse_snapshot_content_changes_count = changes.len();

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
        let path: Arc<str> = Arc::from(path);
        let Some(work) = self
            .apply_did_change_current_revision_fast_lane_v2(
                file_id,
                &uri,
                version,
                &changes,
                path,
                parse_snapshot_change_shape,
                parse_snapshot_replay_order,
                parse_snapshot_content_changes_count,
            )
            .await
        else {
            return;
        };
        self.publish_same_file_ingress_token_v2(
            file_id,
            version,
            super::super::SameFileIngressTokenSourceV2::DidChange,
        )
        .await;
        self.spawn_did_change_post_handoff_v2(work);
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
        let diagnostics_generation = self.bump_diagnostics_generation_v2(file_id).await;
        let save_cycle_sequence = self.bump_diagnostics_save_cycle_sequence_v2(file_id).await;
        self.cancel_type_index_precompute_v2(file_id).await;
        self.cleanup_stale_completed_type_index_precompute_task_v2(file_id, version)
            .await;
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
            let (
                save_parser_edits,
                save_parser_base_recovery_text,
                save_parser_base_recovery_reuse_parse_result,
                save_forced_full_parse_reason,
            ) = {
                let ready_state = self
                    .latest_ready_parse_snapshots_v2
                    .read()
                    .await
                    .get(&file_id)
                    .cloned()
                    .filter(|state| {
                        state.parse_snapshot.file_version < version
                            && state.text.as_ref() != text.as_ref()
                    });
                if let Some(ready_state) = ready_state {
                    let parser_edits =
                        whole_text_change_to_parser_edit(ready_state.text.as_ref(), text.as_ref())
                            .into_iter()
                            .collect::<Vec<_>>();
                    let can_reuse_ready_ast = ready_state.syntax_errors_complete
                        && !ready_state.parse_snapshot.parse_result.has_errors()
                        && !ready_state
                            .parse_snapshot
                            .parse_result
                            .program
                            .statements
                            .is_empty();
                    let tree_cache_matches_ready_text =
                        self.coordinator.parser_coordinator().is_some_and(|parser| {
                            parser.tree_cache_matches_source_for_file(
                                Path::new(path.as_str()),
                                ready_state.text.as_ref(),
                            )
                        });
                    let forced_full_parse_reason = if !parser_edits.is_empty()
                        && !tree_cache_matches_ready_text
                    {
                        Some(
                                bsl_runtime::system::parser_coordinator::ParserCoordinator::parse_snapshot_fallback_stale_parser_base_reason(),
                            )
                    } else {
                        None
                    };
                    (
                        parser_edits,
                        (!tree_cache_matches_ready_text).then(|| ready_state.text.clone()),
                        can_reuse_ready_ast
                            .then(|| ready_state.parse_snapshot.parse_result.clone()),
                        forced_full_parse_reason,
                    )
                } else {
                    (Vec::new(), None, None, None)
                }
            };
            // Save can be followed by an immediate outline refresh without a new version bump.
            // Coalesce identical same-version refresh behind the existing worker so save does
            // not restart the same cold/full parse for unchanged shadow text.
            self.schedule_background_parse_snapshot_apply_v2(BackgroundParseSnapshotApplyArgs {
                file_id,
                requested_version: version,
                save_cycle_sequence: Some(save_cycle_sequence),
                path: Arc::from(path),
                text,
                cpu_work_class: bsl_runtime::application::CpuWorkClass::Interactive,
                parser_base_recovery_text: save_parser_base_recovery_text,
                parser_base_recovery_reuse_parse_result:
                    save_parser_base_recovery_reuse_parse_result,
                parser_edits: save_parser_edits,
                forced_full_parse_reason: save_forced_full_parse_reason,
                async_delay_mode: ParseSnapshotAsyncDelayMode::DidSaveTestOnly,
                blocking_delay_env_key: Some("BSL_TEST_DID_SAVE_BLOCKING_PARSE_DELAY_MS"),
                force_reschedule_same_version: true,
                source: super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidSave,
                did_change_attribution: None,
            })
            .await;
        }
        if self
            .latest_current_revision_handoff_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied()
            == Some(version)
        {
            self.publish_same_file_ingress_token_v2(
                file_id,
                version,
                super::super::SameFileIngressTokenSourceV2::DidSave,
            )
            .await;
        }

        let flow_sensitive_enabled = {
            let settings = self.settings.read().await;
            settings.enable_flow_sensitive
        };
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
            if matches!(
                *profile,
                bsl_runtime::application::DiagnosticsProfile::SaveFastlane
            ) {
                self.run_diagnostics_save_profile_immediate_v2(
                    uri.clone(),
                    file_id,
                    version,
                    diagnostics_generation,
                    save_cycle_sequence,
                    *profile,
                )
                .await;
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
            let closing_version = {
                let current_revision_handoff_version = self
                    .latest_current_revision_handoff_versions_v2
                    .read()
                    .await
                    .get(&file_id)
                    .copied();
                if current_revision_handoff_version.is_some() {
                    current_revision_handoff_version
                } else {
                    self.latest_received_file_versions_v2
                        .read()
                        .await
                        .get(&file_id)
                        .copied()
                }
            };
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
            self.latest_detached_diagnostics_ready_artifacts_v2
                .write()
                .await
                .remove(&file_id);
            self.did_save_exact_producer_lifecycle_v2
                .write()
                .await
                .retain(|key, _| key.file_id != file_id);
            self.latest_snapshot_failures_v2
                .write()
                .await
                .remove(&file_id);
            self.latest_snapshot_status_v2
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
            if let Some(closing_version) = closing_version {
                self.publish_same_file_ingress_token_v2(
                    file_id,
                    closing_version,
                    super::super::SameFileIngressTokenSourceV2::DidClose,
                )
                .await;
            }
            self.clear_same_file_ingress_token_v2(file_id).await;
            self.file_id_to_uri_v2.write().await.remove(&file_id);
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

#[cfg(test)]
mod refactor59_same_version_seed_tests {
    use super::*;

    fn parse_snapshot_for_seed_test(
        file_id: bsl_analysis_v2::FileId,
        version: i32,
        text: &str,
    ) -> bsl_analysis_v2::ParseSnapshot {
        let parse_result =
            Arc::new(bsl_syntax::parse(text, &bsl_syntax::ParseOptions::default()).expect("parse"));
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_bsl::LANGUAGE.into())
            .expect("tree-sitter-bsl language");
        let backend_tree = Arc::new(parser.parse(text, None).expect("tree-sitter parse"));

        bsl_analysis_v2::ParseSnapshot {
            file_id,
            file_version: version,
            parse_result,
            line_index: Arc::new(bsl_line_index::LineIndex::new(text)),
            backend_tree,
            changed_ranges: Arc::new(Vec::new()),
            produced_at_millis: 0,
            backend_tree_hash: 0,
            incremental: true,
            fallback_reason: None,
        }
    }

    #[test]
    fn same_version_same_text_ready_snapshot_can_seed_didsave_rebuild() {
        let text: Arc<str> = Arc::from("Процедура Test()\n    Сообщить(\"ok\");\nКонецПроцедуры\n");
        let parse_snapshot =
            parse_snapshot_for_seed_test(bsl_analysis_v2::FileId(59), 15, text.as_ref());
        let parse_result = parse_snapshot.parse_result.clone();
        let ready_state = ReadyParseSnapshotStateV2 {
            text: text.clone(),
            parse_snapshot,
            source: crate::server::BackgroundParseSnapshotApplyTaskSourceV2::DidChange,
            syntax_errors_complete: true,
            phase_attribution: crate::server::ReadyParseSnapshotPhaseAttributionV2::default(),
            program_lowering_summary: None,
        };

        let seed = derive_same_version_rebuild_previous_ready_seed_v2(&ready_state, 15, &text)
            .expect("same-version same-text ready snapshot should seed didSave rebuild");

        assert!(Arc::ptr_eq(&seed.parse_result, &parse_result));
        assert_eq!(seed.source_text.as_ref(), text.as_ref());
    }

    #[test]
    fn terminal_didsave_lifecycle_states_map_to_lowering_reuse_seed_evictions() {
        use crate::server::DidSaveExactProducerLifecycleStateV2 as State;
        use bsl_runtime::system::parser_coordinator::ParseSnapshotProgramLoweringReuseSeedEvictionReason as Reason;

        assert_eq!(
            lowering_reuse_seed_eviction_reason_for_did_save_lifecycle_state_v2(State::Admitted),
            None
        );
        assert_eq!(
            lowering_reuse_seed_eviction_reason_for_did_save_lifecycle_state_v2(State::Started),
            None
        );
        assert_eq!(
            lowering_reuse_seed_eviction_reason_for_did_save_lifecycle_state_v2(
                State::DetachedDiagnosticsReadyPublished
            ),
            Some(Reason::TerminalCleanup)
        );
        assert_eq!(
            lowering_reuse_seed_eviction_reason_for_did_save_lifecycle_state_v2(
                State::FullyMaterialized
            ),
            Some(Reason::TerminalCleanup)
        );
        assert_eq!(
            lowering_reuse_seed_eviction_reason_for_did_save_lifecycle_state_v2(
                State::ExactTypeIndexDeadline
            ),
            Some(Reason::TerminalCleanup)
        );
        assert_eq!(
            lowering_reuse_seed_eviction_reason_for_did_save_lifecycle_state_v2(State::Superseded),
            Some(Reason::Superseded)
        );
        assert_eq!(
            lowering_reuse_seed_eviction_reason_for_did_save_lifecycle_state_v2(State::Cancelled),
            Some(Reason::Cancelled)
        );
        assert_eq!(
            lowering_reuse_seed_eviction_reason_for_did_save_lifecycle_state_v2(State::Failed),
            Some(Reason::Failed)
        );
        assert_eq!(
            lowering_reuse_seed_eviction_reason_for_did_save_lifecycle_state_v2(
                State::ContinuityLost
            ),
            Some(Reason::ContinuityLost)
        );
    }
}
