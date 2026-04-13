import * as vscode from 'vscode';

import {
    getSnapshotStatusFetchResult,
    SnapshotStatusResponse,
} from './customRequests';

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
const statusListeners = new Set<() => void>();
const subscriptions: vscode.Disposable[] = [];
let initialized = false;

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
        parts.push(`fallback=${status.fallbackReason}`);
    }

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
        detailLines.push(`fallback=${status.fallbackReason}`);
    }

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
        case 'unsupported':
            statusBarItem.hide();
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

    const cached = snapshotStatusCache.get(uri);
    if (cached) {
        renderCurrentSnapshotStatus();
        fireSnapshotStatusChange();
    }

    const result = await getSnapshotStatusFetchResult({ uri });
    if (activeUri !== uri) {
        return;
    }

    switch (result.kind) {
        case 'ok':
            applySnapshotStatusUpdate(result.response);
            return;
        case 'unsupported':
            snapshotStatusUnsupported = true;
            activeUnavailableMessage = null;
            renderCurrentSnapshotStatus();
            fireSnapshotStatusChange();
            return;
        case 'error':
            activeUnavailableMessage = result.message;
            renderCurrentSnapshotStatus();
            fireSnapshotStatusChange();
            return;
    }
}

export function initializeSnapshotStatus(
    channel: vscode.OutputChannel,
    statusBar: vscode.StatusBarItem
): vscode.Disposable {
    outputChannel = channel;
    statusBarItem = statusBar;

    if (!initialized) {
        initialized = true;
        subscriptions.push(
            vscode.window.onDidChangeActiveTextEditor((editor) => {
                void hydrateActiveEditorSnapshotStatus(editor);
            }),
            vscode.workspace.onDidCloseTextDocument((document) => {
                if (activeUri === document.uri.toString()) {
                    activeUnavailableMessage = null;
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
    statusListeners.clear();
    initialized = false;
}
