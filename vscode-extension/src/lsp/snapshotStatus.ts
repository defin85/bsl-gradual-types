import * as vscode from 'vscode';

import {
    getSnapshotStatusFetchResult,
} from './customRequests';
import type { SnapshotArtifactStatus, SnapshotStatusResponse } from './customRequests';

type ActiveSnapshotStatusView =
    | { kind: 'inactive' }
    | { kind: 'unsupported' }
    | { kind: 'unavailable'; message: string }
    | { kind: 'ok'; status: SnapshotStatusResponse };

let outputChannel: vscode.OutputChannel | undefined;
let statusBarItem: vscode.StatusBarItem | undefined;
let activeUri: string | null = null;
let snapshotStatusUnsupported = false;
let activeUnavailableMessage: string | null = null;
const snapshotStatusCache = new Map<string, SnapshotStatusResponse>();
const snapshotStatusHistory = new Map<string, SnapshotStatusResponse[]>();
const statusListeners = new Set<() => void>();
const subscriptions: vscode.Disposable[] = [];
let initialized = false;

const SNAPSHOT_TRANSITION_HISTORY_LIMIT = 20;
const SNAPSHOT_TEXT_DETAIL_LIMIT = 160;

function isBslEditor(
    editor: vscode.TextEditor | undefined
): editor is vscode.TextEditor & { document: vscode.TextDocument } {
    return !!editor && editor.document.languageId === 'bsl';
}

function fireSnapshotStatusChange(): void {
    for (const listener of statusListeners) {
        listener();
    }
}

function formatRevision(value: number | undefined): string {
    return typeof value === 'number' ? `v${value}` : 'n/a';
}

export function sanitizeSnapshotDetail(value: string | undefined): string | undefined {
    if (!value) {
        return undefined;
    }

    const compact = value.replace(/[\r\n\t]+/g, ' ').replace(/\s+/g, ' ').trim();
    if (compact.length <= SNAPSHOT_TEXT_DETAIL_LIMIT) {
        return compact;
    }

    return compact.slice(0, SNAPSHOT_TEXT_DETAIL_LIMIT);
}

function formatArtifactStatus(name: string, artifact: SnapshotArtifactStatus | undefined): string | undefined {
    if (!artifact || typeof artifact !== 'object') {
        return undefined;
    }

    const parts = [`${name}=${artifact.state}`];
    if (typeof artifact.version === 'number') {
        parts.push(`version=${formatRevision(artifact.version)}`);
    }
    const detail = sanitizeSnapshotDetail(artifact.detail);
    if (detail) {
        parts.push(`detail=${detail}`);
    }
    return parts.join(' ');
}

function pushSnapshotStatusHistory(cacheKey: string, status: SnapshotStatusResponse): void {
    const history = snapshotStatusHistory.get(cacheKey) ?? [];
    const last = history[history.length - 1];
    if (
        last
        && last.updatedAtMs === status.updatedAtMs
        && last.state === status.state
        && last.taskState === status.taskState
    ) {
        return;
    }

    history.push(status);
    if (history.length > SNAPSHOT_TRANSITION_HISTORY_LIMIT) {
        history.splice(0, history.length - SNAPSHOT_TRANSITION_HISTORY_LIMIT);
    }
    snapshotStatusHistory.set(cacheKey, history);
}

function buildSnapshotDiagnosticLines(status: SnapshotStatusResponse): string[] {
    const lines: string[] = [];
    if (status.reason) {
        lines.push(`reason=${sanitizeSnapshotDetail(status.reason.code) ?? status.reason.code}`);
        const message = sanitizeSnapshotDetail(status.reason.message);
        if (message) {
            lines.push(`reasonMessage=${message}`);
        }
    }
    if (status.worker) {
        const workerParts = ['worker'];
        if (typeof status.worker.targetVersion === 'number') {
            workerParts.push(`target=${formatRevision(status.worker.targetVersion)}`);
        }
        if (status.worker.phase) {
            workerParts.push(`phase=${status.worker.phase}`);
        }
        if (status.worker.trigger) {
            workerParts.push(`trigger=${status.worker.trigger}`);
        }
        if (typeof status.worker.ageMs === 'number') {
            workerParts.push(`ageMs=${status.worker.ageMs}`);
        }
        lines.push(workerParts.join(' '));
    }
    if (status.artifacts) {
        const artifacts = [
            formatArtifactStatus('shadow', status.artifacts.shadowState),
            formatArtifactStatus('readyParse', status.artifacts.readyParseSnapshot),
            formatArtifactStatus('exactIndex', status.artifacts.exactTypeIndex),
            formatArtifactStatus('completionHead', status.artifacts.completionHead),
        ].filter((value): value is string => !!value);
        lines.push(...artifacts);
    }
    if (status.lastFailure) {
        const failureParts = [
            `lastFailure=${status.lastFailure.stage}`,
            `reason=${sanitizeSnapshotDetail(status.lastFailure.reason) ?? 'unknown'}`,
        ];
        if (typeof status.lastFailure.requestedVersion === 'number') {
            failureParts.push(`requested=${formatRevision(status.lastFailure.requestedVersion)}`);
        }
        const message = sanitizeSnapshotDetail(status.lastFailure.message);
        if (message) {
            failureParts.push(`message=${message}`);
        }
        lines.push(failureParts.join(' '));
    }
    if (status.recommendation) {
        lines.push(`recommendation=${status.recommendation}`);
    }
    return lines;
}

export function formatSnapshotStatusLogLine(status: SnapshotStatusResponse): string {
    const parts = [
        `[SnapshotStatus] uri=${status.uri ?? '<none>'}`,
        `state=${status.state}`,
        `requested=${formatRevision(status.requestedVersion)}`,
        `ready=${formatRevision(status.readyVersion)}`,
        `exact=${status.exact}`,
        `task=${status.taskState}`,
    ];

    if (status.phase) {
        parts.push(`phase=${status.phase}`);
    }
    if (status.trigger) {
        parts.push(`trigger=${status.trigger}`);
    }
    if (status.fallbackReason) {
        parts.push(`fallback=${sanitizeSnapshotDetail(status.fallbackReason) ?? status.fallbackReason}`);
    }
    parts.push(...buildSnapshotDiagnosticLines(status));

    parts.push(`updatedAtMs=${status.updatedAtMs}`);
    return parts.join(' ');
}

function renderStatusBar(status: SnapshotStatusResponse): void {
    if (!statusBarItem) {
        return;
    }

    const requested = formatRevision(status.requestedVersion);
    const ready = formatRevision(status.readyVersion);
    const detailLines = [
        `state=${status.state}`,
        `exact=${status.exact}`,
        `task=${status.taskState}`,
    ];
    if (status.phase) {
        detailLines.push(`phase=${status.phase}`);
    }
    if (status.trigger) {
        detailLines.push(`trigger=${status.trigger}`);
    }
    if (status.fallbackReason) {
        detailLines.push(`fallback=${sanitizeSnapshotDetail(status.fallbackReason) ?? status.fallbackReason}`);
    }
    detailLines.push(...buildSnapshotDiagnosticLines(status));

    switch (status.state) {
        case 'building':
            statusBarItem.text = `$(sync~spin) BSL Snap: building ${requested}`;
            break;
        case 'ready':
            statusBarItem.text = `$(check) BSL Snap: ready ${requested}`;
            break;
        case 'shadow_only':
            statusBarItem.text = `$(warning) BSL Snap: shadow-only ${requested}`;
            break;
        case 'stale':
            statusBarItem.text = `$(history) BSL Snap: stale ${ready}/${requested}`;
            break;
        case 'failed':
            statusBarItem.text = `$(error) BSL Snap: failed ${requested}`;
            break;
        default:
            statusBarItem.text = `$(circle-slash) BSL Snap: idle ${requested}`;
            break;
    }

    statusBarItem.tooltip = [
        'Live snapshot readiness',
        `requested=${requested}`,
        `ready=${ready}`,
        ...detailLines,
    ].join('\n');
    statusBarItem.show();
}

function renderCurrentSnapshotStatus(): void {
    if (!statusBarItem) {
        return;
    }

    const snapshot = getActiveSnapshotStatusSnapshot();
    switch (snapshot.kind) {
        case 'inactive':
            statusBarItem.hide();
            break;
        case 'unsupported':
            statusBarItem.text = '$(warning) BSL Snap: unsupported';
            statusBarItem.tooltip = 'Live snapshot readiness\nunsupported by current server';
            statusBarItem.show();
            break;
        case 'unavailable':
            statusBarItem.text = '$(warning) BSL Snap: unavailable';
            statusBarItem.tooltip = `Live snapshot readiness\n${snapshot.message}`;
            statusBarItem.show();
            break;
        case 'ok':
            renderStatusBar(snapshot.status);
            break;
    }
}

function applySnapshotStatusUpdate(status: SnapshotStatusResponse): void {
    const cacheKey = status.uri ?? activeUri;
    if (!cacheKey) {
        return;
    }

    const previous = snapshotStatusCache.get(cacheKey);
    if (previous && previous.updatedAtMs > status.updatedAtMs) {
        return;
    }

    snapshotStatusUnsupported = false;
    activeUnavailableMessage = null;
    snapshotStatusCache.set(cacheKey, status);
    pushSnapshotStatusHistory(cacheKey, status);
    if (activeUri === cacheKey) {
        renderCurrentSnapshotStatus();
    }
    fireSnapshotStatusChange();
}

async function hydrateActiveEditorSnapshotStatus(editor?: vscode.TextEditor): Promise<void> {
    if (!isBslEditor(editor)) {
        activeUri = null;
        activeUnavailableMessage = null;
        renderCurrentSnapshotStatus();
        fireSnapshotStatusChange();
        return;
    }

    const uri = editor.document.uri.toString();
    activeUri = uri;
    activeUnavailableMessage = null;

    renderCurrentSnapshotStatus();
    fireSnapshotStatusChange();
    await refreshSnapshotStatusForUri(uri);
}

export async function refreshSnapshotStatusForUri(
    uri: string
): Promise<SnapshotStatusResponse | undefined> {
    const result = await getSnapshotStatusFetchResult({ uri });
    if (activeUri !== uri) {
        if (result.kind === 'ok') {
            applySnapshotStatusUpdate(result.response);
            return result.response;
        }
        return undefined;
    }

    switch (result.kind) {
        case 'ok':
            applySnapshotStatusUpdate(result.response);
            return result.response;
        case 'unsupported':
            snapshotStatusUnsupported = true;
            activeUnavailableMessage = null;
            renderCurrentSnapshotStatus();
            fireSnapshotStatusChange();
            return undefined;
        case 'error':
            activeUnavailableMessage = result.message;
            renderCurrentSnapshotStatus();
            fireSnapshotStatusChange();
            return undefined;
    }
}

export function initializeSnapshotStatus(
    channel: vscode.OutputChannel,
    statusBar: vscode.StatusBarItem
): vscode.Disposable {
    outputChannel = channel;
    statusBarItem = statusBar;
    statusBarItem.command = 'bslAnalyzer.openSnapshotReadiness';

    if (!initialized) {
        initialized = true;
        subscriptions.push(
            vscode.window.onDidChangeActiveTextEditor((editor) => {
                void hydrateActiveEditorSnapshotStatus(editor);
            }),
            vscode.workspace.onDidCloseTextDocument((document) => {
                if (activeUri === document.uri.toString()) {
                    activeUri = null;
                    activeUnavailableMessage = null;
                    snapshotStatusHistory.delete(document.uri.toString());
                    snapshotStatusCache.delete(document.uri.toString());
                    renderCurrentSnapshotStatus();
                    fireSnapshotStatusChange();
                }
            })
        );
    }

    void hydrateActiveEditorSnapshotStatus(vscode.window.activeTextEditor);

    return new vscode.Disposable(() => {
        for (const disposable of subscriptions.splice(0, subscriptions.length)) {
            disposable.dispose();
        }
        initialized = false;
        outputChannel = undefined;
        statusBarItem = undefined;
        activeUri = null;
        snapshotStatusUnsupported = false;
        activeUnavailableMessage = null;
        snapshotStatusCache.clear();
        snapshotStatusHistory.clear();
    });
}

export function handleSnapshotStatusNotification(status: SnapshotStatusResponse): void {
    outputChannel?.appendLine(formatSnapshotStatusLogLine(status));
    applySnapshotStatusUpdate(status);
}

export async function refreshSnapshotStatus(): Promise<void> {
    await hydrateActiveEditorSnapshotStatus(vscode.window.activeTextEditor);
}

export function onSnapshotStatusChange(listener: () => void): vscode.Disposable {
    statusListeners.add(listener);
    return new vscode.Disposable(() => {
        statusListeners.delete(listener);
    });
}

export function getActiveSnapshotStatusSnapshot(): ActiveSnapshotStatusView {
    if (!activeUri) {
        return { kind: 'inactive' };
    }
    if (snapshotStatusUnsupported) {
        return { kind: 'unsupported' };
    }
    const status = snapshotStatusCache.get(activeUri);
    if (status) {
        return { kind: 'ok', status };
    }
    if (activeUnavailableMessage) {
        return { kind: 'unavailable', message: activeUnavailableMessage };
    }
    return { kind: 'unavailable', message: 'snapshot status is not available yet' };
}

export function getSnapshotStatusForUri(uri: string): SnapshotStatusResponse | undefined {
    return snapshotStatusCache.get(uri);
}

export function getSnapshotStatusHistoryForUri(uri: string): SnapshotStatusResponse[] {
    return [...(snapshotStatusHistory.get(uri) ?? [])];
}

export function getActiveSnapshotStatusHistory(): SnapshotStatusResponse[] {
    return activeUri ? getSnapshotStatusHistoryForUri(activeUri) : [];
}

export function resetSnapshotStatusForTests(): void {
    for (const disposable of subscriptions.splice(0, subscriptions.length)) {
        disposable.dispose();
    }
    outputChannel = undefined;
    statusBarItem = undefined;
    activeUri = null;
    snapshotStatusUnsupported = false;
    activeUnavailableMessage = null;
    snapshotStatusCache.clear();
    snapshotStatusHistory.clear();
    statusListeners.clear();
    initialized = false;
}
