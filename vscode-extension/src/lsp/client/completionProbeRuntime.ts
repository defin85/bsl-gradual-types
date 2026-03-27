import * as vscode from 'vscode';
import { CompletionProbeRecorder } from '../../providers/completionProbeRecorder';

const COMPLETION_METHOD = 'textDocument/completion';
const COMPLETION_PROBE_ID_FIELD = 'bslProbeId';
const COMPLETION_PROBE_TRANSPORT_INSTRUMENTED = Symbol('completionProbeTransportInstrumented');

type SendRequestLike = {
    sendRequest: (...args: any[]) => Promise<unknown>;
    [COMPLETION_PROBE_TRANSPORT_INSTRUMENTED]?: boolean;
};

type TextEditorSelectionWindowLike = Pick<typeof vscode.window, 'onDidChangeTextEditorSelection'>;

export function instrumentCompletionProbeTransport(
    client: SendRequestLike,
    recorder: CompletionProbeRecorder,
    now: () => number = Date.now
): void {
    if (client[COMPLETION_PROBE_TRANSPORT_INSTRUMENTED]) {
        return;
    }

    const originalSendRequest = client.sendRequest.bind(client);
    client.sendRequest = async (type: unknown, ...params: unknown[]) => {
        const token = extractCancellationToken(params);
        const isCompletionRequest = resolveMethod(type) === COMPLETION_METHOD;
        const probeId = isCompletionRequest && token
            ? recorder.getProbeIdForToken(token)
            : undefined;
        const requestParams = isCompletionRequest && probeId
            ? injectCompletionProbeId(params, probeId)
            : params;

        if (isCompletionRequest && token) {
            recorder.recordCompletionLspRequestStarted(token, now());
        }

        try {
            const result = await originalSendRequest(type, ...requestParams);
            if (isCompletionRequest && token) {
                recorder.recordCompletionLspResponseReceived(token, now());
            }
            return result;
        } catch (error) {
            if (isCompletionRequest && token) {
                recorder.recordCompletionLspResponseReceived(token, now());
            }
            throw error;
        }
    };
    client[COMPLETION_PROBE_TRANSPORT_INSTRUMENTED] = true;
}

export function registerCompletionProbeSelectionObserver(
    recorder: CompletionProbeRecorder,
    windowLike: TextEditorSelectionWindowLike = vscode.window
): vscode.Disposable {
    return windowLike.onDidChangeTextEditorSelection((event) => {
        if (event.textEditor.document.languageId !== 'bsl') {
            return;
        }

        recorder.recordTextEditorSelectionChanged(event.textEditor);
    });
}

function resolveMethod(type: unknown): string | undefined {
    if (typeof type === 'string') {
        return type;
    }

    if (type && typeof type === 'object' && 'method' in type) {
        const method = (type as { method?: unknown }).method;
        return typeof method === 'string' ? method : undefined;
    }

    return undefined;
}

function extractCancellationToken(params: unknown[]): vscode.CancellationToken | undefined {
    const candidate = params.at(-1);
    if (
        candidate
        && typeof candidate === 'object'
        && 'isCancellationRequested' in candidate
        && 'onCancellationRequested' in candidate
    ) {
        return candidate as vscode.CancellationToken;
    }

    return undefined;
}

function injectCompletionProbeId(params: unknown[], probeId: string): unknown[] {
    const nextParams = [...params];
    const firstParam = nextParams[0];

    if (firstParam && typeof firstParam === 'object' && !Array.isArray(firstParam)) {
        nextParams[0] = {
            ...(firstParam as Record<string, unknown>),
            [COMPLETION_PROBE_ID_FIELD]: probeId,
        };
        return nextParams;
    }

    nextParams.unshift({ [COMPLETION_PROBE_ID_FIELD]: probeId });
    return nextParams;
}
