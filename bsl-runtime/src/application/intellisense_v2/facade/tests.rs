use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::time::{timeout, Duration};

use bsl_analysis_v2::{LineIndex, ParseChangedRange, ParseSnapshot};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::resolver::TypeResolver;
use bsl_syntax::ParseOptions;
use tree_sitter::{Parser as TreeSitterParser, Tree};

#[tokio::test]
async fn p7_apply_changes_and_wait_for_version_works() {
    let runtime = IntellisenseV2Facade::new(
        AnalysisHostV2::default(),
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p7"))),
        None,
    );
    let file_id = FileId(1);

    runtime.apply_changes(vec![Change::SetFile {
        file_id,
        text: Arc::from("abc"),
        version: 7,
        path: Arc::from("test.bsl"),
    }]);

    let ok = timeout(
        Duration::from_secs(1),
        runtime.wait_for_file_version(file_id, 7),
    )
    .await
    .expect("wait_for_file_version timeout");
    assert!(ok, "expected wait_for_file_version to succeed");

    let analysis = runtime.snapshot().await;
    assert_eq!(analysis.file_version(file_id).unwrap(), Some(7));

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn p7_waiters_are_released_on_shutdown() {
    let runtime = IntellisenseV2Facade::new(
        AnalysisHostV2::default(),
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p7"))),
        None,
    );
    let file_id = FileId(1);

    let wait_task = tokio::spawn({
        let runtime = runtime.clone();
        async move { runtime.wait_for_file_version(file_id, 42).await }
    });

    runtime.shutdown_for_test().await;

    let ok = timeout(Duration::from_secs(1), wait_task)
        .await
        .expect("wait task timeout")
        .expect("wait task join");
    assert!(!ok, "expected waiter to return false on shutdown");
}

#[tokio::test]
async fn interactive_commands_preempt_background_backlog() {
    let runtime = IntellisenseV2Facade::new(
        AnalysisHostV2::default(),
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p7"))),
        None,
    );
    let mut sleepers = Vec::new();
    for _ in 0..6 {
        sleepers.push(
            runtime.enqueue_test_sleep(RuntimeQueuePriority::Background, Duration::from_millis(40)),
        );
    }

    let started = Instant::now();
    let interactive_ack = runtime.enqueue_test_noop(RuntimeQueuePriority::Interactive);
    timeout(Duration::from_millis(120), interactive_ack)
        .await
        .expect("interactive noop must not wait for full background backlog")
        .expect("interactive noop ack");
    assert!(
        started.elapsed() < Duration::from_millis(120),
        "interactive noop should complete before background backlog drains"
    );

    for sleeper_ack in sleepers {
        timeout(Duration::from_secs(1), sleeper_ack)
            .await
            .expect("background sleeper ack timeout")
            .expect("background sleeper ack");
    }

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn interactive_apply_changes_preempts_background_backlog_for_wait_for_file_version() {
    let runtime = IntellisenseV2Facade::new(
        AnalysisHostV2::default(),
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p7"))),
        None,
    );
    let file_id = FileId(19);

    let mut sleepers = Vec::new();
    for _ in 0..6 {
        sleepers.push(
            runtime.enqueue_test_sleep(RuntimeQueuePriority::Background, Duration::from_millis(40)),
        );
    }

    runtime.apply_changes_interactive(
        ObservabilityOrigin::Lsp,
        vec![Change::SetFile {
            file_id,
            text: Arc::from("x = 1;"),
            version: 7,
            path: Arc::from("interactive_apply_latest.bsl"),
        }],
    );

    let wait_result = timeout(
        Duration::from_millis(120),
        runtime.wait_for_file_version_with_priority(
            ObservabilityOrigin::Lsp,
            RuntimeQueuePriority::Interactive,
            file_id,
            7,
        ),
    )
    .await
    .expect(
        "interactive apply_changes should let interactive wait_for_file_version complete before background backlog drains",
    );
    let wait_result = wait_result.ready;
    assert!(
        wait_result,
        "wait_for_file_version should observe the interactively enqueued revision"
    );

    for sleeper_ack in sleepers {
        timeout(Duration::from_secs(1), sleeper_ack)
            .await
            .expect("background sleeper ack timeout")
            .expect("background sleeper ack");
    }

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn interactive_snapshot_for_completion_preempts_background_backlog() {
    let index_snapshot = Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p7")));
    let runtime =
        IntellisenseV2Facade::new(AnalysisHostV2::default(), index_snapshot.clone(), None);
    let file_id = FileId(20);

    runtime.apply_changes_interactive(
        ObservabilityOrigin::Lsp,
        vec![Change::SetFile {
            file_id,
            text: Arc::from("x = 1;"),
            version: 7,
            path: Arc::from("interactive_snapshot_latest.bsl"),
        }],
    );
    let applied = timeout(
        Duration::from_millis(120),
        runtime.wait_for_file_version_with_priority(
            ObservabilityOrigin::Lsp,
            RuntimeQueuePriority::Interactive,
            file_id,
            7,
        ),
    )
    .await
    .expect("interactive apply must publish the latest version before snapshot probe")
    .ready;
    assert!(applied, "interactive apply must publish the latest version");

    let mut sleepers = Vec::new();
    for _ in 0..6 {
        sleepers.push(
            runtime.enqueue_test_sleep(RuntimeQueuePriority::Background, Duration::from_millis(40)),
        );
    }

    let snapshot = timeout(
        Duration::from_millis(120),
        runtime.completion_current_revision_snapshot_for_origin_and_operation(
            ObservabilityOrigin::Lsp,
            SemanticOperation::Completion,
        ),
    )
    .await
    .expect("completion current-revision snapshot must not wait for the full background backlog");
    assert_eq!(snapshot.analysis.file_version(file_id).unwrap(), Some(7));
    assert_eq!(
        snapshot.index_snapshot.id.as_str(),
        index_snapshot.id.as_str()
    );

    for sleeper_ack in sleepers {
        timeout(Duration::from_secs(1), sleeper_ack)
            .await
            .expect("background sleeper ack timeout")
            .expect("background sleeper ack");
    }

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn interactive_set_file_burst_coalesces_latest_version_for_wait_for_file_version() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    static ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    let _env_lock = ENV_LOCK
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .expect("env lock");
    let _apply_delay_guard = EnvVarGuard::set("BSL_TEST_RUNTIME_APPLY_SET_FILE_DELAY_MS", "80");

    let runtime = IntellisenseV2Facade::new(
        AnalysisHostV2::default(),
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p7"))),
        None,
    );
    let file_id = FileId(21);

    for version in 1..=3 {
        runtime.apply_changes_interactive(
            ObservabilityOrigin::Lsp,
            vec![Change::SetFile {
                file_id,
                text: Arc::from(format!("x = {version};")),
                version,
                path: Arc::from("interactive_burst_latest.bsl"),
            }],
        );
    }

    let started = Instant::now();
    let wait_result = timeout(
        Duration::from_millis(170),
        runtime.wait_for_file_version_with_priority(
            ObservabilityOrigin::Lsp,
            RuntimeQueuePriority::Interactive,
            file_id,
            3,
        ),
    )
    .await
    .expect(
        "interactive SetFile burst should not require sequentially applying superseded revisions",
    );
    assert!(
        wait_result.ready,
        "wait_for_file_version must observe the latest burst revision"
    );
    assert!(
        started.elapsed() < Duration::from_millis(170),
        "latest burst revision must become applied within a single-delay budget"
    );

    let analysis = runtime.snapshot().await;
    assert_eq!(analysis.file_version(file_id).unwrap(), Some(3));

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn delayed_interactive_set_file_burst_still_coalesces_latest_version() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    static ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    let _env_lock = ENV_LOCK
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .expect("env lock");
    let _apply_delay_guard = EnvVarGuard::set("BSL_TEST_RUNTIME_APPLY_SET_FILE_DELAY_MS", "80");

    let runtime = IntellisenseV2Facade::new(
        AnalysisHostV2::default(),
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p7"))),
        None,
    );
    let file_id = FileId(211);

    runtime.apply_changes_interactive(
        ObservabilityOrigin::Lsp,
        vec![Change::SetFile {
            file_id,
            text: Arc::from("x = 1;"),
            version: 1,
            path: Arc::from("interactive_burst_delayed_latest.bsl"),
        }],
    );

    let burst_sender = tokio::spawn({
        let runtime = runtime.clone();
        async move {
            tokio::time::sleep(Duration::from_millis(1)).await;
            for version in 2..=4 {
                runtime.apply_changes_interactive(
                    ObservabilityOrigin::Lsp,
                    vec![Change::SetFile {
                        file_id,
                        text: Arc::from(format!("x = {version};")),
                        version,
                        path: Arc::from("interactive_burst_delayed_latest.bsl"),
                    }],
                );
            }
        }
    });

    let started = Instant::now();
    let wait_result = timeout(
        Duration::from_millis(170),
        runtime.wait_for_file_version_with_priority(
            ObservabilityOrigin::Lsp,
            RuntimeQueuePriority::Interactive,
            file_id,
            4,
        ),
    )
    .await
    .expect(
        "delayed interactive SetFile burst should still converge to the latest revision without sequential superseded applies",
    );
    assert!(
        wait_result.ready,
        "wait_for_file_version must observe the latest delayed burst revision"
    );
    assert!(
        started.elapsed() < Duration::from_millis(170),
        "latest delayed burst revision must become applied within a single-delay budget"
    );

    burst_sender.await.expect("burst sender task");
    let analysis = runtime.snapshot().await;
    assert_eq!(analysis.file_version(file_id).unwrap(), Some(4));

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn interleaved_interactive_non_apply_work_does_not_strand_latest_set_file() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    static ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    let _env_lock = ENV_LOCK
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .expect("env lock");
    let _apply_delay_guard = EnvVarGuard::set("BSL_TEST_RUNTIME_APPLY_SET_FILE_DELAY_MS", "80");

    let runtime = IntellisenseV2Facade::new(
        AnalysisHostV2::default(),
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p7"))),
        None,
    );
    let file_id = FileId(213);

    runtime.apply_changes_interactive(
        ObservabilityOrigin::Lsp,
        vec![Change::SetFile {
            file_id,
            text: Arc::from("x = 1;"),
            version: 1,
            path: Arc::from("interactive_interleaved_latest.bsl"),
        }],
    );
    let stale_interactive_ack =
        runtime.enqueue_test_sleep(RuntimeQueuePriority::Interactive, Duration::from_millis(80));
    for version in 2..=3 {
        runtime.apply_changes_interactive(
            ObservabilityOrigin::Lsp,
            vec![Change::SetFile {
                file_id,
                text: Arc::from(format!("x = {version};")),
                version,
                path: Arc::from("interactive_interleaved_latest.bsl"),
            }],
        );
    }

    let started = Instant::now();
    let wait_result = timeout(
        Duration::from_millis(170),
        runtime.wait_for_file_version_with_priority(
            ObservabilityOrigin::Lsp,
            RuntimeQueuePriority::Interactive,
            file_id,
            3,
        ),
    )
    .await
    .expect(
        "latest interactive SetFile should not be stranded behind interleaved stale interactive work",
    );
    assert!(
        wait_result.ready,
        "wait_for_file_version must observe the latest interleaved revision"
    );
    assert!(
        started.elapsed() < Duration::from_millis(170),
        "latest interleaved revision must become applied within a single-delay budget"
    );

    timeout(Duration::from_secs(1), stale_interactive_ack)
        .await
        .expect("stale interactive sleeper ack timeout")
        .expect("stale interactive sleeper ack");

    let analysis = runtime.snapshot().await;
    assert_eq!(analysis.file_version(file_id).unwrap(), Some(3));

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn pending_interactive_backlog_does_not_block_later_lsp_set_file() {
    let runtime = IntellisenseV2Facade::new(
        AnalysisHostV2::default(),
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p7"))),
        None,
    );
    let file_id = FileId(214);

    runtime.apply_changes_interactive(
        ObservabilityOrigin::Lsp,
        vec![Change::SetFile {
            file_id,
            text: Arc::from("x = 1;"),
            version: 1,
            path: Arc::from("interactive_pending_backlog.bsl"),
        }],
    );
    let mut sleepers = Vec::new();
    for _ in 0..4 {
        sleepers.push(
            runtime
                .enqueue_test_sleep(RuntimeQueuePriority::Interactive, Duration::from_millis(60)),
        );
    }

    tokio::time::sleep(Duration::from_millis(10)).await;

    runtime.apply_changes_interactive(
        ObservabilityOrigin::Lsp,
        vec![Change::SetFile {
            file_id,
            text: Arc::from("x = 2;"),
            version: 2,
            path: Arc::from("interactive_pending_backlog.bsl"),
        }],
    );

    let started = Instant::now();
    let wait_result = timeout(
        Duration::from_millis(140),
        runtime.wait_for_file_version_with_priority(
            ObservabilityOrigin::Lsp,
            RuntimeQueuePriority::Interactive,
            file_id,
            2,
        ),
    )
    .await
    .expect("later LSP SetFile should not wait behind stale pending interactive backlog");
    assert!(
        wait_result.ready,
        "wait_for_file_version must observe the later LSP SetFile despite pending interactive backlog"
    );
    assert!(
        started.elapsed() < Duration::from_millis(140),
        "later LSP SetFile must preempt stale pending interactive backlog"
    );

    for sleeper_ack in sleepers {
        timeout(Duration::from_secs(1), sleeper_ack)
            .await
            .expect("interactive sleeper ack timeout")
            .expect("interactive sleeper ack");
    }

    let analysis = runtime.snapshot().await;
    assert_eq!(analysis.file_version(file_id).unwrap(), Some(2));

    runtime.shutdown_for_test().await;
}

#[test]
fn coalesced_whitespace_append_burst_preserves_earliest_reuse_base() {
    let (tx, rx) = std::sync::mpsc::channel();
    let file_id = FileId(212);
    let path: Arc<str> = Arc::from("<coalesced-head-reuse-chain>");

    let first = Command::ApplyChanges {
        origin: ObservabilityOrigin::Lsp,
        enqueued_at: Instant::now(),
        changes: vec![
            Change::SetFile {
                file_id,
                text: Arc::from("v2"),
                version: 2,
                path: path.clone(),
            },
            Change::ReuseCompletionHeadFromPreviousVersion {
                file_id,
                expected_version: 2,
                previous_version: 1,
            },
        ],
    };

    for version in 3..=5 {
        tx.send(Command::ApplyChanges {
            origin: ObservabilityOrigin::Lsp,
            enqueued_at: Instant::now(),
            changes: vec![
                Change::SetFile {
                    file_id,
                    text: Arc::from(format!("v{version}")),
                    version,
                    path: path.clone(),
                },
                Change::ReuseCompletionHeadFromPreviousVersion {
                    file_id,
                    expected_version: version,
                    previous_version: version - 1,
                },
            ],
        })
        .expect("enqueue coalesced burst command");
    }
    drop(tx);

    let mut pending = std::collections::VecDeque::new();
    let coalesced = coalesce_interactive_current_revision_apply_command(&rx, first, &mut pending);
    assert!(
        pending.is_empty(),
        "burst should coalesce into a single latest command"
    );

    let Command::ApplyChanges { changes, .. } = coalesced else {
        panic!("coalesced command must stay ApplyChanges");
    };
    match changes.as_slice() {
        [Change::SetFile {
            file_id: set_file_id,
            text,
            version,
            path: set_file_path,
        }, Change::ReuseCompletionHeadFromPreviousVersion {
            file_id: reuse_file_id,
            expected_version,
            previous_version,
        }] => {
            assert_eq!(*set_file_id, file_id);
            assert_eq!(text.as_ref(), "v5");
            assert_eq!(*version, 5);
            assert_eq!(set_file_path.as_ref(), path.as_ref());
            assert_eq!(*reuse_file_id, file_id);
            assert_eq!(*expected_version, 5);
            assert_eq!(
                *previous_version, 1,
                "coalesced whitespace-append chain must preserve the earliest reusable base revision for the latest SetFile"
            );
        }
        other => panic!("unexpected coalesced change shape: {other:?}"),
    }
}

#[tokio::test]
async fn stale_background_snapshot_apply_does_not_regress_newer_interactive_revision() {
    let runtime = IntellisenseV2Facade::new(
        AnalysisHostV2::default(),
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p7"))),
        None,
    );
    let file_id = FileId(22);
    let latest_text: Arc<str> = Arc::from("x = 2;");
    let stale_text: Arc<str> = Arc::from("x = 1;");

    runtime.apply_changes_interactive(
        ObservabilityOrigin::Lsp,
        vec![Change::SetFile {
            file_id,
            text: latest_text.clone(),
            version: 2,
            path: Arc::from("stale_snapshot_guard.bsl"),
        }],
    );
    let ready = timeout(
        Duration::from_secs(1),
        runtime.wait_for_file_version(file_id, 2),
    )
    .await
    .expect("wait_for_file_version timeout");
    assert!(
        ready,
        "latest interactive revision must become visible first"
    );

    runtime.apply_changes(vec![Change::SetFileWithSnapshot {
        file_id,
        text: stale_text,
        version: 1,
        path: Arc::from("stale_snapshot_guard.bsl"),
        parse_snapshot: parse_snapshot_for_test(file_id, 1, "x = 1;", vec![], false, None),
    }]);

    tokio::time::sleep(Duration::from_millis(50)).await;

    let analysis = runtime.snapshot().await;
    assert_eq!(analysis.file_version(file_id).unwrap(), Some(2));
    assert_eq!(
        analysis.file_text(file_id).ok().flatten().as_deref(),
        Some(latest_text.as_ref())
    );

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn wait_for_file_version_runtime_trace_distinguishes_immediate_and_waiter_paths() {
    let runtime = IntellisenseV2Facade::new(
        AnalysisHostV2::default(),
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p7"))),
        None,
    );
    let file_id = FileId(20);

    runtime.apply_changes_interactive(
        ObservabilityOrigin::Lsp,
        vec![Change::SetFile {
            file_id,
            text: Arc::from("x = 1;"),
            version: 7,
            path: Arc::from("wait_runtime_immediate.bsl"),
        }],
    );

    let immediate = runtime
        .wait_for_file_version_with_priority(
            ObservabilityOrigin::Lsp,
            RuntimeQueuePriority::Interactive,
            file_id,
            7,
        )
        .await;
    assert!(immediate.ready, "immediate wait must succeed");
    assert_eq!(
        immediate.trace.resolution,
        Some(WaitForFileVersionResolutionKind::Immediate)
    );
    assert!(immediate.trace.queue_wait_elapsed.is_some());
    assert!(immediate.trace.exec_elapsed.is_some());
    assert_eq!(immediate.trace.wake_wait_elapsed, None);

    let waiter_task = tokio::spawn({
        let runtime = runtime.clone();
        async move {
            runtime
                .wait_for_file_version_with_priority(
                    ObservabilityOrigin::Lsp,
                    RuntimeQueuePriority::Interactive,
                    file_id,
                    8,
                )
                .await
        }
    });
    tokio::time::sleep(Duration::from_millis(10)).await;
    runtime.apply_changes(vec![Change::SetFile {
        file_id,
        text: Arc::from("x = 2;"),
        version: 8,
        path: Arc::from("wait_runtime_waiter.bsl"),
    }]);
    let waiter = timeout(Duration::from_secs(1), waiter_task)
        .await
        .expect("waiter task timeout")
        .expect("waiter task join");
    assert!(waiter.ready, "waiter path must succeed after apply");
    assert_eq!(
        waiter.trace.resolution,
        Some(WaitForFileVersionResolutionKind::Waiter)
    );
    assert!(waiter.trace.queue_wait_elapsed.is_some());
    assert!(waiter.trace.exec_elapsed.is_some());
    assert!(waiter.trace.wake_wait_elapsed.is_some());

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn background_commands_make_progress_under_interactive_flood() {
    let runtime = IntellisenseV2Facade::new(
        AnalysisHostV2::default(),
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p7"))),
        None,
    );

    let mut interactive_sleep_acks = Vec::new();
    for _ in 0..100 {
        interactive_sleep_acks.push(
            runtime.enqueue_test_sleep(RuntimeQueuePriority::Interactive, Duration::from_millis(5)),
        );
    }
    let background_ack = runtime.enqueue_test_noop(RuntimeQueuePriority::Background);
    timeout(Duration::from_millis(200), background_ack)
        .await
        .expect("background command should make progress despite interactive flood")
        .expect("background noop ack");

    for interactive_ack in interactive_sleep_acks {
        timeout(Duration::from_secs(2), interactive_ack)
            .await
            .expect("interactive sleeper ack timeout")
            .expect("interactive sleeper ack");
    }

    runtime.shutdown_for_test().await;
}

fn make_deps() -> Arc<SemanticDeps> {
    let repository: Arc<dyn TypeRepository> = Arc::new(InMemoryTypeRepository::new());
    let signature_index = repository.get_signature_index_clone();
    let resolver = Some(Arc::new(TypeResolver::new(repository.clone())));
    let platform_signatures_loaded = repository.platform_docs_loaded();
    Arc::new(SemanticDeps {
        repository,
        signature_index,
        resolver,
        platform_signatures_loaded,
    })
}

fn make_index_snapshot(raw_id: &str) -> Arc<IndexSnapshot> {
    Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash(raw_id)))
}

fn parse_backend_tree_for_test(text: &str) -> Arc<Tree> {
    let mut parser = TreeSitterParser::new();
    parser
        .set_language(&tree_sitter_bsl::LANGUAGE.into())
        .expect("tree-sitter-bsl language");
    Arc::new(
        parser
            .parse(text, None)
            .expect("tree-sitter parse for snapshot"),
    )
}

fn parse_snapshot_for_test(
    file_id: FileId,
    file_version: i32,
    text: &str,
    changed_ranges: Vec<ParseChangedRange>,
    incremental: bool,
    fallback_reason: Option<&str>,
) -> ParseSnapshot {
    ParseSnapshot {
        file_id,
        file_version,
        parse_result: Arc::new(
            bsl_syntax::parse(text, &ParseOptions::default()).expect("snapshot parse"),
        ),
        line_index: Arc::new(LineIndex::new(text)),
        backend_tree: parse_backend_tree_for_test(text),
        changed_ranges: Arc::new(changed_ranges),
        produced_at_millis: 0,
        backend_tree_hash: 0,
        incremental,
        fallback_reason: fallback_reason.map(Arc::from),
    }
}

fn counters(metrics: &serde_json::Value) -> &serde_json::Map<String, serde_json::Value> {
    metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object")
}

fn histograms(metrics: &serde_json::Value) -> &serde_json::Map<String, serde_json::Value> {
    metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object")
}

fn counter_value(counters: &serde_json::Map<String, serde_json::Value>, key: &str) -> u64 {
    counters
        .get(key)
        .and_then(|value| value.as_u64())
        .unwrap_or(0)
}

fn histogram_count(histograms: &serde_json::Map<String, serde_json::Value>, key: &str) -> u64 {
    histograms
        .get(key)
        .and_then(|value| value.get("count"))
        .and_then(|value| value.as_u64())
        .unwrap_or(0)
}

#[tokio::test]
async fn p8_snapshot_with_deps_is_atomic() {
    let mut host = AnalysisHostV2::default();

    let deps_old = make_deps();
    let deps_id_old = DepsSnapshotId::from_hash("deps_old");
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id_old.clone(),
        deps: deps_old,
    });

    let runtime = IntellisenseV2Facade::new(host, make_index_snapshot("index_old"), None);

    {
        let (analysis, index_snapshot, deps_id) = runtime.snapshot_with_deps().await;
        assert_eq!(deps_id.as_str(), "deps_old");
        assert_eq!(index_snapshot.id.as_str(), "index_old");
        assert_eq!(analysis.deps_id().unwrap().as_str(), "deps_old");
    }

    let deps_new = make_deps();
    let deps_id_new = DepsSnapshotId::from_hash("deps_new");
    let index_new = make_index_snapshot("index_new");

    let apply_task = tokio::spawn({
        let runtime = runtime.clone();
        let deps_new = deps_new.clone();
        let deps_id_new = deps_id_new.clone();
        let index_new = index_new.clone();
        async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            let ok = runtime
                .apply_deps_bundle(deps_id_new, deps_new, index_new)
                .await;
            assert!(ok, "apply_deps_bundle should succeed");
        }
    });

    let watch_task = tokio::spawn({
        let runtime = runtime.clone();
        async move {
            for _ in 0..200 {
                let (_analysis, index_snapshot, deps_id) = runtime.snapshot_with_deps().await;
                match deps_id.as_str() {
                    "deps_old" => assert_eq!(index_snapshot.id.as_str(), "index_old"),
                    "deps_new" => assert_eq!(index_snapshot.id.as_str(), "index_new"),
                    other => panic!("unexpected deps_id: {}", other),
                }
            }
        }
    });

    apply_task.await.expect("apply task join");
    watch_task.await.expect("watch task join");

    let (analysis, index_snapshot, deps_id) = runtime.snapshot_with_deps().await;
    assert_eq!(deps_id.as_str(), "deps_new");
    assert_eq!(index_snapshot.id.as_str(), "index_new");
    assert_eq!(analysis.deps_id().unwrap().as_str(), "deps_new");

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn p8_completion_current_revision_snapshot_keeps_deps_and_index_atomic() {
    let mut host = AnalysisHostV2::default();

    let deps_old = make_deps();
    let deps_id_old = DepsSnapshotId::from_hash("deps_old");
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id_old.clone(),
        deps: deps_old,
    });

    let runtime = IntellisenseV2Facade::new(host, make_index_snapshot("index_old"), None);

    {
        let snapshot = runtime
            .completion_current_revision_snapshot_for_origin_and_operation(
                ObservabilityOrigin::Lsp,
                SemanticOperation::Completion,
            )
            .await;
        assert_eq!(snapshot.deps_id.as_str(), "deps_old");
        assert_eq!(snapshot.index_snapshot.id.as_str(), "index_old");
        assert_eq!(snapshot.analysis.deps_id().unwrap().as_str(), "deps_old");
    }

    let deps_new = make_deps();
    let deps_id_new = DepsSnapshotId::from_hash("deps_new");
    let index_new = make_index_snapshot("index_new");

    let apply_task = tokio::spawn({
        let runtime = runtime.clone();
        let deps_new = deps_new.clone();
        let deps_id_new = deps_id_new.clone();
        let index_new = index_new.clone();
        async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            let ok = runtime
                .apply_deps_bundle(deps_id_new, deps_new, index_new)
                .await;
            assert!(ok, "apply_deps_bundle should succeed");
        }
    });

    let watch_task = tokio::spawn({
        let runtime = runtime.clone();
        async move {
            for _ in 0..200 {
                let snapshot = runtime
                    .completion_current_revision_snapshot_for_origin_and_operation(
                        ObservabilityOrigin::Lsp,
                        SemanticOperation::Completion,
                    )
                    .await;
                match snapshot.deps_id.as_str() {
                    "deps_old" => assert_eq!(snapshot.index_snapshot.id.as_str(), "index_old"),
                    "deps_new" => assert_eq!(snapshot.index_snapshot.id.as_str(), "index_new"),
                    other => panic!("unexpected deps_id: {}", other),
                }
                assert_eq!(
                    snapshot.analysis.deps_id().unwrap().as_str(),
                    snapshot.deps_id.as_str()
                );
            }
        }
    });

    apply_task.await.expect("apply task join");
    watch_task.await.expect("watch task join");

    let snapshot = runtime
        .completion_current_revision_snapshot_for_origin_and_operation(
            ObservabilityOrigin::Lsp,
            SemanticOperation::Completion,
        )
        .await;
    assert_eq!(snapshot.deps_id.as_str(), "deps_new");
    assert_eq!(snapshot.index_snapshot.id.as_str(), "index_new");
    assert_eq!(snapshot.analysis.deps_id().unwrap().as_str(), "deps_new");

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn completion_current_revision_snapshot_falls_back_to_writer_snapshot_on_persistent_bundle_mismatch(
) {
    let mut host = AnalysisHostV2::default();

    let deps_old = make_deps();
    let deps_id_old = DepsSnapshotId::from_hash("deps_old");
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id_old.clone(),
        deps: deps_old,
    });

    let runtime = IntellisenseV2Facade::new(host, make_index_snapshot("index_old"), None);

    runtime
        .inner
        .completion_deps_index_snapshot
        .store(Arc::new(CompletionDepsIndexSnapshot {
            deps: make_deps(),
            deps_id: DepsSnapshotId::from_hash("deps_mismatch"),
            index_snapshot: make_index_snapshot("index_mismatch"),
        }));

    let snapshot = timeout(
        Duration::from_secs(1),
        runtime.completion_current_revision_snapshot_for_origin_and_operation(
            ObservabilityOrigin::Lsp,
            SemanticOperation::Completion,
        ),
    )
    .await
    .expect("completion current-revision snapshot must not spin on persistent deps/index mismatch");

    assert_eq!(snapshot.deps_id.as_str(), "deps_old");
    assert_eq!(snapshot.index_snapshot.id.as_str(), "index_old");
    assert_eq!(snapshot.analysis.deps_id().unwrap().as_str(), "deps_old");

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn snapshot_with_deps_runtime_trace_exposes_queue_and_exec_latency() {
    let runtime = IntellisenseV2Facade::new(
        AnalysisHostV2::default(),
        make_index_snapshot("snapshot_with_deps_runtime"),
        None,
    );

    let snapshot = runtime
        .snapshot_with_deps_with_priority(
            ObservabilityOrigin::Lsp,
            RuntimeQueuePriority::Interactive,
            None,
        )
        .await;
    assert!(snapshot.trace.queue_wait_elapsed.is_some());
    assert!(snapshot.trace.exec_elapsed.is_some());

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn p8_apply_changes_ignores_set_deps_snapshot() {
    let mut host = AnalysisHostV2::default();

    let deps_old = make_deps();
    let deps_id_old = DepsSnapshotId::from_hash("deps_old");
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id_old.clone(),
        deps: deps_old,
    });

    let runtime = IntellisenseV2Facade::new(host, make_index_snapshot("index_old"), None);

    let deps_new = make_deps();
    let deps_id_new = DepsSnapshotId::from_hash("deps_new");
    runtime.apply_changes(vec![Change::SetDepsSnapshot {
        deps_id: deps_id_new,
        deps: deps_new,
    }]);

    let (analysis, _index_snapshot, deps_id) = runtime.snapshot_with_deps().await;
    assert_eq!(deps_id.as_str(), "deps_old");
    assert_eq!(analysis.deps_id().unwrap().as_str(), "deps_old");

    runtime.shutdown_for_test().await;
}

#[test]
fn ephemeral_snapshot_sets_contract_inputs() {
    let deps = make_deps();
    let deps_id = DepsSnapshotId::from_hash("deps_ephemeral");
    let settings = ExecutionSettings {
        settings_id: SettingsId::from_hash("settings_ephemeral"),
        diagnostics_detail_level: DetailLevel::Full,
    };
    let snapshot = IntellisenseV2Facade::ephemeral_snapshot(
        deps_id.clone(),
        deps,
        make_index_snapshot("index_ephemeral"),
        settings.clone(),
        FileId(7),
        Arc::from("Перем х;"),
        42,
        Arc::from("<ephemeral>"),
    );

    assert_eq!(
        snapshot.analysis.file_version(FileId(7)).unwrap(),
        Some(42),
        "ephemeral snapshot should carry file version"
    );
    assert_eq!(
        snapshot.analysis.deps_id().unwrap().as_str(),
        deps_id.as_str(),
        "ephemeral snapshot should carry deps id"
    );
    assert_eq!(
        snapshot.analysis.settings_id().unwrap().as_str(),
        settings.settings_id.as_str(),
        "ephemeral snapshot should carry settings id"
    );
}

#[test]
fn prepare_ephemeral_operation_keeps_discovery_snapshot_outside_semantic_snapshot() {
    let deps_id = DepsSnapshotId::from_hash("deps_prepared_ephemeral");
    let settings = ExecutionSettings {
        settings_id: SettingsId::from_hash("settings_prepared_ephemeral"),
        diagnostics_detail_level: DetailLevel::Full,
    };
    let context = ExecutionContext {
        origin: ObservabilityOrigin::Agent,
        operation: SemanticOperation::Members,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id: FileId(7),
        min_file_version: Some(42),
        expected_deps_id: Some(deps_id.clone()),
        flow_sensitive: false,
        settings: settings.clone(),
        cancellation: CancellationPolicy::BestEffort,
    };

    let prepared = IntellisenseV2Facade::prepare_ephemeral_operation(
        &context,
        deps_id.clone(),
        make_deps(),
        make_index_snapshot("index_prepared_ephemeral"),
        Arc::from("Перем х;"),
        42,
        Arc::from("<prepared-ephemeral>"),
        None,
    )
    .expect("prepare_ephemeral_operation");

    assert_eq!(
        prepared.snapshot.analysis.file_version(FileId(7)).unwrap(),
        Some(42),
        "prepared semantic snapshot should carry file version"
    );
    assert_eq!(
        prepared.snapshot.deps_id.as_str(),
        deps_id.as_str(),
        "prepared semantic snapshot should carry deps id"
    );
    assert_eq!(
        prepared.index_snapshot.id.as_str(),
        "index_prepared_ephemeral",
        "discovery snapshot must stay outside semantic snapshot payload"
    );
}

#[test]
fn prepare_ephemeral_operation_warms_exact_type_index_only_for_shared_interactive_queries() {
    let deps_id = DepsSnapshotId::from_hash("deps_prepared_exact_type_index");
    let settings = ExecutionSettings {
        settings_id: SettingsId::from_hash("settings_prepared_exact_type_index"),
        diagnostics_detail_level: DetailLevel::Full,
    };
    let file_id = FileId(9);
    let file_text: Arc<str> = Arc::from(
        "Procedure Test()\n\
             arr = Новый Массив;\n\
             result = arr;\n\
             EndProcedure",
    );
    let probe = file_text
        .find("result = arr;")
        .map(|idx| idx as u32 + "result = ".len() as u32)
        .expect("probe for exact type index");

    for operation in [
        SemanticOperation::Completion,
        SemanticOperation::Hover,
        SemanticOperation::Members,
        SemanticOperation::TypeAtPosition,
        SemanticOperation::SignatureHelp,
        SemanticOperation::Definition,
    ] {
        let context = ExecutionContext {
            origin: ObservabilityOrigin::Web,
            operation,
            completion_mode: None,
            completion_large_churn_active: false,
            file_id,
            min_file_version: Some(3),
            expected_deps_id: Some(deps_id.clone()),
            flow_sensitive: false,
            settings: settings.clone(),
            cancellation: CancellationPolicy::BestEffort,
        };

        let prepared = IntellisenseV2Facade::prepare_ephemeral_operation(
            &context,
            deps_id.clone(),
            make_deps(),
            make_index_snapshot("index_prepared_exact_type_index"),
            file_text.clone(),
            3,
            Arc::from("<prepared-exact-type-index>"),
            None,
        )
        .expect("prepare_ephemeral_operation");

        let resolution = prepared
            .snapshot
            .analysis
            .type_at_byte_offset_serve_only(file_id, probe)
            .expect("serve-only lookup after shared prepare");
        let type_name = resolution
            .as_ref()
            .map(|value| value.type_name())
            .unwrap_or_default();
        assert!(
            type_name.starts_with("Массив"),
            "shared ephemeral prepare must warm exact type index for {}; got={type_name:?}",
            operation.as_str()
        );
    }

    let diagnostics_context = ExecutionContext {
        origin: ObservabilityOrigin::Web,
        operation: SemanticOperation::Diagnostics,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id,
        min_file_version: Some(3),
        expected_deps_id: Some(deps_id.clone()),
        flow_sensitive: false,
        settings,
        cancellation: CancellationPolicy::BestEffort,
    };

    let diagnostics_prepared = IntellisenseV2Facade::prepare_ephemeral_operation(
        &diagnostics_context,
        deps_id,
        make_deps(),
        make_index_snapshot("index_prepared_exact_type_index_diagnostics"),
        file_text,
        3,
        Arc::from("<prepared-exact-type-index-diagnostics>"),
        None,
    )
    .expect("prepare_ephemeral_operation for diagnostics");

    assert!(
        diagnostics_prepared
            .snapshot
            .analysis
            .type_at_byte_offset_serve_only(file_id, probe)
            .expect("serve-only lookup for diagnostics")
            .is_none(),
        "diagnostics prepare must not warm exact type index implicitly"
    );
}

#[tokio::test]
async fn prepare_stateful_operation_warms_exact_type_index_only_for_shared_interactive_queries() {
    let deps_id = DepsSnapshotId::from_hash("deps_prepared_stateful_exact_type_index");
    let settings = ExecutionSettings {
        settings_id: SettingsId::from_hash("settings_prepared_stateful_exact_type_index"),
        diagnostics_detail_level: DetailLevel::Full,
    };
    let file_id = FileId(19);
    let file_text: Arc<str> = Arc::from(
        "Procedure Test()\n\
             arr = Новый Массив;\n\
             result = arr;\n\
             EndProcedure",
    );
    let probe = file_text
        .find("result = arr;")
        .map(|idx| idx as u32 + "result = ".len() as u32)
        .expect("probe for stateful exact type index");

    for operation in [
        SemanticOperation::Completion,
        SemanticOperation::Hover,
        SemanticOperation::Members,
        SemanticOperation::TypeAtPosition,
        SemanticOperation::SignatureHelp,
        SemanticOperation::Definition,
    ] {
        let mut host = AnalysisHostV2::default();
        host.apply_change(Change::SetDepsSnapshot {
            deps_id: deps_id.clone(),
            deps: make_deps(),
        });
        host.apply_change(Change::SetSettingsSnapshot {
            settings_id: settings.settings_id.clone(),
            diagnostics_detail_level: settings.diagnostics_detail_level,
        });
        let runtime = IntellisenseV2Facade::new(
            host,
            make_index_snapshot("index_prepared_stateful_exact_type_index"),
            None,
        );
        runtime.apply_changes(vec![Change::SetFile {
            file_id,
            text: file_text.clone(),
            version: 3,
            path: Arc::from("<prepared-stateful-exact-type-index>"),
        }]);
        let _ = runtime.snapshot().await;

        let context = ExecutionContext {
            origin: ObservabilityOrigin::Runtime,
            operation,
            completion_mode: None,
            completion_large_churn_active: false,
            file_id,
            min_file_version: Some(3),
            expected_deps_id: Some(deps_id.clone()),
            flow_sensitive: false,
            settings: settings.clone(),
            cancellation: CancellationPolicy::Ignore,
        };

        let prepared = runtime
            .prepare_stateful_operation(&context, None)
            .await
            .expect("prepare_stateful_operation");

        let resolution = prepared
            .snapshot
            .analysis
            .type_at_byte_offset_serve_only(file_id, probe)
            .expect("serve-only lookup after shared stateful prepare");
        let type_name = resolution
            .as_ref()
            .map(|value| value.type_name())
            .unwrap_or_default();
        assert!(
            type_name.starts_with("Массив"),
            "shared stateful prepare must warm exact type index for {}; got={type_name:?}",
            operation.as_str()
        );

        runtime.shutdown_for_test().await;
    }

    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id.clone(),
        deps: make_deps(),
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: settings.settings_id.clone(),
        diagnostics_detail_level: settings.diagnostics_detail_level,
    });
    let runtime = IntellisenseV2Facade::new(
        host,
        make_index_snapshot("index_prepared_stateful_exact_type_index_diagnostics"),
        None,
    );
    runtime.apply_changes(vec![Change::SetFile {
        file_id,
        text: file_text.clone(),
        version: 3,
        path: Arc::from("<prepared-stateful-exact-type-index-diagnostics>"),
    }]);
    let _ = runtime.snapshot().await;

    let diagnostics_context = ExecutionContext {
        origin: ObservabilityOrigin::Runtime,
        operation: SemanticOperation::Diagnostics,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id,
        min_file_version: Some(3),
        expected_deps_id: Some(deps_id),
        flow_sensitive: false,
        settings,
        cancellation: CancellationPolicy::Ignore,
    };

    let diagnostics_prepared = runtime
        .prepare_stateful_operation(&diagnostics_context, None)
        .await
        .expect("prepare_stateful_operation for diagnostics");

    assert!(
        diagnostics_prepared
            .snapshot
            .analysis
            .type_at_byte_offset_serve_only(file_id, probe)
            .expect("serve-only lookup after diagnostics stateful prepare")
            .is_none(),
        "stateful diagnostics prepare must not warm exact type index implicitly"
    );

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn prepare_stateful_operation_skips_eager_exact_type_index_warm_for_lsp_completion() {
    let deps_id = DepsSnapshotId::from_hash("deps_lsp_completion_exact_type_index");
    let settings = ExecutionSettings {
        settings_id: SettingsId::from_hash("settings_lsp_completion_exact_type_index"),
        diagnostics_detail_level: DetailLevel::Full,
    };
    let file_id = FileId(190);
    let file_text: Arc<str> = Arc::from(
        "Procedure Test()\n\
             arr = Новый Массив;\n\
             result = arr;\n\
             EndProcedure",
    );
    let probe = file_text
        .find("result = arr;")
        .map(|idx| idx as u32 + "result = ".len() as u32)
        .expect("probe for lsp completion exact type index");

    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id.clone(),
        deps: make_deps(),
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: settings.settings_id.clone(),
        diagnostics_detail_level: settings.diagnostics_detail_level,
    });
    let runtime = IntellisenseV2Facade::new(
        host,
        make_index_snapshot("index_lsp_completion_exact_type_index"),
        None,
    );
    runtime.apply_changes(vec![Change::SetFile {
        file_id,
        text: file_text,
        version: 3,
        path: Arc::from("<lsp-completion-exact-type-index>"),
    }]);
    let _ = runtime.snapshot().await;

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Completion,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id,
        min_file_version: Some(3),
        expected_deps_id: Some(deps_id),
        flow_sensitive: false,
        settings,
        cancellation: CancellationPolicy::BestEffort,
    };

    let prepared = runtime
        .prepare_stateful_operation(&context, None)
        .await
        .expect("prepare_stateful_operation");

    assert!(
        !prepared
            .snapshot
            .analysis
            .current_type_index_serve_only_ready(file_id)
            .expect("serve-only readiness after lsp completion prepare"),
        "lsp completion prepare must leave exact type index warming to background precompute"
    );
    assert!(
        prepared
            .snapshot
            .analysis
            .type_at_byte_offset_serve_only(file_id, probe)
            .expect("serve-only lookup after lsp completion prepare")
            .is_none(),
        "lsp completion prepare must not materialize exact type index eagerly"
    );

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn prepare_stateful_operation_skips_eager_exact_type_index_warm_for_lsp_members() {
    let deps_id = DepsSnapshotId::from_hash("deps_lsp_members_exact_type_index");
    let settings = ExecutionSettings {
        settings_id: SettingsId::from_hash("settings_lsp_members_exact_type_index"),
        diagnostics_detail_level: DetailLevel::Full,
    };
    let file_id = FileId(191);
    let file_text: Arc<str> = Arc::from(
        "Procedure Test()\n\
             arr = Новый Массив;\n\
             result = arr;\n\
             EndProcedure",
    );
    let probe = file_text
        .find("result = arr;")
        .map(|idx| idx as u32 + "result = ".len() as u32)
        .expect("probe for lsp members exact type index");

    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id.clone(),
        deps: make_deps(),
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: settings.settings_id.clone(),
        diagnostics_detail_level: settings.diagnostics_detail_level,
    });
    let runtime = IntellisenseV2Facade::new(
        host,
        make_index_snapshot("index_lsp_members_exact_type_index"),
        None,
    );
    runtime.apply_changes(vec![Change::SetFile {
        file_id,
        text: file_text,
        version: 3,
        path: Arc::from("<lsp-members-exact-type-index>"),
    }]);
    let _ = runtime.snapshot().await;

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Members,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id,
        min_file_version: Some(3),
        expected_deps_id: Some(deps_id),
        flow_sensitive: false,
        settings,
        cancellation: CancellationPolicy::BestEffort,
    };

    let prepared = runtime
        .prepare_stateful_operation(&context, None)
        .await
        .expect("prepare_stateful_operation");

    assert!(
        !prepared
            .snapshot
            .analysis
            .current_type_index_serve_only_ready(file_id)
            .expect("serve-only readiness after lsp members prepare"),
        "lsp members prepare must not materialize exact type index eagerly"
    );
    assert!(
        prepared
            .snapshot
            .analysis
            .type_at_byte_offset_serve_only(file_id, probe)
            .expect("serve-only lookup after lsp members prepare")
            .is_none(),
        "lsp members prepare must leave exact type index cold"
    );

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn prepare_stateful_operation_skips_eager_exact_type_index_warm_for_lsp_exact_only_queries() {
    let deps_id = DepsSnapshotId::from_hash("deps_lsp_exact_only_queries_exact_type_index");
    let settings = ExecutionSettings {
        settings_id: SettingsId::from_hash("settings_lsp_exact_only_queries_exact_type_index"),
        diagnostics_detail_level: DetailLevel::Full,
    };
    let file_text: Arc<str> = Arc::from(
        "Procedure Test()\n\
             arr = Новый Массив;\n\
             result = arr;\n\
             EndProcedure",
    );
    let probe = file_text
        .find("result = arr;")
        .map(|idx| idx as u32 + "result = ".len() as u32)
        .expect("probe for lsp exact-only queries exact type index");

    for (index, operation) in [
        SemanticOperation::Hover,
        SemanticOperation::TypeAtPosition,
        SemanticOperation::SignatureHelp,
        SemanticOperation::Definition,
    ]
    .into_iter()
    .enumerate()
    {
        let file_id = FileId(200 + index as u32);
        let mut host = AnalysisHostV2::default();
        host.apply_change(Change::SetDepsSnapshot {
            deps_id: deps_id.clone(),
            deps: make_deps(),
        });
        host.apply_change(Change::SetSettingsSnapshot {
            settings_id: settings.settings_id.clone(),
            diagnostics_detail_level: settings.diagnostics_detail_level,
        });
        let runtime = IntellisenseV2Facade::new(
            host,
            make_index_snapshot("index_lsp_exact_only_queries_exact_type_index"),
            None,
        );
        runtime.apply_changes(vec![Change::SetFile {
            file_id,
            text: file_text.clone(),
            version: 3,
            path: Arc::from("<lsp-exact-only-queries-exact-type-index>"),
        }]);
        let _ = runtime.snapshot().await;

        let context = ExecutionContext {
            origin: ObservabilityOrigin::Lsp,
            operation,
            completion_mode: None,
            completion_large_churn_active: false,
            file_id,
            min_file_version: Some(3),
            expected_deps_id: Some(deps_id.clone()),
            flow_sensitive: false,
            settings: settings.clone(),
            cancellation: CancellationPolicy::BestEffort,
        };

        let prepared = runtime
            .prepare_stateful_operation(&context, None)
            .await
            .expect("prepare_stateful_operation");

        assert!(
            !prepared
                .snapshot
                .analysis
                .current_type_index_serve_only_ready(file_id)
                .expect("serve-only readiness after lsp exact-only prepare"),
            "lsp {} prepare must keep exact type index cold",
            operation.as_str()
        );
        assert!(
            prepared
                .snapshot
                .analysis
                .type_at_byte_offset_serve_only(file_id, probe)
                .expect("serve-only lookup after lsp exact-only prepare")
                .is_none(),
            "lsp {} prepare must not materialize exact type index eagerly",
            operation.as_str()
        );

        runtime.shutdown_for_test().await;
    }
}

#[tokio::test]
async fn prepare_completion_first_response_reports_not_ready_before_current_revision_head_publish()
{
    let deps_id = DepsSnapshotId::from_hash("deps_completion_first_response_not_ready");
    let settings = ExecutionSettings {
        settings_id: SettingsId::from_hash("settings_completion_first_response_not_ready"),
        diagnostics_detail_level: DetailLevel::Full,
    };
    let file_id = FileId(240);
    let file_text: Arc<str> = Arc::from(
        "Процедура Тест()\n\
             Результат = (Новый Массив()).\n\
             КонецПроцедуры",
    );
    let completion_line = 1;
    let completion_column = file_text
        .lines()
        .nth(completion_line as usize)
        .expect("completion line")
        .chars()
        .count() as u32;
    let index_snapshot = make_index_snapshot("index_completion_first_response_not_ready");

    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id.clone(),
        deps: make_deps(),
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: settings.settings_id.clone(),
        diagnostics_detail_level: settings.diagnostics_detail_level,
    });
    let runtime = IntellisenseV2Facade::new(host, index_snapshot.clone(), None);
    runtime.apply_changes(vec![Change::SetFile {
        file_id,
        text: file_text,
        version: 3,
        path: Arc::from("<completion-first-response-not-ready>"),
    }]);
    let _ = runtime.snapshot().await;

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Completion,
        completion_mode: Some("event_driven"),
        completion_large_churn_active: false,
        file_id,
        min_file_version: Some(3),
        expected_deps_id: Some(deps_id),
        flow_sensitive: false,
        settings,
        cancellation: CancellationPolicy::BestEffort,
    };

    let prepared = runtime
        .prepare_completion_first_response(&context, None, completion_line, completion_column)
        .await
        .expect("prepare_completion_first_response");

    assert_eq!(
        prepared.readiness,
        CompletionFirstResponseReadiness::NotReady,
        "completion first-response prepare must stay bounded fail-closed before current-revision head/exact truth is published"
    );
    assert_eq!(prepared.observed_file_version, Some(3));
    assert_eq!(
        prepared.support.index_snapshot.id.as_str(),
        index_snapshot.id.as_str(),
        "lightweight current-revision prepare must carry the representative index snapshot needed by head route"
    );
    assert!(
        !prepared.support.head_ready,
        "current-revision head must stay unavailable before publish"
    );
    assert!(
        !prepared.support.exact_ready,
        "exact artifact must stay unavailable before publish"
    );
    assert!(
        prepared.support.head_owner_type_hints.is_empty(),
        "not-ready lightweight prepare must not fabricate head owner hints"
    );

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn prepare_completion_first_response_reports_head_ready_after_current_revision_head_publish()
{
    let deps_id = DepsSnapshotId::from_hash("deps_completion_first_response_head_ready");
    let settings = ExecutionSettings {
        settings_id: SettingsId::from_hash("settings_completion_first_response_head_ready"),
        diagnostics_detail_level: DetailLevel::Full,
    };
    let file_id = FileId(241);
    let file_text: Arc<str> = Arc::from(
        "Процедура Тест()\n\
             Результат = (Новый Массив()).\n\
             КонецПроцедуры",
    );
    let completion_line = 1;
    let completion_column = file_text
        .lines()
        .nth(completion_line as usize)
        .expect("completion line")
        .chars()
        .count() as u32;

    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id.clone(),
        deps: make_deps(),
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: settings.settings_id.clone(),
        diagnostics_detail_level: settings.diagnostics_detail_level,
    });
    let runtime = IntellisenseV2Facade::new(
        host,
        make_index_snapshot("index_completion_first_response_head_ready"),
        None,
    );
    runtime.apply_changes(vec![Change::SetFile {
        file_id,
        text: file_text,
        version: 3,
        path: Arc::from("<completion-first-response-head-ready>"),
    }]);
    let analysis = runtime.snapshot().await;
    let _ = analysis.ir(file_id).expect("publish current-revision head");

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Completion,
        completion_mode: Some("event_driven"),
        completion_large_churn_active: false,
        file_id,
        min_file_version: Some(3),
        expected_deps_id: Some(deps_id),
        flow_sensitive: false,
        settings,
        cancellation: CancellationPolicy::BestEffort,
    };

    let prepared = runtime
        .prepare_completion_first_response(&context, None, completion_line, completion_column)
        .await
        .expect("prepare_completion_first_response");

    assert_eq!(
        prepared.readiness,
        CompletionFirstResponseReadiness::HeadReady,
        "completion first-response prepare must classify current revision as head-ready once the head artifact is published"
    );
    assert!(
        prepared.support.head_ready,
        "head readiness must be observable through the lightweight completion prepare boundary"
    );
    assert!(
        !prepared.support.exact_ready,
        "head-ready path must stay logically distinct from exact type-index readiness"
    );
    assert!(
        !prepared.support.head_owner_type_hints.is_empty(),
        "head-ready lightweight prepare must carry owner hints as immutable DTO payload"
    );

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn prepare_completion_first_response_reports_exact_ready_after_exact_precompute() {
    let deps_id = DepsSnapshotId::from_hash("deps_completion_first_response_exact_ready");
    let settings = ExecutionSettings {
        settings_id: SettingsId::from_hash("settings_completion_first_response_exact_ready"),
        diagnostics_detail_level: DetailLevel::Full,
    };
    let file_id = FileId(242);
    let file_text: Arc<str> = Arc::from(
        "Процедура Тест()\n\
             Результат = (Новый Массив()).\n\
             КонецПроцедуры",
    );
    let completion_line = 1;
    let completion_column = file_text
        .lines()
        .nth(completion_line as usize)
        .expect("completion line")
        .chars()
        .count() as u32;

    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id.clone(),
        deps: make_deps(),
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: settings.settings_id.clone(),
        diagnostics_detail_level: settings.diagnostics_detail_level,
    });
    let runtime = IntellisenseV2Facade::new(
        host,
        make_index_snapshot("index_completion_first_response_exact_ready"),
        None,
    );
    runtime.apply_changes(vec![Change::SetFile {
        file_id,
        text: file_text,
        version: 3,
        path: Arc::from("<completion-first-response-exact-ready>"),
    }]);
    let analysis = runtime.snapshot().await;
    let _ = analysis
        .precompute_type_index_for_file(file_id, Some(3), 0)
        .expect("precompute exact artifact");
    assert!(
        analysis
            .current_type_index_serve_only_ready(file_id)
            .expect("exact readiness after precompute"),
        "test setup must materialize the exact type-index artifact"
    );

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Completion,
        completion_mode: Some("event_driven"),
        completion_large_churn_active: false,
        file_id,
        min_file_version: Some(3),
        expected_deps_id: Some(deps_id),
        flow_sensitive: false,
        settings,
        cancellation: CancellationPolicy::BestEffort,
    };

    let prepared = runtime
        .prepare_completion_first_response(&context, None, completion_line, completion_column)
        .await
        .expect("prepare_completion_first_response");

    assert_eq!(
        prepared.readiness,
        CompletionFirstResponseReadiness::ExactReady,
        "completion first-response prepare must classify exact-ready state once the exact artifact is already available"
    );
    assert!(
        prepared.support.exact_ready,
        "exact readiness must be observable through the lightweight completion prepare boundary"
    );

    runtime.shutdown_for_test().await;
}

#[test]
fn semantic_operation_contract_values_are_stable() {
    assert_eq!(SemanticOperation::Completion.as_str(), "completion");
    assert_eq!(SemanticOperation::Hover.as_str(), "hover");
    assert_eq!(SemanticOperation::SignatureHelp.as_str(), "signature_help");
    assert_eq!(SemanticOperation::Definition.as_str(), "definition");
    assert_eq!(
        SemanticOperation::DocumentSymbol.as_str(),
        "document_symbol"
    );
    assert_eq!(SemanticOperation::Rename.as_str(), "rename");
    assert_eq!(SemanticOperation::Diagnostics.as_str(), "diagnostics");
    assert_eq!(SemanticOperation::Members.as_str(), "members");
    assert_eq!(
        SemanticOperation::TypeAtPosition.as_str(),
        "type_at_position"
    );
    assert_eq!(SemanticOperation::SymbolSearch.as_str(), "symbol_search");
    assert_eq!(SemanticOperation::References.as_str(), "references");
}

#[test]
fn runtime_queue_priority_aligns_user_facing_member_access_with_interactive_operations() {
    for operation in [
        SemanticOperation::Completion,
        SemanticOperation::Hover,
        SemanticOperation::SignatureHelp,
        SemanticOperation::Definition,
        SemanticOperation::Members,
        SemanticOperation::TypeAtPosition,
    ] {
        assert_eq!(
            RuntimeQueuePriority::for_operation(operation),
            RuntimeQueuePriority::Interactive,
            "{operation:?} must stay on interactive queue"
        );
    }

    assert_eq!(
        RuntimeQueuePriority::for_operation(SemanticOperation::DocumentSymbol),
        RuntimeQueuePriority::Background,
        "non-interactive operations must remain on background queue"
    );
}

#[test]
fn observability_contract_values_are_stable() {
    assert_eq!(
        ObservabilityStage::RuntimeQueueWait.as_str(),
        "runtime_queue_wait"
    );
    assert_eq!(
        ObservabilityStage::RuntimeWaitForFileVersion.as_str(),
        "runtime_wait_for_file_version"
    );
    assert_eq!(
        ObservabilityStage::RuntimeSnapshotWithDeps.as_str(),
        "runtime_snapshot_with_deps"
    );
    assert_eq!(ObservabilityStage::IrQuery.as_str(), "ir_query");
    assert_eq!(
        ObservabilityStage::SyntaxDiagnosticsQuery.as_str(),
        "syntax_diagnostics_query"
    );
    assert_eq!(
        ObservabilityStage::SemanticDiagnosticsQuery.as_str(),
        "semantic_diagnostics_query"
    );
    assert_eq!(
        ObservabilityStage::ParseResultQuery.as_str(),
        "parse_result_query"
    );
    assert_eq!(SemanticOutcome::Success.as_str(), "success");
    assert_eq!(SemanticOutcome::Empty.as_str(), "empty");
    assert_eq!(SemanticOutcome::Cancelled.as_str(), "cancelled");
    assert_eq!(SemanticOutcome::Error.as_str(), "error");
    assert_eq!(SemanticOutcome::StaleVersion.as_str(), "stale_version");
    assert_eq!(SemanticOutcome::MissingDeps.as_str(), "missing_deps");
}

#[tokio::test]
async fn stateful_prepare_operation_returns_missing_deps_on_mismatch() {
    let mut host = AnalysisHostV2::default();
    let deps_old = make_deps();
    let deps_id_old = DepsSnapshotId::from_hash("deps_old");
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id_old,
        deps: deps_old,
    });
    let runtime = IntellisenseV2Facade::new(host, make_index_snapshot("index"), None);

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Hover,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id: FileId(1),
        min_file_version: None,
        expected_deps_id: Some(DepsSnapshotId::from_hash("deps_expected")),
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id: SettingsId::from_hash("settings"),
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let result = runtime.prepare_stateful_operation(&context, None).await;
    assert!(matches!(result, Err(SemanticOutcome::MissingDeps)));

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn interactive_prepare_timeout_fails_closed_when_gap_within_default() {
    let coordinator = SystemCoordinator::new();
    let file_id = FileId(10);
    let deps_id = DepsSnapshotId::from_hash("deps_stale_ok");
    let settings_id = SettingsId::from_hash("settings");

    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id.clone(),
        deps: make_deps(),
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: settings_id.clone(),
        diagnostics_detail_level: DetailLevel::Full,
    });
    let runtime = IntellisenseV2Facade::new(
        host,
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p9"))),
        None,
    );
    runtime.apply_changes(vec![Change::SetFile {
        file_id,
        text: Arc::from("x = 1;"),
        version: 4,
        path: Arc::from("stale_ok.bsl"),
    }]);
    let _ = runtime.snapshot().await;

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Completion,
        completion_mode: None,
        completion_large_churn_active: true,
        file_id,
        min_file_version: Some(5),
        expected_deps_id: Some(deps_id),
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id: settings_id.clone(),
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let prepared = runtime
        .prepare_stateful_operation(&context, Some(&coordinator))
        .await;
    assert!(
        matches!(prepared, Err(SemanticOutcome::StaleVersion)),
        "interactive completion must fail closed instead of serving stale snapshot"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");
    assert!(
        counters.contains_key("intellisense_v2_interactive_wait_budget_exhausted_total"),
        "wait budget exhausted metric should be recorded"
    );
    assert!(
        counters
            .get("intellisense_v2_interactive_stale_served_total")
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            == 0,
        "stale served metric must stay zero under fail-closed policy"
    );
    assert!(
        counters.contains_key("intellisense_v2_runtime_queue_wait_interactive_total"),
        "interactive queue-class counter should be recorded"
    );
    assert!(
        counters.contains_key("intellisense_v2_runtime_exec_interactive_total"),
        "interactive exec-class counter should be recorded"
    );
    assert!(
        counters
            .get("intellisense_v2_completion_stale_fallback_total")
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            == 0,
        "completion stale-fallback counter must stay zero under fail-closed policy"
    );
    assert!(
        counters.contains_key("intellisense_v2_completion_fallback_unavailable_total"),
        "completion fallback-unavailable counter should be recorded"
    );
    assert!(
        counters.contains_key("intellisense_v2_revision_lag_sample_total"),
        "revision lag counter should be recorded"
    );
    assert!(
        histograms.contains_key("intellisense_v2_runtime_queue_wait_interactive_ms"),
        "interactive queue-class histogram should be recorded"
    );
    assert!(
        histograms.contains_key("intellisense_v2_runtime_exec_interactive_ms"),
        "interactive exec-class histogram should be recorded"
    );
    assert!(
        histograms.contains_key("intellisense_v2_revision_lag_versions"),
        "revision lag histogram should be recorded"
    );

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn interactive_prepare_prefers_latest_when_version_is_ready_under_large_churn() {
    let file_id = FileId(110);
    let deps_id = DepsSnapshotId::from_hash("deps_latest_ready");
    let settings_id = SettingsId::from_hash("settings");

    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id.clone(),
        deps: make_deps(),
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: settings_id.clone(),
        diagnostics_detail_level: DetailLevel::Full,
    });
    let runtime = IntellisenseV2Facade::new(
        host,
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p9"))),
        None,
    );
    runtime.apply_changes(vec![Change::SetFile {
        file_id,
        text: Arc::from("x = 1;"),
        version: 5,
        path: Arc::from("latest_ready.bsl"),
    }]);
    let _ = runtime.snapshot().await;

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Completion,
        completion_mode: None,
        completion_large_churn_active: true,
        file_id,
        min_file_version: Some(5),
        expected_deps_id: Some(deps_id),
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id,
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let prepared = runtime
        .prepare_stateful_operation(&context, None)
        .await
        .expect("latest snapshot should be available without stale fallback");
    assert!(
        !prepared.wait_budget_exhausted,
        "latest path should not exceed wait budget when requested version is ready"
    );
    assert!(
        !prepared.stale_served,
        "stale fallback must not be served when latest version is already available"
    );
    assert_eq!(
        prepared.observed_file_version,
        Some(5),
        "prepared snapshot should observe requested latest file version"
    );

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn completion_mode_propagates_into_stage_drilldown_metrics() {
    let coordinator = SystemCoordinator::new();
    let file_id = FileId(21);
    let deps_id = DepsSnapshotId::from_hash("deps_mode_split");
    let settings_id = SettingsId::from_hash("settings");

    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id.clone(),
        deps: make_deps(),
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: settings_id.clone(),
        diagnostics_detail_level: DetailLevel::Full,
    });
    let runtime = IntellisenseV2Facade::new(
        host,
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash(
            "mode_split",
        ))),
        None,
    );
    runtime.apply_changes(vec![Change::SetFile {
        file_id,
        text: Arc::from("x = 1;"),
        version: 1,
        path: Arc::from("mode_split.bsl"),
    }]);
    let _ = runtime.snapshot().await;

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Completion,
        completion_mode: Some("event_driven"),
        completion_large_churn_active: false,
        file_id,
        min_file_version: Some(1),
        expected_deps_id: Some(deps_id),
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id,
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let prepared = runtime
        .prepare_stateful_operation(&context, Some(&coordinator))
        .await
        .expect("prepare_stateful_operation");
    let analysis = prepared.snapshot.analysis;

    let _: Result<Option<()>, ()> = IntellisenseV2Facade::run_optional_query(
        &context,
        ObservabilityStage::IrQuery,
        &analysis,
        Some(&coordinator),
        |_analysis| Ok(None),
    );
    let _: Result<Option<()>, ()> = IntellisenseV2Facade::run_optional_query(
        &context,
        ObservabilityStage::ParseResultQuery,
        &analysis,
        Some(&coordinator),
        |_analysis| Ok(None),
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");

    assert!(
            counters.contains_key(
                "intellisense_v2_drilldown_stage_total_origin_lsp_mode_event_driven_operation_completion_stage_runtime_wait_for_file_version"
            ),
            "wait stage counter must include completion mode dimension"
        );
    assert!(
            counters.contains_key(
                "intellisense_v2_drilldown_stage_total_origin_lsp_mode_event_driven_operation_completion_stage_runtime_snapshot_with_deps"
            ),
            "snapshot stage counter must include completion mode dimension"
        );
    assert!(
            counters.contains_key(
                "intellisense_v2_drilldown_stage_total_origin_lsp_mode_event_driven_operation_completion_stage_ir_query"
            ),
            "ir stage counter must include completion mode dimension"
        );
    assert!(
            counters.contains_key(
                "intellisense_v2_drilldown_stage_total_origin_lsp_mode_event_driven_operation_completion_stage_parse_result_query"
            ),
            "parse_result stage counter must include completion mode dimension"
        );
    assert!(
        counters.contains_key("intellisense_v2_wait_for_file_version_completion_total"),
        "legacy wait counter must still be projected"
    );
    assert!(
        counters.contains_key("intellisense_v2_snapshot_completion_total"),
        "legacy snapshot counter must still be projected"
    );
    assert!(
        counters.contains_key("intellisense_v2_ir_query_completion_total"),
        "legacy ir counter must still be projected"
    );
    assert!(
        counters.contains_key("intellisense_v2_parse_result_query_total"),
        "legacy parse_result counter must still be projected"
    );

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn syntax_diagnostics_metrics_follow_shared_parse_snapshot_mode_and_keep_aggregate_projection(
) {
    let coordinator = Arc::new(SystemCoordinator::new());
    let file_id = FileId(22);
    let deps_id = DepsSnapshotId::from_hash("deps_syntax_incremental");
    let settings_id = SettingsId::from_hash("settings_syntax_incremental");
    let text: Arc<str> = Arc::from("Процедура Тест()\n\tЕсли Истина Тогда\nКонецПроцедуры\n");
    let path: Arc<str> = Arc::from("syntax_incremental.bsl");

    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id.clone(),
        deps: make_deps(),
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: settings_id.clone(),
        diagnostics_detail_level: DetailLevel::Full,
    });
    let runtime = IntellisenseV2Facade::new(
        host,
        make_index_snapshot("syntax_incremental"),
        Some(coordinator.clone()),
    );

    runtime.apply_changes(vec![Change::SetFileWithSnapshot {
        file_id,
        text: text.clone(),
        version: 1,
        path: path.clone(),
        parse_snapshot: parse_snapshot_for_test(
            file_id,
            1,
            text.as_ref(),
            vec![ParseChangedRange {
                start_byte: 18,
                old_end_byte: 18,
                new_end_byte: 30,
            }],
            true,
            None,
        ),
    }]);

    let ready = timeout(
        Duration::from_secs(1),
        runtime.wait_for_file_version(file_id, 1),
    )
    .await
    .expect("wait_for_file_version timeout");
    assert!(ready, "expected incremental snapshot revision to be ready");

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Diagnostics,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id,
        min_file_version: Some(1),
        expected_deps_id: Some(deps_id),
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id,
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let prepared = runtime
        .prepare_stateful_operation(&context, Some(&coordinator))
        .await
        .expect("prepare_stateful_operation");
    let diagnostics = IntellisenseV2Facade::run_syntax_diagnostics_query_singleflight(
        &context,
        &prepared.snapshot.analysis,
        Some(&coordinator),
        file_id,
    )
    .expect("syntax diagnostics query");
    assert!(
        diagnostics.is_some(),
        "syntax diagnostics query should execute against snapshot-backed analysis"
    );

    let metrics = coordinator.observability_metrics();
    let counters = counters(&metrics);
    let histograms = histograms(&metrics);
    let drilldown_counter =
        "intellisense_v2_drilldown_stage_total_origin_lsp_mode_incremental_operation_diagnostics_stage_syntax_diagnostics_query";
    let drilldown_histogram =
        "intellisense_v2_drilldown_stage_latency_ms_origin_lsp_mode_incremental_operation_diagnostics_stage_syntax_diagnostics_query";

    assert_eq!(
        counter_value(counters, drilldown_counter),
        1,
        "syntax diagnostics stage must publish parse-mode drilldown for incremental snapshots"
    );
    assert_eq!(
        histogram_count(histograms, drilldown_histogram),
        1,
        "syntax diagnostics stage latency histogram must publish parse-mode drilldown"
    );
    assert_eq!(
        counter_value(counters, "intellisense_v2_syntax_diagnostics_query_total"),
        counter_value(counters, drilldown_counter),
        "legacy syntax_diagnostics total must remain deterministic aggregate projection"
    );
    assert_eq!(
        histogram_count(histograms, "intellisense_v2_syntax_diagnostics_query_ms"),
        histogram_count(histograms, drilldown_histogram),
        "legacy syntax_diagnostics latency histogram must remain deterministic aggregate projection"
    );

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn syntax_diagnostics_metrics_use_mode_other_for_non_lsp_without_parse_snapshot() {
    let coordinator = Arc::new(SystemCoordinator::new());
    let file_id = FileId(23);
    let deps_id = DepsSnapshotId::from_hash("deps_syntax_other");
    let settings_id = SettingsId::from_hash("settings_syntax_other");

    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id.clone(),
        deps: make_deps(),
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: settings_id.clone(),
        diagnostics_detail_level: DetailLevel::Full,
    });
    let runtime = IntellisenseV2Facade::new(
        host,
        make_index_snapshot("syntax_other"),
        Some(coordinator.clone()),
    );

    runtime.apply_changes(vec![Change::SetFile {
        file_id,
        text: Arc::from("Процедура Тест()\n\tЕсли Истина Тогда\nКонецПроцедуры\n"),
        version: 1,
        path: Arc::from("syntax_other.bsl"),
    }]);

    let ready = timeout(
        Duration::from_secs(1),
        runtime.wait_for_file_version(file_id, 1),
    )
    .await
    .expect("wait_for_file_version timeout");
    assert!(ready, "expected non-LSP diagnostics revision to be ready");

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Web,
        operation: SemanticOperation::Diagnostics,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id,
        min_file_version: Some(1),
        expected_deps_id: Some(deps_id),
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id,
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let prepared = runtime
        .prepare_stateful_operation(&context, Some(&coordinator))
        .await
        .expect("prepare_stateful_operation");
    let diagnostics = IntellisenseV2Facade::run_syntax_diagnostics_query_singleflight(
        &context,
        &prepared.snapshot.analysis,
        Some(&coordinator),
        file_id,
    )
    .expect("syntax diagnostics query");
    assert!(
        diagnostics.is_some(),
        "syntax diagnostics query should execute without snapshot-bound parse mode"
    );

    let metrics = coordinator.observability_metrics();
    let counters = counters(&metrics);
    let histograms = histograms(&metrics);
    let drilldown_counter =
        "intellisense_v2_drilldown_stage_total_origin_web_mode_other_operation_diagnostics_stage_syntax_diagnostics_query";
    let drilldown_histogram =
        "intellisense_v2_drilldown_stage_latency_ms_origin_web_mode_other_operation_diagnostics_stage_syntax_diagnostics_query";

    assert_eq!(
        counter_value(counters, drilldown_counter),
        1,
        "non-LSP syntax diagnostics without version-bound ParseSnapshot must publish mode_other"
    );
    assert_eq!(
        histogram_count(histograms, drilldown_histogram),
        1,
        "non-LSP syntax diagnostics latency must publish mode_other"
    );

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn interactive_prepare_timeout_rejects_stale_when_gap_exceeds_default() {
    let coordinator = SystemCoordinator::new();
    let file_id = FileId(11);
    let deps_id = DepsSnapshotId::from_hash("deps_stale_reject");
    let settings_id = SettingsId::from_hash("settings");

    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id.clone(),
        deps: make_deps(),
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: settings_id.clone(),
        diagnostics_detail_level: DetailLevel::Full,
    });
    let runtime = IntellisenseV2Facade::new(
        host,
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p9"))),
        None,
    );
    runtime.apply_changes(vec![Change::SetFile {
        file_id,
        text: Arc::from("x = 1;"),
        version: 2,
        path: Arc::from("stale_reject.bsl"),
    }]);
    let _ = runtime.snapshot().await;

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Completion,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id,
        min_file_version: Some(5),
        expected_deps_id: Some(deps_id),
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id: settings_id.clone(),
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let wait_budget_ms = crate::system::global_runtime_config()
        .get_u64(crate::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);
    let started = Instant::now();
    let result = runtime
        .prepare_stateful_operation(&context, Some(&coordinator))
        .await;
    let elapsed = started.elapsed();
    assert!(
        matches!(result, Err(SemanticOutcome::StaleVersion)),
        "gap > 1 should reject stale fallback under default policy"
    );
    let min_expected = Duration::from_millis(wait_budget_ms.saturating_sub(30));
    let max_expected = Duration::from_millis(wait_budget_ms.saturating_add(300));
    assert!(
            elapsed >= min_expected,
            "stale reject should spend wait budget before fail (elapsed={elapsed:?}, budget_ms={wait_budget_ms})"
        );
    assert!(
            elapsed <= max_expected,
            "stale reject should stay bounded near wait budget (elapsed={elapsed:?}, budget_ms={wait_budget_ms})"
        );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");
    assert!(
        counters.contains_key("intellisense_v2_completion_fallback_unavailable_total"),
        "completion fallback-unavailable counter should be recorded"
    );
    assert!(
        counters.contains_key("intellisense_v2_revision_lag_sample_total"),
        "revision lag counter should be recorded"
    );
    assert!(
        histograms.contains_key("intellisense_v2_revision_lag_versions"),
        "revision lag histogram should be recorded"
    );

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn interactive_prepare_timeout_rejects_preexisting_snapshot_and_stays_bounded() {
    let file_id = FileId(111);
    let deps_id = DepsSnapshotId::from_hash("deps_stale_age");
    let settings_id = SettingsId::from_hash("settings");

    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id.clone(),
        deps: make_deps(),
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: settings_id.clone(),
        diagnostics_detail_level: DetailLevel::Full,
    });
    let runtime = IntellisenseV2Facade::new(
        host,
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p9"))),
        None,
    );
    runtime.apply_changes(vec![Change::SetFile {
        file_id,
        text: Arc::from("x = 1;"),
        version: 4,
        path: Arc::from("stale_age.bsl"),
    }]);
    let _ = runtime.snapshot().await;

    // Older snapshots are no longer eligible for semantic rescue; keep an older snapshot only
    // to verify that the runtime still fails closed within the configured wait budget.
    tokio::time::sleep(Duration::from_millis(1100)).await;

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Completion,
        completion_mode: None,
        completion_large_churn_active: true,
        file_id,
        min_file_version: Some(5),
        expected_deps_id: Some(deps_id),
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id,
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let wait_budget_ms = crate::system::global_runtime_config()
        .get_u64(crate::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);
    let started = Instant::now();
    let result = runtime.prepare_stateful_operation(&context, None).await;
    let elapsed = started.elapsed();
    assert!(
        matches!(result, Err(SemanticOutcome::StaleVersion)),
        "interactive preparation must fail closed instead of reviving an older snapshot"
    );
    let min_expected = Duration::from_millis(wait_budget_ms.saturating_sub(30));
    let max_expected = Duration::from_millis(wait_budget_ms.saturating_add(400));
    assert!(
            elapsed >= min_expected,
            "bounded fail-closed reject should spend wait budget before fail (elapsed={elapsed:?}, budget_ms={wait_budget_ms})"
        );
    assert!(
            elapsed <= max_expected,
            "bounded fail-closed reject should stay near wait budget (elapsed={elapsed:?}, budget_ms={wait_budget_ms})"
        );

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn interactive_wait_budget_timeout_can_still_report_timeout_attribution_on_success() {
    let file_id = FileId(113);
    let deps_id = DepsSnapshotId::from_hash("deps_wait_budget_success");
    let settings_id = SettingsId::from_hash("settings_wait_budget_success");

    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id.clone(),
        deps: make_deps(),
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: settings_id.clone(),
        diagnostics_detail_level: DetailLevel::Full,
    });
    let runtime = IntellisenseV2Facade::new(
        host,
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p10"))),
        None,
    );
    runtime.apply_changes(vec![Change::SetFile {
        file_id,
        text: Arc::from("x = 1;"),
        version: 5,
        path: Arc::from("wait_budget_success.bsl"),
    }]);
    let _ = runtime.snapshot().await;

    let wait_budget_ms = crate::system::global_runtime_config()
        .get_u64(crate::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);
    let blocker = runtime.enqueue_test_sleep(
        RuntimeQueuePriority::Interactive,
        Duration::from_millis(wait_budget_ms.saturating_add(40)),
    );

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Completion,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id,
        min_file_version: Some(5),
        expected_deps_id: Some(deps_id),
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id,
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let prepared = runtime
        .prepare_stateful_operation(&context, None)
        .await
        .expect("prepare_stateful_operation should still succeed after timeout budget exhaustion");
    timeout(Duration::from_secs(1), blocker)
        .await
        .expect("interactive sleep ack timeout")
        .expect("interactive sleep ack");

    assert!(
        prepared.wait_budget_exhausted,
        "wait budget must be exhausted for this path"
    );
    let timeout_attribution = prepared
        .timeout_attribution
        .expect("timeout attribution must be captured on exhausted wait budget");
    assert_eq!(
        timeout_attribution.source,
        PrepareTimeoutSourceKind::InteractiveWaitBudget
    );
    assert_eq!(timeout_attribution.phase, "wait_for_file_version");
    assert_eq!(
        timeout_attribution.budget,
        Duration::from_millis(wait_budget_ms)
    );
    assert!(
        timeout_attribution.elapsed >= timeout_attribution.budget,
        "elapsed must not be smaller than configured wait budget"
    );
    assert!(
        timeout_attribution.overshoot > Duration::ZERO,
        "overshoot must be positive when timeout wakes late"
    );

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn snapshot_with_deps_timeout_can_report_queue_wait_runtime_split_via_progress() {
    let file_id = FileId(114);
    let deps_id = DepsSnapshotId::from_hash("deps_snapshot_timeout_queue_wait");
    let settings_id = SettingsId::from_hash("settings_snapshot_timeout_queue_wait");

    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id.clone(),
        deps: make_deps(),
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: settings_id.clone(),
        diagnostics_detail_level: DetailLevel::Full,
    });
    let runtime = IntellisenseV2Facade::new(
        host,
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p11"))),
        None,
    );
    runtime.apply_changes(vec![Change::SetFile {
        file_id,
        text: Arc::from("x = 1;"),
        version: 5,
        path: Arc::from("snapshot_timeout_queue_wait.bsl"),
    }]);

    let blocker = runtime.enqueue_test_sleep(
        RuntimeQueuePriority::Interactive,
        Duration::from_millis(200),
    );
    let progress = PrepareStatefulProgress::new();
    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Completion,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id,
        min_file_version: None,
        expected_deps_id: Some(deps_id),
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id,
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let result = tokio::time::timeout(
        Duration::from_millis(40),
        runtime.prepare_stateful_operation_with_progress(&context, None, Some(&progress)),
    )
    .await;
    assert!(
        result.is_err(),
        "outer timeout must fire while snapshot_with_deps is still queued"
    );
    let progress_snapshot = progress.snapshot();
    timeout(Duration::from_secs(1), blocker)
        .await
        .expect("interactive sleep ack timeout")
        .expect("interactive sleep ack");

    let snapshot_timeout_runtime = progress_snapshot
        .snapshot_with_deps_timeout_runtime
        .expect("snapshot timeout runtime must be captured on timeout");
    assert_eq!(
        snapshot_timeout_runtime.resolution,
        SnapshotWithDepsTimeoutResolutionKind::QueueWait
    );
    assert!(
        snapshot_timeout_runtime.queue_wait_elapsed.is_some(),
        "queue_wait resolution must carry bounded queue wait elapsed"
    );
    assert_eq!(snapshot_timeout_runtime.exec_elapsed, None);
    assert_eq!(snapshot_timeout_runtime.wake_wait_elapsed, None);

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn interactive_prepare_completion_reports_missing_deps_before_stale_acceptance() {
    let file_id = FileId(112);
    let deps_id_actual = DepsSnapshotId::from_hash("deps_actual");
    let deps_id_requested = DepsSnapshotId::from_hash("deps_requested");
    let settings_id = SettingsId::from_hash("settings");

    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id_actual.clone(),
        deps: make_deps(),
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: settings_id.clone(),
        diagnostics_detail_level: DetailLevel::Full,
    });
    let runtime = IntellisenseV2Facade::new(
        host,
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p9"))),
        None,
    );
    runtime.apply_changes(vec![Change::SetFile {
        file_id,
        text: Arc::from("x = 1;"),
        version: 4,
        path: Arc::from("stale_deps_mismatch.bsl"),
    }]);
    let _ = runtime.snapshot().await;

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Completion,
        completion_mode: None,
        completion_large_churn_active: true,
        file_id,
        min_file_version: Some(5),
        expected_deps_id: Some(deps_id_requested),
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id,
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let result = runtime.prepare_stateful_operation(&context, None).await;
    assert!(
        matches!(result, Err(SemanticOutcome::MissingDeps)),
        "deps mismatch must short-circuit stale acceptance with MissingDeps"
    );

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn interactive_prepare_timeout_rejects_stale_on_settings_mismatch() {
    let file_id = FileId(12);
    let deps_id = DepsSnapshotId::from_hash("deps_stale_mismatch");
    let stale_settings_id = SettingsId::from_hash("settings_old");
    let requested_settings_id = SettingsId::from_hash("settings_new");

    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id.clone(),
        deps: make_deps(),
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: stale_settings_id,
        diagnostics_detail_level: DetailLevel::Full,
    });
    let runtime = IntellisenseV2Facade::new(
        host,
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p9"))),
        None,
    );
    runtime.apply_changes(vec![Change::SetFile {
        file_id,
        text: Arc::from("x = 1;"),
        version: 4,
        path: Arc::from("stale_mismatch.bsl"),
    }]);
    let _ = runtime.snapshot().await;

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::SignatureHelp,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id,
        min_file_version: Some(5),
        expected_deps_id: Some(deps_id),
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id: requested_settings_id,
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let wait_budget_ms = crate::system::global_runtime_config()
        .get_u64(crate::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);
    let started = Instant::now();
    let result = runtime.prepare_stateful_operation(&context, None).await;
    let elapsed = started.elapsed();
    assert!(
        matches!(result, Err(SemanticOutcome::StaleVersion)),
        "settings mismatch must reject stale fallback"
    );
    let min_expected = Duration::from_millis(wait_budget_ms.saturating_sub(30));
    let max_expected = Duration::from_millis(wait_budget_ms.saturating_add(300));
    assert!(
            elapsed >= min_expected,
            "settings-mismatch reject should spend wait budget before fail (elapsed={elapsed:?}, budget_ms={wait_budget_ms})"
        );
    assert!(
            elapsed <= max_expected,
            "settings-mismatch reject should stay bounded near wait budget (elapsed={elapsed:?}, budget_ms={wait_budget_ms})"
        );

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn interactive_prepare_timeout_rejects_stale_without_expected_deps() {
    let runtime = IntellisenseV2Facade::new(
        AnalysisHostV2::default(),
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p9"))),
        None,
    );
    let file_id = FileId(13);
    let settings_id = SettingsId::from_hash("settings");

    runtime.apply_changes(vec![
        Change::SetSettingsSnapshot {
            settings_id: settings_id.clone(),
            diagnostics_detail_level: DetailLevel::Full,
        },
        Change::SetFile {
            file_id,
            text: Arc::from("x = 1;"),
            version: 4,
            path: Arc::from("stale_no_expected_deps.bsl"),
        },
    ]);

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Completion,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id,
        min_file_version: Some(5),
        expected_deps_id: None,
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id,
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let result = runtime.prepare_stateful_operation(&context, None).await;
    assert!(
        matches!(result, Err(SemanticOutcome::StaleVersion)),
        "stale fallback must be rejected when expected deps snapshot is unknown"
    );

    runtime.shutdown_for_test().await;
}

#[test]
fn run_parse_result_query_skips_when_policy_disallows_it() {
    let analysis = AnalysisHostV2::default().snapshot();
    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Hover,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id: FileId(1),
        min_file_version: None,
        expected_deps_id: None,
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id: SettingsId::from_hash("settings"),
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let mut called = false;
    let result = IntellisenseV2Facade::run_parse_result_query(
        &context,
        &analysis,
        false,
        None,
        |_analysis| {
            called = true;
            Ok::<Option<()>, ()>(None)
        },
    )
    .expect("query should not fail");

    assert!(result.is_none(), "parse_result should be skipped by policy");
    assert!(
        !called,
        "query closure must not be called when policy skips"
    );
}

#[test]
fn run_optional_query_records_ir_metrics() {
    let coordinator = SystemCoordinator::new();
    let analysis = AnalysisHostV2::default().snapshot();
    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Completion,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id: FileId(1),
        min_file_version: None,
        expected_deps_id: None,
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id: SettingsId::from_hash("settings"),
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let _ = IntellisenseV2Facade::run_optional_query(
        &context,
        ObservabilityStage::IrQuery,
        &analysis,
        Some(&coordinator),
        |_analysis| Ok::<Option<()>, ()>(None),
    )
    .expect("query should succeed");

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");

    assert!(
        counters.contains_key("intellisense_v2_ir_query_completion_total"),
        "IR counter should be recorded for completion"
    );
    assert!(
        histograms.contains_key("intellisense_v2_ir_query_completion_ms"),
        "IR histogram should be recorded for completion"
    );
}

#[test]
fn run_optional_query_best_effort_downgrades_cancellation_to_empty() {
    let coordinator = SystemCoordinator::new();
    let analysis = AnalysisHostV2::default().snapshot();
    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Members,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id: FileId(1),
        min_file_version: None,
        expected_deps_id: None,
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id: SettingsId::from_hash("settings"),
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let result = IntellisenseV2Facade::run_optional_query(
        &context,
        ObservabilityStage::IrQuery,
        &analysis,
        Some(&coordinator),
        |_analysis| Err::<Option<()>, ()>(()),
    )
    .expect("best effort should downgrade cancellation");
    assert!(
        result.is_none(),
        "best effort cancellation must return empty"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let cancelled = counters
        .get("intellisense_v2_ir_query_cancelled_total_other")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    assert!(
        cancelled > 0,
        "best effort should still expose cancelled counters"
    );
}

#[test]
fn run_optional_query_ignore_drops_cancellation_counters() {
    let coordinator = SystemCoordinator::new();
    let analysis = AnalysisHostV2::default().snapshot();
    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Members,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id: FileId(1),
        min_file_version: None,
        expected_deps_id: None,
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id: SettingsId::from_hash("settings"),
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::Ignore,
    };

    let result = IntellisenseV2Facade::run_optional_query(
        &context,
        ObservabilityStage::IrQuery,
        &analysis,
        Some(&coordinator),
        |_analysis| Err::<Option<()>, ()>(()),
    )
    .expect("ignore policy should drop cancellation error");
    assert!(result.is_none(), "ignore policy must return empty result");

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let cancelled = counters
        .get("intellisense_v2_ir_query_cancelled_total_other")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    assert_eq!(
        cancelled, 0,
        "ignore policy should suppress cancelled counters"
    );
}

#[test]
fn singleflight_scope_is_bound_only_for_ir() {
    assert!(
        IntellisenseV2Facade::singleflight_requires_snapshot_identity(SingleflightQueryKind::Ir),
        "IR should remain tied to deps/settings snapshots"
    );
    assert!(
        !IntellisenseV2Facade::singleflight_requires_snapshot_identity(
            SingleflightQueryKind::ParseResult
        ),
        "parse_result should not be tied to deps/settings snapshots"
    );
    assert!(
        !IntellisenseV2Facade::singleflight_requires_snapshot_identity(
            SingleflightQueryKind::SyntaxDiagnostics
        ),
        "syntax_diagnostics should not be tied to deps/settings snapshots"
    );
}

#[test]
fn singleflight_runs_leader_once_and_shares_result() {
    static TEST_FLIGHTS: OnceLock<SingleflightMap<Arc<String>>> = OnceLock::new();
    let key = SingleflightRevisionKey {
        file_id: FileId(777),
        file_version: 10,
        file_signature: "path:test://singleflight/777.bsl".to_string(),
        deps_id: Some(DepsSnapshotId::from_hash("deps")),
        settings_id: Some(SettingsId::from_hash("settings")),
        query_kind: SingleflightQueryKind::Ir,
    };
    let calls = Arc::new(AtomicUsize::new(0));

    let first_calls = calls.clone();
    let first_key = key.clone();
    let first = std::thread::spawn(move || {
        IntellisenseV2Facade::run_singleflight_query(
            &TEST_FLIGHTS,
            first_key,
            ObservabilityOrigin::Runtime,
            SingleflightQueryKind::Ir,
            None,
            || {
                first_calls.fetch_add(1, Ordering::SeqCst);
                std::thread::sleep(std::time::Duration::from_millis(60));
                Ok(Some(Arc::new(String::from("shared"))))
            },
        )
    });

    std::thread::sleep(std::time::Duration::from_millis(5));

    let second_calls = calls.clone();
    let second = std::thread::spawn(move || {
        IntellisenseV2Facade::run_singleflight_query(
            &TEST_FLIGHTS,
            key,
            ObservabilityOrigin::Runtime,
            SingleflightQueryKind::Ir,
            None,
            || {
                second_calls.fetch_add(1, Ordering::SeqCst);
                Ok(Some(Arc::new(String::from("second"))))
            },
        )
    });

    let first_result = first.join().expect("first thread join").expect("first ok");
    let second_result = second
        .join()
        .expect("second thread join")
        .expect("second ok");

    assert_eq!(
        first_result.as_ref().map(|value| value.as_str()),
        Some("shared")
    );
    assert_eq!(
        second_result.as_ref().map(|value| value.as_str()),
        Some("shared")
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn singleflight_propagates_leader_cancel_without_retry_and_cleans_up() {
    static TEST_FLIGHTS: OnceLock<SingleflightMap<Arc<String>>> = OnceLock::new();
    let key = SingleflightRevisionKey {
        file_id: FileId(778),
        file_version: 10,
        file_signature: "path:test://singleflight/778.bsl".to_string(),
        deps_id: None,
        settings_id: None,
        query_kind: SingleflightQueryKind::ParseResult,
    };
    let calls = Arc::new(AtomicUsize::new(0));

    let first_calls = calls.clone();
    let first_key = key.clone();
    let first = std::thread::spawn(move || {
        IntellisenseV2Facade::run_singleflight_query(
            &TEST_FLIGHTS,
            first_key,
            ObservabilityOrigin::Runtime,
            SingleflightQueryKind::ParseResult,
            None,
            || {
                first_calls.fetch_add(1, Ordering::SeqCst);
                std::thread::sleep(std::time::Duration::from_millis(60));
                Err(SingleflightQueryError::Cancelled)
            },
        )
    });

    std::thread::sleep(std::time::Duration::from_millis(5));

    let second_calls = calls.clone();
    let second_key = key.clone();
    let second = std::thread::spawn(move || {
        IntellisenseV2Facade::run_singleflight_query(
            &TEST_FLIGHTS,
            second_key,
            ObservabilityOrigin::Runtime,
            SingleflightQueryKind::ParseResult,
            None,
            || {
                second_calls.fetch_add(1, Ordering::SeqCst);
                Ok(Some(Arc::new(String::from("unexpected-retry"))))
            },
        )
    });

    let first_result = first.join().expect("first thread join");
    let second_result = second.join().expect("second thread join");
    assert!(first_result.is_err(), "leader must fail");
    assert!(
        second_result.is_err(),
        "follower must receive leader cancel"
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "follower must not trigger retry inside the same flight"
    );

    let map = TEST_FLIGHTS
        .get()
        .expect("test singleflight map should be initialized");
    let inflight_len = map
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .len();
    assert_eq!(inflight_len, 0, "flight entry must be cleaned up");

    let rerun_calls = calls.clone();
    let rerun = IntellisenseV2Facade::run_singleflight_query(
        &TEST_FLIGHTS,
        key,
        ObservabilityOrigin::Runtime,
        SingleflightQueryKind::ParseResult,
        None,
        || {
            rerun_calls.fetch_add(1, Ordering::SeqCst);
            Ok(Some(Arc::new(String::from("after-cleanup"))))
        },
    )
    .expect("new request after cleanup should run as new leader");
    assert_eq!(rerun.as_deref().map(String::as_str), Some("after-cleanup"));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn external_cancelled_ir_query_singleflight_does_not_publish_partial_head_artifact() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    static ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    let _env_lock = ENV_LOCK
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .expect("env lock");
    let _ir_delay_guard = EnvVarGuard::set("BSL_TEST_ANALYSIS_IR_BUILD_DELAY_MS", "200");

    let mut host = AnalysisHostV2::default();
    let file_id = FileId(901);
    let path: Arc<str> = Arc::from("cancelled_runtime_ir_singleflight.bsl");
    let file_text: Arc<str> = Arc::from(
        "Процедура Тест()\n    Значение = 1;\n    Результат = Значение + 1;\nКонецПроцедуры\n",
    );
    host.apply_change(Change::SetFileWithSnapshot {
        file_id,
        text: file_text.clone(),
        version: 1,
        path: path.clone(),
        parse_snapshot: parse_snapshot_for_test(file_id, 1, file_text.as_ref(), vec![], true, None),
    });

    let analysis = host.snapshot();
    let deps_id = analysis.deps_id().expect("deps id");
    let settings_id = analysis.settings_id().expect("settings id");
    let deps_id_for_query = deps_id.clone();
    let settings_id_for_query = settings_id.clone();
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancellation_check = bsl_analysis_v2::ExternalCancellationCheck::new({
        let cancelled = cancelled.clone();
        move || cancelled.load(Ordering::SeqCst)
    });
    let query = std::thread::spawn(move || {
        let context = ExecutionContext {
            origin: ObservabilityOrigin::Runtime,
            operation: SemanticOperation::Completion,
            completion_mode: None,
            completion_large_churn_active: false,
            file_id,
            min_file_version: Some(1),
            expected_deps_id: Some(deps_id_for_query),
            flow_sensitive: false,
            settings: ExecutionSettings {
                settings_id: settings_id_for_query,
                diagnostics_detail_level: bsl_shared::formatting::DetailLevel::Full,
            },
            cancellation: CancellationPolicy::RespectClientAbort,
        };

        IntellisenseV2Facade::run_ir_query_singleflight_with_cancellation(
            &context,
            &analysis,
            None,
            file_id,
            Some(cancellation_check),
        )
    });

    std::thread::sleep(std::time::Duration::from_millis(50));
    cancelled.store(true, Ordering::SeqCst);

    let cancelled = query.join().expect("ir query join");
    assert!(
        cancelled.is_err(),
        "explicit external cancellation must terminate exact IR singleflight with cancelled outcome"
    );

    let analysis_after_cancel = host.snapshot();
    assert!(
        !analysis_after_cancel
            .completion_head_ready_for_version(file_id, 1, &deps_id, &settings_id)
            .expect("completion_head_ready_for_version after external cancel"),
        "externally cancelled exact build must not publish partial completion head"
    );

    let rebuilt = analysis_after_cancel
        .ir_profiled(file_id)
        .expect("rebuilt ir")
        .expect("rebuilt ir present");
    assert!(
        rebuilt.profile.total_ms > 0,
        "rebuild after explicit cancel must still succeed for the same revision"
    );
    assert!(
        analysis_after_cancel
            .completion_head_ready_for_version(file_id, 1, &deps_id, &settings_id)
            .expect("completion_head_ready_for_version after rebuild"),
        "successful rebuild after explicit cancel must publish completion head"
    );
}

#[test]
fn singleflight_leader_panic_is_downgraded_and_cleans_up() {
    static TEST_FLIGHTS: OnceLock<SingleflightMap<Arc<String>>> = OnceLock::new();
    let key = SingleflightRevisionKey {
        file_id: FileId(780),
        file_version: 10,
        file_signature: "path:test://singleflight/780.bsl".to_string(),
        deps_id: None,
        settings_id: None,
        query_kind: SingleflightQueryKind::SyntaxDiagnostics,
    };

    let first_key = key.clone();
    let first = std::thread::spawn(move || {
        IntellisenseV2Facade::run_singleflight_query(
            &TEST_FLIGHTS,
            first_key,
            ObservabilityOrigin::Runtime,
            SingleflightQueryKind::SyntaxDiagnostics,
            None,
            || {
                std::thread::sleep(std::time::Duration::from_millis(60));
                panic!("leader panic must not leak in-flight entry")
            },
        )
    });

    std::thread::sleep(std::time::Duration::from_millis(5));

    let second = std::thread::spawn(move || {
        IntellisenseV2Facade::run_singleflight_query(
            &TEST_FLIGHTS,
            key,
            ObservabilityOrigin::Runtime,
            SingleflightQueryKind::SyntaxDiagnostics,
            None,
            || Ok(Some(Arc::new(String::from("unexpected-after-panic")))),
        )
    });

    let first_result = first.join().expect("first thread join");
    let second_result = second.join().expect("second thread join");
    assert!(
        matches!(first_result, Err(SingleflightQueryError::Cancelled)),
        "leader panic must be exposed as cancelled outcome"
    );
    assert!(
        matches!(second_result, Err(SingleflightQueryError::Cancelled)),
        "follower must receive terminal leader outcome when panic happens"
    );

    let map = TEST_FLIGHTS
        .get()
        .expect("test singleflight map should be initialized");
    let inflight_len = map
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .len();
    assert_eq!(
        inflight_len, 0,
        "singleflight key must be cleaned up after panic"
    );
}

#[test]
fn singleflight_records_leader_shared_and_wait_metrics() {
    static TEST_FLIGHTS: OnceLock<SingleflightMap<Arc<String>>> = OnceLock::new();
    let key = SingleflightRevisionKey {
        file_id: FileId(779),
        file_version: 10,
        file_signature: "path:test://singleflight/779.bsl".to_string(),
        deps_id: None,
        settings_id: None,
        query_kind: SingleflightQueryKind::SyntaxDiagnostics,
    };
    let coordinator = Arc::new(SystemCoordinator::new());

    let first_coordinator = coordinator.clone();
    let first_key = key.clone();
    let first = std::thread::spawn(move || {
        IntellisenseV2Facade::run_singleflight_query(
            &TEST_FLIGHTS,
            first_key,
            ObservabilityOrigin::Runtime,
            SingleflightQueryKind::SyntaxDiagnostics,
            Some(first_coordinator.as_ref()),
            || {
                std::thread::sleep(std::time::Duration::from_millis(50));
                Ok(Some(Arc::new(String::from("shared"))))
            },
        )
    });

    std::thread::sleep(std::time::Duration::from_millis(5));

    let second_coordinator = coordinator.clone();
    let second = std::thread::spawn(move || {
        IntellisenseV2Facade::run_singleflight_query(
            &TEST_FLIGHTS,
            key,
            ObservabilityOrigin::Runtime,
            SingleflightQueryKind::SyntaxDiagnostics,
            Some(second_coordinator.as_ref()),
            || Ok(Some(Arc::new(String::from("second")))),
        )
    });

    let _ = first.join().expect("first thread join").expect("first ok");
    let _ = second
        .join()
        .expect("second thread join")
        .expect("second ok");

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");

    assert!(
        counters.contains_key("intellisense_v2_singleflight_leader_total"),
        "singleflight leader counter should be recorded"
    );
    assert!(
        counters.contains_key("intellisense_v2_singleflight_shared_total"),
        "singleflight shared counter should be recorded"
    );
    assert!(
        histograms.contains_key("intellisense_v2_singleflight_wait_ms"),
        "singleflight wait histogram should be recorded"
    );
}

#[tokio::test]
async fn parity_stateful_and_ephemeral_diagnostics_are_equal() {
    let deps = make_deps();
    let deps_id = DepsSnapshotId::from_hash("deps_parity");
    let settings_id = SettingsId::from_hash("settings_parity");
    let settings = ExecutionSettings {
        settings_id: settings_id.clone(),
        diagnostics_detail_level: DetailLevel::Full,
    };
    let file_id = FileId(11);
    let code: Arc<str> = Arc::from("Процедура Тест()\n\tМассив1.Добавить(1);\nКонецПроцедуры\n");
    let path: Arc<str> = Arc::from("parity_test.bsl");

    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id.clone(),
        deps: deps.clone(),
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: settings_id.clone(),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(Change::SetFile {
        file_id,
        text: code.clone(),
        version: 1,
        path: path.clone(),
    });
    let runtime = IntellisenseV2Facade::new(host, make_index_snapshot("index_parity"), None);
    let stateful = runtime.snapshot().await;

    let ephemeral = IntellisenseV2Facade::ephemeral_snapshot(
        deps_id,
        deps,
        make_index_snapshot("index_parity"),
        settings,
        file_id,
        code,
        1,
        path,
    )
    .analysis;

    let stateful_syntax = stateful
        .syntax_diagnostics(file_id)
        .expect("stateful syntax query")
        .unwrap_or_else(|| Arc::new(Vec::new()));
    let ephemeral_syntax = ephemeral
        .syntax_diagnostics(file_id)
        .expect("ephemeral syntax query")
        .unwrap_or_else(|| Arc::new(Vec::new()));

    let stateful_semantic = stateful
        .semantic_diagnostics(file_id)
        .expect("stateful semantic query")
        .unwrap_or_else(|| Arc::new(Vec::new()));
    let ephemeral_semantic = ephemeral
        .semantic_diagnostics(file_id)
        .expect("ephemeral semantic query")
        .unwrap_or_else(|| Arc::new(Vec::new()));

    let syntax_key =
        |d: &bsl_shared::domain::types::ParseError| (d.message.clone(), d.span.start, d.span.end);
    let semantic_key = |d: &bsl_shared::domain::types::TypeDiagnostic| {
        (
            d.message.clone(),
            d.span.start,
            d.span.end,
            format!("{:?}", d.severity),
        )
    };

    let mut left_syntax: Vec<_> = stateful_syntax.iter().map(syntax_key).collect();
    let mut right_syntax: Vec<_> = ephemeral_syntax.iter().map(syntax_key).collect();
    left_syntax.sort();
    right_syntax.sort();
    assert_eq!(
        left_syntax, right_syntax,
        "syntax diagnostics parity mismatch"
    );

    let mut left_semantic: Vec<_> = stateful_semantic.iter().map(semantic_key).collect();
    let mut right_semantic: Vec<_> = ephemeral_semantic.iter().map(semantic_key).collect();
    left_semantic.sort();
    right_semantic.sort();
    assert_eq!(
        left_semantic, right_semantic,
        "semantic diagnostics parity mismatch"
    );

    runtime.shutdown_for_test().await;
}
