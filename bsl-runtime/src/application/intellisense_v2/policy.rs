use std::sync::{Arc, OnceLock};
use std::time::Duration;

use super::facade::SemanticOperation;
use crate::system::{global_runtime_config, RuntimeKey};
use tokio::sync::Semaphore;

#[derive(Debug, Clone, Copy, Default)]
pub struct RuntimePerfKnobs {
    pub slow_wait_warn_threshold: Option<Duration>,
    pub slow_snapshot_warn_threshold: Option<Duration>,
    pub slow_query_warn_threshold: Option<Duration>,
    pub slow_client_log_threshold: Option<Duration>,
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

pub fn should_query_parse_result(operation: SemanticOperation, ir_available: bool) -> bool {
    match operation {
        SemanticOperation::Completion | SemanticOperation::Members => ir_available,
        SemanticOperation::DocumentSymbol
        | SemanticOperation::Rename
        | SemanticOperation::SymbolSearch
        | SemanticOperation::References => true,
        SemanticOperation::Hover
        | SemanticOperation::SignatureHelp
        | SemanticOperation::Definition
        | SemanticOperation::Diagnostics
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

pub async fn spawn_bounded_blocking<F, R>(f: F) -> Result<R, tokio::task::JoinError>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let permit = cpu_bound_semaphore()
        .acquire_owned()
        .await
        .expect("cpu-bound semaphore closed");
    let result = tokio::task::spawn_blocking(f).await;
    drop(permit);
    result
}
