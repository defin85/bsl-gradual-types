export const COMPLETION_PROBE_MAX_TRIGGER_CHARACTER_LENGTH = 8;
export const COMPLETION_PROBE_MAX_IDENTIFIER_TAIL_LENGTH = 1024;
const COMPLETION_PROBE_MAX_ID_LENGTH = 128;

export type CompletionProbeTriggerMode =
    | 'invoked'
    | 'trigger_character'
    | 'trigger_for_incomplete_completions';

export type CompletionProbeTerminalState =
    | 'ok_non_empty'
    | 'ok_empty'
    | 'cancelled'
    | 'error';

export type CompletionProbeCancelReasonHint =
    | 'superseded_same_version'
    | 'superseded_newer_version'
    | 'editor_state_changed'
    | 'unknown';

export type CompletionProbeResultKind =
    | 'non_empty'
    | 'empty_array'
    | 'empty_list'
    | 'nullish';

export type CompletionProbeItemCountBucket =
    | '0'
    | '1_5'
    | '6_20'
    | '21_plus';

export type CompletionProbeDidChangeDeltaMs = number | 'unknown';
export type CompletionProbeTransportReceiveState = 'observed' | 'unavailable';

export interface CompletionProbeInput {
    probe_id: string;
    uri: string;
    document_version: number;
    document_version_at_terminal: number;
    trigger_mode: CompletionProbeTriggerMode;
    trigger_character?: string | null;
    request_started_at_ms: number;
    lsp_request_started_at_ms: number;
    transport_response_receive_state: CompletionProbeTransportReceiveState;
    transport_response_received_at_ms?: number | null;
    lsp_response_received_at_ms: number;
    request_completed_at_ms: number;
    client_terminal_state: CompletionProbeTerminalState;
    cancel_reason_hint: CompletionProbeCancelReasonHint;
    result_kind: CompletionProbeResultKind;
    item_count_bucket: CompletionProbeItemCountBucket;
    is_incomplete?: boolean | null;
    time_since_last_local_edit_ms: number;
    time_since_last_did_change_sent_ms: CompletionProbeDidChangeDeltaMs;
    did_change_count_during_probe: number;
    cursor_moved_during_probe: boolean;
    active_completion_count_at_start: number;
    same_uri_probe_overlap_count: number;
    newer_probe_started_before_terminal: boolean;
    superseded_by_probe_id?: string | null;
    superseded_after_ms?: number | null;
    is_after_dot: boolean;
    identifier_tail_length: number;
}

export interface CompletionProbe {
    probe_id: string;
    uri: string;
    document_version: number;
    document_version_at_terminal: number;
    trigger_mode: CompletionProbeTriggerMode;
    trigger_character?: string;
    request_started_at_ms: number;
    lsp_request_started_at_ms: number;
    transport_response_receive_state: CompletionProbeTransportReceiveState;
    transport_response_received_at_ms?: number;
    lsp_response_received_at_ms: number;
    request_completed_at_ms: number;
    client_duration_ms: number;
    client_terminal_state: CompletionProbeTerminalState;
    cancel_reason_hint: CompletionProbeCancelReasonHint;
    result_kind: CompletionProbeResultKind;
    item_count_bucket: CompletionProbeItemCountBucket;
    is_incomplete?: boolean;
    time_since_last_local_edit_ms: number;
    time_since_last_did_change_sent_ms: CompletionProbeDidChangeDeltaMs;
    did_change_count_during_probe: number;
    cursor_moved_during_probe: boolean;
    active_completion_count_at_start: number;
    same_uri_probe_overlap_count: number;
    newer_probe_started_before_terminal: boolean;
    superseded_by_probe_id?: string;
    superseded_after_ms?: number;
    is_after_dot: boolean;
    identifier_tail_length: number;
}

export function buildCompletionProbe(input: CompletionProbeInput): CompletionProbe {
    const requestStartedAtMs = clampNonNegativeInteger(input.request_started_at_ms);
    const lspRequestStartedAtMs = clampNonNegativeInteger(input.lsp_request_started_at_ms);
    const lspResponseReceivedAtMs = clampNonNegativeInteger(input.lsp_response_received_at_ms);
    const requestCompletedAtMs = clampNonNegativeInteger(input.request_completed_at_ms);

    const probe: CompletionProbe = {
        probe_id: sanitizeProbeId(input.probe_id),
        uri: String(input.uri),
        document_version: clampNonNegativeInteger(input.document_version),
        document_version_at_terminal: clampNonNegativeInteger(input.document_version_at_terminal),
        trigger_mode: input.trigger_mode,
        request_started_at_ms: requestStartedAtMs,
        lsp_request_started_at_ms: lspRequestStartedAtMs,
        transport_response_receive_state: sanitizeTransportReceiveState(
            input.transport_response_receive_state
        ),
        lsp_response_received_at_ms: lspResponseReceivedAtMs,
        request_completed_at_ms: requestCompletedAtMs,
        client_duration_ms: Math.max(0, requestCompletedAtMs - requestStartedAtMs),
        client_terminal_state: input.client_terminal_state,
        cancel_reason_hint: sanitizeCancelReasonHint(input.cancel_reason_hint),
        result_kind: sanitizeResultKind(input.result_kind),
        item_count_bucket: sanitizeItemCountBucket(input.item_count_bucket),
        time_since_last_local_edit_ms: clampNonNegativeInteger(input.time_since_last_local_edit_ms),
        time_since_last_did_change_sent_ms: sanitizeDidChangeDelta(
            input.time_since_last_did_change_sent_ms
        ),
        did_change_count_during_probe: clampNonNegativeInteger(input.did_change_count_during_probe),
        cursor_moved_during_probe: Boolean(input.cursor_moved_during_probe),
        active_completion_count_at_start: clampNonNegativeInteger(input.active_completion_count_at_start),
        same_uri_probe_overlap_count: clampNonNegativeInteger(input.same_uri_probe_overlap_count),
        newer_probe_started_before_terminal: Boolean(input.newer_probe_started_before_terminal),
        is_after_dot: Boolean(input.is_after_dot),
        identifier_tail_length: Math.min(
            COMPLETION_PROBE_MAX_IDENTIFIER_TAIL_LENGTH,
            clampNonNegativeInteger(input.identifier_tail_length)
        ),
    };

    const triggerCharacter = sanitizeTriggerCharacter(input.trigger_character);
    if (triggerCharacter) {
        probe.trigger_character = triggerCharacter;
    }

    if (typeof input.is_incomplete === 'boolean') {
        probe.is_incomplete = input.is_incomplete;
    }

    if (typeof input.transport_response_received_at_ms === 'number') {
        probe.transport_response_received_at_ms = clampNonNegativeInteger(
            input.transport_response_received_at_ms
        );
    }

    if (typeof input.superseded_by_probe_id === 'string' && input.superseded_by_probe_id.length > 0) {
        probe.superseded_by_probe_id = sanitizeProbeId(input.superseded_by_probe_id);
    }

    if (typeof input.superseded_after_ms === 'number') {
        probe.superseded_after_ms = clampNonNegativeInteger(input.superseded_after_ms);
    }

    return probe;
}

function clampNonNegativeInteger(value: number): number {
    if (!Number.isFinite(value)) {
        return 0;
    }
    return Math.max(0, Math.trunc(value));
}

function sanitizeDidChangeDelta(value: CompletionProbeDidChangeDeltaMs): CompletionProbeDidChangeDeltaMs {
    if (value === 'unknown') {
        return value;
    }
    return clampNonNegativeInteger(value);
}

function sanitizeTriggerCharacter(value?: string | null): string | undefined {
    if (typeof value !== 'string') {
        return undefined;
    }
    if (value.length === 0) {
        return undefined;
    }
    return value.slice(0, COMPLETION_PROBE_MAX_TRIGGER_CHARACTER_LENGTH);
}

function sanitizeTransportReceiveState(
    value: CompletionProbeTransportReceiveState
): CompletionProbeTransportReceiveState {
    return value === 'observed' ? value : 'unavailable';
}

function sanitizeProbeId(value: string): string {
    if (typeof value !== 'string' || value.length === 0) {
        return 'probe';
    }
    return value.slice(0, COMPLETION_PROBE_MAX_ID_LENGTH);
}

function sanitizeCancelReasonHint(
    value: CompletionProbeCancelReasonHint
): CompletionProbeCancelReasonHint {
    switch (value) {
        case 'superseded_same_version':
        case 'superseded_newer_version':
        case 'editor_state_changed':
            return value;
        default:
            return 'unknown';
    }
}

function sanitizeResultKind(value: CompletionProbeResultKind): CompletionProbeResultKind {
    switch (value) {
        case 'non_empty':
        case 'empty_array':
        case 'empty_list':
            return value;
        default:
            return 'nullish';
    }
}

function sanitizeItemCountBucket(
    value: CompletionProbeItemCountBucket
): CompletionProbeItemCountBucket {
    switch (value) {
        case '1_5':
        case '6_20':
        case '21_plus':
            return value;
        default:
            return '0';
    }
}
