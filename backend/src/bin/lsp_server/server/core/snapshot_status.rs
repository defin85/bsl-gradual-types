use std::sync::atomic::Ordering;

use bsl_shared::api::dtos::{
    SnapshotArtifactStateDto, SnapshotArtifactStatusDto, SnapshotArtifactsDto,
    SnapshotFailureStageDto, SnapshotLastFailureDto, SnapshotPhaseDto, SnapshotReadinessDto,
    SnapshotReadinessStateDto, SnapshotRecommendationDto, SnapshotStatusReasonDto,
    SnapshotTaskStateDto, SnapshotTriggerDto, SnapshotWorkerDto, SNAPSHOT_READINESS_SCHEMA_VERSION,
};

use super::super::{
    BackgroundParseSnapshotApplyTaskControlV2, BackgroundParseSnapshotApplyTaskPhaseV2,
    BackgroundParseSnapshotApplyTaskSourceV2, DidChangeParseSnapshotEvidenceKey,
};
use super::*;
use crate::server::unix_timestamp_ms;

#[derive(Debug, Clone, Copy)]
struct SnapshotTaskObservationV2 {
    state: SnapshotTaskStateDto,
    target_version: i32,
    phase: Option<SnapshotPhaseDto>,
    trigger: Option<SnapshotTriggerDto>,
    age_ms: Option<u64>,
}

const SNAPSHOT_DIAGNOSTIC_TEXT_LIMIT: usize = 160;

fn snapshot_trigger_from_source(
    source: BackgroundParseSnapshotApplyTaskSourceV2,
) -> SnapshotTriggerDto {
    match source {
        BackgroundParseSnapshotApplyTaskSourceV2::DidOpen => SnapshotTriggerDto::DidOpen,
        BackgroundParseSnapshotApplyTaskSourceV2::DidChange => SnapshotTriggerDto::DidChange,
        BackgroundParseSnapshotApplyTaskSourceV2::DidSave => SnapshotTriggerDto::DidSave,
    }
}

fn snapshot_phase_from_control(
    control: &BackgroundParseSnapshotApplyTaskControlV2,
) -> Option<SnapshotPhaseDto> {
    match BackgroundParseSnapshotApplyTaskPhaseV2::from_raw(control.phase.load(Ordering::SeqCst)) {
        Some(BackgroundParseSnapshotApplyTaskPhaseV2::Waiting) => Some(SnapshotPhaseDto::Waiting),
        Some(BackgroundParseSnapshotApplyTaskPhaseV2::Parsing) => Some(SnapshotPhaseDto::Parsing),
        Some(BackgroundParseSnapshotApplyTaskPhaseV2::Materializing) => {
            Some(SnapshotPhaseDto::Materializing)
        }
        None => None,
    }
}

fn snapshot_status_eq_ignoring_updated_at(
    left: &SnapshotReadinessDto,
    right: &SnapshotReadinessDto,
) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    left.updated_at_ms = 0;
    right.updated_at_ms = 0;
    left == right
}

fn snapshot_status_eq_for_live_notification(
    left: &SnapshotReadinessDto,
    right: &SnapshotReadinessDto,
) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    left.updated_at_ms = 0;
    right.updated_at_ms = 0;

    // Keep request/manual fetch truthful about the latest coarse worker phase, but do not
    // fan out live notifications for phase/trigger-only churn under the same semantic state.
    left.phase = None;
    right.phase = None;
    left.trigger = None;
    right.trigger = None;
    if let Some(worker) = left.worker.as_mut() {
        worker.phase = None;
        worker.trigger = None;
        worker.age_ms = None;
    }
    if let Some(worker) = right.worker.as_mut() {
        worker.phase = None;
        worker.trigger = None;
        worker.age_ms = None;
    }

    left == right
}

fn snapshot_duration_ms(started_at: std::time::Instant) -> u64 {
    let millis = started_at.elapsed().as_millis();
    u64::try_from(millis).unwrap_or(u64::MAX)
}

fn bounded_snapshot_detail(value: impl AsRef<str>) -> String {
    let mut sanitized = value
        .as_ref()
        .chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect::<String>();
    sanitized = sanitized.split_whitespace().collect::<Vec<_>>().join(" ");
    if sanitized.chars().count() <= SNAPSHOT_DIAGNOSTIC_TEXT_LIMIT {
        return sanitized;
    }

    sanitized
        .chars()
        .take(SNAPSHOT_DIAGNOSTIC_TEXT_LIMIT)
        .collect::<String>()
}

fn snapshot_artifact(
    state: SnapshotArtifactStateDto,
    version: Option<i32>,
    detail: Option<String>,
) -> SnapshotArtifactStatusDto {
    SnapshotArtifactStatusDto {
        state,
        version: version.map(i64::from),
        age_ms: None,
        detail: detail.map(bounded_snapshot_detail),
    }
}

impl BslLanguageServer {
    pub(crate) async fn snapshot_status_for_uri_v2(&self, uri: &Url) -> SnapshotReadinessDto {
        let file_id = self.get_or_create_file_id_v2(uri).await;
        self.upsert_snapshot_status_v2(file_id, Some(uri), false)
            .await
    }

    pub(crate) async fn refresh_snapshot_status_v2(&self, file_id: V2FileId) {
        let _ = self.upsert_snapshot_status_v2(file_id, None, true).await;
    }

    async fn upsert_snapshot_status_v2(
        &self,
        file_id: V2FileId,
        uri_hint: Option<&Url>,
        emit_notification: bool,
    ) -> SnapshotReadinessDto {
        let computed = self
            .compute_snapshot_status_v2(file_id, uri_hint.cloned())
            .await;
        let mut notification = None;
        let status = {
            let mut store = self.latest_snapshot_status_v2.write().await;
            let previous = store.get(&file_id).cloned();
            if let Some(previous) = previous {
                if snapshot_status_eq_ignoring_updated_at(&previous, &computed) {
                    previous
                } else {
                    let mut next = computed;
                    next.updated_at_ms =
                        unix_timestamp_ms().max(previous.updated_at_ms.saturating_add(1));
                    if emit_notification
                        && !snapshot_status_eq_for_live_notification(&previous, &next)
                    {
                        notification = Some(next.clone());
                    }
                    store.insert(file_id, next.clone());
                    next
                }
            } else {
                let mut next = computed;
                next.updated_at_ms = unix_timestamp_ms();
                if emit_notification {
                    notification = Some(next.clone());
                }
                store.insert(file_id, next.clone());
                next
            }
        };

        if let Some(notification) = notification {
            let _ = self
                .client
                .send_notification::<crate::types::SnapshotStatusNotification>(notification)
                .await;
        }

        status
    }

    async fn compute_snapshot_status_v2(
        &self,
        file_id: V2FileId,
        uri_hint: Option<Url>,
    ) -> SnapshotReadinessDto {
        let uri = if let Some(uri) = uri_hint {
            self.file_id_to_uri_v2
                .write()
                .await
                .insert(file_id, uri.clone());
            Some(uri.to_string())
        } else {
            self.file_id_to_uri_v2
                .read()
                .await
                .get(&file_id)
                .cloned()
                .map(|value| value.to_string())
        };

        let shadow_state = self
            .latest_document_shadow_state_v2
            .read()
            .await
            .get(&file_id)
            .cloned();
        let requested_version = self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied()
            .or_else(|| shadow_state.as_ref().map(|state| state.version));
        let ready_state = self
            .latest_ready_parse_snapshots_v2
            .read()
            .await
            .get(&file_id)
            .cloned();
        let current_text_hash = shadow_state
            .as_ref()
            .map(|state| *blake3::hash(state.text.as_bytes()).as_bytes());
        let apply_enqueued_at = self
            .latest_apply_enqueued_at_v2
            .read()
            .await
            .get(&file_id)
            .copied();
        let task_observation = {
            let tasks = self.background_parse_snapshot_apply_tasks_v2.lock().await;
            tasks.get(&file_id).map(|task| {
                let target = task
                    .target
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .clone();
                let same_revision = requested_version.is_some_and(|requested_version| {
                    target.requested_version == requested_version
                        && current_text_hash.is_some_and(|text_hash| target.text_hash == text_hash)
                });
                SnapshotTaskObservationV2 {
                    state: if same_revision {
                        SnapshotTaskStateDto::InFlightSameRevision
                    } else {
                        SnapshotTaskStateDto::InFlightOtherRevision
                    },
                    target_version: target.requested_version,
                    phase: snapshot_phase_from_control(task.control.as_ref()),
                    trigger: Some(snapshot_trigger_from_source(target.source)),
                    age_ms: apply_enqueued_at.map(snapshot_duration_ms),
                }
            })
        };

        let ready_version = ready_state
            .as_ref()
            .map(|state| state.parse_snapshot.file_version);
        let failed_state = self
            .latest_snapshot_failures_v2
            .read()
            .await
            .get(&file_id)
            .cloned()
            .filter(|failure| Some(failure.requested_version) == requested_version);
        let ready_matches_requested = match (requested_version, ready_state.as_ref()) {
            (Some(requested_version), Some(ready_state)) => {
                ready_state.parse_snapshot.file_version == requested_version
                    && shadow_state
                        .as_ref()
                        .is_none_or(|shadow| ready_state.text.as_ref() == shadow.text.as_ref())
            }
            _ => false,
        };
        let ready_is_stale = matches!(
            (requested_version, ready_version),
            (Some(requested_version), Some(ready_version)) if ready_version < requested_version
        );
        let fallback_reason = if matches!(
            task_observation.as_ref().map(|task| task.state),
            Some(
                SnapshotTaskStateDto::InFlightSameRevision
                    | SnapshotTaskStateDto::InFlightOtherRevision
            )
        ) || ready_is_stale
        {
            requested_version.and_then(|requested_version| {
                let store = self
                    .did_change_parse_snapshot_evidence_store
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                store
                    .entries
                    .get(&DidChangeParseSnapshotEvidenceKey {
                        file_id,
                        requested_version,
                    })
                    .and_then(|entry| entry.fallback_reason.clone())
            })
        } else {
            failed_state
                .as_ref()
                .map(|failed_state| failed_state.reason.as_ref().to_string())
        }
        .map(bounded_snapshot_detail);

        let (state, exact, task_state, phase, trigger) = if ready_matches_requested {
            (
                SnapshotReadinessStateDto::Ready,
                true,
                SnapshotTaskStateDto::ReadySameRevision,
                None,
                ready_state
                    .as_ref()
                    .map(|value| snapshot_trigger_from_source(value.source)),
            )
        } else if let Some(task) = task_observation {
            if task.state == SnapshotTaskStateDto::InFlightSameRevision {
                (
                    SnapshotReadinessStateDto::Building,
                    false,
                    task.state,
                    task.phase,
                    task.trigger,
                )
            } else if shadow_state
                .as_ref()
                .is_some_and(|shadow| Some(shadow.version) == requested_version)
            {
                (
                    SnapshotReadinessStateDto::ShadowOnly,
                    false,
                    task.state,
                    None,
                    task.trigger,
                )
            } else if ready_is_stale {
                (
                    SnapshotReadinessStateDto::Stale,
                    false,
                    SnapshotTaskStateDto::ReadyStaleRevision,
                    None,
                    ready_state
                        .as_ref()
                        .map(|value| snapshot_trigger_from_source(value.source)),
                )
            } else {
                (
                    SnapshotReadinessStateDto::Idle,
                    false,
                    task.state,
                    task.phase,
                    task.trigger,
                )
            }
        } else if shadow_state
            .as_ref()
            .is_some_and(|shadow| Some(shadow.version) == requested_version)
        {
            (
                SnapshotReadinessStateDto::ShadowOnly,
                false,
                if ready_is_stale {
                    SnapshotTaskStateDto::ReadyStaleRevision
                } else {
                    SnapshotTaskStateDto::Absent
                },
                None,
                ready_state
                    .as_ref()
                    .map(|value| snapshot_trigger_from_source(value.source)),
            )
        } else if ready_is_stale {
            (
                SnapshotReadinessStateDto::Stale,
                false,
                SnapshotTaskStateDto::ReadyStaleRevision,
                None,
                ready_state
                    .as_ref()
                    .map(|value| snapshot_trigger_from_source(value.source)),
            )
        } else if failed_state.is_some() {
            (
                SnapshotReadinessStateDto::Failed,
                false,
                SnapshotTaskStateDto::Absent,
                None,
                None,
            )
        } else {
            (
                SnapshotReadinessStateDto::Idle,
                false,
                SnapshotTaskStateDto::Absent,
                None,
                None,
            )
        };

        let exact_type_index_ready = self
            .analysis_v2
            .snapshot()
            .await
            .current_type_index_serve_only_ready(file_id)
            .ok();
        let completion_head_ready = self
            .analysis_v2
            .snapshot()
            .await
            .current_completion_head_ready(file_id)
            .ok();

        let matching_worker = matches!(
            task_observation.as_ref().map(|task| task.state),
            Some(SnapshotTaskStateDto::InFlightSameRevision)
        );
        let shadow_artifact = shadow_state.as_ref().map(|shadow| {
            let artifact_state = if Some(shadow.version) == requested_version {
                SnapshotArtifactStateDto::Ready
            } else {
                SnapshotArtifactStateDto::Stale
            };
            snapshot_artifact(artifact_state, Some(shadow.version), None)
        });
        let ready_parse_artifact = Some(snapshot_artifact(
            if ready_matches_requested {
                SnapshotArtifactStateDto::Ready
            } else if ready_state.is_some() {
                SnapshotArtifactStateDto::Stale
            } else if matching_worker {
                SnapshotArtifactStateDto::Building
            } else if failed_state.is_some() {
                SnapshotArtifactStateDto::Failed
            } else {
                SnapshotArtifactStateDto::Missing
            },
            ready_version,
            fallback_reason.clone(),
        ));
        let exact_type_index_artifact = Some(snapshot_artifact(
            if state == SnapshotReadinessStateDto::Ready && exact {
                SnapshotArtifactStateDto::Ready
            } else {
                match exact_type_index_ready {
                    Some(true) => SnapshotArtifactStateDto::Ready,
                    Some(false) if matching_worker => SnapshotArtifactStateDto::Building,
                    Some(false) if failed_state.is_some() => SnapshotArtifactStateDto::Failed,
                    Some(false) => SnapshotArtifactStateDto::Missing,
                    None => SnapshotArtifactStateDto::Unknown,
                }
            },
            requested_version,
            None,
        ));
        let completion_head_artifact = Some(snapshot_artifact(
            match completion_head_ready {
                Some(true) => SnapshotArtifactStateDto::Ready,
                Some(false) if matching_worker => SnapshotArtifactStateDto::Building,
                Some(false) => SnapshotArtifactStateDto::Missing,
                None => SnapshotArtifactStateDto::Unknown,
            },
            requested_version,
            None,
        ));

        let reason = Some(match state {
            SnapshotReadinessStateDto::Ready => SnapshotStatusReasonDto {
                code: "ready".to_string(),
                message: "Requested revision has canonical snapshot artifacts".to_string(),
            },
            SnapshotReadinessStateDto::Building => SnapshotStatusReasonDto {
                code: "building".to_string(),
                message: "A matching snapshot worker is building the requested revision"
                    .to_string(),
            },
            SnapshotReadinessStateDto::ShadowOnly => SnapshotStatusReasonDto {
                code: if ready_is_stale {
                    "shadow_only_ready_snapshot_stale"
                } else {
                    "shadow_only_exact_missing"
                }
                .to_string(),
                message:
                    "Only the editor shadow is current; exact snapshot artifacts are not ready"
                        .to_string(),
            },
            SnapshotReadinessStateDto::Stale => SnapshotStatusReasonDto {
                code: "ready_snapshot_stale".to_string(),
                message: "The latest ready snapshot is older than the requested revision"
                    .to_string(),
            },
            SnapshotReadinessStateDto::Failed => SnapshotStatusReasonDto {
                code: "snapshot_build_failed".to_string(),
                message: "The last matching snapshot build failed".to_string(),
            },
            SnapshotReadinessStateDto::Idle => SnapshotStatusReasonDto {
                code: "idle".to_string(),
                message: "No matching snapshot worker or ready artifact is active".to_string(),
            },
        });

        let worker = task_observation.map(|task| SnapshotWorkerDto {
            target_version: Some(i64::from(task.target_version)),
            phase: task.phase,
            trigger: task.trigger,
            age_ms: task.age_ms,
            cancellation_reason: None,
            superseded_by_version: requested_version
                .filter(|requested| *requested != task.target_version)
                .map(i64::from),
        });
        let last_failure = failed_state
            .as_ref()
            .map(|failed_state| SnapshotLastFailureDto {
                stage: SnapshotFailureStageDto::SnapshotBuild,
                reason: bounded_snapshot_detail(failed_state.reason.as_ref()),
                message: fallback_reason.clone(),
                requested_version: Some(i64::from(failed_state.requested_version)),
                occurred_at_ms: None,
            });
        let recommendation = match state {
            SnapshotReadinessStateDto::Ready => None,
            SnapshotReadinessStateDto::Building => Some(SnapshotRecommendationDto::Wait),
            SnapshotReadinessStateDto::ShadowOnly => {
                Some(SnapshotRecommendationDto::PrimeExactIndex)
            }
            SnapshotReadinessStateDto::Stale | SnapshotReadinessStateDto::Idle => {
                Some(SnapshotRecommendationDto::Refresh)
            }
            SnapshotReadinessStateDto::Failed => {
                Some(SnapshotRecommendationDto::ExportIncidentBundle)
            }
        };

        SnapshotReadinessDto {
            schema_version: SNAPSHOT_READINESS_SCHEMA_VERSION,
            uri,
            path: None,
            session_id: None,
            requested_version: requested_version.map(i64::from),
            ready_version: ready_version.map(i64::from),
            analysis_revision: None,
            state,
            exact,
            task_state,
            phase,
            trigger,
            updated_at_ms: 0,
            fallback_reason,
            reason,
            artifacts: Some(SnapshotArtifactsDto {
                shadow_state: shadow_artifact,
                ready_parse_snapshot: ready_parse_artifact,
                exact_type_index: exact_type_index_artifact,
                completion_head: completion_head_artifact,
            }),
            worker,
            last_failure,
            recommendation,
        }
    }
}
