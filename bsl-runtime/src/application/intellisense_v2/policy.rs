use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use super::facade::SemanticOperation;
use crate::system::{global_runtime_config, RuntimeKey, SystemCoordinator};
use tokio::sync::Semaphore;

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
    pub max_stale_version_gap: i32,
    pub max_stale_age: Duration,
}

impl InteractiveFreshnessKnobs {
    pub fn from_runtime_config(observability: Option<&SystemCoordinator>) -> Self {
        let (wait_budget, wait_budget_clamped) = read_clamped_duration(
            RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs,
            Duration::from_millis(120),
            Duration::from_millis(10),
            Duration::from_millis(2000),
        );
        let (max_stale_version_gap, stale_gap_clamped) = read_clamped_i32(
            RuntimeKey::IntellisenseV2InteractiveMaxStaleVersionGap,
            1,
            0,
            10,
        );
        let (max_stale_age, stale_age_clamped) = read_clamped_duration(
            RuntimeKey::IntellisenseV2InteractiveMaxStaleAgeMs,
            Duration::from_millis(1000),
            Duration::from_millis(0),
            Duration::from_millis(10_000),
        );
        if wait_budget_clamped || stale_gap_clamped || stale_age_clamped {
            if let Some(coordinator) = observability {
                coordinator.record_intellisense_v2_interactive_knob_clamped();
            }
        }
        Self {
            wait_budget,
            max_stale_version_gap,
            max_stale_age,
        }
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

impl CompletionPipelineKnobs {
    pub fn from_runtime_config() -> Self {
        let mode = global_runtime_config()
            .get_string(RuntimeKey::IntellisenseV2CompletionMode)
            .map(|value| CompletionMode::parse(&value))
            .unwrap_or(CompletionMode::Off);
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

fn read_clamped_i32(key: RuntimeKey, default: i32, min: i32, max: i32) -> (i32, bool) {
    let raw = global_runtime_config()
        .get_u64(key)
        .unwrap_or(default as u64);
    let raw_i32 = if raw > i32::MAX as u64 {
        i32::MAX
    } else {
        raw as i32
    };
    let clamped = raw_i32.clamp(min, max);
    (clamped, clamped != raw_i32)
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
        | SemanticOperation::SignatureHelp => Some(InteractiveFreshnessKnobs::from_runtime_config(
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuWorkClass {
    Interactive,
    Background,
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
    IdleHeavy,
}

impl DiagnosticsProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            DiagnosticsProfile::Fast => "fast",
            DiagnosticsProfile::DebouncedFull => "debounced_full",
            DiagnosticsProfile::IdleHeavy => "idle_heavy",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticsDisposition {
    Published,
    SupersededVersion,
    SupersededGeneration,
    Cancelled,
}

impl DiagnosticsDisposition {
    pub fn as_str(self) -> &'static str {
        match self {
            DiagnosticsDisposition::Published => "published",
            DiagnosticsDisposition::SupersededVersion => "superseded_version",
            DiagnosticsDisposition::SupersededGeneration => "superseded_generation",
            DiagnosticsDisposition::Cancelled => "cancelled",
        }
    }
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
const PROFILES_DID_SAVE: &[DiagnosticsProfile] = &[DiagnosticsProfile::IdleHeavy];
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
    interactive_permits: usize,
    background_permits: usize,
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
    shared: Arc<Semaphore>,
    interactive_waiters: AtomicUsize,
    background_waiters: AtomicUsize,
}

impl CpuBoundBudget {
    fn with_total_permits(permits: usize) -> Self {
        let permits = permits.max(2);
        // Keep one dedicated background permit, but reserve an extra interactive
        // permit when capacity allows to reduce interactive tail under batch load.
        let interactive_reserved_permits = if permits >= 4 { 2 } else { 1 };
        let background_reserved_permits = 1;
        let shared_permits =
            permits.saturating_sub(interactive_reserved_permits + background_reserved_permits);
        Self {
            interactive_reserved: Arc::new(Semaphore::new(interactive_reserved_permits)),
            background_reserved: Arc::new(Semaphore::new(background_reserved_permits)),
            shared: Arc::new(Semaphore::new(shared_permits)),
            interactive_waiters: AtomicUsize::new(0),
            background_waiters: AtomicUsize::new(0),
        }
    }

    fn new() -> Self {
        let permits = std::thread::available_parallelism()
            .map(|parallelism| parallelism.get().max(2))
            .unwrap_or(4);
        Self::with_total_permits(permits)
    }

    async fn acquire(&self, class: CpuWorkClass) -> tokio::sync::OwnedSemaphorePermit {
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

        own_waiters.fetch_add(1, Ordering::AcqRel);
        let permit = loop {
            let other_has_waiters = other_waiters.load(Ordering::Acquire) > 0;
            let can_borrow = !other_has_waiters;
            // Keep shared permits biased toward interactive work when interactive queue is non-empty.
            let can_take_shared = matches!(class, CpuWorkClass::Interactive) || !other_has_waiters;

            if let Ok(permit) = own_reserved.clone().try_acquire_owned() {
                break permit;
            }
            if can_take_shared {
                if let Ok(permit) = self.shared.clone().try_acquire_owned() {
                    break permit;
                }
            }
            if can_borrow {
                if let Ok(permit) = other_reserved.clone().try_acquire_owned() {
                    break permit;
                }
            }

            match (can_take_shared, can_borrow) {
                (true, true) => {
                    tokio::select! {
                        permit = own_reserved.clone().acquire_owned() => break permit.expect("interactive/background reserved semaphore closed"),
                        permit = self.shared.clone().acquire_owned() => break permit.expect("shared semaphore closed"),
                        permit = other_reserved.clone().acquire_owned() => break permit.expect("borrowed semaphore closed"),
                    }
                }
                (true, false) => {
                    tokio::select! {
                        permit = own_reserved.clone().acquire_owned() => break permit.expect("interactive/background reserved semaphore closed"),
                        permit = self.shared.clone().acquire_owned() => break permit.expect("shared semaphore closed"),
                    }
                }
                (false, true) => {
                    tokio::select! {
                        permit = own_reserved.clone().acquire_owned() => break permit.expect("interactive/background reserved semaphore closed"),
                        permit = other_reserved.clone().acquire_owned() => break permit.expect("borrowed semaphore closed"),
                    }
                }
                (false, false) => {
                    let permit = own_reserved
                        .clone()
                        .acquire_owned()
                        .await
                        .expect("interactive/background reserved semaphore closed");
                    break permit;
                }
            }
        };
        own_waiters.fetch_sub(1, Ordering::AcqRel);
        permit
    }

    fn saturation_snapshot(&self) -> CpuBudgetSaturationSnapshot {
        CpuBudgetSaturationSnapshot {
            interactive_waiters: self.interactive_waiters.load(Ordering::Acquire),
            background_waiters: self.background_waiters.load(Ordering::Acquire),
            interactive_permits: self.interactive_reserved.available_permits(),
            background_permits: self.background_reserved.available_permits(),
            shared_permits: self.shared.available_permits(),
        }
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
    let queue_wait_started = Instant::now();
    let permit = if std::thread::available_parallelism()
        .map(|parallelism| parallelism.get() >= 2)
        .unwrap_or(true)
    {
        cpu_bound_budget().acquire(class).await
    } else {
        cpu_bound_semaphore()
            .acquire_owned()
            .await
            .expect("cpu-bound semaphore closed")
    };
    let queue_wait_elapsed = queue_wait_started.elapsed();
    if let Some(coordinator) = observability {
        coordinator.record_intellisense_v2_runtime_queue_wait_class_latency_with_origin(
            origin,
            cpu_class_label(class),
            queue_wait_elapsed,
        );
    }
    emit_runtime_saturation_gauges(origin, observability);

    let exec_started = Instant::now();
    let result = tokio::task::spawn_blocking(f).await;
    let exec_elapsed = exec_started.elapsed();
    if let Some(coordinator) = observability {
        coordinator.record_intellisense_v2_runtime_exec_class_latency_with_origin(
            origin,
            cpu_class_label(class),
            exec_elapsed,
        );
    }
    drop(permit);
    emit_runtime_saturation_gauges(origin, observability);
    result
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
            interactive_permits: 0,
            background_permits: 0,
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
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tokio::sync::oneshot;
    use tokio::time::timeout;

    #[test]
    fn parse_result_policy_keeps_diagnostics_enabled() {
        assert!(
            should_query_parse_result(SemanticOperation::Diagnostics, false),
            "diagnostics must keep parse_result query enabled for singleflight sharing"
        );
        assert!(
            !should_query_parse_result(SemanticOperation::Completion, false),
            "completion parse_result remains gated by IR availability"
        );
    }

    #[test]
    fn diagnostics_profiles_follow_tiered_trigger_contract() {
        assert_eq!(
            diagnostics_profiles_for_trigger(DiagnosticsTrigger::DidChange),
            &[
                DiagnosticsProfile::Fast,
                DiagnosticsProfile::DebouncedFull,
                DiagnosticsProfile::IdleHeavy,
            ]
        );
        assert_eq!(
            diagnostics_profiles_for_trigger(DiagnosticsTrigger::DidOpen),
            &[DiagnosticsProfile::DebouncedFull]
        );
        assert_eq!(
            diagnostics_profiles_for_trigger(DiagnosticsTrigger::DidSave),
            &[DiagnosticsProfile::IdleHeavy]
        );
        assert_eq!(
            diagnostics_profiles_for_trigger(DiagnosticsTrigger::Idle),
            &[DiagnosticsProfile::IdleHeavy]
        );
        assert_eq!(
            diagnostics_profiles_for_trigger(DiagnosticsTrigger::DocumentsSet),
            &[DiagnosticsProfile::DebouncedFull]
        );
        assert_eq!(
            diagnostics_profiles_for_trigger(DiagnosticsTrigger::JobStart),
            &[DiagnosticsProfile::DebouncedFull]
        );
    }

    #[test]
    fn diagnostics_execution_plan_matches_profile_expectations() {
        let fast = diagnostics_execution_plan(DiagnosticsProfile::Fast, true);
        assert!(!fast.run_syntax);
        assert!(!fast.run_semantic);
        assert!(!fast.flow_sensitive_semantic);
        assert_eq!(fast.cpu_class, CpuWorkClass::Interactive);

        let debounced = diagnostics_execution_plan(DiagnosticsProfile::DebouncedFull, true);
        assert!(debounced.run_syntax);
        assert!(debounced.run_semantic);
        assert!(!debounced.flow_sensitive_semantic);
        assert_eq!(debounced.cpu_class, CpuWorkClass::Background);

        let idle_heavy_flow_off = diagnostics_execution_plan(DiagnosticsProfile::IdleHeavy, false);
        assert!(idle_heavy_flow_off.run_syntax);
        assert!(idle_heavy_flow_off.run_semantic);
        assert!(!idle_heavy_flow_off.flow_sensitive_semantic);
        assert_eq!(idle_heavy_flow_off.cpu_class, CpuWorkClass::Background);

        let idle_heavy_flow_on = diagnostics_execution_plan(DiagnosticsProfile::IdleHeavy, true);
        assert!(idle_heavy_flow_on.flow_sensitive_semantic);
        assert_eq!(idle_heavy_flow_on.cpu_class, CpuWorkClass::Background);
    }

    #[test]
    fn cpu_work_class_keeps_interactive_tools_out_of_background_queue() {
        assert_eq!(
            cpu_work_class_for_operation(SemanticOperation::TypeAtPosition),
            CpuWorkClass::Interactive
        );
        assert_eq!(
            cpu_work_class_for_operation(SemanticOperation::Members),
            CpuWorkClass::Interactive
        );
        assert_eq!(
            cpu_work_class_for_operation(SemanticOperation::Definition),
            CpuWorkClass::Interactive
        );
        assert_eq!(
            cpu_work_class_for_operation(SemanticOperation::SymbolSearch),
            CpuWorkClass::Background
        );
        assert_eq!(
            cpu_work_class_for_operation(SemanticOperation::References),
            CpuWorkClass::Background
        );
    }

    struct EnvVarGuard {
        key: &'static str,
        prev: Option<String>,
    }

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let prev = std::env::var(key).ok();
            std::env::set_var(key, value);
            global_runtime_config().reload_env_bootstrap_from_env();
            Self { key, prev }
        }

        fn unset(key: &'static str) -> Self {
            let prev = std::env::var(key).ok();
            std::env::remove_var(key);
            global_runtime_config().reload_env_bootstrap_from_env();
            Self { key, prev }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(prev) = &self.prev {
                std::env::set_var(self.key, prev);
            } else {
                std::env::remove_var(self.key);
            }
            global_runtime_config().reload_env_bootstrap_from_env();
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn cpu_budget_allows_borrow_when_other_queue_idle() {
        let budget = Arc::new(CpuBoundBudget::with_total_permits(2));
        let _first = budget.acquire(CpuWorkClass::Interactive).await;

        let budget_clone = budget.clone();
        let borrowed = timeout(Duration::from_millis(150), async move {
            let _permit = budget_clone.acquire(CpuWorkClass::Interactive).await;
        })
        .await;
        assert!(
            borrowed.is_ok(),
            "second interactive acquire should borrow background permit when background queue is idle"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn cpu_budget_background_progresses_under_interactive_load() {
        let budget = Arc::new(CpuBoundBudget::with_total_permits(2));
        let interactive_reserved = budget.acquire(CpuWorkClass::Interactive).await;

        let (borrowed_ready_tx, borrowed_ready_rx) = oneshot::channel::<()>();
        let (borrowed_release_tx, borrowed_release_rx) = oneshot::channel::<()>();

        let budget_for_borrowed = budget.clone();
        let borrowed_task = tokio::spawn(async move {
            let permit = budget_for_borrowed.acquire(CpuWorkClass::Interactive).await;
            let _ = borrowed_ready_tx.send(());
            let _ = borrowed_release_rx.await;
            drop(permit);
        });
        borrowed_ready_rx
            .await
            .expect("borrowed interactive task should signal readiness");

        let budget_for_background = budget.clone();
        let background_task = tokio::spawn(async move {
            budget_for_background
                .acquire(CpuWorkClass::Background)
                .await
        });

        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(
            !background_task.is_finished(),
            "background acquire must wait while both permits are occupied by interactive load"
        );

        drop(interactive_reserved);
        let background_permit = timeout(Duration::from_millis(250), background_task)
            .await
            .expect("background should make progress after one interactive permit is released")
            .expect("background task join should succeed");
        drop(background_permit);

        let _ = borrowed_release_tx.send(());
        borrowed_task.await.expect("borrowed interactive task join");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn cpu_budget_interactive_progresses_under_background_load() {
        let budget = Arc::new(CpuBoundBudget::with_total_permits(2));
        let background_reserved = budget.acquire(CpuWorkClass::Background).await;

        let (borrowed_ready_tx, borrowed_ready_rx) = oneshot::channel::<()>();
        let (borrowed_release_tx, borrowed_release_rx) = oneshot::channel::<()>();

        let budget_for_borrowed = budget.clone();
        let borrowed_task = tokio::spawn(async move {
            let permit = budget_for_borrowed.acquire(CpuWorkClass::Background).await;
            let _ = borrowed_ready_tx.send(());
            let _ = borrowed_release_rx.await;
            drop(permit);
        });
        borrowed_ready_rx
            .await
            .expect("borrowed background task should signal readiness");

        let budget_for_interactive = budget.clone();
        let interactive_task = tokio::spawn(async move {
            budget_for_interactive
                .acquire(CpuWorkClass::Interactive)
                .await
        });

        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(
            !interactive_task.is_finished(),
            "interactive acquire must wait while both permits are occupied by background load"
        );

        drop(background_reserved);
        let interactive_permit = timeout(Duration::from_millis(250), interactive_task)
            .await
            .expect("interactive should make progress after one background permit is released")
            .expect("interactive task join should succeed");
        drop(interactive_permit);

        let _ = borrowed_release_tx.send(());
        borrowed_task.await.expect("borrowed background task join");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn cpu_budget_background_does_not_take_shared_while_interactive_waits() {
        let budget = Arc::new(CpuBoundBudget::with_total_permits(3));
        let interactive_reserved = budget.acquire(CpuWorkClass::Interactive).await;
        let background_reserved = budget.acquire(CpuWorkClass::Background).await;
        let shared_taken_by_background = budget.acquire(CpuWorkClass::Background).await;

        let budget_for_interactive = budget.clone();
        let interactive_waiter = tokio::spawn(async move {
            budget_for_interactive
                .acquire(CpuWorkClass::Interactive)
                .await
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(
            !interactive_waiter.is_finished(),
            "interactive waiter should be queued while all permits are occupied"
        );

        let budget_for_background = budget.clone();
        let background_waiter = tokio::spawn(async move {
            budget_for_background
                .acquire(CpuWorkClass::Background)
                .await
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(
            !background_waiter.is_finished(),
            "background waiter should also be queued before shared release"
        );

        drop(shared_taken_by_background);
        let interactive_permit = timeout(Duration::from_millis(300), interactive_waiter)
            .await
            .expect("interactive waiter should win released shared permit")
            .expect("interactive waiter join should succeed");

        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(
            !background_waiter.is_finished(),
            "background waiter must not steal shared while interactive queue is non-empty"
        );

        drop(interactive_permit);
        drop(interactive_reserved);
        drop(background_reserved);
        let background_permit = timeout(Duration::from_millis(300), background_waiter)
            .await
            .expect("background waiter should eventually make progress")
            .expect("background waiter join should succeed");
        drop(background_permit);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn observed_spawn_records_runtime_class_metrics() {
        let coordinator = SystemCoordinator::new();

        let interactive = spawn_bounded_blocking_with_class_observed(
            CpuWorkClass::Interactive,
            Some(&coordinator),
            || 1_u32,
        )
        .await
        .expect("interactive spawn should succeed");
        assert_eq!(interactive, 1);

        let background = spawn_bounded_blocking_with_class_observed(
            CpuWorkClass::Background,
            Some(&coordinator),
            || 2_u32,
        )
        .await
        .expect("background spawn should succeed");
        assert_eq!(background, 2);

        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        let gauges = metrics
            .get("gauges")
            .and_then(|value| value.as_object())
            .expect("metrics.gauges object");
        let histograms = metrics
            .get("histograms")
            .and_then(|value| value.as_object())
            .expect("metrics.histograms object");

        assert!(
            counters.contains_key("intellisense_v2_runtime_queue_wait_interactive_total"),
            "interactive queue counter should be recorded"
        );
        assert!(
            counters.contains_key("intellisense_v2_runtime_queue_wait_background_total"),
            "background queue counter should be recorded"
        );
        assert!(
            counters.contains_key("intellisense_v2_runtime_exec_interactive_total"),
            "interactive exec counter should be recorded"
        );
        assert!(
            counters.contains_key("intellisense_v2_runtime_exec_background_total"),
            "background exec counter should be recorded"
        );
        assert!(
            histograms.contains_key("intellisense_v2_runtime_queue_wait_interactive_ms"),
            "interactive queue histogram should be recorded"
        );
        assert!(
            histograms.contains_key("intellisense_v2_runtime_queue_wait_background_ms"),
            "background queue histogram should be recorded"
        );
        assert!(
            histograms.contains_key("intellisense_v2_runtime_exec_interactive_ms"),
            "interactive exec histogram should be recorded"
        );
        assert!(
            histograms.contains_key("intellisense_v2_runtime_exec_background_ms"),
            "background exec histogram should be recorded"
        );
        assert!(
            counters.contains_key(
                "intellisense_v2_drilldown_saturation_sample_total_origin_runtime_reason_queue_wait_work_class_interactive"
            ),
            "interactive drilldown queue_wait counter should be recorded"
        );
        assert!(
            counters.contains_key(
                "intellisense_v2_drilldown_saturation_sample_total_origin_runtime_reason_queue_wait_work_class_background"
            ),
            "background drilldown queue_wait counter should be recorded"
        );
        assert!(
            histograms.contains_key(
                "intellisense_v2_drilldown_saturation_sample_latency_ms_origin_runtime_reason_exec_work_class_interactive"
            ),
            "interactive drilldown exec histogram should be recorded"
        );
        assert!(
            histograms.contains_key(
                "intellisense_v2_drilldown_saturation_sample_latency_ms_origin_runtime_reason_exec_work_class_background"
            ),
            "background drilldown exec histogram should be recorded"
        );
        assert!(
            gauges.contains_key("intellisense_v2_runtime_saturation_waiters_interactive"),
            "legacy saturation gauge should be exported"
        );
        assert!(
            gauges.contains_key(
                "intellisense_v2_drilldown_saturation_gauge_origin_runtime_saturation_metric_queue_depth_total"
            ),
            "drilldown saturation gauge should be exported"
        );
    }

    #[test]
    fn cpu_budget_reserves_extra_interactive_permit_when_capacity_allows() {
        let budget = CpuBoundBudget::with_total_permits(4);
        let snapshot = budget.saturation_snapshot();
        assert_eq!(
            snapshot.interactive_permits, 2,
            "interactive pool should get extra reserved permit on wider runtimes"
        );
        assert_eq!(
            snapshot.background_permits, 1,
            "background pool should keep one dedicated permit"
        );
        assert_eq!(
            snapshot.shared_permits, 1,
            "remaining capacity should stay shared"
        );
    }

    #[test]
    fn interactive_knobs_clamp_and_emit_metric() {
        let _env_guard = ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock should not be poisoned");

        let _wait_guard =
            EnvVarGuard::set("BSL_INTELLISENSE_V2_INTERACTIVE_WAIT_BUDGET_MS", "999999");
        let _gap_guard = EnvVarGuard::set(
            "BSL_INTELLISENSE_V2_INTERACTIVE_MAX_STALE_VERSION_GAP",
            "999",
        );
        let _age_guard =
            EnvVarGuard::set("BSL_INTELLISENSE_V2_INTERACTIVE_MAX_STALE_AGE_MS", "999999");

        let coordinator = SystemCoordinator::new();
        let knobs = interactive_freshness_knobs(SemanticOperation::Completion, Some(&coordinator))
            .expect("completion should use interactive knobs");
        assert_eq!(knobs.wait_budget, Duration::from_millis(2000));
        assert_eq!(knobs.max_stale_version_gap, 10);
        assert_eq!(knobs.max_stale_age, Duration::from_millis(10_000));

        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        assert!(
            counters.contains_key("intellisense_v2_interactive_knob_clamped_total"),
            "clamped interactive knobs should emit metric"
        );
    }

    #[test]
    fn completion_pipeline_knobs_use_defaults_when_env_missing() {
        let _env_guard = ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock should not be poisoned");

        let _mode_guard = EnvVarGuard::unset("BSL_INTELLISENSE_V2_COMPLETION_MODE");
        let _canary_guard = EnvVarGuard::unset("BSL_INTELLISENSE_V2_COMPLETION_CANARY_PERCENT");
        let _capacity_guard = EnvVarGuard::unset("BSL_INTELLISENSE_V2_COMPLETION_QUEUE_CAPACITY");

        let knobs = CompletionPipelineKnobs::from_runtime_config();
        assert_eq!(knobs.mode, CompletionMode::Off);
        assert_eq!(knobs.canary_percent, 0);
        assert_eq!(knobs.queue_capacity, 256);
    }

    #[test]
    fn completion_pipeline_knobs_normalize_mode_and_clamp_values() {
        let _env_guard = ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock should not be poisoned");

        let _mode_guard = EnvVarGuard::set("BSL_INTELLISENSE_V2_COMPLETION_MODE", "CANARY");
        let _canary_guard =
            EnvVarGuard::set("BSL_INTELLISENSE_V2_COMPLETION_CANARY_PERCENT", "999");
        let _capacity_guard =
            EnvVarGuard::set("BSL_INTELLISENSE_V2_COMPLETION_QUEUE_CAPACITY", "1");

        let knobs = CompletionPipelineKnobs::from_runtime_config();
        assert_eq!(knobs.mode, CompletionMode::Canary);
        assert_eq!(knobs.canary_percent, 100);
        assert_eq!(knobs.queue_capacity, 16);
    }

    #[test]
    fn completion_pipeline_knobs_fallback_to_off_for_unknown_mode() {
        let _env_guard = ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock should not be poisoned");

        let _mode_guard = EnvVarGuard::set("BSL_INTELLISENSE_V2_COMPLETION_MODE", "legacy_like");
        let knobs = CompletionPipelineKnobs::from_runtime_config();
        assert_eq!(knobs.mode, CompletionMode::Off);
    }
}
