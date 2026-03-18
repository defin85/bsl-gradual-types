import * as vscode from 'vscode';
import {
    buildCompletionProbe,
    CompletionProbe,
    CompletionProbeDidChangeDeltaMs,
    CompletionProbeTerminalState,
    CompletionProbeTriggerMode,
} from './completionProbe';
import { CompletionProbeStore } from './completionProbeStore';

interface DocumentClock {
    version: number;
    timestampMs: number;
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
    wasCancelled: boolean;
    error?: unknown;
}

const IDENTIFIER_TAIL_PATTERN = /[A-Za-zА-Яа-яЁё0-9_]+$/;

export class CompletionProbeRecorder {
    private readonly now: () => number;
    private readonly store: CompletionProbeStore;
    private readonly lastLocalEditByUri = new Map<string, DocumentClock>();
    private readonly lastDidChangeSentByUri = new Map<string, DocumentClock>();
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

        this.lastLocalEditByUri.set(toDocumentKey(event.document), {
            version: event.document.version,
            timestampMs: this.now(),
        });
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
    }

    recordCompletionOutcome(input: CompletionProbeOutcomeInput): CompletionProbe {
        const documentKey = toDocumentKey(input.document);
        const requestStartedAtMs = clampTimestamp(input.requestStartedAtMs);
        const requestCompletedAtMs = clampTimestamp(input.requestCompletedAtMs);
        const localEditClock = this.lastLocalEditByUri.get(documentKey);
        const didChangeClock = this.lastDidChangeSentByUri.get(documentKey);
        const linePrefix = getLinePrefix(input.document, input.position);

        const probe = buildCompletionProbe({
            probe_id: `probe-${this.nextProbeSequence++}`,
            uri: documentKey,
            document_version: input.document.version,
            trigger_mode: mapTriggerMode(input.context),
            trigger_character: input.context.triggerCharacter,
            request_started_at_ms: requestStartedAtMs,
            request_completed_at_ms: requestCompletedAtMs,
            client_terminal_state: classifyTerminalState(
                input.result,
                input.wasCancelled,
                input.error
            ),
            time_since_last_local_edit_ms: computeExactDeltaMs(
                localEditClock,
                input.document.version,
                requestStartedAtMs,
                0
            ),
            time_since_last_did_change_sent_ms: computeDidChangeDeltaMs(
                didChangeClock,
                input.document.version,
                requestStartedAtMs
            ),
            is_after_dot: linePrefix.endsWith('.'),
            identifier_tail_length: measureIdentifierTailLength(linePrefix),
        });

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
