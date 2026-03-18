import * as vscode from 'vscode';
import {
    buildCompletionProbe,
    CompletionProbeCancelReasonHint,
    CompletionProbe,
    CompletionProbeDidChangeDeltaMs,
    CompletionProbeItemCountBucket,
    CompletionProbeResultKind,
    CompletionProbeTerminalState,
    CompletionProbeTriggerMode,
} from './completionProbe';
import { CompletionProbeStore } from './completionProbeStore';

interface DocumentClock {
    version: number;
    timestampMs: number;
}

interface CursorClock {
    sequence: number;
}

interface ActiveCompletionProbeSession {
    probeId: string;
    documentKey: string;
    documentVersion: number;
    requestStartedAtMs: number;
    triggerMode: CompletionProbeTriggerMode;
    triggerCharacter?: string;
    isAfterDot: boolean;
    identifierTailLength: number;
    timeSinceLastLocalEditMs: number;
    timeSinceLastDidChangeSentMs: CompletionProbeDidChangeDeltaMs;
    startDidChangeCount: number;
    startCursorSequence: number;
    activeCompletionCountAtStart: number;
    sameUriProbeOverlapCount: number;
    newerProbeStartedBeforeTerminal: boolean;
    cancelReasonHint: CompletionProbeCancelReasonHint;
    supersededByProbeId?: string;
    supersededAfterMs?: number;
    lspRequestStartedAtMs: number;
    lspResponseReceivedAtMs: number;
}

export interface CompletionProbeStartInput {
    document: vscode.TextDocument;
    position: vscode.Position;
    context: vscode.CompletionContext;
    token: vscode.CancellationToken;
    requestStartedAtMs: number;
}

export interface CompletionProbeRecorderOptions {
    now?: () => number;
    store?: CompletionProbeStore;
}

export interface CompletionProbeOutcomeInput {
    document: vscode.TextDocument;
    position: vscode.Position;
    context: vscode.CompletionContext;
    result: vscode.CompletionItem[] | vscode.CompletionList | null | undefined;
    requestStartedAtMs: number;
    requestCompletedAtMs: number;
    token?: vscode.CancellationToken;
    wasCancelled: boolean;
    error?: unknown;
}

const IDENTIFIER_TAIL_PATTERN = /[A-Za-zА-Яа-яЁё0-9_]+$/;

export class CompletionProbeRecorder {
    private readonly now: () => number;
    private readonly store: CompletionProbeStore;
    private readonly lastLocalEditByUri = new Map<string, DocumentClock>();
    private readonly lastDidChangeSentByUri = new Map<string, DocumentClock>();
    private readonly didChangeCountByUri = new Map<string, number>();
    private readonly cursorClockByUri = new Map<string, CursorClock>();
    private readonly activeSessionsByToken = new Map<vscode.CancellationToken, ActiveCompletionProbeSession>();
    private readonly activeSessionsByUri = new Map<string, ActiveCompletionProbeSession[]>();
    private nextProbeSequence = 1;

    constructor(options: CompletionProbeRecorderOptions = {}) {
        this.now = options.now ?? Date.now;
        this.store = options.store ?? new CompletionProbeStore();
    }

    get size(): number {
        return this.store.size;
    }

    clear(): void {
        this.store.clear();
        this.lastLocalEditByUri.clear();
        this.lastDidChangeSentByUri.clear();
    }

    snapshot(): CompletionProbe[] {
        return this.store.snapshot();
    }

    recordTextDocumentDidChange(event: vscode.TextDocumentChangeEvent): void {
        if (!hasMeaningfulContentChanges(event.contentChanges)) {
            return;
        }

        const documentKey = toDocumentKey(event.document);
        this.lastLocalEditByUri.set(documentKey, {
            version: event.document.version,
            timestampMs: this.now(),
        });
        this.didChangeCountByUri.set(
            documentKey,
            (this.didChangeCountByUri.get(documentKey) ?? 0) + 1
        );
    }

    recordTextDocumentDidChangeSent(document: vscode.TextDocument): void {
        this.lastDidChangeSentByUri.set(toDocumentKey(document), {
            version: document.version,
            timestampMs: this.now(),
        });
    }

    recordTextDocumentDidClose(document: vscode.TextDocument): void {
        const key = toDocumentKey(document);
        this.lastLocalEditByUri.delete(key);
        this.lastDidChangeSentByUri.delete(key);
        this.didChangeCountByUri.delete(key);
        this.cursorClockByUri.delete(key);
    }

    recordTextEditorSelectionChanged(editor: Pick<vscode.TextEditor, 'document'>): void {
        const key = toDocumentKey(editor.document);
        const current = this.cursorClockByUri.get(key);
        this.cursorClockByUri.set(key, {
            sequence: (current?.sequence ?? 0) + 1,
        });
    }

    recordCompletionStarted(input: CompletionProbeStartInput): string {
        const documentKey = toDocumentKey(input.document);
        const requestStartedAtMs = clampTimestamp(input.requestStartedAtMs);
        const localEditClock = this.lastLocalEditByUri.get(documentKey);
        const didChangeClock = this.lastDidChangeSentByUri.get(documentKey);
        const linePrefix = getLinePrefix(input.document, input.position);
        const activeSessionsForUri = this.activeSessionsByUri.get(documentKey) ?? [];
        const session: ActiveCompletionProbeSession = {
            probeId: `probe-${this.nextProbeSequence++}`,
            documentKey,
            documentVersion: input.document.version,
            requestStartedAtMs,
            triggerMode: mapTriggerMode(input.context),
            triggerCharacter: input.context.triggerCharacter ?? undefined,
            isAfterDot: linePrefix.endsWith('.'),
            identifierTailLength: measureIdentifierTailLength(linePrefix),
            timeSinceLastLocalEditMs: computeExactDeltaMs(
                localEditClock,
                input.document.version,
                requestStartedAtMs,
                0
            ),
            timeSinceLastDidChangeSentMs: computeDidChangeDeltaMs(
                didChangeClock,
                input.document.version,
                requestStartedAtMs
            ),
            startDidChangeCount: this.didChangeCountByUri.get(documentKey) ?? 0,
            startCursorSequence: this.cursorClockByUri.get(documentKey)?.sequence ?? 0,
            activeCompletionCountAtStart: this.activeSessionsByToken.size,
            sameUriProbeOverlapCount: activeSessionsForUri.length,
            newerProbeStartedBeforeTerminal: false,
            cancelReasonHint: 'unknown',
            lspRequestStartedAtMs: requestStartedAtMs,
            lspResponseReceivedAtMs: requestStartedAtMs,
        };

        for (const activeSession of activeSessionsForUri) {
            activeSession.newerProbeStartedBeforeTerminal = true;
            activeSession.supersededByProbeId = session.probeId;
            activeSession.supersededAfterMs = Math.max(
                0,
                requestStartedAtMs - activeSession.requestStartedAtMs
            );
            activeSession.cancelReasonHint =
                session.documentVersion > activeSession.documentVersion
                    ? 'superseded_newer_version'
                    : 'superseded_same_version';
        }

        this.activeSessionsByToken.set(input.token, session);
        this.activeSessionsByUri.set(documentKey, [...activeSessionsForUri, session]);
        return session.probeId;
    }

    recordCompletionLspRequestStarted(
        token: vscode.CancellationToken,
        timestampMs: number = this.now()
    ): void {
        const session = this.activeSessionsByToken.get(token);
        if (!session) {
            return;
        }

        session.lspRequestStartedAtMs = clampTimestamp(timestampMs);
    }

    recordCompletionLspResponseReceived(
        token: vscode.CancellationToken,
        timestampMs: number = this.now()
    ): void {
        const session = this.activeSessionsByToken.get(token);
        if (!session) {
            return;
        }

        session.lspResponseReceivedAtMs = clampTimestamp(timestampMs);
    }

    recordCompletionOutcome(input: CompletionProbeOutcomeInput): CompletionProbe {
        const documentKey = toDocumentKey(input.document);
        const requestCompletedAtMs = clampTimestamp(input.requestCompletedAtMs);
        const session = input.token
            ? this.activeSessionsByToken.get(input.token)
            : undefined;
        const requestStartedAtMs = session
            ? session.requestStartedAtMs
            : clampTimestamp(input.requestStartedAtMs);
        const localEditClock = this.lastLocalEditByUri.get(documentKey);
        const didChangeClock = this.lastDidChangeSentByUri.get(documentKey);
        const linePrefix = getLinePrefix(input.document, input.position);
        const didChangeCountAtTerminal = this.didChangeCountByUri.get(documentKey) ?? 0;
        const cursorClockAtTerminal = this.cursorClockByUri.get(documentKey)?.sequence ?? 0;
        const resultShape = classifyResultShape(input.result);
        const cancelReasonHint = classifyCancelReasonHint(
            session,
            input.document.version,
            didChangeCountAtTerminal,
            cursorClockAtTerminal,
            input.wasCancelled
        );

        const probe = buildCompletionProbe({
            probe_id: session?.probeId ?? `probe-${this.nextProbeSequence++}`,
            uri: documentKey,
            document_version: session?.documentVersion ?? input.document.version,
            document_version_at_terminal: input.document.version,
            trigger_mode: session?.triggerMode ?? mapTriggerMode(input.context),
            trigger_character: session?.triggerCharacter ?? input.context.triggerCharacter,
            request_started_at_ms: requestStartedAtMs,
            lsp_request_started_at_ms: session?.lspRequestStartedAtMs ?? requestStartedAtMs,
            lsp_response_received_at_ms:
                session?.lspResponseReceivedAtMs ?? requestCompletedAtMs,
            request_completed_at_ms: requestCompletedAtMs,
            client_terminal_state: classifyTerminalState(
                input.result,
                input.wasCancelled,
                input.error
            ),
            cancel_reason_hint: cancelReasonHint,
            result_kind: resultShape.kind,
            item_count_bucket: resultShape.itemCountBucket,
            is_incomplete: resultShape.isIncomplete,
            time_since_last_local_edit_ms:
                session?.timeSinceLastLocalEditMs
                ?? computeExactDeltaMs(
                    localEditClock,
                    input.document.version,
                    requestStartedAtMs,
                    0
                ),
            time_since_last_did_change_sent_ms:
                session?.timeSinceLastDidChangeSentMs
                ?? computeDidChangeDeltaMs(
                    didChangeClock,
                    input.document.version,
                    requestStartedAtMs
                ),
            did_change_count_during_probe: Math.max(
                0,
                didChangeCountAtTerminal - (session?.startDidChangeCount ?? didChangeCountAtTerminal)
            ),
            cursor_moved_during_probe:
                cursorClockAtTerminal > (session?.startCursorSequence ?? cursorClockAtTerminal),
            active_completion_count_at_start: session?.activeCompletionCountAtStart ?? 0,
            same_uri_probe_overlap_count: session?.sameUriProbeOverlapCount ?? 0,
            newer_probe_started_before_terminal: session?.newerProbeStartedBeforeTerminal ?? false,
            superseded_by_probe_id: session?.supersededByProbeId,
            superseded_after_ms: session?.supersededAfterMs,
            is_after_dot: session?.isAfterDot ?? linePrefix.endsWith('.'),
            identifier_tail_length: session?.identifierTailLength ?? measureIdentifierTailLength(linePrefix),
        });

        if (input.token) {
            this.activeSessionsByToken.delete(input.token);
        }
        if (session) {
            const activeSessionsForUri = this.activeSessionsByUri.get(session.documentKey) ?? [];
            const nextActiveSessions = activeSessionsForUri.filter(
                (activeSession) => activeSession.probeId !== session.probeId
            );
            if (nextActiveSessions.length === 0) {
                this.activeSessionsByUri.delete(session.documentKey);
            } else {
                this.activeSessionsByUri.set(session.documentKey, nextActiveSessions);
            }
        }

        this.store.add(probe);
        return probe;
    }
}

let sharedCompletionProbeRecorder: CompletionProbeRecorder | undefined;

export function getSharedCompletionProbeRecorder(): CompletionProbeRecorder {
    if (!sharedCompletionProbeRecorder) {
        sharedCompletionProbeRecorder = new CompletionProbeRecorder();
    }

    return sharedCompletionProbeRecorder;
}

export function resetSharedCompletionProbeRecorderForTests(): void {
    sharedCompletionProbeRecorder = undefined;
}

function hasMeaningfulContentChanges(
    contentChanges: readonly vscode.TextDocumentContentChangeEvent[]
): boolean {
    return contentChanges.length > 0;
}

function toDocumentKey(document: Pick<vscode.TextDocument, 'uri'>): string {
    return document.uri.toString();
}

function mapTriggerMode(context: vscode.CompletionContext): CompletionProbeTriggerMode {
    switch (context.triggerKind) {
        case vscode.CompletionTriggerKind.TriggerCharacter:
            return 'trigger_character';
        case vscode.CompletionTriggerKind.TriggerForIncompleteCompletions:
            return 'trigger_for_incomplete_completions';
        default:
            return 'invoked';
    }
}

function classifyTerminalState(
    result: vscode.CompletionItem[] | vscode.CompletionList | null | undefined,
    wasCancelled: boolean,
    error?: unknown
): CompletionProbeTerminalState {
    if (wasCancelled) {
        return 'cancelled';
    }

    if (error !== undefined && error !== null) {
        return 'error';
    }

    if (isNonEmptyCompletionResult(result)) {
        return 'ok_non_empty';
    }

    return 'ok_empty';
}

function classifyResultShape(
    result: vscode.CompletionItem[] | vscode.CompletionList | null | undefined
): {
    kind: CompletionProbeResultKind;
    itemCountBucket: CompletionProbeItemCountBucket;
    isIncomplete?: boolean;
} {
    if (Array.isArray(result)) {
        return {
            kind: result.length > 0 ? 'non_empty' : 'empty_array',
            itemCountBucket: classifyItemCountBucket(result.length),
        };
    }

    if (result && Array.isArray(result.items)) {
        return {
            kind: result.items.length > 0 ? 'non_empty' : 'empty_list',
            itemCountBucket: classifyItemCountBucket(result.items.length),
            isIncomplete: typeof result.isIncomplete === 'boolean' ? result.isIncomplete : undefined,
        };
    }

    return {
        kind: 'nullish',
        itemCountBucket: '0',
    };
}

function classifyItemCountBucket(count: number): CompletionProbeItemCountBucket {
    if (count <= 0) {
        return '0';
    }
    if (count <= 5) {
        return '1_5';
    }
    if (count <= 20) {
        return '6_20';
    }
    return '21_plus';
}

function classifyCancelReasonHint(
    session: ActiveCompletionProbeSession | undefined,
    documentVersionAtTerminal: number,
    didChangeCountAtTerminal: number,
    cursorClockAtTerminal: number,
    wasCancelled: boolean
): CompletionProbeCancelReasonHint {
    if (!wasCancelled) {
        return session?.cancelReasonHint ?? 'unknown';
    }

    if (session?.cancelReasonHint && session.cancelReasonHint !== 'unknown') {
        return session.cancelReasonHint;
    }

    if (
        session &&
        (documentVersionAtTerminal !== session.documentVersion
            || didChangeCountAtTerminal > session.startDidChangeCount
            || cursorClockAtTerminal > session.startCursorSequence)
    ) {
        return 'editor_state_changed';
    }

    return 'unknown';
}

function isNonEmptyCompletionResult(
    result: vscode.CompletionItem[] | vscode.CompletionList | null | undefined
): boolean {
    if (Array.isArray(result)) {
        return result.length > 0;
    }

    return Boolean(result && Array.isArray(result.items) && result.items.length > 0);
}

function computeExactDeltaMs(
    clock: DocumentClock | undefined,
    documentVersion: number,
    requestStartedAtMs: number,
    fallback: number
): number {
    if (!clock || clock.version !== documentVersion) {
        return fallback;
    }

    return Math.max(0, requestStartedAtMs - clock.timestampMs);
}

function computeDidChangeDeltaMs(
    clock: DocumentClock | undefined,
    documentVersion: number,
    requestStartedAtMs: number
): CompletionProbeDidChangeDeltaMs {
    if (!clock || clock.version !== documentVersion) {
        return 'unknown';
    }

    return Math.max(0, requestStartedAtMs - clock.timestampMs);
}

function getLinePrefix(document: vscode.TextDocument, position: vscode.Position): string {
    try {
        const line = document.lineAt(position.line);
        return line.text.slice(0, Math.max(0, position.character));
    } catch {
        return '';
    }
}

function measureIdentifierTailLength(linePrefix: string): number {
    const match = IDENTIFIER_TAIL_PATTERN.exec(linePrefix);
    return match ? match[0].length : 0;
}

function clampTimestamp(value: number): number {
    if (!Number.isFinite(value)) {
        return 0;
    }

    return Math.max(0, Math.trunc(value));
}
