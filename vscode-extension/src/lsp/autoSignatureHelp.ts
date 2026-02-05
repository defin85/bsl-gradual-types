import * as vscode from 'vscode';

import { BslAnalyzerConfig } from '../config';
import { logger } from './logger';

const TRIGGER_COMMAND = 'editor.action.triggerParameterHints';

function isLikelyCallContextOnLine(lineText: string, character: number): boolean {
    if (character < 0) {
        return false;
    }

    const caret = Math.min(character, lineText.length);
    const openParen = lineText.lastIndexOf('(', caret);
    if (openParen < 0) {
        return false;
    }

    // If the call is already closed before the caret, ignore.
    const closeParen = lineText.indexOf(')', openParen + 1);
    if (closeParen >= 0 && caret > closeParen) {
        return false;
    }

    // Must look like a callee before '(' (identifier or property access).
    const before = lineText.slice(0, openParen).trimEnd();
    if (!/[0-9A-Za-zА-Яа-яЁё_\\.]+$/.test(before)) {
        return false;
    }

    return true;
}

function shouldAutoTriggerSignatureHelp(
    document: vscode.TextDocument,
    selection: vscode.Selection,
): boolean {
    if (document.languageId !== 'bsl') {
        return false;
    }

    // Only for single-cursor UX (multi-cursor would be noisy and ambiguous).
    // onDidChangeTextEditorSelection fires per editor, so we validate here.
    if (!selection.isSingleLine) {
        return false;
    }

    const lineText = document.lineAt(selection.active.line).text;

    const candidatePositions: number[] = [];
    if (selection.isEmpty) {
        candidatePositions.push(selection.active.character);
    } else {
        candidatePositions.push(selection.start.character);
        candidatePositions.push(selection.active.character);
    }

    for (const character of candidatePositions) {
        if (!isLikelyCallContextOnLine(lineText, character)) {
            continue;
        }

        const before = lineText.slice(0, Math.min(character, lineText.length));
        const beforeTrimmed = before.replace(/\s+$/, '');
        const prevNonWs = beforeTrimmed.at(-1);
        if (prevNonWs === '(' || prevNonWs === ',') {
            return true;
        }
    }

    return false;
}

export function initializeAutoSignatureHelpOnCursorMove(context: vscode.ExtensionContext) {
    let debounceTimer: NodeJS.Timeout | undefined;
    let lastTriggeredKey = '';
    let lastTriggeredAt = 0;

    const disposable = vscode.window.onDidChangeTextEditorSelection((e) => {
        if (!BslAnalyzerConfig.autoSignatureHelpOnCursorMove) {
            return;
        }

        if (e.selections.length !== 1) {
            return;
        }

        const editor = e.textEditor;
        const document = editor.document;
        const selection = e.selections[0];

        if (!shouldAutoTriggerSignatureHelp(document, selection)) {
            return;
        }

        const pos = selection.active;
        const key = `${document.uri.toString()}@${pos.line}:${pos.character}:${selection.isEmpty ? 0 : 1}`;
        const now = Date.now();
        if (key === lastTriggeredKey && now - lastTriggeredAt < 200) {
            return;
        }

        lastTriggeredKey = key;
        if (debounceTimer) {
            clearTimeout(debounceTimer);
        }

        // Small debounce to let snippet tab-jumps/selection settle.
        debounceTimer = setTimeout(() => {
            vscode.commands.executeCommand(TRIGGER_COMMAND).then(
                () => {
                    lastTriggeredAt = Date.now();
                },
                (err) => {
                    // Non-fatal: the command can be unavailable in some contexts.
                    logger.debug(`Auto signature help trigger failed: ${String(err)}`);
                },
            );
        }, 30);
    });

    context.subscriptions.push(disposable);
    context.subscriptions.push(
        new vscode.Disposable(() => {
            if (debounceTimer) {
                clearTimeout(debounceTimer);
            }
        }),
    );
}

