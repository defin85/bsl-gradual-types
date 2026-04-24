use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use super::facade::SemanticOperation;
use crate::system::{global_runtime_config, RuntimeKey, SystemCoordinator};
use tokio::sync::{Notify, Semaphore};

#[derive(Debug, Clone, Copy, Default)]
pub struct RuntimePerfKnobs {
    pub slow_wait_warn_threshold: Option<Duration>,
    pub slow_snapshot_warn_threshold: Option<Duration>,
    pub slow_query_warn_threshold: Option<Duration>,
    pub slow_client_log_threshold: Option<Duration>,
}

#[derive(Debug, Clone, Copy)]
pub struct InteractiveFreshnessKnobs {
    pub wait_budget: Duration,
}

impl InteractiveFreshnessKnobs {
    pub fn from_runtime_config(observability: Option<&SystemCoordinator>) -> Self {
        let (wait_budget, wait_budget_clamped) = read_clamped_duration(
            RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs,
            Duration::from_millis(120),
            Duration::from_millis(10),
            Duration::from_millis(2000),
        );
        if wait_budget_clamped {
            if let Some(coordinator) = observability {
                coordinator.record_intellisense_v2_interactive_knob_clamped();
            }
        }
        Self { wait_budget }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletionFastpathPreconditions {
    pub operation_is_completion: bool,
    pub large_churn_active: bool,
    pub has_min_file_version: bool,
    pub has_expected_deps: bool,
    pub has_interactive_knobs: bool,
}

impl CompletionFastpathPreconditions {
    pub fn can_attempt_bounded_stale_fallback(self) -> bool {
        let _ = self;
        false
    }

    pub fn churn_aware_fastpath_active(self) -> bool {
        let _ = self;
        false
    }
}

pub fn completion_fastpath_preconditions(
    operation: SemanticOperation,
    large_churn_active: bool,
    min_file_version: Option<i32>,
    expected_deps_present: bool,
    interactive_knobs_present: bool,
) -> CompletionFastpathPreconditions {
    CompletionFastpathPreconditions {
        operation_is_completion: matches!(operation, SemanticOperation::Completion),
        large_churn_active,
        has_min_file_version: min_file_version.is_some(),
        has_expected_deps: expected_deps_present,
        has_interactive_knobs: interactive_knobs_present,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionMode {
    Off,
    Shadow,
    Canary,
    On,
}

impl CompletionMode {
    fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "shadow" => Self::Shadow,
            "canary" => Self::Canary,
            "on" => Self::On,
            _ => Self::Off,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CompletionPipelineKnobs {
    pub mode: CompletionMode,
    pub canary_percent: u8,
    pub queue_capacity: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct ScaleAwareDiagnosticsKnobs {
    pub enabled: bool,
    pub large_doc_bytes: usize,
    pub large_doc_lines: usize,
    pub churn_window: Duration,
    pub churn_min_changes: u32,
}

impl ScaleAwareDiagnosticsKnobs {
    pub fn from_runtime_config() -> Self {
        let enabled = global_runtime_config()
            .get_bool(RuntimeKey::IntellisenseV2ScaleAwarePolicyEnabled)
            .unwrap_or(true);
        let (large_doc_bytes, _) = read_clamped_usize(
            RuntimeKey::IntellisenseV2ScaleAwareLargeDocBytes,
            64 * 1024,
            1024,
            10 * 1024 * 1024,
        );
        let (large_doc_lines, _) = read_clamped_usize(
            RuntimeKey::IntellisenseV2ScaleAwareLargeDocLines,
            2_000,
            50,
            100_000,
        );
        let (churn_window, _) = read_clamped_duration(
            RuntimeKey::IntellisenseV2ScaleAwareChurnWindowMs,
            Duration::from_millis(1_500),
            Duration::from_millis(100),
            Duration::from_millis(10_000),
        );
        let (churn_min_changes, _) = read_clamped_u32(
            RuntimeKey::IntellisenseV2ScaleAwareChurnMinChanges,
            6,
            2,
            200,
        );

        Self {
            enabled,
            large_doc_bytes,
            large_doc_lines,
            churn_window,
            churn_min_changes,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeferredHeavyDiagnosticsReason {
    LargeAndChurn,
}

impl DeferredHeavyDiagnosticsReason {
    pub fn as_str(self) -> &'static str {
        match self {
            DeferredHeavyDiagnosticsReason::LargeAndChurn => "large_and_churn",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionMissingIrPolicyDecision {
    FailClosedUnavailable,
}

pub fn completion_missing_ir_policy_decision(
    _has_strict_cached_items: bool,
    _member_access_context: bool,
    _degraded_available: bool,
    _has_relaxed_cached_items: bool,
) -> CompletionMissingIrPolicyDecision {
    CompletionMissingIrPolicyDecision::FailClosedUnavailable
}

impl CompletionPipelineKnobs {
    pub fn from_runtime_config() -> Self {
        let mode = global_runtime_config()
            .get_string(RuntimeKey::IntellisenseV2CompletionMode)
            .map(|value| CompletionMode::parse(&value))
            .unwrap_or(CompletionMode::On);
        let (canary_percent, _) =
            read_clamped_u8(RuntimeKey::IntellisenseV2CompletionCanaryPercent, 0, 0, 100);
        let (queue_capacity, _) = read_clamped_usize(
            RuntimeKey::IntellisenseV2CompletionQueueCapacity,
            256,
            16,
            4096,
        );
        Self {
            mode,
            canary_percent,
            queue_capacity,
        }
    }
}

impl RuntimePerfKnobs {
    pub fn from_runtime_config() -> Self {
        Self {
            slow_wait_warn_threshold: read_duration(RuntimeKey::IntellisenseV2SlowWaitWarnMs),
            slow_snapshot_warn_threshold: read_duration(
                RuntimeKey::IntellisenseV2SlowSnapshotWarnMs,
            ),
            slow_query_warn_threshold: read_duration(RuntimeKey::IntellisenseV2SlowQueryWarnMs),
            slow_client_log_threshold: read_duration(RuntimeKey::IntellisenseV2SlowClientLogMs),
        }
    }
}

pub fn scale_aware_document_is_large(text: &str, knobs: ScaleAwareDiagnosticsKnobs) -> bool {
    text.len() >= knobs.large_doc_bytes || text.lines().count() >= knobs.large_doc_lines
}

fn read_duration(key: RuntimeKey) -> Option<Duration> {
    global_runtime_config()
        .get_u64(key)
        .map(Duration::from_millis)
}

fn read_clamped_duration(
    key: RuntimeKey,
    default: Duration,
    min: Duration,
    max: Duration,
) -> (Duration, bool) {
    let raw = global_runtime_config()
        .get_u64(key)
        .map(Duration::from_millis)
        .unwrap_or(default);
    if raw < min {
        (min, true)
    } else if raw > max {
        (max, true)
    } else {
        (raw, false)
    }
}

fn read_clamped_u8(key: RuntimeKey, default: u8, min: u8, max: u8) -> (u8, bool) {
    let raw = global_runtime_config()
        .get_u64(key)
        .unwrap_or(default as u64);
    let raw_u8 = if raw > u8::MAX as u64 {
        u8::MAX
    } else {
        raw as u8
    };
    let clamped = raw_u8.clamp(min, max);
    (clamped, clamped != raw_u8)
}

fn read_clamped_u32(key: RuntimeKey, default: u32, min: u32, max: u32) -> (u32, bool) {
    let raw = global_runtime_config()
        .get_u64(key)
        .unwrap_or(default as u64);
    let raw_u32 = if raw > u32::MAX as u64 {
        u32::MAX
    } else {
        raw as u32
    };
    let clamped = raw_u32.clamp(min, max);
    (clamped, clamped != raw_u32)
}

fn read_clamped_usize(key: RuntimeKey, default: usize, min: usize, max: usize) -> (usize, bool) {
    let raw = global_runtime_config().get_usize(key).unwrap_or(default);
    let clamped = raw.clamp(min, max);
    (clamped, clamped != raw)
}

pub fn interactive_freshness_knobs(
    operation: SemanticOperation,
    observability: Option<&SystemCoordinator>,
) -> Option<InteractiveFreshnessKnobs> {
    match operation {
        SemanticOperation::Completion
        | SemanticOperation::Hover
        | SemanticOperation::SignatureHelp
        | SemanticOperation::Definition => Some(InteractiveFreshnessKnobs::from_runtime_config(
            observability,
        )),
        _ => None,
    }
}

pub fn should_query_parse_result(operation: SemanticOperation, ir_available: bool) -> bool {
    match operation {
        SemanticOperation::Completion | SemanticOperation::Members => ir_available,
        SemanticOperation::DocumentSymbol
        | SemanticOperation::Rename
        | SemanticOperation::SymbolSearch
        | SemanticOperation::References
        | SemanticOperation::Diagnostics => true,
        SemanticOperation::Hover
        | SemanticOperation::SignatureHelp
        | SemanticOperation::Definition
        | SemanticOperation::TypeAtPosition => false,
    }
}

pub fn classify_optional_query<T, E>(result: &Result<Option<T>, E>) -> super::SemanticOutcome {
    match result {
        Ok(Some(_)) => super::SemanticOutcome::Success,
        Ok(None) => super::SemanticOutcome::Empty,
        Err(_) => super::SemanticOutcome::Cancelled,
    }
}

static CPU_BOUND_SEMAPHORE: OnceLock<Arc<Semaphore>> = OnceLock::new();

fn cpu_bound_semaphore() -> Arc<Semaphore> {
    CPU_BOUND_SEMAPHORE
        .get_or_init(|| {
            let permits = std::thread::available_parallelism()
                .map(|parallelism| parallelism.get().max(2))
                .unwrap_or(4);
            Arc::new(Semaphore::new(permits))
        })
        .clone()
}

#[cfg(debug_assertions)]
fn background_reserved_only_for_test() -> bool {
    std::env::var_os("BSL_TEST_RUNTIME_BACKGROUND_RESERVED_ONLY").is_some()
}

#[cfg(not(debug_assertions))]
fn background_reserved_only_for_test() -> bool {
    false
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuWorkClass {
    Interactive,
    Background,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdmissionLane {
    DidSaveFollowup,
}

impl AdmissionLane {
    pub fn as_str(self) -> &'static str {
        match self {
            AdmissionLane::DidSaveFollowup => "did_save_followup",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticsTrigger {
    DidChange,
    DidOpen,
    DidSave,
    Idle,
    DocumentsSet,
    JobStart,
}

impl DiagnosticsTrigger {
    pub fn as_str(self) -> &'static str {
        match self {
            DiagnosticsTrigger::DidChange => "did_change",
            DiagnosticsTrigger::DidOpen => "did_open",
            DiagnosticsTrigger::DidSave => "did_save",
            DiagnosticsTrigger::Idle => "idle",
            DiagnosticsTrigger::DocumentsSet => "documents_set",
            DiagnosticsTrigger::JobStart => "job_start",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticsProfile {
    Fast,
    DebouncedFull,
    SaveFastlane,
    IdleHeavy,
}

impl DiagnosticsProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            DiagnosticsProfile::Fast => "fast",
            DiagnosticsProfile::DebouncedFull => "debounced_full",
            DiagnosticsProfile::SaveFastlane => "save_fastlane",
            DiagnosticsProfile::IdleHeavy => "idle_heavy",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticsDisposition {
    Published,
    SupersededVersion,
    SupersededGeneration,
    ClientCancel,
    OtherCancel,
    DisabledByConfig,
}

impl DiagnosticsDisposition {
    pub fn as_str(self) -> &'static str {
        match self {
            DiagnosticsDisposition::Published => "published",
            DiagnosticsDisposition::SupersededVersion => "superseded_version",
            DiagnosticsDisposition::SupersededGeneration => "superseded_generation",
            DiagnosticsDisposition::ClientCancel => "client_cancel",
            DiagnosticsDisposition::OtherCancel => "other_cancel",
            DiagnosticsDisposition::DisabledByConfig => "disabled_by_config",
        }
    }
}

pub fn did_save_followup_lane_quota() -> usize {
    let (quota, _) = read_clamped_usize(
        RuntimeKey::IntellisenseV2DidSaveFollowupLaneQuota,
        1,
        0,
        1024,
    );
    quota
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticsExecutionPlan {
    pub run_syntax: bool,
    pub run_semantic: bool,
    pub flow_sensitive_semantic: bool,
    pub cpu_class: CpuWorkClass,
}

const PROFILES_DID_CHANGE: &[DiagnosticsProfile] = &[
    DiagnosticsProfile::Fast,
    DiagnosticsProfile::DebouncedFull,
    DiagnosticsProfile::IdleHeavy,
];
const PROFILES_DID_OPEN: &[DiagnosticsProfile] = &[DiagnosticsProfile::DebouncedFull];
const PROFILES_DID_SAVE: &[DiagnosticsProfile] = &[
    DiagnosticsProfile::SaveFastlane,
    DiagnosticsProfile::IdleHeavy,
];
const PROFILES_IDLE: &[DiagnosticsProfile] = &[DiagnosticsProfile::IdleHeavy];
const PROFILES_DOCUMENTS_SET: &[DiagnosticsProfile] = &[DiagnosticsProfile::DebouncedFull];
const PROFILES_JOB_START: &[DiagnosticsProfile] = &[DiagnosticsProfile::DebouncedFull];

pub fn diagnostics_profiles_for_trigger(
    trigger: DiagnosticsTrigger,
) -> &'static [DiagnosticsProfile] {
    match trigger {
        DiagnosticsTrigger::DidChange => PROFILES_DID_CHANGE,
        DiagnosticsTrigger::DidOpen => PROFILES_DID_OPEN,
        DiagnosticsTrigger::DidSave => PROFILES_DID_SAVE,
        DiagnosticsTrigger::Idle => PROFILES_IDLE,
        DiagnosticsTrigger::DocumentsSet => PROFILES_DOCUMENTS_SET,
        DiagnosticsTrigger::JobStart => PROFILES_JOB_START,
    }
}

pub fn diagnostics_execution_plan(
    profile: DiagnosticsProfile,
    flow_sensitive_enabled: bool,
) -> DiagnosticsExecutionPlan {
    match profile {
        DiagnosticsProfile::Fast => DiagnosticsExecutionPlan {
            run_syntax: false,
            run_semantic: false,
            flow_sensitive_semantic: false,
            cpu_class: CpuWorkClass::Interactive,
        },
        DiagnosticsProfile::DebouncedFull => DiagnosticsExecutionPlan {
            run_syntax: true,
            run_semantic: true,
            flow_sensitive_semantic: false,
            cpu_class: CpuWorkClass::Background,
        },
        DiagnosticsProfile::SaveFastlane => DiagnosticsExecutionPlan {
            run_syntax: true,
            run_semantic: false,
            flow_sensitive_semantic: false,
            cpu_class: CpuWorkClass::Interactive,
        },
        DiagnosticsProfile::IdleHeavy => DiagnosticsExecutionPlan {
            run_syntax: true,
            run_semantic: true,
            flow_sensitive_semantic: flow_sensitive_enabled,
            cpu_class: CpuWorkClass::Background,
        },
    }
}

pub fn cpu_work_class_for_operation(operation: SemanticOperation) -> CpuWorkClass {
    match operation {
        SemanticOperation::Completion
        | SemanticOperation::Hover
        | SemanticOperation::SignatureHelp
        | SemanticOperation::Members
        | SemanticOperation::TypeAtPosition
        | SemanticOperation::Definition => CpuWorkClass::Interactive,
        _ => CpuWorkClass::Background,
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct CpuBudgetSaturationSnapshot {
    interactive_waiters: usize,
    background_waiters: usize,
    did_save_followup_waiters: usize,
    interactive_permits: usize,
    background_permits: usize,
    did_save_followup_permits: usize,
    shared_permits: usize,
}

impl CpuBudgetSaturationSnapshot {
    fn queue_depth_total(self) -> usize {
        self.interactive_waiters + self.background_waiters
    }
}

struct CpuBoundBudget {
    interactive_reserved: Arc<Semaphore>,
    background_reserved: Arc<Semaphore>,
    did_save_followup_reserved: Arc<Semaphore>,
    did_save_followup_reserved_capacity: usize,
    shared: Arc<Semaphore>,
    interactive_waiters: AtomicUsize,
    background_waiters: AtomicUsize,
    did_save_followup_waiters: AtomicUsize,
}

impl CpuBoundBudget {
    fn with_total_permits(permits: usize) -> Self {
        let permits = permits.max(2);
        let did_save_followup_reserved_permits = usize::from(permits >= 4);
        // Keep one dedicated background permit and reserve an extra interactive
        // permit once capacity still leaves room for the save-critical tier.
        let interactive_reserved_permits = if permits >= 5 { 2 } else { 1 };
        let background_reserved_permits = 1;
        let shared_permits = permits.saturating_sub(
            interactive_reserved_permits
                + background_reserved_permits
                + did_save_followup_reserved_permits,
        );
        Self {
            interactive_reserved: Arc::new(Semaphore::new(interactive_reserved_permits)),
            background_reserved: Arc::new(Semaphore::new(background_reserved_permits)),
            did_save_followup_reserved: Arc::new(Semaphore::new(
                did_save_followup_reserved_permits,
            )),
            did_save_followup_reserved_capacity: did_save_followup_reserved_permits,
            shared: Arc::new(Semaphore::new(shared_permits)),
            interactive_waiters: AtomicUsize::new(0),
            background_waiters: AtomicUsize::new(0),
            did_save_followup_waiters: AtomicUsize::new(0),
        }
    }

    fn new() -> Self {
        let permits = std::thread::available_parallelism()
            .map(|parallelism| parallelism.get().max(2))
            .unwrap_or(4);
        Self::with_total_permits(permits)
    }

    #[allow(dead_code)]
    async fn acquire(&self, class: CpuWorkClass) -> tokio::sync::OwnedSemaphorePermit {
        self.acquire_with_queue_wait_hook::<fn()>(class, None).await
    }

    async fn acquire_with_queue_wait_hook<Q>(
        &self,
        class: CpuWorkClass,
        on_queue_wait_started: Option<Q>,
    ) -> tokio::sync::OwnedSemaphorePermit
    where
        Q: FnOnce(),
    {
        self.acquire_with_lane_queue_wait_hook(class, None, on_queue_wait_started)
            .await
    }

    async fn acquire_with_lane_queue_wait_hook<Q>(
        &self,
        class: CpuWorkClass,
        lane: Option<AdmissionLane>,
        on_queue_wait_started: Option<Q>,
    ) -> tokio::sync::OwnedSemaphorePermit
    where
        Q: FnOnce(),
    {
        struct WaiterCountGuard<'a> {
            counter: &'a AtomicUsize,
        }

        impl<'a> WaiterCountGuard<'a> {
            fn new(counter: &'a AtomicUsize) -> Self {
                counter.fetch_add(1, Ordering::AcqRel);
                Self { counter }
            }
        }

        impl Drop for WaiterCountGuard<'_> {
            fn drop(&mut self) {
                self.counter.fetch_sub(1, Ordering::AcqRel);
            }
        }

        let is_did_save_followup = matches!(lane, Some(AdmissionLane::DidSaveFollowup));
        let (own_reserved, other_reserved, own_waiters, other_waiters) = match class {
            CpuWorkClass::Interactive => (
                self.interactive_reserved.clone(),
                self.background_reserved.clone(),
                &self.interactive_waiters,
                &self.background_waiters,
            ),
            CpuWorkClass::Background => (
                self.background_reserved.clone(),
                self.interactive_reserved.clone(),
                &self.background_waiters,
                &self.interactive_waiters,
            ),
        };

        let own_waiter_guard = WaiterCountGuard::new(own_waiters);
        let lane_waiter_guard =
            is_did_save_followup.then(|| WaiterCountGuard::new(&self.did_save_followup_waiters));
        let other_has_waiters = if is_did_save_followup {
            self.interactive_waiters
                .load(Ordering::Acquire)
                .saturating_add(self.background_waiters.load(Ordering::Acquire))
                > 1
        } else {
            other_waiters.load(Ordering::Acquire) > 0
        };
        let competing_did_save_waiters = self.did_save_followup_waiters.load(Ordering::Acquire)
            > usize::from(is_did_save_followup);
        let background_reserved_only = matches!(class, CpuWorkClass::Background)
            && !is_did_save_followup
            && background_reserved_only_for_test();
        let can_borrow = !other_has_waiters && !background_reserved_only && !is_did_save_followup;
        // Keep shared permits biased toward interactive work when interactive queue is non-empty.
        let can_take_shared = match class {
            CpuWorkClass::Interactive => !background_reserved_only,
            CpuWorkClass::Background if is_did_save_followup => !other_has_waiters,
            CpuWorkClass::Background => {
                !background_reserved_only && !other_has_waiters && !competing_did_save_waiters
            }
        };
        let mut on_queue_wait_started = on_queue_wait_started;
        let mut mark_queue_wait_started = || {
            if let Some(callback) = on_queue_wait_started.take() {
                callback();
            }
        };

        let permit = if is_did_save_followup {
            let lane_reserved = self.did_save_followup_reserved.clone();
            if self.did_save_followup_reserved_capacity > 0 {
                if let Ok(permit) = lane_reserved.clone().try_acquire_owned() {
                    permit
                } else if can_take_shared {
                    if let Ok(permit) = self.shared.clone().try_acquire_owned() {
                        permit
                    } else {
                        mark_queue_wait_started();
                        tokio::select! {
                            permit = self.shared.clone().acquire_owned() => permit.expect("shared semaphore closed"),
                            permit = lane_reserved.clone().acquire_owned() => permit.expect("didSave follow-up reserved semaphore closed"),
                        }
                    }
                } else {
                    mark_queue_wait_started();
                    lane_reserved
                        .clone()
                        .acquire_owned()
                        .await
                        .expect("didSave follow-up reserved semaphore closed")
                }
            } else if can_take_shared {
                if let Ok(permit) = self.shared.clone().try_acquire_owned() {
                    permit
                } else if let Ok(permit) = own_reserved.clone().try_acquire_owned() {
                    permit
                } else {
                    mark_queue_wait_started();
                    tokio::select! {
                        permit = self.shared.clone().acquire_owned() => permit.expect("shared semaphore closed"),
                        permit = own_reserved.clone().acquire_owned() => permit.expect("interactive/background reserved semaphore closed"),
                    }
                }
            } else if let Ok(permit) = own_reserved.clone().try_acquire_owned() {
                permit
            } else {
                mark_queue_wait_started();
                own_reserved
                    .clone()
                    .acquire_owned()
                    .await
                    .expect("interactive/background reserved semaphore closed")
            }
        } else {
            if let Ok(permit) = own_reserved.clone().try_acquire_owned() {
                permit
            } else if can_take_shared {
                if let Ok(permit) = self.shared.clone().try_acquire_owned() {
                    permit
                } else if can_borrow {
                    if let Ok(permit) = other_reserved.clone().try_acquire_owned() {
                        permit
                    } else {
                        mark_queue_wait_started();
                        tokio::select! {
                            permit = own_reserved.clone().acquire_owned() => permit.expect("interactive/background reserved semaphore closed"),
                            permit = self.shared.clone().acquire_owned() => permit.expect("shared semaphore closed"),
                            permit = other_reserved.clone().acquire_owned() => permit.expect("borrowed semaphore closed"),
                        }
                    }
                } else {
                    mark_queue_wait_started();
                    tokio::select! {
                        permit = own_reserved.clone().acquire_owned() => permit.expect("interactive/background reserved semaphore closed"),
                        permit = self.shared.clone().acquire_owned() => permit.expect("shared semaphore closed"),
                    }
                }
            } else if can_borrow {
                if let Ok(permit) = other_reserved.clone().try_acquire_owned() {
                    permit
                } else {
                    mark_queue_wait_started();
                    tokio::select! {
                        permit = own_reserved.clone().acquire_owned() => permit.expect("interactive/background reserved semaphore closed"),
                        permit = other_reserved.clone().acquire_owned() => permit.expect("borrowed semaphore closed"),
                    }
                }
            } else {
                mark_queue_wait_started();
                own_reserved
                    .clone()
                    .acquire_owned()
                    .await
                    .expect("interactive/background reserved semaphore closed")
            }
        };
        drop(lane_waiter_guard);
        drop(own_waiter_guard);
        permit
    }

    async fn acquire_with_dynamic_lane_queue_wait_hook<L, Q>(
        &self,
        class: CpuWorkClass,
        current_lane: L,
        lane_change_notify: &Notify,
        on_queue_wait_started: Option<Q>,
    ) -> (tokio::sync::OwnedSemaphorePermit, Option<AdmissionLane>)
    where
        L: Fn() -> Option<AdmissionLane>,
        Q: FnOnce(),
    {
        let mut on_queue_wait_started = on_queue_wait_started;
        loop {
            let lane = current_lane();
            let permit = match self
                .acquire_with_lane_queue_wait_hook_until_notified(
                    class,
                    lane,
                    lane_change_notify,
                    &mut on_queue_wait_started,
                )
                .await
            {
                AcquireWithLaneQueueWaitOutcome::Acquired(permit) => permit,
                AcquireWithLaneQueueWaitOutcome::Retry => continue,
            };
            if current_lane() != lane {
                drop(permit);
                continue;
            }
            return (permit, lane);
        }
    }

    async fn acquire_with_lane_queue_wait_hook_until_notified<Q>(
        &self,
        class: CpuWorkClass,
        lane: Option<AdmissionLane>,
        lane_change_notify: &Notify,
        on_queue_wait_started: &mut Option<Q>,
    ) -> AcquireWithLaneQueueWaitOutcome
    where
        Q: FnOnce(),
    {
        struct WaiterCountGuard<'a> {
            counter: &'a AtomicUsize,
        }

        impl<'a> WaiterCountGuard<'a> {
            fn new(counter: &'a AtomicUsize) -> Self {
                counter.fetch_add(1, Ordering::AcqRel);
                Self { counter }
            }
        }

        impl Drop for WaiterCountGuard<'_> {
            fn drop(&mut self) {
                self.counter.fetch_sub(1, Ordering::AcqRel);
            }
        }

        let is_did_save_followup = matches!(lane, Some(AdmissionLane::DidSaveFollowup));
        let (own_reserved, other_reserved, own_waiters, other_waiters) = match class {
            CpuWorkClass::Interactive => (
                self.interactive_reserved.clone(),
                self.background_reserved.clone(),
                &self.interactive_waiters,
                &self.background_waiters,
            ),
            CpuWorkClass::Background => (
                self.background_reserved.clone(),
                self.interactive_reserved.clone(),
                &self.background_waiters,
                &self.interactive_waiters,
            ),
        };

        let own_waiter_guard = WaiterCountGuard::new(own_waiters);
        let lane_waiter_guard =
            is_did_save_followup.then(|| WaiterCountGuard::new(&self.did_save_followup_waiters));
        let other_has_waiters = if is_did_save_followup {
            self.interactive_waiters
                .load(Ordering::Acquire)
                .saturating_add(self.background_waiters.load(Ordering::Acquire))
                > 1
        } else {
            other_waiters.load(Ordering::Acquire) > 0
        };
        let competing_did_save_waiters = self.did_save_followup_waiters.load(Ordering::Acquire)
            > usize::from(is_did_save_followup);
        let background_reserved_only = matches!(class, CpuWorkClass::Background)
            && !is_did_save_followup
            && background_reserved_only_for_test();
        let can_borrow = !other_has_waiters && !background_reserved_only && !is_did_save_followup;
        let can_take_shared = match class {
            CpuWorkClass::Interactive => !background_reserved_only,
            CpuWorkClass::Background if is_did_save_followup => !other_has_waiters,
            CpuWorkClass::Background => {
                !background_reserved_only && !other_has_waiters && !competing_did_save_waiters
            }
        };

        if is_did_save_followup {
            let lane_reserved = self.did_save_followup_reserved.clone();
            let permit = if self.did_save_followup_reserved_capacity > 0 {
                if let Ok(permit) = lane_reserved.clone().try_acquire_owned() {
                    permit
                } else if can_take_shared {
                    if let Ok(permit) = self.shared.clone().try_acquire_owned() {
                        permit
                    } else {
                        mark_queue_wait_started(on_queue_wait_started);
                        let shared = self.shared.clone().acquire_owned();
                        let own = lane_reserved.clone().acquire_owned();
                        let lane_changed = lane_change_notify.notified();
                        tokio::pin!(shared);
                        tokio::pin!(own);
                        tokio::pin!(lane_changed);
                        tokio::select! {
                            _ = &mut lane_changed => {
                                drop(lane_waiter_guard);
                                drop(own_waiter_guard);
                                return AcquireWithLaneQueueWaitOutcome::Retry;
                            }
                            permit = &mut shared => permit.expect("shared semaphore closed"),
                            permit = &mut own => permit.expect("didSave follow-up reserved semaphore closed"),
                        }
                    }
                } else {
                    mark_queue_wait_started(on_queue_wait_started);
                    let own = lane_reserved.clone().acquire_owned();
                    let lane_changed = lane_change_notify.notified();
                    tokio::pin!(own);
                    tokio::pin!(lane_changed);
                    tokio::select! {
                        _ = &mut lane_changed => {
                            drop(lane_waiter_guard);
                            drop(own_waiter_guard);
                            return AcquireWithLaneQueueWaitOutcome::Retry;
                        }
                        permit = &mut own => permit.expect("didSave follow-up reserved semaphore closed"),
                    }
                }
            } else if can_take_shared {
                if let Ok(permit) = self.shared.clone().try_acquire_owned() {
                    permit
                } else if let Ok(permit) = own_reserved.clone().try_acquire_owned() {
                    permit
                } else {
                    mark_queue_wait_started(on_queue_wait_started);
                    let shared = self.shared.clone().acquire_owned();
                    let own = own_reserved.clone().acquire_owned();
                    let lane_changed = lane_change_notify.notified();
                    tokio::pin!(shared);
                    tokio::pin!(own);
                    tokio::pin!(lane_changed);
                    tokio::select! {
                        _ = &mut lane_changed => {
                            drop(lane_waiter_guard);
                            drop(own_waiter_guard);
                            return AcquireWithLaneQueueWaitOutcome::Retry;
                        }
                        permit = &mut shared => permit.expect("shared semaphore closed"),
                        permit = &mut own => permit.expect("interactive/background reserved semaphore closed"),
                    }
                }
            } else if let Ok(permit) = own_reserved.clone().try_acquire_owned() {
                permit
            } else {
                mark_queue_wait_started(on_queue_wait_started);
                let own = own_reserved.clone().acquire_owned();
                let lane_changed = lane_change_notify.notified();
                tokio::pin!(own);
                tokio::pin!(lane_changed);
                tokio::select! {
                    _ = &mut lane_changed => {
                        drop(lane_waiter_guard);
                        drop(own_waiter_guard);
                            return AcquireWithLaneQueueWaitOutcome::Retry;
                        }
                    permit = &mut own => permit.expect("interactive/background reserved semaphore closed"),
                }
            };
            drop(lane_waiter_guard);
            drop(own_waiter_guard);
            return AcquireWithLaneQueueWaitOutcome::Acquired(permit);
        }

        let permit = if let Ok(permit) = own_reserved.clone().try_acquire_owned() {
            permit
        } else if can_take_shared {
            if let Ok(permit) = self.shared.clone().try_acquire_owned() {
                permit
            } else if can_borrow {
                if let Ok(permit) = other_reserved.clone().try_acquire_owned() {
                    permit
                } else {
                    mark_queue_wait_started(on_queue_wait_started);
                    let own = own_reserved.clone().acquire_owned();
                    let shared = self.shared.clone().acquire_owned();
                    let other = other_reserved.clone().acquire_owned();
                    let lane_changed = lane_change_notify.notified();
                    tokio::pin!(own);
                    tokio::pin!(shared);
                    tokio::pin!(other);
                    tokio::pin!(lane_changed);
                    tokio::select! {
                        _ = &mut lane_changed => {
                            drop(lane_waiter_guard);
                            drop(own_waiter_guard);
                            return AcquireWithLaneQueueWaitOutcome::Retry;
                        }
                        permit = &mut own => permit.expect("interactive/background reserved semaphore closed"),
                        permit = &mut shared => permit.expect("shared semaphore closed"),
                        permit = &mut other => permit.expect("borrowed semaphore closed"),
                    }
                }
            } else {
                mark_queue_wait_started(on_queue_wait_started);
                let own = own_reserved.clone().acquire_owned();
                let shared = self.shared.clone().acquire_owned();
                let lane_changed = lane_change_notify.notified();
                tokio::pin!(own);
                tokio::pin!(shared);
                tokio::pin!(lane_changed);
                tokio::select! {
                    _ = &mut lane_changed => {
                        drop(lane_waiter_guard);
                        drop(own_waiter_guard);
                        return AcquireWithLaneQueueWaitOutcome::Retry;
                    }
                    permit = &mut own => permit.expect("interactive/background reserved semaphore closed"),
                    permit = &mut shared => permit.expect("shared semaphore closed"),
                }
            }
        } else if can_borrow {
            if let Ok(permit) = other_reserved.clone().try_acquire_owned() {
                permit
            } else {
                mark_queue_wait_started(on_queue_wait_started);
                let own = own_reserved.clone().acquire_owned();
                let other = other_reserved.clone().acquire_owned();
                let lane_changed = lane_change_notify.notified();
                tokio::pin!(own);
                tokio::pin!(other);
                tokio::pin!(lane_changed);
                tokio::select! {
                    _ = &mut lane_changed => {
                        drop(lane_waiter_guard);
                        drop(own_waiter_guard);
                        return AcquireWithLaneQueueWaitOutcome::Retry;
                    }
                    permit = &mut own => permit.expect("interactive/background reserved semaphore closed"),
                    permit = &mut other => permit.expect("borrowed semaphore closed"),
                }
            }
        } else {
            mark_queue_wait_started(on_queue_wait_started);
            let own = own_reserved.clone().acquire_owned();
            let lane_changed = lane_change_notify.notified();
            tokio::pin!(own);
            tokio::pin!(lane_changed);
            tokio::select! {
                _ = &mut lane_changed => {
                    drop(lane_waiter_guard);
                    drop(own_waiter_guard);
                    return AcquireWithLaneQueueWaitOutcome::Retry;
                }
                permit = &mut own => permit.expect("interactive/background reserved semaphore closed"),
            }
        };
        drop(lane_waiter_guard);
        drop(own_waiter_guard);
        AcquireWithLaneQueueWaitOutcome::Acquired(permit)
    }

    fn saturation_snapshot(&self) -> CpuBudgetSaturationSnapshot {
        CpuBudgetSaturationSnapshot {
            interactive_waiters: self.interactive_waiters.load(Ordering::Acquire),
            background_waiters: self.background_waiters.load(Ordering::Acquire),
            did_save_followup_waiters: self.did_save_followup_waiters.load(Ordering::Acquire),
            interactive_permits: self.interactive_reserved.available_permits(),
            background_permits: self.background_reserved.available_permits(),
            did_save_followup_permits: self.did_save_followup_reserved.available_permits(),
            shared_permits: self.shared.available_permits(),
        }
    }
}

enum AcquireWithLaneQueueWaitOutcome {
    Acquired(tokio::sync::OwnedSemaphorePermit),
    Retry,
}

fn mark_queue_wait_started<Q>(on_queue_wait_started: &mut Option<Q>)
where
    Q: FnOnce(),
{
    if let Some(callback) = on_queue_wait_started.take() {
        callback();
    }
}

static CPU_BOUND_BUDGET: OnceLock<Arc<CpuBoundBudget>> = OnceLock::new();

fn cpu_bound_budget() -> Arc<CpuBoundBudget> {
    CPU_BOUND_BUDGET
        .get_or_init(|| Arc::new(CpuBoundBudget::new()))
        .clone()
}

pub async fn spawn_bounded_blocking<F, R>(f: F) -> Result<R, tokio::task::JoinError>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    spawn_bounded_blocking_with_class(CpuWorkClass::Interactive, f).await
}

pub async fn spawn_bounded_blocking_with_class<F, R>(
    class: CpuWorkClass,
    f: F,
) -> Result<R, tokio::task::JoinError>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    spawn_bounded_blocking_with_class_observed(class, None, f).await
}

pub async fn spawn_bounded_blocking_with_class_observed<F, R>(
    class: CpuWorkClass,
    observability: Option<&SystemCoordinator>,
    f: F,
) -> Result<R, tokio::task::JoinError>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    spawn_bounded_blocking_with_class_observed_origin(class, "runtime", observability, f).await
}

#[derive(Debug)]
pub struct ObservedBlockingCall<R> {
    pub queue_wait_elapsed: Duration,
    pub exec_elapsed: Duration,
    pub join_result: Result<R, tokio::task::JoinError>,
}

pub async fn spawn_bounded_blocking_with_class_observed_origin<F, R>(
    class: CpuWorkClass,
    origin: &'static str,
    observability: Option<&SystemCoordinator>,
    f: F,
) -> Result<R, tokio::task::JoinError>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    spawn_bounded_blocking_with_class_observed_call_origin(class, origin, observability, f)
        .await
        .join_result
}

pub async fn spawn_bounded_blocking_with_class_observed_call_origin<F, R>(
    class: CpuWorkClass,
    origin: &'static str,
    observability: Option<&SystemCoordinator>,
    f: F,
) -> ObservedBlockingCall<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    spawn_bounded_blocking_with_class_observed_call_origin_lane_hooks(
        class,
        origin,
        None,
        observability,
        None::<fn()>,
        None::<fn(Duration)>,
        f,
    )
    .await
}

pub async fn spawn_bounded_blocking_with_class_observed_call_origin_hooks<F, R, Q, S>(
    class: CpuWorkClass,
    origin: &'static str,
    observability: Option<&SystemCoordinator>,
    on_queue_wait_started: Option<Q>,
    on_exec_started: Option<S>,
    f: F,
) -> ObservedBlockingCall<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
    Q: FnOnce(),
    S: FnOnce(Duration),
{
    spawn_bounded_blocking_with_class_observed_call_origin_lane_hooks(
        class,
        origin,
        None,
        observability,
        on_queue_wait_started,
        on_exec_started,
        f,
    )
    .await
}

pub async fn spawn_bounded_blocking_with_class_observed_call_origin_lane_hooks<F, R, Q, S>(
    class: CpuWorkClass,
    origin: &'static str,
    lane: Option<AdmissionLane>,
    observability: Option<&SystemCoordinator>,
    on_queue_wait_started: Option<Q>,
    on_exec_started: Option<S>,
    f: F,
) -> ObservedBlockingCall<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
    Q: FnOnce(),
    S: FnOnce(Duration),
{
    let queue_wait_started = Instant::now();
    let permit = if std::thread::available_parallelism()
        .map(|parallelism| parallelism.get() >= 2)
        .unwrap_or(true)
    {
        cpu_bound_budget()
            .acquire_with_lane_queue_wait_hook(class, lane, on_queue_wait_started)
            .await
    } else {
        let semaphore = cpu_bound_semaphore();
        if let Ok(permit) = semaphore.clone().try_acquire_owned() {
            permit
        } else {
            if let Some(callback) = on_queue_wait_started {
                callback();
            }
            semaphore
                .acquire_owned()
                .await
                .expect("cpu-bound semaphore closed")
        }
    };
    let queue_wait_elapsed = queue_wait_started.elapsed();
    if let Some(coordinator) = observability {
        coordinator.record_intellisense_v2_runtime_queue_wait_class_latency_with_origin(
            origin,
            cpu_class_label(class),
            queue_wait_elapsed,
        );
        if let Some(lane) = lane {
            coordinator.record_intellisense_v2_runtime_lane_queue_wait_latency_with_origin(
                origin,
                lane.as_str(),
                queue_wait_elapsed,
            );
        }
    }
    emit_runtime_saturation_gauges(origin, observability);
    if let Some(callback) = on_exec_started {
        callback(queue_wait_elapsed);
    }

    let exec_started = Instant::now();
    let join_result = tokio::task::spawn_blocking(f).await;
    let exec_elapsed = exec_started.elapsed();
    if let Some(coordinator) = observability {
        coordinator.record_intellisense_v2_runtime_exec_class_latency_with_origin(
            origin,
            cpu_class_label(class),
            exec_elapsed,
        );
        if let Some(lane) = lane {
            coordinator.record_intellisense_v2_runtime_lane_exec_latency_with_origin(
                origin,
                lane.as_str(),
                exec_elapsed,
            );
        }
    }
    drop(permit);
    emit_runtime_saturation_gauges(origin, observability);
    ObservedBlockingCall {
        queue_wait_elapsed,
        exec_elapsed,
        join_result,
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn spawn_bounded_blocking_with_class_observed_call_origin_dynamic_lane_hooks<
    F,
    R,
    L,
    Q,
    S,
>(
    class: CpuWorkClass,
    origin: &'static str,
    current_lane: L,
    lane_change_notify: &Notify,
    observability: Option<&SystemCoordinator>,
    on_queue_wait_started: Option<Q>,
    on_exec_started: Option<S>,
    f: F,
) -> ObservedBlockingCall<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
    L: Fn() -> Option<AdmissionLane>,
    Q: FnOnce(),
    S: FnOnce(Duration),
{
    let queue_wait_started = Instant::now();
    let (permit, lane) = if std::thread::available_parallelism()
        .map(|parallelism| parallelism.get() >= 2)
        .unwrap_or(true)
    {
        cpu_bound_budget()
            .acquire_with_dynamic_lane_queue_wait_hook(
                class,
                current_lane,
                lane_change_notify,
                on_queue_wait_started,
            )
            .await
    } else {
        let semaphore = cpu_bound_semaphore();
        let permit = if let Ok(permit) = semaphore.clone().try_acquire_owned() {
            permit
        } else {
            if let Some(callback) = on_queue_wait_started {
                callback();
            }
            semaphore
                .acquire_owned()
                .await
                .expect("cpu-bound semaphore closed")
        };
        (permit, current_lane())
    };
    let queue_wait_elapsed = queue_wait_started.elapsed();
    if let Some(coordinator) = observability {
        coordinator.record_intellisense_v2_runtime_queue_wait_class_latency_with_origin(
            origin,
            cpu_class_label(class),
            queue_wait_elapsed,
        );
        if let Some(lane) = lane {
            coordinator.record_intellisense_v2_runtime_lane_queue_wait_latency_with_origin(
                origin,
                lane.as_str(),
                queue_wait_elapsed,
            );
        }
    }
    emit_runtime_saturation_gauges(origin, observability);
    if let Some(callback) = on_exec_started {
        callback(queue_wait_elapsed);
    }

    let exec_started = Instant::now();
    let join_result = tokio::task::spawn_blocking(f).await;
    let exec_elapsed = exec_started.elapsed();
    if let Some(coordinator) = observability {
        coordinator.record_intellisense_v2_runtime_exec_class_latency_with_origin(
            origin,
            cpu_class_label(class),
            exec_elapsed,
        );
        if let Some(lane) = lane {
            coordinator.record_intellisense_v2_runtime_lane_exec_latency_with_origin(
                origin,
                lane.as_str(),
                exec_elapsed,
            );
        }
    }
    drop(permit);
    emit_runtime_saturation_gauges(origin, observability);
    ObservedBlockingCall {
        queue_wait_elapsed,
        exec_elapsed,
        join_result,
    }
}

fn emit_runtime_saturation_gauges(origin: &str, observability: Option<&SystemCoordinator>) {
    let Some(coordinator) = observability else {
        return;
    };

    let snapshot = if std::thread::available_parallelism()
        .map(|parallelism| parallelism.get() >= 2)
        .unwrap_or(true)
    {
        cpu_bound_budget().saturation_snapshot()
    } else {
        CpuBudgetSaturationSnapshot {
            interactive_waiters: 0,
            background_waiters: 0,
            did_save_followup_waiters: 0,
            interactive_permits: 0,
            background_permits: 0,
            did_save_followup_permits: 0,
            shared_permits: cpu_bound_semaphore().available_permits(),
        }
    };

    coordinator.record_intellisense_v2_runtime_saturation_gauge_with_origin(
        origin,
        "waiters_interactive",
        snapshot.interactive_waiters as f64,
        "intellisense_v2_runtime_saturation_waiters_interactive",
    );
    coordinator.record_intellisense_v2_runtime_saturation_gauge_with_origin(
        origin,
        "waiters_background",
        snapshot.background_waiters as f64,
        "intellisense_v2_runtime_saturation_waiters_background",
    );
    coordinator.record_intellisense_v2_runtime_saturation_gauge_with_origin(
        origin,
        "waiters_did_save_followup",
        snapshot.did_save_followup_waiters as f64,
        "intellisense_v2_runtime_saturation_waiters_did_save_followup",
    );
    coordinator.record_intellisense_v2_runtime_saturation_gauge_with_origin(
        origin,
        "permits_interactive",
        snapshot.interactive_permits as f64,
        "intellisense_v2_runtime_saturation_permits_interactive",
    );
    coordinator.record_intellisense_v2_runtime_saturation_gauge_with_origin(
        origin,
        "permits_background",
        snapshot.background_permits as f64,
        "intellisense_v2_runtime_saturation_permits_background",
    );
    coordinator.record_intellisense_v2_runtime_saturation_gauge_with_origin(
        origin,
        "permits_did_save_followup",
        snapshot.did_save_followup_permits as f64,
        "intellisense_v2_runtime_saturation_permits_did_save_followup",
    );
    coordinator.record_intellisense_v2_runtime_saturation_gauge_with_origin(
        origin,
        "permits_shared",
        snapshot.shared_permits as f64,
        "intellisense_v2_runtime_saturation_permits_shared",
    );
    coordinator.record_intellisense_v2_runtime_saturation_gauge_with_origin(
        origin,
        "queue_depth_total",
        snapshot.queue_depth_total() as f64,
        "intellisense_v2_runtime_saturation_queue_depth_total",
    );
}

fn cpu_class_label(class: CpuWorkClass) -> &'static str {
    match class {
        CpuWorkClass::Interactive => "interactive",
        CpuWorkClass::Background => "background",
    }
}

#[cfg(test)]
#[path = "policy/tests.rs"]
mod tests;
