use super::*;
use serde_json::json;
use tokio::sync::oneshot;

#[tokio::test]
async fn running_job_progress_never_reports_100() {
    let manager = JobManager::new_in_memory();
    let (allow_finish_tx, allow_finish_rx) = oneshot::channel::<()>();
    let (reported_tx, reported_rx) = oneshot::channel::<()>();

    let job_id = manager
        .spawn("test", move |ctx| async move {
            ctx.set_progress("test/running", 100).await;
            let _ = reported_tx.send(());
            let _ = allow_finish_rx.await;
            Ok(json!({ "ok": true }))
        })
        .await;

    let _ = reported_rx.await;
    let status = manager.status(&job_id).await.expect("job_status");
    assert!(matches!(
        status.state,
        JobStateDto::Queued | JobStateDto::Running
    ));
    assert_eq!(status.progress.percent, 99);

    let _ = allow_finish_tx.send(());
    let done = manager.wait(&job_id, 60_000).await.expect("job_wait");
    assert_eq!(done.state, JobStateDto::Succeeded);
    assert_eq!(done.progress.percent, 100);
}
