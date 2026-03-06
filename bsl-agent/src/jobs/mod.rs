use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use bsl_runtime::application::CpuWorkClass;
use bsl_runtime::system::runtime_config::{global_runtime_config, RuntimeKey};
use serde::{Deserialize, Serialize};
use tokio::sync::{Notify, RwLock, Semaphore};
use uuid::Uuid;

use crate::state::{now_unix_secs, state_root, write_atomic};
use crate::types::{JobCancelResponse, JobStateDto, JobStatusResponse, ProgressDto};

const JOBS_DIR: &str = "jobs";
const DEFAULT_TTL_SECS: u64 = 7 * 24 * 60 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedJob {
    pub job_id: String,
    pub state: JobStateDto,
    pub phase: String,
    pub progress: ProgressDto,
    #[serde(default)]
    pub error: Option<String>,
    pub created_at: u64,
    #[serde(default)]
    pub started_at: Option<u64>,
    #[serde(default)]
    pub finished_at: Option<u64>,
    pub updated_at: u64,
}

impl PersistedJob {
    fn as_status(&self) -> JobStatusResponse {
        JobStatusResponse {
            job_id: self.job_id.clone(),
            state: self.state,
            phase: self.phase.clone(),
            progress: self.progress,
            error: self.error.clone(),
        }
    }

    fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            JobStateDto::Succeeded
                | JobStateDto::Failed
                | JobStateDto::Canceled
                | JobStateDto::AbortedByRestart
        )
    }
}

struct JobEntry {
    job: RwLock<PersistedJob>,
    result: RwLock<Option<serde_json::Value>>,
    notify: Notify,
    canceled: AtomicBool,
}

#[derive(Debug)]
struct JobStore {
    jobs_dir: PathBuf,
}

impl JobStore {
    fn new() -> Option<Self> {
        let jobs_dir = state_root().join(JOBS_DIR);
        if let Err(err) = fs::create_dir_all(&jobs_dir) {
            tracing::warn!(
                "Failed to create bsl-agent state dir {}: {}",
                jobs_dir.display(),
                err
            );
            return None;
        }
        Some(Self { jobs_dir })
    }

    fn ttl_secs() -> u64 {
        global_runtime_config()
            .get_u64(RuntimeKey::AgentStateTtlSecs)
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_TTL_SECS)
    }

    fn job_path(&self, job_id: &str) -> PathBuf {
        self.jobs_dir.join(format!("{job_id}.json"))
    }

    fn job_result_path(&self, job_id: &str) -> PathBuf {
        self.jobs_dir.join(format!("{job_id}.result.json"))
    }

    fn write_job(&self, job: &PersistedJob) {
        let path = self.job_path(&job.job_id);
        if let Ok(bytes) = serde_json::to_vec(job) {
            if let Err(err) = write_atomic(&path, &bytes) {
                tracing::warn!("Failed to persist job {}: {}", path.display(), err);
            }
        }
    }

    fn write_result(&self, job_id: &str, value: &serde_json::Value) {
        let path = self.job_result_path(job_id);
        if let Ok(bytes) = serde_json::to_vec(value) {
            if let Err(err) = write_atomic(&path, &bytes) {
                tracing::warn!("Failed to persist job result {}: {}", path.display(), err);
            }
        }
    }

    fn load_jobs(&self) -> Vec<PersistedJob> {
        let mut jobs = Vec::new();
        let Ok(entries) = fs::read_dir(&self.jobs_dir) else {
            return jobs;
        };

        let ttl_secs = Self::ttl_secs();
        let now = now_unix_secs();

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension() != Some(OsStr::new("json")) {
                continue;
            }
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".result.json"))
            {
                continue;
            }

            let Ok(data) = fs::read(&path) else {
                continue;
            };
            let Ok(job) = serde_json::from_slice::<PersistedJob>(&data) else {
                tracing::warn!("Failed to parse persisted job: {}", path.display());
                continue;
            };

            if now.saturating_sub(job.updated_at) > ttl_secs {
                let _ = fs::remove_file(&path);
                let _ = fs::remove_file(self.job_result_path(&job.job_id));
                continue;
            }

            jobs.push(job);
        }

        jobs
    }

    fn load_result(&self, job_id: &str) -> Option<serde_json::Value> {
        let path = self.job_result_path(job_id);
        let data = fs::read(&path).ok()?;
        serde_json::from_slice(&data).ok()
    }

    fn remove_job_files(&self, job_id: &str) {
        let _ = fs::remove_file(self.job_path(job_id));
        let _ = fs::remove_file(self.job_result_path(job_id));
    }

    fn remove_result_file(&self, job_id: &str) {
        let _ = fs::remove_file(self.job_result_path(job_id));
    }
}

#[derive(Clone)]
pub struct JobManager {
    store: Option<Arc<JobStore>>,
    jobs: Arc<RwLock<HashMap<Uuid, Arc<JobEntry>>>>,
    interactive_limiter: Arc<Semaphore>,
    background_limiter: Arc<Semaphore>,
}

#[derive(Clone)]
pub struct JobContext {
    job_id: String,
    entry: Arc<JobEntry>,
    store: Option<Arc<JobStore>>,
}

impl JobContext {
    pub fn job_id(&self) -> &str {
        &self.job_id
    }

    pub fn is_canceled(&self) -> bool {
        self.entry.canceled.load(Ordering::Relaxed)
    }

    pub async fn set_progress(&self, phase: impl Into<String>, percent: u8) {
        let mut job = self.entry.job.write().await;
        if job.is_terminal() {
            return;
        }

        let mut percent = percent.min(100);
        if matches!(job.state, JobStateDto::Queued | JobStateDto::Running) && percent == 100 {
            percent = 99;
        }
        if percent >= job.progress.percent {
            job.progress.percent = percent;
        }
        job.phase = phase.into();
        job.updated_at = now_unix_secs();

        if let Some(store) = self.store.as_ref() {
            store.write_job(&job);
        }

        self.entry.notify.notify_waiters();
    }
}

impl JobManager {
    fn default_concurrency_limits() -> (usize, usize) {
        let total = std::thread::available_parallelism()
            .map(|parallelism| parallelism.get().max(2))
            .unwrap_or(4);
        let interactive = 1;
        let background = total.saturating_sub(interactive).max(1);
        (interactive, background)
    }

    pub fn new() -> Self {
        let store = JobStore::new().map(Arc::new);
        let mut map = HashMap::new();

        if let Some(store) = store.as_ref() {
            let loaded = store.load_jobs();
            for mut job in loaded {
                if matches!(job.state, JobStateDto::Queued | JobStateDto::Running) {
                    job.state = JobStateDto::AbortedByRestart;
                    job.phase = "aborted_by_restart".to_string();
                    job.progress.percent = 0;
                    job.error = Some("job aborted by restart".to_string());
                    job.updated_at = now_unix_secs();
                    job.finished_at = Some(job.updated_at);
                    store.write_job(&job);
                    store.remove_result_file(&job.job_id);
                }

                let job_id = match Uuid::parse_str(&job.job_id) {
                    Ok(value) => value,
                    Err(_) => continue,
                };

                let result = store.load_result(&job.job_id);
                map.insert(
                    job_id,
                    Arc::new(JobEntry {
                        job: RwLock::new(job),
                        result: RwLock::new(result),
                        notify: Notify::new(),
                        canceled: AtomicBool::new(false),
                    }),
                );
            }
        }

        let (interactive_limit, background_limit) = Self::default_concurrency_limits();
        Self {
            store,
            jobs: Arc::new(RwLock::new(map)),
            interactive_limiter: Arc::new(Semaphore::new(interactive_limit)),
            background_limiter: Arc::new(Semaphore::new(background_limit)),
        }
    }

    pub fn new_in_memory() -> Self {
        let (interactive_limit, background_limit) = Self::default_concurrency_limits();
        Self {
            store: None,
            jobs: Arc::new(RwLock::new(HashMap::new())),
            interactive_limiter: Arc::new(Semaphore::new(interactive_limit)),
            background_limiter: Arc::new(Semaphore::new(background_limit)),
        }
    }

    pub async fn spawn<F, Fut>(&self, phase: impl Into<String>, job_fn: F) -> String
    where
        F: FnOnce(JobContext) -> Fut + Send + 'static,
        Fut:
            std::future::Future<Output = Result<serde_json::Value, anyhow::Error>> + Send + 'static,
    {
        self.spawn_with_class(phase, CpuWorkClass::Background, job_fn)
            .await
    }

    pub async fn spawn_with_class<F, Fut>(
        &self,
        phase: impl Into<String>,
        class: CpuWorkClass,
        job_fn: F,
    ) -> String
    where
        F: FnOnce(JobContext) -> Fut + Send + 'static,
        Fut:
            std::future::Future<Output = Result<serde_json::Value, anyhow::Error>> + Send + 'static,
    {
        let phase = phase.into();
        let job_id = Uuid::new_v4();
        let now = now_unix_secs();
        let job = PersistedJob {
            job_id: job_id.to_string(),
            state: JobStateDto::Queued,
            phase: phase.clone(),
            progress: ProgressDto { percent: 0 },
            error: None,
            created_at: now,
            started_at: None,
            finished_at: None,
            updated_at: now,
        };

        let entry = Arc::new(JobEntry {
            job: RwLock::new(job),
            result: RwLock::new(None),
            notify: Notify::new(),
            canceled: AtomicBool::new(false),
        });
        self.jobs.write().await.insert(job_id, Arc::clone(&entry));
        self.persist_job(&entry).await;

        let store = self.store.clone();
        let jobs = self.jobs.clone();
        let limiter = match class {
            CpuWorkClass::Interactive => Arc::clone(&self.interactive_limiter),
            CpuWorkClass::Background => Arc::clone(&self.background_limiter),
        };
        let job_id_str = job_id.to_string();
        tracing::info!(
            job_id = %job_id_str,
            phase = %phase,
            cpu_class = ?class,
            "job queued"
        );
        let job_id_for_task = job_id_str.clone();
        let job_id_str_for_cleanup = job_id_str.clone();
        let phase_for_task = phase.clone();
        let class_for_task = class;
        let ctx = JobContext {
            job_id: job_id_str.clone(),
            entry: Arc::clone(&entry),
            store: store.clone(),
        };

        tokio::spawn(async move {
            let _permit = limiter
                .acquire_owned()
                .await
                .expect("job class concurrency limiter closed");

            if entry.canceled.load(Ordering::Relaxed) {
                tracing::info!(
                    job_id = %job_id_for_task,
                    phase = %phase_for_task,
                    cpu_class = ?class_for_task,
                    "job canceled before start"
                );
                {
                    let mut job = entry.job.write().await;
                    job.state = JobStateDto::Canceled;
                    job.phase = "canceled".to_string();
                    job.error = Some("job canceled".to_string());
                    job.updated_at = now_unix_secs();
                    job.finished_at = Some(job.updated_at);
                }
                if let Some(store) = store.as_ref() {
                    let job = entry.job.read().await.clone();
                    store.write_job(&job);
                }
                entry.notify.notify_waiters();
                return;
            }

            {
                let mut job = entry.job.write().await;
                job.state = JobStateDto::Running;
                job.started_at = Some(now_unix_secs());
                job.updated_at = now_unix_secs();
            }
            if let Some(store) = store.as_ref() {
                let job = entry.job.read().await.clone();
                store.write_job(&job);
            }
            entry.notify.notify_waiters();
            tracing::info!(
                job_id = %job_id_for_task,
                phase = %phase_for_task,
                cpu_class = ?class_for_task,
                "job started"
            );

            let result = job_fn(ctx.clone()).await;
            if entry.canceled.load(Ordering::Relaxed) {
                tracing::info!(
                    job_id = %job_id_for_task,
                    phase = %phase_for_task,
                    cpu_class = ?class_for_task,
                    "job canceled while running"
                );
                entry.notify.notify_waiters();
                return;
            }
            match result {
                Ok(value) => {
                    {
                        let mut result_slot = entry.result.write().await;
                        *result_slot = Some(value.clone());
                    }
                    {
                        let mut job = entry.job.write().await;
                        job.state = JobStateDto::Succeeded;
                        job.phase = "finished".to_string();
                        job.progress.percent = 100;
                        job.updated_at = now_unix_secs();
                        job.finished_at = Some(job.updated_at);
                    }
                    if let Some(store) = store.as_ref() {
                        let job = entry.job.read().await.clone();
                        store.write_job(&job);
                        store.write_result(&job.job_id, &value);
                    }
                    tracing::info!(
                        job_id = %job_id_for_task,
                        phase = %phase_for_task,
                        cpu_class = ?class_for_task,
                        "job succeeded"
                    );
                }
                Err(err) => {
                    {
                        let mut job = entry.job.write().await;
                        job.state = JobStateDto::Failed;
                        job.phase = "failed".to_string();
                        job.error = Some(err.to_string());
                        job.updated_at = now_unix_secs();
                        job.finished_at = Some(job.updated_at);
                    }
                    if let Some(store) = store.as_ref() {
                        let job = entry.job.read().await.clone();
                        store.write_job(&job);
                    }
                    tracing::warn!(
                        job_id = %job_id_for_task,
                        phase = %phase_for_task,
                        cpu_class = ?class_for_task,
                        error = %err,
                        "job failed"
                    );
                }
            }

            entry.notify.notify_waiters();

            let ttl_secs = JobStore::ttl_secs();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(ttl_secs)).await;
                jobs.write().await.remove(&job_id);
                if let Some(store) = store.as_ref() {
                    store.remove_job_files(&job_id_str_for_cleanup);
                }
            });
        });

        job_id_str
    }

    pub async fn status(&self, job_id: &str) -> Result<JobStatusResponse, rmcp::ErrorData> {
        let uuid = parse_job_id(job_id)?;
        let jobs = self.jobs.read().await;
        let entry = jobs
            .get(&uuid)
            .ok_or_else(|| rmcp::ErrorData::invalid_params("job not found", None))?;
        let job = entry.job.read().await;
        Ok(job.as_status())
    }

    pub async fn list_statuses(&self) -> Vec<JobStatusResponse> {
        let entries: Vec<Arc<JobEntry>> = {
            let jobs = self.jobs.read().await;
            jobs.values().cloned().collect()
        };

        let mut statuses = Vec::with_capacity(entries.len());
        for entry in entries {
            let job = entry.job.read().await;
            statuses.push(job.as_status());
        }

        statuses.sort_by(|a, b| a.job_id.cmp(&b.job_id));
        statuses
    }

    pub async fn wait(
        &self,
        job_id: &str,
        timeout_ms: u64,
    ) -> Result<JobStatusResponse, rmcp::ErrorData> {
        let uuid = parse_job_id(job_id)?;
        let entry = {
            let jobs = self.jobs.read().await;
            jobs.get(&uuid)
                .cloned()
                .ok_or_else(|| rmcp::ErrorData::invalid_params("job not found", None))?
        };

        {
            let job = entry.job.read().await;
            if job.is_terminal() {
                tracing::info!(
                    job_id = job_id,
                    timeout_ms,
                    state = job.state.as_str(),
                    phase = %job.phase,
                    "job_wait returning terminal status immediately"
                );
                return Ok(job.as_status());
            }
        }

        tracing::info!(job_id = job_id, timeout_ms, "job_wait begin");
        let timeout = Duration::from_millis(timeout_ms);
        let notified = entry.notify.notified();
        let wake_reason = tokio::select! {
            _ = notified => "notify",
            _ = tokio::time::sleep(timeout) => "timeout",
        };

        let job = entry.job.read().await;
        tracing::info!(
            job_id = job_id,
            timeout_ms,
            wake_reason,
            state = job.state.as_str(),
            phase = %job.phase,
            "job_wait returning status"
        );
        Ok(job.as_status())
    }

    pub async fn result(&self, job_id: &str) -> Result<serde_json::Value, rmcp::ErrorData> {
        let uuid = parse_job_id(job_id)?;
        let entry = {
            let jobs = self.jobs.read().await;
            jobs.get(&uuid)
                .cloned()
                .ok_or_else(|| rmcp::ErrorData::invalid_params("job not found", None))?
        };

        let job = entry.job.read().await;
        if job.state != JobStateDto::Succeeded {
            let message = job
                .error
                .clone()
                .unwrap_or_else(|| format!("job is not succeeded: {}", job.state.as_str()));
            return Err(rmcp::ErrorData::invalid_params(message, None));
        }
        drop(job);

        let value = entry
            .result
            .read()
            .await
            .clone()
            .ok_or_else(|| rmcp::ErrorData::internal_error("missing job result", None))?;
        Ok(value)
    }

    pub async fn cancel(&self, job_id: &str) -> Result<JobCancelResponse, rmcp::ErrorData> {
        let uuid = parse_job_id(job_id)?;
        let entry = {
            let jobs = self.jobs.read().await;
            jobs.get(&uuid)
                .cloned()
                .ok_or_else(|| rmcp::ErrorData::invalid_params("job not found", None))?
        };

        let mut current_state = JobStateDto::Canceled;
        let mut should_cancel = false;
        {
            let mut job = entry.job.write().await;
            if job.is_terminal() {
                current_state = job.state;
            } else {
                should_cancel = true;
                entry.canceled.store(true, Ordering::Relaxed);
                job.state = JobStateDto::Canceled;
                job.phase = "canceled".to_string();
                job.error = Some("job canceled".to_string());
                job.updated_at = now_unix_secs();
                job.finished_at = Some(job.updated_at);
            }
        }

        if !should_cancel {
            return Ok(JobCancelResponse {
                ok: true,
                job_id: job_id.to_string(),
                state: current_state,
            });
        }

        {
            let mut result_slot = entry.result.write().await;
            *result_slot = None;
        }
        if let Some(store) = self.store.as_ref() {
            store.remove_result_file(job_id);
        }

        self.persist_job(&entry).await;
        entry.notify.notify_waiters();

        Ok(JobCancelResponse {
            ok: true,
            job_id: job_id.to_string(),
            state: JobStateDto::Canceled,
        })
    }

    async fn persist_job(&self, entry: &JobEntry) {
        let Some(store) = self.store.as_ref() else {
            return;
        };
        let job = entry.job.read().await.clone();
        store.write_job(&job);
    }
}

impl Default for JobManager {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_job_id(job_id: &str) -> Result<Uuid, rmcp::ErrorData> {
    Uuid::parse_str(job_id).map_err(|_| rmcp::ErrorData::invalid_params("invalid job_id", None))
}

#[cfg(test)]
mod tests;
