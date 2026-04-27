import {
    DiagnosticsSaveTimelineFetchResult,
    DiagnosticsSaveTimelinePublishTrace,
} from '../lsp/customRequests';

export interface ObservabilityIncidentDiagnosticsSaveSummary {
    trace_id: string;
    uri: string;
    requested_version: number;
    save_cycle_sequence: number;
    diagnostics_generation: number;
    trigger: string;
    first_publish?: DiagnosticsSaveTimelinePublishTrace;
    followup_publish?: DiagnosticsSaveTimelinePublishTrace;
    save_fastlane_outcome?: string;
    idle_heavy_outcome?: string;
    followup_syntax_work_mode?: string;
    followup_semantic_path?: string;
    followup_semantic_parse_source?: string;
    followup_semantic_ir_source?: string;
    followup_semantic_attribution_note?: string;
    followup_ready_snapshot_zero_probe?: string;
    followup_ready_snapshot_wait_probe?: string;
    followup_ready_snapshot_task_state?: string;
    followup_ready_snapshot_timeout_phase?: string;
    followup_ready_snapshot_timeout_phase_elapsed_ms?: number;
    followup_ready_snapshot_timeout_leaf?: string;
    followup_ready_snapshot_timeout_leaf_elapsed_ms?: number;
    followup_ready_snapshot_parse_exec_ms?: number;
    followup_ready_snapshot_parse_exec_timeout_subphase?: string;
    followup_ready_snapshot_parse_exec_timeout_subphase_elapsed_ms?: number;
    followup_ready_snapshot_parse_exec_core_parse_build_ms?: number;
    followup_ready_snapshot_parse_exec_core_build_pre_parse_setup_ms?: number;
    followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint?: string;
    followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint_elapsed_ms?: number;
    followup_ready_snapshot_parse_exec_core_build_parser_base_recovery_ms?: number;
    followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms?: number;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_ms?: number;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint?: string;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint_elapsed_ms?: number;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms?: number;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms?: number;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_outcome?: string;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_lowering_units?: number;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_lowering_units?: number;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_window_count?: number;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_window_count?: number;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_source?: string | null;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_seed_source?: string | null;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_seed_candidate_count?: number;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_seed_eviction_reason?: string | null;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_failure_reason?: string | null;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_take_if_unique_hit?: boolean;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_cache_hit?: boolean;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms?: number;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms?: number;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint?: string;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint_ms?: number;
    followup_ready_snapshot_parse_exec_core_build_tree_cache_install_ms?: number;
    followup_ready_snapshot_parse_exec_optional_cache_enrichment_ms?: number;
    followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint?: string;
    followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint_ms?: number;
    followup_ready_snapshot_parse_exec_dominant_subphase?: string;
    followup_ready_snapshot_parse_exec_dominant_subphase_ms?: number;
    followup_ready_snapshot_post_parse_pre_materialization_ms?: number;
    followup_ready_snapshot_ready_install_ms?: number;
    followup_ready_snapshot_document_symbol_side_work_ms?: number;
    followup_ready_snapshot_dominant_phase?: string;
    followup_ready_snapshot_dominant_phase_ms?: number;
    followup_ready_snapshot_relief_valve_outcome?: string;
    followup_ready_snapshot_relief_valve_budget_ms?: number;
    followup_ready_snapshot_relief_valve_elapsed_ms?: number;
    followup_shadow_state_available?: boolean;
    followup_ready_snapshot_attribution_note?: string;
    followup_ready_snapshot_phase_attribution_note?: string;
    followup_ready_snapshot_timeout_leaf_note?: string;
    followup_ready_snapshot_parse_exec_subphase_note?: string;
    followup_ready_snapshot_core_build_checkpoint_note?: string;
    followup_ready_snapshot_exact_ready_snapshot_assembly_checkpoint_note?: string;
    followup_ready_snapshot_relief_valve_note?: string;
    followup_wait_reason?: string;
    followup_blocker_reason?: string;
    followup_blocker_note?: string;
    followup_runtime_queue_wait_ms?: number;
    followup_apply_lag_ms?: number;
    followup_wait_for_file_version_ms?: number;
    followup_snapshot_with_deps_ms?: number;
    followup_readiness_blocker_bucket?: string;
    followup_unclassified_readiness_residual_ms?: number;
    terminal_outcome?: string;
}

export interface ObservabilityIncidentDiagnosticsSaveSection {
    requestCount: number;
    requests: ObservabilityIncidentDiagnosticsSaveSummary[];
    gaps: string[];
}

export function buildObservabilityIncidentDiagnosticsSaveSection(
    diagnosticsSaveTimeline: DiagnosticsSaveTimelineFetchResult
): ObservabilityIncidentDiagnosticsSaveSection {
    if (diagnosticsSaveTimeline.kind !== 'ok') {
        return {
            requestCount: 0,
            requests: [],
            gaps: [],
        };
    }

    const semanticAttributionNote =
        diagnosticsSaveTimeline.response.version < 8
            ? `unavailable_by_design(version=${diagnosticsSaveTimeline.response.version})`
            : undefined;
    const readySnapshotAttributionNote =
        diagnosticsSaveTimeline.response.version < 9
            ? `unavailable_by_design(version=${diagnosticsSaveTimeline.response.version})`
            : undefined;
    const readySnapshotPhaseAttributionNote =
        diagnosticsSaveTimeline.response.version < 10
            ? `unavailable_by_design(version=${diagnosticsSaveTimeline.response.version})`
            : undefined;
    const readySnapshotReliefValveNote =
        diagnosticsSaveTimeline.response.version < 11
            ? `unavailable_by_design(version=${diagnosticsSaveTimeline.response.version})`
            : undefined;
    const timeoutLeafNote =
        diagnosticsSaveTimeline.response.version < 21
            ? `unavailable_by_design(version=${diagnosticsSaveTimeline.response.version})`
            : undefined;
    const blockerNote =
        diagnosticsSaveTimeline.response.version < 12
            ? `unavailable_by_design(version=${diagnosticsSaveTimeline.response.version})`
            : undefined;
    const parseExecSubphaseNote =
        diagnosticsSaveTimeline.response.version < 13
            ? `unavailable_by_design(version=${diagnosticsSaveTimeline.response.version})`
            : undefined;
    const coreBuildCheckpointNote =
        diagnosticsSaveTimeline.response.version < 14
            ? `unavailable_by_design(version=${diagnosticsSaveTimeline.response.version})`
            : undefined;
    const exactReadySnapshotAssemblyCheckpointNote =
        diagnosticsSaveTimeline.response.version < 15
            ? `unavailable_by_design(version=${diagnosticsSaveTimeline.response.version})`
            : undefined;
    const gaps: string[] = [];
    if (diagnosticsSaveTimeline.response.version < 8) {
        gaps.push(
            `Diagnostics save timeline v${diagnosticsSaveTimeline.response.version} does not expose semantic path/source attribution by design.`
        );
    }
    if (diagnosticsSaveTimeline.response.version < 9) {
        gaps.push(
            `Diagnostics save timeline v${diagnosticsSaveTimeline.response.version} does not expose ready-snapshot miss attribution by design.`
        );
    }
    if (diagnosticsSaveTimeline.response.version < 10) {
        gaps.push(
            `Diagnostics save timeline v${diagnosticsSaveTimeline.response.version} does not expose ready-snapshot phase attribution by design.`
        );
    }
    if (diagnosticsSaveTimeline.response.version < 11) {
        gaps.push(
            `Diagnostics save timeline v${diagnosticsSaveTimeline.response.version} does not expose ready-snapshot relief-valve attribution by design.`
        );
    }
    if (diagnosticsSaveTimeline.response.version < 21) {
        gaps.push(
            `Diagnostics save timeline v${diagnosticsSaveTimeline.response.version} does not expose diagnostics-save timeout-leaf fidelity in derived request summaries by design.`
        );
    }
    if (diagnosticsSaveTimeline.response.version < 12) {
        gaps.push(
            `Diagnostics save timeline v${diagnosticsSaveTimeline.response.version} does not expose follow-up blocker attribution by design.`
        );
    }
    if (diagnosticsSaveTimeline.response.version < 13) {
        gaps.push(
            `Diagnostics save timeline v${diagnosticsSaveTimeline.response.version} does not expose parse-exec subphase attribution by design.`
        );
    }
    if (diagnosticsSaveTimeline.response.version < 14) {
        gaps.push(
            `Diagnostics save timeline v${diagnosticsSaveTimeline.response.version} does not expose core-build checkpoint attribution by design.`
        );
    }
    if (diagnosticsSaveTimeline.response.version < 15) {
        gaps.push(
            `Diagnostics save timeline v${diagnosticsSaveTimeline.response.version} does not expose exact ready-snapshot assembly checkpoint attribution by design.`
        );
    }
    for (const trace of diagnosticsSaveTimeline.response.traces) {
        if (
            trace.followup_readiness_blocker_bucket === 'unclassified_readiness_residual'
            && isPositiveTimingValue(trace.followup_unclassified_readiness_residual_ms)
        ) {
            gaps.push(
                `Diagnostics save trace ${trace.trace_id} has unclassified readiness residual ${trace.followup_unclassified_readiness_residual_ms}ms; budget widening alone is not accepted without explicit blocker attribution.`
            );
        }
        if (
            trace.followup_readiness_blocker_bucket === 'snapshot_with_deps'
            && hasProgramLoweringTailEvidence(trace)
        ) {
            gaps.push(
                `Diagnostics save trace ${trace.trace_id} is classified as snapshot_with_deps even though the ready-snapshot timeout leaf/checkpoint is program_lowering; exact program-lowering tail must be classified separately.`
            );
        }
        if (hasProgramLoweringTailEvidence(trace) && !hasProgramLoweringReuseEvidence(trace)) {
            gaps.push(
                `Diagnostics save trace ${trace.trace_id} has a program_lowering tail without complete reuse evidence; expected reuse_outcome, reused/rebuilt lowering units, seed candidate count, reuse_plan_build_source or failure_reason, take_if_unique_hit, and borrowed_cache_hit.`
            );
        }
    }

    return {
        requestCount: diagnosticsSaveTimeline.response.traces.length,
        requests: diagnosticsSaveTimeline.response.traces.map((trace) => ({
            trace_id: trace.trace_id,
            uri: trace.uri,
            requested_version: trace.requested_version,
            save_cycle_sequence: trace.save_cycle_sequence,
            diagnostics_generation: trace.diagnostics_generation,
            trigger: trace.trigger,
            first_publish: trace.first_publish,
            followup_publish: trace.followup_publish,
            save_fastlane_outcome: trace.save_fastlane_outcome,
            idle_heavy_outcome: trace.idle_heavy_outcome,
            followup_syntax_work_mode: trace.followup_syntax_work_mode,
            followup_semantic_path: trace.followup_semantic_path,
            followup_semantic_parse_source: trace.followup_semantic_parse_source,
            followup_semantic_ir_source: trace.followup_semantic_ir_source,
            followup_semantic_attribution_note: semanticAttributionNote,
            followup_ready_snapshot_zero_probe: trace.followup_ready_snapshot_zero_probe,
            followup_ready_snapshot_wait_probe: trace.followup_ready_snapshot_wait_probe,
            followup_ready_snapshot_task_state: trace.followup_ready_snapshot_task_state,
            followup_ready_snapshot_timeout_phase: trace.followup_ready_snapshot_timeout_phase,
            followup_ready_snapshot_timeout_phase_elapsed_ms:
                trace.followup_ready_snapshot_timeout_phase_elapsed_ms,
            followup_ready_snapshot_timeout_leaf:
                diagnosticsSaveTimeline.response.version >= 21
                    ? trace.followup_ready_snapshot_timeout_leaf
                    : undefined,
            followup_ready_snapshot_timeout_leaf_elapsed_ms:
                diagnosticsSaveTimeline.response.version >= 21
                    ? trace.followup_ready_snapshot_timeout_leaf_elapsed_ms
                    : undefined,
            followup_ready_snapshot_parse_exec_ms:
                trace.followup_ready_snapshot_parse_exec_ms,
            followup_ready_snapshot_parse_exec_timeout_subphase:
                trace.followup_ready_snapshot_parse_exec_timeout_subphase,
            followup_ready_snapshot_parse_exec_timeout_subphase_elapsed_ms:
                trace.followup_ready_snapshot_parse_exec_timeout_subphase_elapsed_ms,
            followup_ready_snapshot_parse_exec_core_parse_build_ms:
                trace.followup_ready_snapshot_parse_exec_core_parse_build_ms,
            followup_ready_snapshot_parse_exec_core_build_pre_parse_setup_ms:
                trace.followup_ready_snapshot_parse_exec_core_build_pre_parse_setup_ms,
            followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint:
                trace.followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint,
            followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint_elapsed_ms:
                trace.followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint_elapsed_ms,
            followup_ready_snapshot_parse_exec_core_build_parser_base_recovery_ms:
                trace.followup_ready_snapshot_parse_exec_core_build_parser_base_recovery_ms,
            followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms:
                trace.followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_ms:
                trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_ms,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint:
                trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint_elapsed_ms:
                trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint_elapsed_ms,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms:
                trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms:
                trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_outcome:
                trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_outcome,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_lowering_units:
                trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_lowering_units,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_lowering_units:
                trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_lowering_units,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_window_count:
                trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_window_count,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_window_count:
                trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_window_count,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_source:
                trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_source,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_seed_source:
                trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_seed_source,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_seed_candidate_count:
                trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_seed_candidate_count,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_seed_eviction_reason:
                trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_seed_eviction_reason,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_failure_reason:
                trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_failure_reason,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_take_if_unique_hit:
                trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_take_if_unique_hit,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_cache_hit:
                trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_cache_hit,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms:
                trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms:
                trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint:
                trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint,
            followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint_ms:
                trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint_ms,
            followup_ready_snapshot_parse_exec_core_build_tree_cache_install_ms:
                trace.followup_ready_snapshot_parse_exec_core_build_tree_cache_install_ms,
            followup_ready_snapshot_parse_exec_optional_cache_enrichment_ms:
                trace.followup_ready_snapshot_parse_exec_optional_cache_enrichment_ms,
            followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint:
                trace.followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint,
            followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint_ms:
                trace.followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint_ms,
            followup_ready_snapshot_parse_exec_dominant_subphase:
                trace.followup_ready_snapshot_parse_exec_dominant_subphase,
            followup_ready_snapshot_parse_exec_dominant_subphase_ms:
                trace.followup_ready_snapshot_parse_exec_dominant_subphase_ms,
            followup_ready_snapshot_post_parse_pre_materialization_ms:
                trace.followup_ready_snapshot_post_parse_pre_materialization_ms,
            followup_ready_snapshot_ready_install_ms:
                trace.followup_ready_snapshot_ready_install_ms,
            followup_ready_snapshot_document_symbol_side_work_ms:
                trace.followup_ready_snapshot_document_symbol_side_work_ms,
            followup_ready_snapshot_dominant_phase:
                trace.followup_ready_snapshot_dominant_phase,
            followup_ready_snapshot_dominant_phase_ms:
                trace.followup_ready_snapshot_dominant_phase_ms,
            followup_ready_snapshot_relief_valve_outcome:
                trace.followup_ready_snapshot_relief_valve_outcome,
            followup_ready_snapshot_relief_valve_budget_ms:
                trace.followup_ready_snapshot_relief_valve_budget_ms,
            followup_ready_snapshot_relief_valve_elapsed_ms:
                trace.followup_ready_snapshot_relief_valve_elapsed_ms,
            followup_shadow_state_available: trace.followup_shadow_state_available,
            followup_ready_snapshot_attribution_note: readySnapshotAttributionNote,
            followup_ready_snapshot_phase_attribution_note:
                readySnapshotPhaseAttributionNote,
            followup_ready_snapshot_parse_exec_subphase_note:
                parseExecSubphaseNote,
            followup_ready_snapshot_core_build_checkpoint_note:
                coreBuildCheckpointNote,
            followup_ready_snapshot_exact_ready_snapshot_assembly_checkpoint_note:
                exactReadySnapshotAssemblyCheckpointNote,
            followup_ready_snapshot_relief_valve_note:
                readySnapshotReliefValveNote,
            followup_ready_snapshot_timeout_leaf_note: timeoutLeafNote,
            followup_wait_reason: trace.followup_wait_reason,
            followup_blocker_reason: trace.followup_blocker_reason,
            followup_blocker_note: blockerNote,
            followup_runtime_queue_wait_ms: trace.followup_runtime_queue_wait_ms,
            followup_apply_lag_ms: trace.followup_apply_lag_ms,
            followup_wait_for_file_version_ms: trace.followup_wait_for_file_version_ms,
            followup_snapshot_with_deps_ms: trace.followup_snapshot_with_deps_ms,
            followup_readiness_blocker_bucket: trace.followup_readiness_blocker_bucket,
            followup_unclassified_readiness_residual_ms:
                trace.followup_unclassified_readiness_residual_ms,
            terminal_outcome: trace.terminal_outcome,
        })),
        gaps,
    };
}

function isPositiveTimingValue(value: number | undefined): value is number {
    return typeof value === 'number' && value > 0;
}

function hasValue<T>(value: T | null | undefined): value is T {
    return value !== undefined && value !== null;
}

function hasProgramLoweringTailEvidence(
    trace: ObservabilityIncidentDiagnosticsSaveSummary
): boolean {
    return (
        trace.followup_ready_snapshot_timeout_leaf === 'program_lowering'
        || trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint === 'program_lowering'
        || (
            trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint === 'program_lowering'
            && isPositiveTimingValue(
                trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms
            )
        )
    );
}

function hasProgramLoweringReuseEvidence(
    trace: ObservabilityIncidentDiagnosticsSaveSummary
): boolean {
    const reusePlanBuildSource =
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_source;
    const reusePlanFailureReason =
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_failure_reason;
    return (
        hasValue(trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_outcome)
        && hasValue(trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_lowering_units)
        && hasValue(trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_lowering_units)
        && hasValue(trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_seed_candidate_count)
        && (hasValue(reusePlanBuildSource) || hasValue(reusePlanFailureReason))
        && hasValue(trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_take_if_unique_hit)
        && hasValue(trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_cache_hit)
    );
}

function formatPublish(label: string, publish: DiagnosticsSaveTimelinePublishTrace | undefined): string {
    if (!publish) {
        return `${label}=none`;
    }

    const parts = [
        `${label}=${publish.profile}:${publish.publish_kind}:${publish.outcome}@${publish.elapsed_ms}ms`,
    ];
    if (publish.syntax_work_mode) {
        parts.push(`syntax_work_mode=${publish.syntax_work_mode}`);
    }
    if (isPositiveTimingValue(publish.runtime_queue_wait_ms)) {
        parts.push(`runtime_queue_wait_ms=${publish.runtime_queue_wait_ms}`);
    }
    if (isPositiveTimingValue(publish.apply_lag_ms)) {
        parts.push(`apply_lag_ms=${publish.apply_lag_ms}`);
    }
    if (isPositiveTimingValue(publish.blocking_queue_wait_ms)) {
        parts.push(`blocking_queue_wait_ms=${publish.blocking_queue_wait_ms}`);
    }
    if (isPositiveTimingValue(publish.wait_for_file_version_ms)) {
        parts.push(`wait_for_file_version_ms=${publish.wait_for_file_version_ms}`);
    }
    if (isPositiveTimingValue(publish.snapshot_with_deps_ms)) {
        parts.push(`snapshot_with_deps_ms=${publish.snapshot_with_deps_ms}`);
    }
    if (isPositiveTimingValue(publish.syntax_diagnostics_query_ms)) {
        parts.push(`syntax_diagnostics_query_ms=${publish.syntax_diagnostics_query_ms}`);
    }
    if (isPositiveTimingValue(publish.semantic_diagnostics_query_ms)) {
        parts.push(`semantic_diagnostics_query_ms=${publish.semantic_diagnostics_query_ms}`);
    }
    if (isPositiveTimingValue(publish.publish_wait_ms)) {
        parts.push(`publish_wait_ms=${publish.publish_wait_ms}`);
    }
    return parts.join(' | ');
}

function renderProfileOutcome(
    outcome: string | undefined,
    terminalOutcome: string | undefined
): string {
    if (outcome) {
        return outcome;
    }
    if (!terminalOutcome) {
        return 'pending';
    }
    return 'unknown';
}

function renderTerminalOutcome(terminalOutcome: string | undefined): string {
    return terminalOutcome ?? 'in_flight';
}

function formatPublishWithLifecycle(
    label: string,
    publish: DiagnosticsSaveTimelinePublishTrace | undefined,
    terminalOutcome: string | undefined
): string {
    if (publish) {
        return formatPublish(label, publish);
    }
    return `${label}=${terminalOutcome ? 'none' : 'pending'}`;
}

function formatFollowupWait(
    syntaxWorkMode: string | undefined,
    reason: string | undefined,
    blockerReason: string | undefined,
    blockerNote: string | undefined,
    runtimeQueueWaitMs: number | undefined,
    applyLagMs: number | undefined,
    waitForFileVersionMs: number | undefined,
    snapshotWithDepsMs: number | undefined,
    readinessBlockerBucket: string | undefined,
    unclassifiedReadinessResidualMs: number | undefined
): string | undefined {
    if (
        !reason
        && !blockerReason
        && !blockerNote
        && !syntaxWorkMode
        && !isPositiveTimingValue(runtimeQueueWaitMs)
        && !isPositiveTimingValue(applyLagMs)
        && !isPositiveTimingValue(waitForFileVersionMs)
        && !isPositiveTimingValue(snapshotWithDepsMs)
        && !readinessBlockerBucket
        && !isPositiveTimingValue(unclassifiedReadinessResidualMs)
    ) {
        return undefined;
    }

    const parts: string[] = [];
    if (syntaxWorkMode) {
        parts.push(`followup_syntax_work_mode=${syntaxWorkMode}`);
    }
    if (reason) {
        parts.push(`followup_wait=${reason}`);
    }
    if (blockerNote) {
        parts.push(`followup_blocker=${blockerNote}`);
    }
    if (blockerReason) {
        parts.push(`followup_blocker=${blockerReason}`);
    }
    if (isPositiveTimingValue(runtimeQueueWaitMs)) {
        parts.push(`followup_runtime_queue_wait_ms=${runtimeQueueWaitMs}`);
    }
    if (isPositiveTimingValue(applyLagMs)) {
        parts.push(`followup_apply_lag_ms=${applyLagMs}`);
    }
    if (isPositiveTimingValue(waitForFileVersionMs)) {
        parts.push(`followup_wait_for_file_version_ms=${waitForFileVersionMs}`);
    }
    if (isPositiveTimingValue(snapshotWithDepsMs)) {
        parts.push(`followup_snapshot_with_deps_ms=${snapshotWithDepsMs}`);
    }
    if (readinessBlockerBucket) {
        parts.push(`followup_readiness_blocker_bucket=${readinessBlockerBucket}`);
    }
    if (isPositiveTimingValue(unclassifiedReadinessResidualMs)) {
        parts.push(
            `followup_unclassified_readiness_residual_ms=${unclassifiedReadinessResidualMs}`
        );
    }
    return parts.join(' | ');
}

function formatFollowupSemanticAttribution(
    semanticAttributionNote: string | undefined,
    semanticPath: string | undefined,
    semanticParseSource: string | undefined,
    semanticIrSource: string | undefined
): string | undefined {
    if (semanticAttributionNote) {
        return `followup_semantic_attribution=${semanticAttributionNote}`;
    }
    if (!semanticPath && !semanticParseSource && !semanticIrSource) {
        return undefined;
    }

    const parts: string[] = [];
    if (semanticPath) {
        parts.push(`followup_semantic_path=${semanticPath}`);
    }
    if (semanticParseSource) {
        parts.push(`followup_semantic_parse_source=${semanticParseSource}`);
    }
    if (semanticIrSource) {
        parts.push(`followup_semantic_ir_source=${semanticIrSource}`);
    }
    return parts.join(' | ');
}

function formatFollowupReadySnapshotAttribution(
    attributionNote: string | undefined,
    zeroProbe: string | undefined,
    waitProbe: string | undefined,
    taskState: string | undefined,
    shadowStateAvailable: boolean | undefined
): string | undefined {
    if (attributionNote) {
        return `followup_ready_snapshot_miss_attribution=${attributionNote}`;
    }
    if (
        !zeroProbe
        && !waitProbe
        && !taskState
        && typeof shadowStateAvailable !== 'boolean'
    ) {
        return undefined;
    }

    const parts: string[] = [];
    if (zeroProbe) {
        parts.push(`followup_ready_snapshot_zero_probe=${zeroProbe}`);
    }
    if (waitProbe) {
        parts.push(`followup_ready_snapshot_wait_probe=${waitProbe}`);
    }
    if (taskState) {
        parts.push(`followup_ready_snapshot_task_state=${taskState}`);
    }
    if (typeof shadowStateAvailable === 'boolean') {
        parts.push(`followup_shadow_state_available=${shadowStateAvailable}`);
    }
    return parts.join(' | ');
}

function formatFollowupReadySnapshotPhases(
    attributionNote: string | undefined,
    timeoutLeafNote: string | undefined,
    parseExecSubphaseNote: string | undefined,
    coreBuildCheckpointNote: string | undefined,
    exactReadySnapshotAssemblyCheckpointNote: string | undefined,
    timeoutPhase: string | undefined,
    timeoutPhaseElapsedMs: number | undefined,
    timeoutLeaf: string | undefined,
    timeoutLeafElapsedMs: number | undefined,
    parseExecMs: number | undefined,
    parseExecTimeoutSubphase: string | undefined,
    parseExecTimeoutSubphaseElapsedMs: number | undefined,
    parseExecCoreParseBuildMs: number | undefined,
    parseExecCoreBuildPreParseSetupMs: number | undefined,
    parseExecCoreBuildTimeoutCheckpoint: string | undefined,
    parseExecCoreBuildTimeoutCheckpointElapsedMs: number | undefined,
    parseExecCoreBuildParserBaseRecoveryMs: number | undefined,
    parseExecCoreBuildParserTreeBuildMs: number | undefined,
    parseExecCoreBuildExactReadySnapshotAssemblyMs: number | undefined,
    parseExecCoreBuildExactReadySnapshotAssemblyTimeoutCheckpoint: string | undefined,
    parseExecCoreBuildExactReadySnapshotAssemblyTimeoutCheckpointElapsedMs:
        number | undefined,
    parseExecCoreBuildExactReadySnapshotAssemblyProgramConversionMs:
        number | undefined,
    parseExecCoreBuildExactReadySnapshotAssemblyProgramLoweringMs:
        number | undefined,
    parseExecCoreBuildExactReadySnapshotAssemblyPublishableArtifactPackagingMs:
        number | undefined,
    parseExecCoreBuildExactReadySnapshotAssemblySyntaxErrorCollectionMs:
        number | undefined,
    parseExecCoreBuildExactReadySnapshotAssemblyDominantCheckpoint:
        string | undefined,
    parseExecCoreBuildExactReadySnapshotAssemblyDominantCheckpointMs:
        number | undefined,
    parseExecCoreBuildTreeCacheInstallMs: number | undefined,
    parseExecOptionalCacheEnrichmentMs: number | undefined,
    parseExecCoreBuildDominantCheckpoint: string | undefined,
    parseExecCoreBuildDominantCheckpointMs: number | undefined,
    parseExecDominantSubphase: string | undefined,
    parseExecDominantSubphaseMs: number | undefined,
    postParsePreMaterializationMs: number | undefined,
    readyInstallMs: number | undefined,
    documentSymbolSideWorkMs: number | undefined,
    dominantPhase: string | undefined,
    dominantPhaseMs: number | undefined
): string | undefined {
    if (attributionNote) {
        return `followup_ready_snapshot_phase_attribution=${attributionNote}`;
    }
    if (
        !timeoutPhase
        && !isPositiveTimingValue(timeoutPhaseElapsedMs)
        && !timeoutLeaf
        && !isPositiveTimingValue(timeoutLeafElapsedMs)
        && !isPositiveTimingValue(parseExecMs)
        && !parseExecTimeoutSubphase
        && !isPositiveTimingValue(parseExecTimeoutSubphaseElapsedMs)
        && !isPositiveTimingValue(parseExecCoreParseBuildMs)
        && !isPositiveTimingValue(parseExecCoreBuildPreParseSetupMs)
        && !parseExecCoreBuildTimeoutCheckpoint
        && !isPositiveTimingValue(parseExecCoreBuildTimeoutCheckpointElapsedMs)
        && !isPositiveTimingValue(parseExecCoreBuildParserBaseRecoveryMs)
        && !isPositiveTimingValue(parseExecCoreBuildParserTreeBuildMs)
        && !isPositiveTimingValue(parseExecCoreBuildExactReadySnapshotAssemblyMs)
        && !parseExecCoreBuildExactReadySnapshotAssemblyTimeoutCheckpoint
        && !isPositiveTimingValue(
            parseExecCoreBuildExactReadySnapshotAssemblyTimeoutCheckpointElapsedMs
        )
        && !isPositiveTimingValue(
            parseExecCoreBuildExactReadySnapshotAssemblyProgramConversionMs
        )
        && !isPositiveTimingValue(
            parseExecCoreBuildExactReadySnapshotAssemblyProgramLoweringMs
        )
        && !isPositiveTimingValue(
            parseExecCoreBuildExactReadySnapshotAssemblyPublishableArtifactPackagingMs
        )
        && !isPositiveTimingValue(
            parseExecCoreBuildExactReadySnapshotAssemblySyntaxErrorCollectionMs
        )
        && !parseExecCoreBuildExactReadySnapshotAssemblyDominantCheckpoint
        && !isPositiveTimingValue(
            parseExecCoreBuildExactReadySnapshotAssemblyDominantCheckpointMs
        )
        && !isPositiveTimingValue(parseExecCoreBuildTreeCacheInstallMs)
        && !isPositiveTimingValue(parseExecOptionalCacheEnrichmentMs)
        && !parseExecCoreBuildDominantCheckpoint
        && !isPositiveTimingValue(parseExecCoreBuildDominantCheckpointMs)
        && !parseExecDominantSubphase
        && !isPositiveTimingValue(parseExecDominantSubphaseMs)
        && !isPositiveTimingValue(postParsePreMaterializationMs)
        && !isPositiveTimingValue(readyInstallMs)
        && !isPositiveTimingValue(documentSymbolSideWorkMs)
        && !dominantPhase
        && !isPositiveTimingValue(dominantPhaseMs)
        && !timeoutLeafNote
        && !parseExecSubphaseNote
        && !coreBuildCheckpointNote
        && !exactReadySnapshotAssemblyCheckpointNote
    ) {
        return undefined;
    }

    const parts: string[] = [];
    if (timeoutPhase) {
        parts.push(`followup_ready_snapshot_timeout_phase=${timeoutPhase}`);
    }
    if (isPositiveTimingValue(timeoutPhaseElapsedMs)) {
        parts.push(
            `followup_ready_snapshot_timeout_phase_elapsed_ms=${timeoutPhaseElapsedMs}`
        );
    }
    if (timeoutLeaf) {
        parts.push(`followup_ready_snapshot_timeout_leaf=${timeoutLeaf}`);
    } else if (timeoutLeafNote) {
        parts.push(`followup_ready_snapshot_timeout_leaf=${timeoutLeafNote}`);
    }
    if (isPositiveTimingValue(timeoutLeafElapsedMs)) {
        parts.push(
            `followup_ready_snapshot_timeout_leaf_elapsed_ms=${timeoutLeafElapsedMs}`
        );
    }
    if (isPositiveTimingValue(parseExecMs)) {
        parts.push(`followup_ready_snapshot_parse_exec_ms=${parseExecMs}`);
    }
    if (parseExecTimeoutSubphase) {
        parts.push(
            `followup_ready_snapshot_parse_exec_timeout_subphase=${parseExecTimeoutSubphase}`
        );
    }
    if (isPositiveTimingValue(parseExecTimeoutSubphaseElapsedMs)) {
        parts.push(
            `followup_ready_snapshot_parse_exec_timeout_subphase_elapsed_ms=${parseExecTimeoutSubphaseElapsedMs}`
        );
    }
    if (isPositiveTimingValue(parseExecCoreParseBuildMs)) {
        parts.push(
            `followup_ready_snapshot_parse_exec_core_parse_build_ms=${parseExecCoreParseBuildMs}`
        );
    }
    if (isPositiveTimingValue(parseExecCoreBuildPreParseSetupMs)) {
        parts.push(
            `followup_ready_snapshot_parse_exec_core_build_pre_parse_setup_ms=${parseExecCoreBuildPreParseSetupMs}`
        );
    }
    if (parseExecCoreBuildTimeoutCheckpoint) {
        parts.push(
            `followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint=${parseExecCoreBuildTimeoutCheckpoint}`
        );
    }
    if (isPositiveTimingValue(parseExecCoreBuildTimeoutCheckpointElapsedMs)) {
        parts.push(
            `followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint_elapsed_ms=${parseExecCoreBuildTimeoutCheckpointElapsedMs}`
        );
    }
    if (isPositiveTimingValue(parseExecCoreBuildParserBaseRecoveryMs)) {
        parts.push(
            `followup_ready_snapshot_parse_exec_core_build_parser_base_recovery_ms=${parseExecCoreBuildParserBaseRecoveryMs}`
        );
    }
    if (isPositiveTimingValue(parseExecCoreBuildParserTreeBuildMs)) {
        parts.push(
            `followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms=${parseExecCoreBuildParserTreeBuildMs}`
        );
    }
    if (isPositiveTimingValue(parseExecCoreBuildExactReadySnapshotAssemblyMs)) {
        parts.push(
            `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_ms=${parseExecCoreBuildExactReadySnapshotAssemblyMs}`
        );
    }
    if (parseExecCoreBuildExactReadySnapshotAssemblyTimeoutCheckpoint) {
        parts.push(
            `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint=${parseExecCoreBuildExactReadySnapshotAssemblyTimeoutCheckpoint}`
        );
    }
    if (
        isPositiveTimingValue(
            parseExecCoreBuildExactReadySnapshotAssemblyTimeoutCheckpointElapsedMs
        )
    ) {
        parts.push(
            `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint_elapsed_ms=${parseExecCoreBuildExactReadySnapshotAssemblyTimeoutCheckpointElapsedMs}`
        );
    }
    if (
        isPositiveTimingValue(
            parseExecCoreBuildExactReadySnapshotAssemblyProgramConversionMs
        )
    ) {
        parts.push(
            `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms=${parseExecCoreBuildExactReadySnapshotAssemblyProgramConversionMs}`
        );
    }
    if (
        isPositiveTimingValue(
            parseExecCoreBuildExactReadySnapshotAssemblyProgramLoweringMs
        )
    ) {
        parts.push(
            `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms=${parseExecCoreBuildExactReadySnapshotAssemblyProgramLoweringMs}`
        );
    }
    if (
        isPositiveTimingValue(
            parseExecCoreBuildExactReadySnapshotAssemblyPublishableArtifactPackagingMs
        )
    ) {
        parts.push(
            `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms=${parseExecCoreBuildExactReadySnapshotAssemblyPublishableArtifactPackagingMs}`
        );
    }
    if (
        isPositiveTimingValue(
            parseExecCoreBuildExactReadySnapshotAssemblySyntaxErrorCollectionMs
        )
    ) {
        parts.push(
            `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms=${parseExecCoreBuildExactReadySnapshotAssemblySyntaxErrorCollectionMs}`
        );
    }
    if (parseExecCoreBuildExactReadySnapshotAssemblyDominantCheckpoint) {
        parts.push(
            `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint=${parseExecCoreBuildExactReadySnapshotAssemblyDominantCheckpoint}`
        );
    }
    if (
        isPositiveTimingValue(
            parseExecCoreBuildExactReadySnapshotAssemblyDominantCheckpointMs
        )
    ) {
        parts.push(
            `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint_ms=${parseExecCoreBuildExactReadySnapshotAssemblyDominantCheckpointMs}`
        );
    }
    if (isPositiveTimingValue(parseExecCoreBuildTreeCacheInstallMs)) {
        parts.push(
            `followup_ready_snapshot_parse_exec_core_build_tree_cache_install_ms=${parseExecCoreBuildTreeCacheInstallMs}`
        );
    }
    if (isPositiveTimingValue(parseExecOptionalCacheEnrichmentMs)) {
        parts.push(
            `followup_ready_snapshot_parse_exec_optional_cache_enrichment_ms=${parseExecOptionalCacheEnrichmentMs}`
        );
    }
    if (parseExecCoreBuildDominantCheckpoint) {
        parts.push(
            `followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint=${parseExecCoreBuildDominantCheckpoint}`
        );
    }
    if (isPositiveTimingValue(parseExecCoreBuildDominantCheckpointMs)) {
        parts.push(
            `followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint_ms=${parseExecCoreBuildDominantCheckpointMs}`
        );
    }
    if (parseExecDominantSubphase) {
        parts.push(
            `followup_ready_snapshot_parse_exec_dominant_subphase=${parseExecDominantSubphase}`
        );
    }
    if (isPositiveTimingValue(parseExecDominantSubphaseMs)) {
        parts.push(
            `followup_ready_snapshot_parse_exec_dominant_subphase_ms=${parseExecDominantSubphaseMs}`
        );
    }
    if (isPositiveTimingValue(postParsePreMaterializationMs)) {
        parts.push(
            `followup_ready_snapshot_post_parse_pre_materialization_ms=${postParsePreMaterializationMs}`
        );
    }
    if (isPositiveTimingValue(readyInstallMs)) {
        parts.push(`followup_ready_snapshot_ready_install_ms=${readyInstallMs}`);
    }
    if (isPositiveTimingValue(documentSymbolSideWorkMs)) {
        parts.push(
            `followup_ready_snapshot_document_symbol_side_work_ms=${documentSymbolSideWorkMs}`
        );
    }
    if (dominantPhase) {
        parts.push(`followup_ready_snapshot_dominant_phase=${dominantPhase}`);
    }
    if (isPositiveTimingValue(dominantPhaseMs)) {
        parts.push(`followup_ready_snapshot_dominant_phase_ms=${dominantPhaseMs}`);
    }
    if (parseExecSubphaseNote) {
        parts.push(
            `followup_ready_snapshot_parse_exec_subphase_attribution=${parseExecSubphaseNote}`
        );
    }
    if (coreBuildCheckpointNote) {
        parts.push(
            `followup_ready_snapshot_core_build_checkpoint_attribution=${coreBuildCheckpointNote}`
        );
    }
    if (exactReadySnapshotAssemblyCheckpointNote) {
        parts.push(
            `followup_ready_snapshot_exact_ready_snapshot_assembly_checkpoint_attribution=${exactReadySnapshotAssemblyCheckpointNote}`
        );
    }
    return parts.join(' | ');
}

function formatFollowupProgramLoweringReuse(
    trace: ObservabilityIncidentDiagnosticsSaveSummary
): string | undefined {
    const parts: string[] = [];
    const prefix =
        'followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering';

    const reuseOutcome =
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_outcome;
    if (reuseOutcome) {
        parts.push(`${prefix}_reuse_outcome=${reuseOutcome}`);
    }
    const reusedLoweringUnits =
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_lowering_units;
    if (hasValue(reusedLoweringUnits)) {
        parts.push(`${prefix}_reused_lowering_units=${reusedLoweringUnits}`);
    }
    const rebuiltLoweringUnits =
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_lowering_units;
    if (hasValue(rebuiltLoweringUnits)) {
        parts.push(`${prefix}_rebuilt_lowering_units=${rebuiltLoweringUnits}`);
    }
    const reusedWindowCount =
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_window_count;
    if (hasValue(reusedWindowCount)) {
        parts.push(`${prefix}_reused_window_count=${reusedWindowCount}`);
    }
    const rebuiltWindowCount =
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_window_count;
    if (hasValue(rebuiltWindowCount)) {
        parts.push(`${prefix}_rebuilt_window_count=${rebuiltWindowCount}`);
    }
    const reusePlanBuildSource =
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_source;
    if (reusePlanBuildSource) {
        parts.push(`${prefix}_reuse_plan_build_source=${reusePlanBuildSource}`);
    }
    const reuseSeedSource =
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_seed_source;
    if (reuseSeedSource) {
        parts.push(`${prefix}_reuse_seed_source=${reuseSeedSource}`);
    }
    const reuseSeedCandidateCount =
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_seed_candidate_count;
    if (hasValue(reuseSeedCandidateCount)) {
        parts.push(`${prefix}_reuse_seed_candidate_count=${reuseSeedCandidateCount}`);
    }
    const reuseSeedEvictionReason =
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_seed_eviction_reason;
    if (reuseSeedEvictionReason) {
        parts.push(`${prefix}_reuse_seed_eviction_reason=${reuseSeedEvictionReason}`);
    }
    const reusePlanFailureReason =
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_failure_reason;
    if (reusePlanFailureReason) {
        parts.push(`${prefix}_reuse_plan_failure_reason=${reusePlanFailureReason}`);
    }
    const reusePlanTakeIfUniqueHit =
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_take_if_unique_hit;
    if (hasValue(reusePlanTakeIfUniqueHit)) {
        parts.push(`${prefix}_reuse_plan_take_if_unique_hit=${reusePlanTakeIfUniqueHit}`);
    }
    const reusePlanBorrowedCacheHit =
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_cache_hit;
    if (hasValue(reusePlanBorrowedCacheHit)) {
        parts.push(`${prefix}_reuse_plan_borrowed_cache_hit=${reusePlanBorrowedCacheHit}`);
    }

    if (parts.length === 0) {
        return undefined;
    }
    return parts.join(' | ');
}

function formatFollowupReadySnapshotReliefValve(
    note: string | undefined,
    outcome: string | undefined,
    budgetMs: number | undefined,
    elapsedMs: number | undefined
): string | undefined {
    if (note) {
        return `followup_ready_snapshot_relief_valve=${note}`;
    }
    if (!outcome && !isPositiveTimingValue(budgetMs) && !isPositiveTimingValue(elapsedMs)) {
        return undefined;
    }

    const parts: string[] = [];
    if (outcome) {
        parts.push(`followup_ready_snapshot_relief_valve_outcome=${outcome}`);
    }
    if (isPositiveTimingValue(budgetMs)) {
        parts.push(`followup_ready_snapshot_relief_valve_budget_ms=${budgetMs}`);
    }
    if (isPositiveTimingValue(elapsedMs)) {
        parts.push(`followup_ready_snapshot_relief_valve_elapsed_ms=${elapsedMs}`);
    }
    return parts.join(' | ');
}

export function renderDiagnosticsSaveSummaryLines(
    section: ObservabilityIncidentDiagnosticsSaveSection
): string[] {
    if (section.requests.length === 0) {
        return ['No diagnostics save traces captured in this bundle.'];
    }

    return section.requests.flatMap((request) => [
        `trace=${request.trace_id} | uri=${request.uri} | requested_version=${request.requested_version} | save_cycle_sequence=${request.save_cycle_sequence} | diagnostics_generation=${request.diagnostics_generation} | trigger=${request.trigger} | save_fastlane_outcome=${renderProfileOutcome(request.save_fastlane_outcome, request.terminal_outcome)} | idle_heavy_outcome=${renderProfileOutcome(request.idle_heavy_outcome, request.terminal_outcome)} | terminal=${renderTerminalOutcome(request.terminal_outcome)}`,
        formatPublishWithLifecycle('first_publish', request.first_publish, request.terminal_outcome),
        formatPublishWithLifecycle('followup_publish', request.followup_publish, request.terminal_outcome),
        formatFollowupSemanticAttribution(
            request.followup_semantic_attribution_note,
            request.followup_semantic_path,
            request.followup_semantic_parse_source,
            request.followup_semantic_ir_source
        ),
        formatFollowupReadySnapshotAttribution(
            request.followup_ready_snapshot_attribution_note,
            request.followup_ready_snapshot_zero_probe,
            request.followup_ready_snapshot_wait_probe,
            request.followup_ready_snapshot_task_state,
            request.followup_shadow_state_available
        ),
        formatFollowupReadySnapshotPhases(
            request.followup_ready_snapshot_phase_attribution_note,
            request.followup_ready_snapshot_timeout_leaf_note,
            request.followup_ready_snapshot_parse_exec_subphase_note,
            request.followup_ready_snapshot_core_build_checkpoint_note,
            request.followup_ready_snapshot_exact_ready_snapshot_assembly_checkpoint_note,
            request.followup_ready_snapshot_timeout_phase,
            request.followup_ready_snapshot_timeout_phase_elapsed_ms,
            request.followup_ready_snapshot_timeout_leaf,
            request.followup_ready_snapshot_timeout_leaf_elapsed_ms,
            request.followup_ready_snapshot_parse_exec_ms,
            request.followup_ready_snapshot_parse_exec_timeout_subphase,
            request.followup_ready_snapshot_parse_exec_timeout_subphase_elapsed_ms,
            request.followup_ready_snapshot_parse_exec_core_parse_build_ms,
            request.followup_ready_snapshot_parse_exec_core_build_pre_parse_setup_ms,
            request.followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint,
            request.followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint_elapsed_ms,
            request.followup_ready_snapshot_parse_exec_core_build_parser_base_recovery_ms,
            request.followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms,
            request.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_ms,
            request.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint,
            request.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint_elapsed_ms,
            request.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms,
            request.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms,
            request.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms,
            request.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms,
            request.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint,
            request.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint_ms,
            request.followup_ready_snapshot_parse_exec_core_build_tree_cache_install_ms,
            request.followup_ready_snapshot_parse_exec_optional_cache_enrichment_ms,
            request.followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint,
            request.followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint_ms,
            request.followup_ready_snapshot_parse_exec_dominant_subphase,
            request.followup_ready_snapshot_parse_exec_dominant_subphase_ms,
            request.followup_ready_snapshot_post_parse_pre_materialization_ms,
            request.followup_ready_snapshot_ready_install_ms,
            request.followup_ready_snapshot_document_symbol_side_work_ms,
            request.followup_ready_snapshot_dominant_phase,
            request.followup_ready_snapshot_dominant_phase_ms
        ),
        formatFollowupProgramLoweringReuse(request),
        formatFollowupReadySnapshotReliefValve(
            request.followup_ready_snapshot_relief_valve_note,
            request.followup_ready_snapshot_relief_valve_outcome,
            request.followup_ready_snapshot_relief_valve_budget_ms,
            request.followup_ready_snapshot_relief_valve_elapsed_ms
        ),
        formatFollowupWait(
            request.followup_syntax_work_mode,
            request.followup_wait_reason,
            request.followup_blocker_reason,
            request.followup_blocker_note,
            request.followup_runtime_queue_wait_ms,
            request.followup_apply_lag_ms,
            request.followup_wait_for_file_version_ms,
            request.followup_snapshot_with_deps_ms,
            request.followup_readiness_blocker_bucket,
            request.followup_unclassified_readiness_residual_ms
        ),
    ].filter((line): line is string => Boolean(line)));
}
