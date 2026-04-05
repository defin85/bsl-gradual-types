import * as vscode from 'vscode';
import type { Message, MessageReader, MessageWriter } from 'vscode-jsonrpc/node';
import {
    LanguageClient,
    LanguageClientOptions,
    MessageTransports,
    ServerOptions,
} from 'vscode-languageclient/node';
import { CompletionProbeRecorder } from '../../providers/completionProbeRecorder';

const COMPLETION_METHOD = 'textDocument/completion';
const COMPLETION_PROBE_ID_FIELD = 'bslProbeId';
const COMPLETION_PROBE_TRANSPORT_INSTRUMENTED = Symbol('completionProbeTransportInstrumented');

type SendRequestLike = {
    sendRequest: (...args: any[]) => Promise<unknown>;
    completionProbeTransportTracker?: CompletionProbeTransportTracker;
    [COMPLETION_PROBE_TRANSPORT_INSTRUMENTED]?: boolean;
};

type TextEditorSelectionWindowLike = Pick<typeof vscode.window, 'onDidChangeTextEditorSelection'>;

export class CompletionProbeTransportTracker {
    private readonly completionProbeIdsByRequestId = new Map<string, string>();
    private readonly requestIdsByProbeId = new Map<string, string>();

    get size(): number {
        return this.completionProbeIdsByRequestId.size;
    }

    trackRequest(requestId: string, probeId: string): void {
        const previousProbeId = this.completionProbeIdsByRequestId.get(requestId);
        if (previousProbeId) {
            this.requestIdsByProbeId.delete(previousProbeId);
        }
        const previousRequestId = this.requestIdsByProbeId.get(probeId);
        if (previousRequestId) {
            this.completionProbeIdsByRequestId.delete(previousRequestId);
        }
        this.completionProbeIdsByRequestId.set(requestId, probeId);
        this.requestIdsByProbeId.set(probeId, requestId);
    }

    takeProbeIdForResponse(requestId: string): string | undefined {
        const probeId = this.completionProbeIdsByRequestId.get(requestId);
        if (!probeId) {
            return undefined;
        }
        this.completionProbeIdsByRequestId.delete(requestId);
        this.requestIdsByProbeId.delete(probeId);
        return probeId;
    }

    clearProbe(probeId: string): void {
        const requestId = this.requestIdsByProbeId.get(probeId);
        if (!requestId) {
            return;
        }
        this.requestIdsByProbeId.delete(probeId);
        this.completionProbeIdsByRequestId.delete(requestId);
    }

    clear(): void {
        this.completionProbeIdsByRequestId.clear();
        this.requestIdsByProbeId.clear();
    }
}

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
        const transportTracker = client.completionProbeTransportTracker;
        const requestParams = isCompletionRequest && probeId
            ? injectCompletionProbeId(params, probeId)
            : params;

        if (isCompletionRequest && token) {
            recorder.recordCompletionLspRequestStarted(token, now());
        }

        try {
            const result = await originalSendRequest(type, ...requestParams);
            if (isCompletionRequest && token) {
                recorder.recordCompletionLspResponseResolved(token, now());
            }
            return result;
        } catch (error) {
            if (isCompletionRequest && token) {
                recorder.recordCompletionLspResponseResolved(token, now());
            }
            throw error;
        } finally {
            if (isCompletionRequest && probeId) {
                transportTracker?.clearProbe(probeId);
            }
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

export class CompletionProbeLanguageClient extends LanguageClient {
    readonly completionProbeTransportTracker = new CompletionProbeTransportTracker();

    constructor(
        id: string,
        name: string,
        serverOptions: ServerOptions,
        clientOptions: LanguageClientOptions,
        private readonly completionProbeRecorder: CompletionProbeRecorder,
    ) {
        super(id, name, serverOptions, clientOptions);
    }

    protected async createMessageTransports(encoding: string): Promise<MessageTransports> {
        const transports = await super.createMessageTransports(encoding);
        return instrumentCompletionProbeMessageTransports(
            transports,
            this.completionProbeRecorder,
            Date.now,
            this.completionProbeTransportTracker,
        );
    }
}

export function instrumentCompletionProbeMessageTransports(
    transports: MessageTransports,
    recorder: CompletionProbeRecorder,
    now: () => number = Date.now,
    transportTracker: CompletionProbeTransportTracker = new CompletionProbeTransportTracker(),
): MessageTransports {
    return {
        ...transports,
        reader: new CompletionProbeMessageReader(
            transports.reader,
            transportTracker,
            recorder,
            now,
        ),
        writer: new CompletionProbeMessageWriter(
            transports.writer,
            transportTracker,
            recorder,
            now,
        ),
    };
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

function requestIdFromMessage(message: Message): string | undefined {
    if (!message || typeof message !== 'object' || !('id' in message)) {
        return undefined;
    }
    const id = (message as { id?: unknown }).id;
    if (typeof id === 'number' || typeof id === 'string') {
        return String(id);
    }
    return undefined;
}

function completionProbeIdFromRequestMessage(message: Message): string | undefined {
    if (!message || typeof message !== 'object' || !('method' in message) || !('params' in message)) {
        return undefined;
    }
    const method = (message as { method?: unknown }).method;
    if (method !== COMPLETION_METHOD) {
        return undefined;
    }
    const params = (message as { params?: unknown }).params;
    if (!params || typeof params !== 'object' || Array.isArray(params)) {
        return undefined;
    }
    const probeId = (params as Record<string, unknown>)[COMPLETION_PROBE_ID_FIELD];
    return typeof probeId === 'string' && probeId.length > 0 ? probeId : undefined;
}

class CompletionProbeMessageReader implements MessageReader {
    constructor(
        private readonly inner: MessageReader,
        private readonly transportTracker: CompletionProbeTransportTracker,
        private readonly recorder: CompletionProbeRecorder,
        private readonly now: () => number,
    ) {}

    get onError() {
        return this.inner.onError;
    }

    get onClose() {
        return this.inner.onClose;
    }

    get onPartialMessage() {
        return this.inner.onPartialMessage;
    }

    listen(callback: (data: Message) => void) {
        return this.inner.listen((message) => {
            const requestId = requestIdFromMessage(message);
            if (requestId) {
                const probeId = this.transportTracker.takeProbeIdForResponse(requestId);
                if (probeId) {
                    this.recorder.recordCompletionRawTransportResponseReceived(
                        probeId,
                        this.now(),
                    );
                }
            }
            callback(message);
        });
    }

    dispose(): void {
        this.transportTracker.clear();
        this.inner.dispose();
    }
}

class CompletionProbeMessageWriter implements MessageWriter {
    constructor(
        private readonly inner: MessageWriter,
        private readonly transportTracker: CompletionProbeTransportTracker,
        private readonly recorder: CompletionProbeRecorder,
        private readonly now: () => number,
    ) {}

    get onError() {
        return this.inner.onError;
    }

    get onClose() {
        return this.inner.onClose;
    }

    async write(message: Message): Promise<void> {
        const requestId = requestIdFromMessage(message);
        const probeId = completionProbeIdFromRequestMessage(message);
        await this.inner.write(message);
        if (probeId) {
            this.recorder.recordCompletionTransportRequestWritten(
                probeId,
                this.now(),
            );
        }
        if (requestId && probeId) {
            this.transportTracker.trackRequest(requestId, probeId);
        }
    }

    end(): void {
        this.transportTracker.clear();
        this.inner.end();
    }

    dispose(): void {
        this.transportTracker.clear();
        this.inner.dispose();
    }
}
