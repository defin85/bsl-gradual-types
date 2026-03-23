use super::*;

#[cfg(test)]
fn maybe_inject_apply_change_delay_for_test(change: &Change) {
    let env_key = match change {
        Change::SetFile { .. } => Some("BSL_TEST_RUNTIME_APPLY_SET_FILE_DELAY_MS"),
        Change::SetFileWithSnapshot { .. } => {
            Some("BSL_TEST_RUNTIME_APPLY_SET_FILE_WITH_SNAPSHOT_DELAY_MS")
        }
        _ => None,
    };
    let Some(env_key) = env_key else {
        return;
    };
    if let Some(delay_ms) = std::env::var(env_key)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
    {
        std::thread::sleep(Duration::from_millis(delay_ms));
    }
}

#[cfg(not(test))]
fn maybe_inject_apply_change_delay_for_test(_change: &Change) {}

impl IntellisenseV2Facade {
    pub fn new(
        initial_host: AnalysisHostV2,
        initial_index_snapshot: Arc<IndexSnapshot>,
        observability: Option<Arc<SystemCoordinator>>,
    ) -> Self {
        let (interactive_tx, interactive_rx) = std::sync::mpsc::channel::<Command>();
        let (background_tx, background_rx) = std::sync::mpsc::channel::<Command>();
        let initial_snapshot = initial_host.snapshot();
        let initial_deps_id = initial_snapshot.deps_id().expect("initial deps id");
        let initial_deps = initial_snapshot.deps_data().expect("initial deps data");
        let completion_deps_index_snapshot =
            Arc::new(ArcSwap::from_pointee(CompletionDepsIndexSnapshot {
                deps: initial_deps,
                deps_id: initial_deps_id,
                index_snapshot: initial_index_snapshot.clone(),
            }));
        let completion_deps_index_snapshot_for_writer = completion_deps_index_snapshot.clone();

        let join_handle = std::thread::Builder::new()
            .name("analysis-v2-writer".to_string())
            .spawn(move || {
                let mut host = initial_host;
                let mut current_deps_id = host.deps_id();
                let mut index_snapshot = initial_index_snapshot;
                let mut applied_file_revisions: HashMap<FileId, FileRevisionState> =
                    HashMap::new();
                let mut waiters: HashMap<FileId, Vec<PendingWaiter>> = HashMap::new();
                let mut interactive_streak = 0usize;
                let mut interactive_closed = false;
                let mut background_closed = false;
                let mut pending_interactive_command = None;

                let wake_waiters_for_file =
                    |file_id: FileId,
                     current_version: Option<i32>,
                     waiters: &mut HashMap<FileId, Vec<PendingWaiter>>,
                     observability: &Option<Arc<SystemCoordinator>>| {
                        let Some(pending) = waiters.remove(&file_id) else {
                            return;
                        };

                        let mut still_waiting = Vec::new();
                        for waiter in pending {
                            match current_version {
                                None => {
                                    let exec_elapsed = waiter.started_waiting_at.elapsed();
                                    if let Some(coordinator) = observability {
                                        coordinator.record_intellisense_v2_runtime_exec_latency(
                                            "wait_for_file_version",
                                            exec_elapsed,
                                        );
                                        coordinator.record_intellisense_v2_runtime_exec_class_latency_with_origin(
                                            waiter.origin.as_str(),
                                            waiter.priority.as_work_class(),
                                            exec_elapsed,
                                        );
                                    }
                                    let wake_wait_elapsed = waiter.started_waiting_at.elapsed();
                                    let exec_started = Instant::now();
                                    let exec_elapsed = exec_started.elapsed();
                                    let _ = waiter.reply.send(WaitForFileVersionReply {
                                        ready: false,
                                        trace: WaitForFileVersionRuntimeTrace {
                                            queue_wait_elapsed: Some(waiter.queue_wait_elapsed),
                                            exec_elapsed: Some(exec_elapsed),
                                            wake_wait_elapsed: Some(wake_wait_elapsed),
                                            resolution: Some(
                                                WaitForFileVersionResolutionKind::Waiter,
                                            ),
                                        },
                                    });
                                }
                                Some(version) if version >= waiter.min_version => {
                                    let exec_elapsed = waiter.started_waiting_at.elapsed();
                                    if let Some(coordinator) = observability {
                                        coordinator.record_intellisense_v2_runtime_exec_latency(
                                            "wait_for_file_version",
                                            exec_elapsed,
                                        );
                                        coordinator.record_intellisense_v2_runtime_exec_class_latency_with_origin(
                                            waiter.origin.as_str(),
                                            waiter.priority.as_work_class(),
                                            exec_elapsed,
                                        );
                                    }
                                    let wake_wait_elapsed = waiter.started_waiting_at.elapsed();
                                    let exec_started = Instant::now();
                                    let exec_elapsed = exec_started.elapsed();
                                    let _ = waiter.reply.send(WaitForFileVersionReply {
                                        ready: true,
                                        trace: WaitForFileVersionRuntimeTrace {
                                            queue_wait_elapsed: Some(waiter.queue_wait_elapsed),
                                            exec_elapsed: Some(exec_elapsed),
                                            wake_wait_elapsed: Some(wake_wait_elapsed),
                                            resolution: Some(
                                                WaitForFileVersionResolutionKind::Waiter,
                                            ),
                                        },
                                    });
                                }
                                Some(_) => still_waiting.push(waiter),
                            }
                        }

                        if !still_waiting.is_empty() {
                            waiters.insert(file_id, still_waiting);
                        }
                    };

                while let Some((queue_priority, cmd)) =
                    if let Some(command) = pending_interactive_command.take() {
                        interactive_streak = interactive_streak.saturating_add(1);
                        Some((RuntimeQueuePriority::Interactive, command))
                    } else {
                        recv_next_writer_command(
                            &interactive_rx,
                            &background_rx,
                            &mut interactive_streak,
                            &mut interactive_closed,
                            &mut background_closed,
                        )
                    }
                {
                    let cmd = if queue_priority == RuntimeQueuePriority::Interactive {
                        coalesce_interactive_current_revision_apply_command(
                            &interactive_rx,
                            cmd,
                            &mut pending_interactive_command,
                        )
                    } else {
                        cmd
                    };
                    match cmd {
                        Command::ApplyChanges {
                            origin,
                            enqueued_at,
                            changes,
                        } => {
                            let queue_wait_elapsed = enqueued_at.elapsed();
                            if let Some(coordinator) = &observability {
                                coordinator.record_intellisense_v2_runtime_queue_wait_latency_with_origin(
                                    origin.as_str(),
                                    "apply_changes_batch",
                                    queue_wait_elapsed,
                                );
                                coordinator.record_intellisense_v2_runtime_queue_wait_class_latency_with_origin(
                                    origin.as_str(),
                                    queue_priority.as_work_class(),
                                    queue_wait_elapsed,
                                );
                                coordinator.record_intellisense_v2_runtime_apply_changes_batch_size(
                                    changes.len(),
                                );
                            }

                            let exec_started = Instant::now();
                            let mut changed_files = Vec::new();

                            for change in changes {
                                let per_change_started = Instant::now();
                                let change_kind = match &change {
                                    Change::SetFile { .. } => Some("apply_change_set_file"),
                                    Change::SetFileWithSnapshot { .. } => {
                                        Some("apply_change_set_file_with_snapshot")
                                    }
                                    Change::ReuseCompletionHeadFromPreviousVersion { .. } => {
                                        Some("apply_change_reuse_completion_head_from_previous_version")
                                    }
                                    Change::RemoveFile { .. } => Some("apply_change_remove_file"),
                                    Change::SetSettingsSnapshot { .. } => {
                                        Some("apply_change_set_settings_snapshot")
                                    }
                                    Change::SetDepsSnapshot { .. } => None,
                                };
                                let skip_stale_change = match &change {
                                    Change::SetFile {
                                        file_id, version, ..
                                    } => applied_file_revisions
                                        .get(file_id)
                                        .is_some_and(|state| state.version > *version),
                                    Change::SetFileWithSnapshot {
                                        file_id, version, ..
                                    } => applied_file_revisions
                                        .get(file_id)
                                        .is_some_and(|state| state.version > *version),
                                    Change::ReuseCompletionHeadFromPreviousVersion {
                                        file_id,
                                        expected_version,
                                        ..
                                    } => applied_file_revisions
                                        .get(file_id)
                                        .is_some_and(|state| state.version > *expected_version),
                                    _ => false,
                                };
                                if skip_stale_change {
                                    continue;
                                }
                                maybe_inject_apply_change_delay_for_test(&change);
                                match &change {
                                    Change::SetFile { file_id, version, .. }
                                    | Change::SetFileWithSnapshot {
                                        file_id, version, ..
                                    } => {
                                        applied_file_revisions.insert(
                                            *file_id,
                                            FileRevisionState {
                                                version: *version,
                                                updated_at: Instant::now(),
                                            },
                                        );
                                        changed_files.push(*file_id);
                                    }
                                    Change::RemoveFile { file_id } => {
                                        applied_file_revisions.remove(file_id);
                                        changed_files.push(*file_id);
                                    }
                                    Change::ReuseCompletionHeadFromPreviousVersion { .. } => {}
                                    Change::SetDepsSnapshot { .. } => {
                                        warn!("analysis_v2_runtime: ignoring SetDepsSnapshot in ApplyChanges; use ApplyDepsBundle to keep index_snapshot in sync");
                                        continue;
                                    }
                                    Change::SetSettingsSnapshot { .. } => {}
                                }

                                let cache_effects = host.apply_change(change);
                                if let Some(coordinator) = &observability {
                                    if cache_effects.invalidated_deps_total > 0 {
                                        coordinator.record_intellisense_v2_type_index_reason(
                                            bsl_analysis_v2::TypeIndexArtifactReasonCode::TypeIndexArtifactInvalidatedDeps
                                                .as_str(),
                                        );
                                    }
                                    if cache_effects.invalidated_settings_total > 0 {
                                        coordinator.record_intellisense_v2_type_index_reason(
                                            bsl_analysis_v2::TypeIndexArtifactReasonCode::TypeIndexArtifactInvalidatedSettings
                                                .as_str(),
                                        );
                                    }
                                    if cache_effects.evicted_per_file_window_total > 0 {
                                        coordinator.record_intellisense_v2_type_index_reason(
                                            bsl_analysis_v2::TypeIndexArtifactReasonCode::TypeIndexArtifactEvictedPerFileWindow
                                                .as_str(),
                                        );
                                    }
                                }
                                if let Some(kind) = change_kind {
                                    let exec_elapsed = per_change_started.elapsed();
                                    if let Some(coordinator) = &observability {
                                        coordinator.record_intellisense_v2_runtime_exec_latency_with_origin(
                                            origin.as_str(),
                                            kind,
                                            exec_elapsed,
                                        );
                                    }
                                }
                            }
                            let changed_files_count = changed_files.len();
                            for file_id in changed_files {
                                let version = applied_file_revisions
                                    .get(&file_id)
                                    .map(|state| state.version);
                                wake_waiters_for_file(
                                    file_id,
                                    version,
                                    &mut waiters,
                                    &observability,
                                );
                            }

                            let exec_elapsed = exec_started.elapsed();
                            if let Some(coordinator) = &observability {
                                coordinator.record_intellisense_v2_runtime_exec_latency_with_origin(
                                    origin.as_str(),
                                    "apply_changes_batch",
                                    exec_elapsed,
                                );
                                coordinator.record_intellisense_v2_runtime_exec_class_latency_with_origin(
                                    origin.as_str(),
                                    queue_priority.as_work_class(),
                                    exec_elapsed,
                                );
                                coordinator
                                    .record_intellisense_v2_runtime_apply_changes_changed_files_count(
                                        changed_files_count,
                                    );
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
                            let cache_effects =
                                host.apply_change(Change::SetDepsSnapshot { deps_id, deps });
                            completion_deps_index_snapshot_for_writer.store(Arc::new(
                                CompletionDepsIndexSnapshot {
                                    deps: host
                                        .snapshot()
                                        .deps_data()
                                        .expect("deps after ApplyDepsBundle"),
                                    deps_id: current_deps_id.clone(),
                                    index_snapshot: index_snapshot.clone(),
                                },
                            ));
                            if let Some(coordinator) = &observability {
                                if cache_effects.invalidated_deps_total > 0 {
                                    coordinator.record_intellisense_v2_type_index_reason(
                                        bsl_analysis_v2::TypeIndexArtifactReasonCode::TypeIndexArtifactInvalidatedDeps
                                            .as_str(),
                                    );
                                }
                            }
                            let _ = reply.send(true);
                        }
                        Command::GetSnapshot { reply } => {
                            let _ = reply.send(host.snapshot());
                        }
                        Command::GetSnapshotWithDeps {
                            origin,
                            enqueued_at,
                            progress,
                            reply,
                        } => {
                            let queue_wait_elapsed = enqueued_at.elapsed();
                            if let Some(coordinator) = &observability {
                                coordinator.record_intellisense_v2_runtime_queue_wait_latency(
                                    "snapshot_with_deps",
                                    queue_wait_elapsed,
                                );
                                coordinator.record_intellisense_v2_runtime_queue_wait_class_latency_with_origin(
                                    origin.as_str(),
                                    queue_priority.as_work_class(),
                                    queue_wait_elapsed,
                                );
                            }
                            if let Some(progress) = progress.as_ref() {
                                progress.mark_snapshot_with_deps_exec_started(queue_wait_elapsed);
                            }

                            let exec_started = Instant::now();
                            let response = GetSnapshotWithDepsReply {
                                analysis: host.snapshot(),
                                index_snapshot: index_snapshot.clone(),
                                deps_id: current_deps_id.clone(),
                                trace: SnapshotWithDepsRuntimeTrace {
                                    queue_wait_elapsed: Some(queue_wait_elapsed),
                                    exec_elapsed: None,
                                },
                            };
                            let exec_elapsed = exec_started.elapsed();
                            let response = GetSnapshotWithDepsReply {
                                trace: SnapshotWithDepsRuntimeTrace {
                                    exec_elapsed: Some(exec_elapsed),
                                    ..response.trace
                                },
                                ..response
                            };
                            if let Some(coordinator) = &observability {
                                coordinator.record_intellisense_v2_runtime_exec_latency(
                                    "snapshot_with_deps",
                                    exec_elapsed,
                                );
                                coordinator.record_intellisense_v2_runtime_exec_class_latency_with_origin(
                                    origin.as_str(),
                                    queue_priority.as_work_class(),
                                    exec_elapsed,
                                );
                            }
                            if let Some(progress) = progress.as_ref() {
                                progress.mark_snapshot_with_deps_wake_wait(
                                    queue_wait_elapsed,
                                    exec_elapsed,
                                );
                            }
                            let _ = reply.send(response);
                        }
                        Command::WaitForFileVersion {
                            origin,
                            enqueued_at,
                            file_id,
                            min_version,
                            reply,
                        } => {
                            let queue_wait_elapsed = enqueued_at.elapsed();
                            if let Some(coordinator) = &observability {
                                coordinator.record_intellisense_v2_runtime_queue_wait_latency(
                                    "wait_for_file_version",
                                    queue_wait_elapsed,
                                );
                                coordinator.record_intellisense_v2_runtime_queue_wait_class_latency_with_origin(
                                    origin.as_str(),
                                    queue_priority.as_work_class(),
                                    queue_wait_elapsed,
                                );
                            }

                            match applied_file_revisions.get(&file_id).map(|state| state.version) {
                                Some(version) if version >= min_version => {
                                    let exec_started = Instant::now();
                                    let exec_elapsed = exec_started.elapsed();
                                    let _ = reply.send(WaitForFileVersionReply {
                                        ready: true,
                                        trace: WaitForFileVersionRuntimeTrace {
                                            queue_wait_elapsed: Some(queue_wait_elapsed),
                                            exec_elapsed: Some(exec_elapsed),
                                            wake_wait_elapsed: None,
                                            resolution: Some(
                                                WaitForFileVersionResolutionKind::Immediate,
                                            ),
                                        },
                                    });
                                    if let Some(coordinator) = &observability {
                                        coordinator.record_intellisense_v2_runtime_exec_latency(
                                            "wait_for_file_version",
                                            exec_elapsed,
                                        );
                                        coordinator.record_intellisense_v2_runtime_exec_class_latency_with_origin(
                                            origin.as_str(),
                                            queue_priority.as_work_class(),
                                            exec_elapsed,
                                        );
                                    }
                                }
                                _ => {
                                    waiters.entry(file_id).or_default().push(PendingWaiter {
                                        min_version,
                                        reply,
                                        queue_wait_elapsed,
                                        started_waiting_at: Instant::now(),
                                        origin,
                                        priority: queue_priority,
                                    });
                                }
                            }
                        }
                        Command::GetFileRevisionState { file_id, reply } => {
                            let _ = reply.send(applied_file_revisions.get(&file_id).copied());
                        }
                        #[cfg(test)]
                        Command::TestSleep { duration, ack } => {
                            std::thread::sleep(duration);
                            let _ = ack.send(());
                        }
                        #[cfg(test)]
                        Command::TestNoop { ack } => {
                            let _ = ack.send(());
                        }
                        #[cfg(test)]
                        Command::Shutdown { ack } => {
                            for (_file_id, pending) in waiters.drain() {
                                for waiter in pending {
                                    let exec_elapsed = waiter.started_waiting_at.elapsed();
                                    if let Some(coordinator) = &observability {
                                        coordinator.record_intellisense_v2_runtime_exec_latency(
                                            "wait_for_file_version",
                                            exec_elapsed,
                                        );
                                        coordinator.record_intellisense_v2_runtime_exec_class_latency_with_origin(
                                            waiter.origin.as_str(),
                                            waiter.priority.as_work_class(),
                                            exec_elapsed,
                                        );
                                    }
                                    let wake_wait_elapsed = waiter.started_waiting_at.elapsed();
                                    let exec_started = Instant::now();
                                    let exec_elapsed = exec_started.elapsed();
                                    let _ = waiter.reply.send(WaitForFileVersionReply {
                                        ready: false,
                                        trace: WaitForFileVersionRuntimeTrace {
                                            queue_wait_elapsed: Some(waiter.queue_wait_elapsed),
                                            exec_elapsed: Some(exec_elapsed),
                                            wake_wait_elapsed: Some(wake_wait_elapsed),
                                            resolution: Some(
                                                WaitForFileVersionResolutionKind::Waiter,
                                            ),
                                        },
                                    });
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
                interactive_tx,
                background_tx,
                completion_deps_index_snapshot,
                #[cfg(test)]
                join_handle: std::sync::Mutex::new(Some(join_handle)),
            }),
        }
    }

    fn send_command_with_priority(
        &self,
        priority: RuntimeQueuePriority,
        command: Command,
    ) -> Result<(), std::sync::mpsc::SendError<Command>> {
        match priority {
            RuntimeQueuePriority::Interactive => self.inner.interactive_tx.send(command),
            RuntimeQueuePriority::Background => self.inner.background_tx.send(command),
        }
    }

    fn send_background_command(
        &self,
        command: Command,
    ) -> Result<(), std::sync::mpsc::SendError<Command>> {
        self.send_command_with_priority(RuntimeQueuePriority::Background, command)
    }

    fn apply_changes_with_enqueue_priority(
        &self,
        origin: ObservabilityOrigin,
        priority: RuntimeQueuePriority,
        changes: Vec<Change>,
    ) {
        if changes.is_empty() {
            return;
        }
        if self
            .send_command_with_priority(
                priority,
                Command::ApplyChanges {
                    origin,
                    enqueued_at: Instant::now(),
                    changes,
                },
            )
            .is_err()
        {
            warn!("analysis_v2_runtime: failed to send ApplyChanges (writer thread is gone)");
        }
    }

    pub fn apply_changes(&self, changes: Vec<Change>) {
        self.apply_changes_with_enqueue_priority(
            ObservabilityOrigin::Runtime,
            RuntimeQueuePriority::Background,
            changes,
        );
    }

    pub fn apply_changes_interactive(&self, origin: ObservabilityOrigin, changes: Vec<Change>) {
        self.apply_changes_with_enqueue_priority(
            origin,
            RuntimeQueuePriority::Interactive,
            changes,
        );
    }

    pub async fn apply_deps_bundle(
        &self,
        deps_id: DepsSnapshotId,
        deps: Arc<SemanticDeps>,
        index_snapshot: Arc<IndexSnapshot>,
    ) -> bool {
        let (reply, rx) = oneshot::channel::<bool>();
        if self
            .send_background_command(Command::ApplyDepsBundle {
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

    pub async fn snapshot(&self) -> AnalysisV2 {
        self.snapshot_with_priority(RuntimeQueuePriority::Background)
            .await
    }

    pub(super) async fn snapshot_with_priority(
        &self,
        priority: RuntimeQueuePriority,
    ) -> AnalysisV2 {
        let (reply, rx) = oneshot::channel::<AnalysisV2>();
        if self
            .send_command_with_priority(priority, Command::GetSnapshot { reply })
            .is_err()
        {
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

    pub async fn snapshot_with_deps(&self) -> (AnalysisV2, Arc<IndexSnapshot>, DepsSnapshotId) {
        let reply = self
            .snapshot_with_deps_with_priority(
                ObservabilityOrigin::Runtime,
                RuntimeQueuePriority::Background,
                None,
            )
            .await;
        (reply.analysis, reply.index_snapshot, reply.deps_id)
    }

    pub(super) async fn snapshot_with_deps_with_priority(
        &self,
        origin: ObservabilityOrigin,
        priority: RuntimeQueuePriority,
        progress: Option<PrepareStatefulProgress>,
    ) -> GetSnapshotWithDepsReply {
        let (reply, rx) = oneshot::channel::<GetSnapshotWithDepsReply>();
        if self
            .send_command_with_priority(
                priority,
                Command::GetSnapshotWithDeps {
                    origin,
                    enqueued_at: Instant::now(),
                    progress,
                    reply,
                },
            )
            .is_err()
        {
            warn!(
                "analysis_v2_runtime: failed to send GetSnapshotWithDeps (writer thread is gone)"
            );
            return GetSnapshotWithDepsReply {
                analysis: AnalysisHostV2::default().snapshot(),
                index_snapshot: Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash(""))),
                deps_id: DepsSnapshotId::from_hash(""),
                trace: SnapshotWithDepsRuntimeTrace::default(),
            };
        }

        match rx.await {
            Ok(reply) => reply,
            Err(_) => {
                warn!("analysis_v2_runtime: GetSnapshotWithDeps response cancelled");
                GetSnapshotWithDepsReply {
                    analysis: AnalysisHostV2::default().snapshot(),
                    index_snapshot: Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash(""))),
                    deps_id: DepsSnapshotId::from_hash(""),
                    trace: SnapshotWithDepsRuntimeTrace::default(),
                }
            }
        }
    }

    /// Returns a consistent analysis/index/deps snapshot for a semantic operation.
    /// Operation kind is part of the canonical facade contract and is reserved for
    /// shared policy/observability branching in subsequent migration steps.
    pub async fn wait_for_file_version(&self, file_id: FileId, min_version: i32) -> bool {
        self.wait_for_file_version_with_priority(
            ObservabilityOrigin::Runtime,
            RuntimeQueuePriority::Background,
            file_id,
            min_version,
        )
        .await
        .ready
    }

    pub(super) async fn wait_for_file_version_with_priority(
        &self,
        origin: ObservabilityOrigin,
        priority: RuntimeQueuePriority,
        file_id: FileId,
        min_version: i32,
    ) -> WaitForFileVersionReply {
        let (reply, rx) = oneshot::channel::<WaitForFileVersionReply>();
        if self
            .send_command_with_priority(
                priority,
                Command::WaitForFileVersion {
                    origin,
                    enqueued_at: Instant::now(),
                    file_id,
                    min_version,
                    reply,
                },
            )
            .is_err()
        {
            warn!("analysis_v2_runtime: failed to send WaitForFileVersion (writer thread is gone)");
            return WaitForFileVersionReply {
                ready: false,
                trace: WaitForFileVersionRuntimeTrace::default(),
            };
        }
        match rx.await {
            Ok(reply) => reply,
            Err(_) => {
                warn!("analysis_v2_runtime: WaitForFileVersion response cancelled");
                WaitForFileVersionReply {
                    ready: false,
                    trace: WaitForFileVersionRuntimeTrace::default(),
                }
            }
        }
    }

    pub async fn file_revision_state(&self, file_id: FileId) -> Option<FileRevisionState> {
        let (reply, rx) = oneshot::channel::<Option<FileRevisionState>>();
        if self
            .send_background_command(Command::GetFileRevisionState { file_id, reply })
            .is_err()
        {
            warn!(
                "analysis_v2_runtime: failed to send GetFileRevisionState (writer thread is gone)"
            );
            return None;
        }
        match rx.await {
            Ok(state) => state,
            Err(_) => {
                warn!("analysis_v2_runtime: GetFileRevisionState response cancelled");
                None
            }
        }
    }

    #[cfg(test)]
    pub(crate) async fn shutdown_for_test(&self) {
        let (ack, rx) = oneshot::channel::<()>();
        let _ = self.send_background_command(Command::Shutdown { ack });
        let _ = rx.await;

        let join_handle = self.inner.join_handle.lock().unwrap().take();
        if let Some(handle) = join_handle {
            let _ = handle.join();
        }
    }

    #[cfg(test)]
    pub(super) fn enqueue_test_sleep(
        &self,
        priority: RuntimeQueuePriority,
        duration: Duration,
    ) -> oneshot::Receiver<()> {
        let (ack, rx) = oneshot::channel::<()>();
        let _ = self.send_command_with_priority(priority, Command::TestSleep { duration, ack });
        rx
    }

    #[cfg(test)]
    pub(super) fn enqueue_test_noop(
        &self,
        priority: RuntimeQueuePriority,
    ) -> oneshot::Receiver<()> {
        let (ack, rx) = oneshot::channel::<()>();
        let _ = self.send_command_with_priority(priority, Command::TestNoop { ack });
        rx
    }
}
