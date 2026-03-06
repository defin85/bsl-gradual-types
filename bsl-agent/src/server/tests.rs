use super::*;
use crate::server::types::{
    DocumentRef, FileRef, JobCancelParams, WorkspaceDocumentsSetFile, WorkspaceDocumentsSetParams,
    WorkspaceOpenParams,
};
use crate::types::{JobStateDto, WorkspaceOpenResponse};
use rmcp::handler::server::wrapper::Parameters;
use std::time::Duration;

async fn wait_workspace_ready(
    session_manager: &Arc<SessionManager>,
    job_manager: &Arc<JobManager>,
    open: &WorkspaceOpenResponse,
) {
    let startup_job_id = open
        .startup_job_id
        .as_ref()
        .expect("startup_job_id")
        .clone();

    let startup = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let status = job_manager
                .wait(&startup_job_id, 200)
                .await
                .expect("startup wait");
            if !matches!(status.state, JobStateDto::Queued | JobStateDto::Running) {
                break status;
            }
        }
    })
    .await
    .expect("startup must reach terminal state");
    assert_eq!(
        startup.state,
        JobStateDto::Succeeded,
        "startup job must succeed"
    );

    let ready = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let status = session_manager
                .status(&open.session_id)
                .await
                .expect("status");
            if status.ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    assert!(ready.is_ok(), "workspace must become ready after startup");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_stale_batch_jobs_cancels_running_jobs() {
    let session_manager = Arc::new(SessionManager::new());
    let job_manager = Arc::new(JobManager::new_in_memory());
    let handler = BslAgentHandler::with_state(session_manager, Arc::clone(&job_manager));

    let job_id = job_manager
        .spawn_with_class(
            "batch-test",
            cpu_work_class_for_operation(SemanticOperation::SymbolSearch),
            move |_| async move {
                tokio::time::sleep(Duration::from_secs(5)).await;
                Ok(serde_json::json!({ "ok": true }))
            },
        )
        .await;

    handler
        .register_batch_job("session-1", 1, &job_id, DiagnosticsProfile::DebouncedFull)
        .await;
    handler.cancel_stale_batch_jobs("session-1", 2).await;

    let status = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let status = job_manager.status(&job_id).await.expect("status");
            if matches!(status.state, JobStateDto::Canceled) {
                break status;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("stale batch job must transition to canceled");
    assert_eq!(
        status.state,
        JobStateDto::Canceled,
        "stale running batch job must be canceled on revision advance"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_stale_batch_jobs_does_not_rewrite_terminal_state() {
    let session_manager = Arc::new(SessionManager::new());
    let job_manager = Arc::new(JobManager::new_in_memory());
    let handler = BslAgentHandler::with_state(session_manager, Arc::clone(&job_manager));

    let job_id = job_manager
        .spawn_with_class(
            "batch-test-finished",
            cpu_work_class_for_operation(SemanticOperation::SymbolSearch),
            move |_| async move { Ok(serde_json::json!({ "done": true })) },
        )
        .await;
    let waited = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let status = job_manager.status(&job_id).await.expect("status");
            if !matches!(status.state, JobStateDto::Queued | JobStateDto::Running) {
                break status;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("job should reach terminal state");
    assert_eq!(
        waited.state,
        JobStateDto::Succeeded,
        "test precondition: batch job must finish before stale cancellation"
    );

    handler
        .register_batch_job("session-1", 1, &job_id, DiagnosticsProfile::DebouncedFull)
        .await;
    handler.cancel_stale_batch_jobs("session-1", 2).await;

    let status = job_manager.status(&job_id).await.expect("status");
    assert_eq!(
        status.state,
        JobStateDto::Succeeded,
        "terminal jobs must stay terminal when cleanup scans stale entries"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn job_cancel_records_client_cancel_reason_for_tracked_batch_job() {
    let session_manager = Arc::new(SessionManager::new());
    let job_manager = Arc::new(JobManager::new_in_memory());
    let handler =
        BslAgentHandler::with_state(Arc::clone(&session_manager), Arc::clone(&job_manager));

    let root = tempfile::TempDir::new().expect("tempdir");
    let open = session_manager
        .open(
            WorkspaceOpenParams {
                roots: vec![root.path().to_string_lossy().to_string()],
                platform_docs_archive: None,
                platform_version: None,
                configuration_path: None,
                mode: None,
            },
            Arc::clone(&job_manager),
        )
        .await
        .expect("workspace_open");
    wait_workspace_ready(&session_manager, &job_manager, &open).await;

    let job_id = job_manager
        .spawn_with_class(
            "batch-cancel-observability",
            cpu_work_class_for_operation(SemanticOperation::SymbolSearch),
            move |_| async move {
                tokio::time::sleep(Duration::from_secs(5)).await;
                Ok(serde_json::json!({ "ok": true }))
            },
        )
        .await;

    handler
        .register_batch_job(
            &open.session_id,
            open.analysis_revision,
            &job_id,
            DiagnosticsProfile::DebouncedFull,
        )
        .await;

    let _ = handler
        .job_cancel(Parameters(JobCancelParams {
            job_id: job_id.clone(),
        }))
        .await
        .expect("job_cancel");

    let status = job_manager.status(&job_id).await.expect("job status");
    assert_eq!(
        status.state,
        JobStateDto::Canceled,
        "batch job must be canceled"
    );

    let metrics = session_manager
        .observability_metrics_get(&open.session_id)
        .await
        .expect("metrics");
    let counters = metrics
        .metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let key = "intellisense_v2_diagnostics_pipeline_total_origin_agent_trigger_job_start_profile_debounced_full_reason_client_cancel";
    let value = counters
        .get(key)
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    assert!(
        value > 0,
        "expected diagnostics pipeline client_cancel metric key {key} to be incremented"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn workspace_documents_set_records_superseded_generation_for_stale_batch_jobs() {
    let session_manager = Arc::new(SessionManager::new());
    let job_manager = Arc::new(JobManager::new_in_memory());
    let handler =
        BslAgentHandler::with_state(Arc::clone(&session_manager), Arc::clone(&job_manager));

    let root = tempfile::TempDir::new().expect("tempdir");
    let open = session_manager
        .open(
            WorkspaceOpenParams {
                roots: vec![root.path().to_string_lossy().to_string()],
                platform_docs_archive: None,
                platform_version: None,
                configuration_path: None,
                mode: None,
            },
            Arc::clone(&job_manager),
        )
        .await
        .expect("workspace_open");
    wait_workspace_ready(&session_manager, &job_manager, &open).await;

    let job_id = job_manager
        .spawn_with_class(
            "batch-documents-set-observability",
            cpu_work_class_for_operation(SemanticOperation::SymbolSearch),
            move |_| async move {
                tokio::time::sleep(Duration::from_secs(5)).await;
                Ok(serde_json::json!({ "ok": true }))
            },
        )
        .await;
    handler
        .register_batch_job(
            &open.session_id,
            open.analysis_revision,
            &job_id,
            DiagnosticsProfile::DebouncedFull,
        )
        .await;

    let overlay_path = root.path().join("Module.bsl");
    let _ = handler
        .workspace_documents_set(Parameters(WorkspaceDocumentsSetParams {
            session_id: open.session_id.clone(),
            files: vec![WorkspaceDocumentsSetFile::File(FileRef {
                doc: DocumentRef::Path(overlay_path.to_string_lossy().to_string()),
                text: Some("Procedure T()\nEndProcedure\n".to_string()),
                version: Some(1),
            })],
            mark_hot: true,
        }))
        .await
        .expect("workspace_documents_set");

    let status = job_manager.status(&job_id).await.expect("job status");
    assert_eq!(
        status.state,
        JobStateDto::Canceled,
        "stale batch job must be canceled after documents_set revision bump"
    );

    let metrics = session_manager
        .observability_metrics_get(&open.session_id)
        .await
        .expect("metrics");
    let counters = metrics
        .metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let key = "intellisense_v2_diagnostics_pipeline_total_origin_agent_trigger_documents_set_profile_debounced_full_reason_superseded_generation";
    let value = counters
        .get(key)
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    assert!(
        value > 0,
        "expected diagnostics pipeline superseded metric key {key} to be incremented"
    );
}
