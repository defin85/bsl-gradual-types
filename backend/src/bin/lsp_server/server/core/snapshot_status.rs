use std::sync::atomic::Ordering;

use bsl_shared::api::dtos::{
    SnapshotPhaseDto, SnapshotReadinessDto, SnapshotReadinessStateDto, SnapshotTaskStateDto,
    SnapshotTriggerDto, SNAPSHOT_READINESS_SCHEMA_VERSION,
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
    phase: Option<SnapshotPhaseDto>,
    trigger: Option<SnapshotTriggerDto>,
}

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
                    if emit_notification {
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
        let task_observation = {
            let tasks = self.background_parse_snapshot_apply_tasks_v2.lock().await;
            tasks.get(&file_id).map(|task| {
                let same_revision = requested_version.is_some_and(|requested_version| {
                    task.requested_version.load(Ordering::Relaxed) == requested_version
                        && current_text_hash.is_some_and(|text_hash| task.text_hash == text_hash)
                });
                SnapshotTaskObservationV2 {
                    state: if same_revision {
                        SnapshotTaskStateDto::InFlightSameRevision
                    } else {
                        SnapshotTaskStateDto::InFlightOtherRevision
                    },
                    phase: snapshot_phase_from_control(task.control.as_ref()),
                    trigger: Some(snapshot_trigger_from_source(task.source)),
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
                    && shadow_state.as_ref().map_or(true, |shadow| {
                        ready_state.text.as_ref() == shadow.text.as_ref()
                    })
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
        } else if let Some(failed_state) = failed_state.as_ref() {
            Some(failed_state.reason.as_ref().to_string())
        } else {
            None
        };

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
        }
    }
}
