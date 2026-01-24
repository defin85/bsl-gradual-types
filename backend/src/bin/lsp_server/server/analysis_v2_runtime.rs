use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::oneshot;
use tracing::warn;

use bsl_analysis_v2::{AnalysisHostV2, AnalysisV2, Change, DepsSnapshotId, FileId, SemanticDeps};
use bsl_backend::system::{IndexSnapshot, IndexSnapshotId};

#[derive(Clone)]
pub(crate) struct AnalysisV2Runtime {
    inner: Arc<Inner>,
}

struct Inner {
    tx: std::sync::mpsc::Sender<Command>,
    #[cfg(test)]
    join_handle: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>,
}

enum Command {
    ApplyChanges {
        changes: Vec<Change>,
    },
    ApplyDepsBundle {
        deps_id: DepsSnapshotId,
        deps: Arc<SemanticDeps>,
        index_snapshot: Arc<IndexSnapshot>,
        reply: oneshot::Sender<bool>,
    },
    GetSnapshot {
        reply: oneshot::Sender<AnalysisV2>,
    },
    GetSnapshotWithDeps {
        reply: oneshot::Sender<(AnalysisV2, Arc<IndexSnapshot>, DepsSnapshotId)>,
    },
    WaitForFileVersion {
        file_id: FileId,
        min_version: i32,
        reply: oneshot::Sender<bool>,
    },
    #[cfg(test)]
    Shutdown {
        ack: oneshot::Sender<()>,
    },
}

impl AnalysisV2Runtime {
    pub(crate) fn new(
        initial_host: AnalysisHostV2,
        initial_index_snapshot: Arc<IndexSnapshot>,
    ) -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<Command>();

        let join_handle = std::thread::Builder::new()
            .name("analysis-v2-writer".to_string())
            .spawn(move || {
                let mut host = initial_host;
                let mut current_deps_id = host.deps_id();
                let mut index_snapshot = initial_index_snapshot;
                let mut applied_file_versions: HashMap<FileId, i32> = HashMap::new();
                let mut waiters: HashMap<FileId, Vec<(i32, oneshot::Sender<bool>)>> =
                    HashMap::new();

                let wake_waiters_for_file =
                    |file_id: FileId,
                     current_version: Option<i32>,
                     waiters: &mut HashMap<FileId, Vec<(i32, oneshot::Sender<bool>)>>| {
                        let Some(pending) = waiters.remove(&file_id) else {
                            return;
                        };

                        let mut still_waiting = Vec::new();
                        for (min_version, reply) in pending {
                            match current_version {
                                None => {
                                    let _ = reply.send(false);
                                }
                                Some(version) if version >= min_version => {
                                    let _ = reply.send(true);
                                }
                                Some(_) => still_waiting.push((min_version, reply)),
                            }
                        }

                        if !still_waiting.is_empty() {
                            waiters.insert(file_id, still_waiting);
                        }
                    };

                while let Ok(cmd) = rx.recv() {
                    match cmd {
                        Command::ApplyChanges { changes } => {
                            let mut changed_files = Vec::new();

                            for change in changes {
                                match &change {
                                    Change::SetFile { file_id, version, .. } => {
                                        applied_file_versions.insert(*file_id, *version);
                                        changed_files.push(*file_id);
                                    }
                                    Change::RemoveFile { file_id } => {
                                        applied_file_versions.remove(file_id);
                                        changed_files.push(*file_id);
                                    }
                                    Change::SetDepsSnapshot { .. } => {
                                        warn!("analysis_v2_runtime: ignoring SetDepsSnapshot in ApplyChanges; use ApplyDepsBundle to keep index_snapshot in sync");
                                        continue;
                                    }
                                    Change::SetSettingsSnapshot { .. } => {}
                                }

                                host.apply_change(change);
                            }

                            for file_id in changed_files {
                                let version = applied_file_versions.get(&file_id).copied();
                                wake_waiters_for_file(file_id, version, &mut waiters);
                            }
                        }
                        Command::ApplyDepsBundle {
                            deps_id,
                            deps,
                            index_snapshot: new_index_snapshot,
                            reply,
                        } => {
                            current_deps_id = deps_id.clone();
                            index_snapshot = new_index_snapshot;
                            host.apply_change(Change::SetDepsSnapshot { deps_id, deps });
                            let _ = reply.send(true);
                        }
                        Command::GetSnapshot { reply } => {
                            let _ = reply.send(host.snapshot());
                        }
                        Command::GetSnapshotWithDeps { reply } => {
                            let _ = reply.send((
                                host.snapshot(),
                                index_snapshot.clone(),
                                current_deps_id.clone(),
                            ));
                        }
                        Command::WaitForFileVersion {
                            file_id,
                            min_version,
                            reply,
                        } => match applied_file_versions.get(&file_id).copied() {
                            Some(version) if version >= min_version => {
                                let _ = reply.send(true);
                            }
                            _ => {
                                waiters.entry(file_id).or_default().push((min_version, reply));
                            }
                        },
                        #[cfg(test)]
                        Command::Shutdown { ack } => {
                            for (_file_id, pending) in waiters.drain() {
                                for (_min_version, waiter) in pending {
                                    let _ = waiter.send(false);
                                }
                            }
                            let _ = ack.send(());
                            break;
                        }
                    }
                }
            })
            .expect("failed to spawn analysis-v2 writer thread");

        #[cfg(not(test))]
        let _ = join_handle;

        Self {
            inner: Arc::new(Inner {
                tx,
                #[cfg(test)]
                join_handle: std::sync::Mutex::new(Some(join_handle)),
            }),
        }
    }

    pub(crate) fn apply_changes(&self, changes: Vec<Change>) {
        if changes.is_empty() {
            return;
        }
        if self
            .inner
            .tx
            .send(Command::ApplyChanges { changes })
            .is_err()
        {
            warn!("analysis_v2_runtime: failed to send ApplyChanges (writer thread is gone)");
        }
    }

    pub(crate) async fn apply_deps_bundle(
        &self,
        deps_id: DepsSnapshotId,
        deps: Arc<SemanticDeps>,
        index_snapshot: Arc<IndexSnapshot>,
    ) -> bool {
        let (reply, rx) = oneshot::channel::<bool>();
        if self
            .inner
            .tx
            .send(Command::ApplyDepsBundle {
                deps_id,
                deps,
                index_snapshot,
                reply,
            })
            .is_err()
        {
            warn!("analysis_v2_runtime: failed to send ApplyDepsBundle (writer thread is gone)");
            return false;
        }
        rx.await.unwrap_or(false)
    }

    pub(crate) async fn snapshot(&self) -> AnalysisV2 {
        let (reply, rx) = oneshot::channel::<AnalysisV2>();
        if self.inner.tx.send(Command::GetSnapshot { reply }).is_err() {
            warn!("analysis_v2_runtime: failed to send GetSnapshot (writer thread is gone)");
            return AnalysisHostV2::default().snapshot();
        }
        match rx.await {
            Ok(snapshot) => snapshot,
            Err(_) => {
                warn!("analysis_v2_runtime: GetSnapshot response cancelled");
                AnalysisHostV2::default().snapshot()
            }
        }
    }

    pub(crate) async fn snapshot_with_deps(
        &self,
    ) -> (AnalysisV2, Arc<IndexSnapshot>, DepsSnapshotId) {
        let (reply, rx) = oneshot::channel::<(AnalysisV2, Arc<IndexSnapshot>, DepsSnapshotId)>();
        if self
            .inner
            .tx
            .send(Command::GetSnapshotWithDeps { reply })
            .is_err()
        {
            warn!(
                "analysis_v2_runtime: failed to send GetSnapshotWithDeps (writer thread is gone)"
            );
            return (
                AnalysisHostV2::default().snapshot(),
                Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash(""))),
                DepsSnapshotId::from_hash(""),
            );
        }

        match rx.await {
            Ok(tuple) => tuple,
            Err(_) => {
                warn!("analysis_v2_runtime: GetSnapshotWithDeps response cancelled");
                (
                    AnalysisHostV2::default().snapshot(),
                    Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash(""))),
                    DepsSnapshotId::from_hash(""),
                )
            }
        }
    }

    pub(crate) async fn wait_for_file_version(&self, file_id: FileId, min_version: i32) -> bool {
        let (reply, rx) = oneshot::channel::<bool>();
        if self
            .inner
            .tx
            .send(Command::WaitForFileVersion {
                file_id,
                min_version,
                reply,
            })
            .is_err()
        {
            warn!("analysis_v2_runtime: failed to send WaitForFileVersion (writer thread is gone)");
            return false;
        }
        rx.await.unwrap_or(false)
    }

    #[cfg(test)]
    pub(crate) async fn shutdown_for_test(&self) {
        let (ack, rx) = oneshot::channel::<()>();
        let _ = self.inner.tx.send(Command::Shutdown { ack });
        let _ = rx.await;

        let join_handle = self.inner.join_handle.lock().unwrap().take();
        if let Some(handle) = join_handle {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::time::{timeout, Duration};

    use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
    use bsl_shared::domain::resolver::TypeResolver;

    #[tokio::test]
    async fn p7_apply_changes_and_wait_for_version_works() {
        let runtime = AnalysisV2Runtime::new(
            AnalysisHostV2::default(),
            Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p7"))),
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
        let runtime = AnalysisV2Runtime::new(
            AnalysisHostV2::default(),
            Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p7"))),
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

    #[tokio::test]
    async fn p8_snapshot_with_deps_is_atomic() {
        let mut host = AnalysisHostV2::default();

        let deps_old = make_deps();
        let deps_id_old = DepsSnapshotId::from_hash("deps_old");
        host.apply_change(Change::SetDepsSnapshot {
            deps_id: deps_id_old.clone(),
            deps: deps_old,
        });

        let runtime = AnalysisV2Runtime::new(host, make_index_snapshot("index_old"));

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
    async fn p8_apply_changes_ignores_set_deps_snapshot() {
        let mut host = AnalysisHostV2::default();

        let deps_old = make_deps();
        let deps_id_old = DepsSnapshotId::from_hash("deps_old");
        host.apply_change(Change::SetDepsSnapshot {
            deps_id: deps_id_old.clone(),
            deps: deps_old,
        });

        let runtime = AnalysisV2Runtime::new(host, make_index_snapshot("index_old"));

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
}
