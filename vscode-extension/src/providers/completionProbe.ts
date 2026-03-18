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

export type CompletionProbeDidChangeDeltaMs = number | 'unknown';

export interface CompletionProbeInput {
    probe_id: string;
    uri: string;
    document_version: number;
    trigger_mode: CompletionProbeTriggerMode;
    trigger_character?: string | null;
    request_started_at_ms: number;
    request_completed_at_ms: number;
    client_terminal_state: CompletionProbeTerminalState;
    time_since_last_local_edit_ms: number;
    time_since_last_did_change_sent_ms: CompletionProbeDidChangeDeltaMs;
    is_after_dot: boolean;
    identifier_tail_length: number;
}

export interface CompletionProbe {
    probe_id: string;
    uri: string;
    document_version: number;
    trigger_mode: CompletionProbeTriggerMode;
    trigger_character?: string;
    request_started_at_ms: number;
    request_completed_at_ms: number;
    client_duration_ms: number;
    client_terminal_state: CompletionProbeTerminalState;
    time_since_last_local_edit_ms: number;
    time_since_last_did_change_sent_ms: CompletionProbeDidChangeDeltaMs;
    is_after_dot: boolean;
    identifier_tail_length: number;
}

export function buildCompletionProbe(input: CompletionProbeInput): CompletionProbe {
    const requestStartedAtMs = clampNonNegativeInteger(input.request_started_at_ms);
    const requestCompletedAtMs = clampNonNegativeInteger(input.request_completed_at_ms);

    const probe: CompletionProbe = {
        probe_id: sanitizeProbeId(input.probe_id),
        uri: String(input.uri),
        document_version: clampNonNegativeInteger(input.document_version),
        trigger_mode: input.trigger_mode,
        request_started_at_ms: requestStartedAtMs,
        request_completed_at_ms: requestCompletedAtMs,
        client_duration_ms: Math.max(0, requestCompletedAtMs - requestStartedAtMs),
        client_terminal_state: input.client_terminal_state,
        time_since_last_local_edit_ms: clampNonNegativeInteger(input.time_since_last_local_edit_ms),
        time_since_last_did_change_sent_ms: sanitizeDidChangeDelta(
            input.time_since_last_did_change_sent_ms
        ),
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

function sanitizeProbeId(value: string): string {
    if (typeof value !== 'string' || value.length === 0) {
        return 'probe';
    }
    return value.slice(0, COMPLETION_PROBE_MAX_ID_LENGTH);
}
