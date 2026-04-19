import * as vscode from 'vscode';

import { primeExactTypeIndex } from './customRequests';
import {
    getActiveSnapshotStatusSnapshot,
    onSnapshotStatusChange,
} from './snapshotStatus';

const PRIME_REASON = 'active_editor_cold_hover_warmup';
const PRIME_DEBOUNCE_MS = 50;

let primeTimer: NodeJS.Timeout | undefined;
const primedVersionsByUri = new Map<string, number>();

function isActiveBslEditor(
    editor: vscode.TextEditor | undefined
): editor is vscode.TextEditor & { document: vscode.TextDocument } {
    return !!editor && editor.document.languageId === 'bsl';
}

async function maybePrimeActiveEditorExactIndex(): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    if (!isActiveBslEditor(editor)) {
        return;
    }

    const snapshot = getActiveSnapshotStatusSnapshot();
    if (snapshot.kind !== 'ok') {
        return;
    }
    const status = snapshot.status;
    const uri = editor.document.uri.toString();
    if (status.uri !== uri) {
        return;
    }
    if (status.state !== 'ready' || status.taskState !== 'ready_same_revision') {
        return;
    }

    const requestedVersion = status.requestedVersion;
    if (typeof requestedVersion !== 'number') {
        return;
    }
    if (primedVersionsByUri.get(uri) === requestedVersion) {
        return;
    }

    primedVersionsByUri.set(uri, requestedVersion);
    const response = await primeExactTypeIndex({
        uri,
        requestedVersion,
        reason: PRIME_REASON,
    });
    if (!response?.accepted) {
        primedVersionsByUri.delete(uri);
    }
}

function schedulePrimeActiveEditorExactIndex(): void {
    if (primeTimer) {
        clearTimeout(primeTimer);
    }
    primeTimer = setTimeout(() => {
        void maybePrimeActiveEditorExactIndex();
    }, PRIME_DEBOUNCE_MS);
}

export function initializeExactIndexWarmup(): vscode.Disposable {
    schedulePrimeActiveEditorExactIndex();
    const snapshotDisposable = onSnapshotStatusChange(() => {
        schedulePrimeActiveEditorExactIndex();
    });
    const editorDisposable = vscode.window.onDidChangeActiveTextEditor(() => {
        schedulePrimeActiveEditorExactIndex();
    });
    const closeDisposable = vscode.workspace.onDidCloseTextDocument((document) => {
        primedVersionsByUri.delete(document.uri.toString());
    });

    return new vscode.Disposable(() => {
        if (primeTimer) {
            clearTimeout(primeTimer);
            primeTimer = undefined;
        }
        snapshotDisposable.dispose();
        editorDisposable.dispose();
        closeDisposable.dispose();
        primedVersionsByUri.clear();
    });
}

export function resetExactIndexWarmupForTests(): void {
    if (primeTimer) {
        clearTimeout(primeTimer);
        primeTimer = undefined;
    }
    primedVersionsByUri.clear();
}
