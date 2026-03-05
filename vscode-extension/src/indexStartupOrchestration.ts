import {
    BuildIndexParams,
    BuildIndexResponse,
    GetIndexStateResponse,
} from './lsp/customRequests';

export const ATTACHED_BUILD_INDEX_MESSAGE = 'already running (attached)';

export type StartupIndexAction = 'skip' | 'attach' | 'build';

export interface StartupIndexDecision {
    action: StartupIndexAction;
    reason: string;
}

export interface StartupIndexOrchestrationDeps {
    autoIndexBuild: boolean;
    configPath: string;
    platformVersion: string;
    platformDocsArchive: string;
    workspacePath: string;
    getIndexState: () => Promise<GetIndexStateResponse>;
    buildIndex: (params: BuildIndexParams) => Promise<BuildIndexResponse>;
    isMethodNotFoundError: (error: unknown) => boolean;
    log: (message: string) => void;
    setStatus: (status: string) => void;
    showWarning: (message: string) => Promise<void> | void;
}

export type StartupIndexOutcomeKind =
    | 'disabled'
    | 'no-config'
    | 'legacy-fail-closed'
    | 'index-state-unavailable'
    | 'ready-skip'
    | 'running-attach'
    | 'no-platform-docs'
    | 'build-attached'
    | 'build-failed'
    | 'build-success';

export interface StartupIndexOutcome {
    kind: StartupIndexOutcomeKind;
    reason: string;
}

export function decideStartupIndexAction(
    state: GetIndexStateResponse
): StartupIndexDecision {
    switch (state.state) {
        case 'ready':
            return { action: 'skip', reason: 'ready' };
        case 'running':
            return { action: 'attach', reason: 'running' };
        case 'failed':
            return { action: 'build', reason: 'failed' };
        case 'idle':
        default:
            return { action: 'build', reason: 'idle' };
    }
}

export function isAttachedBuildIndexResponse(
    response: BuildIndexResponse
): boolean {
    return response.message
        .toLowerCase()
        .includes(ATTACHED_BUILD_INDEX_MESSAGE);
}

export async function orchestrateStartupIndex(
    deps: StartupIndexOrchestrationDeps
): Promise<StartupIndexOutcome> {
    if (!deps.autoIndexBuild) {
        deps.log('ℹ️ Auto-index build is disabled');
        return { kind: 'disabled', reason: 'auto-index disabled' };
    }

    if (!deps.configPath) {
        deps.log('⚠️ Configuration path not set - user must configure it');
        deps.setStatus('BSL Analyzer: No Config');
        return { kind: 'no-config', reason: 'configuration path missing' };
    }

    let indexState: GetIndexStateResponse;
    try {
        indexState = await deps.getIndexState();
    } catch (error) {
        if (deps.isMethodNotFoundError(error)) {
            deps.log(
                '⚠️ Legacy LSP: bsl/getIndexState is not supported; startup auto-index is fail-closed'
            );
            deps.setStatus('$(warning) BSL: Legacy LSP (manual Build Index)');
            await deps.showWarning(
                'BSL Analyzer: LSP server не поддерживает bsl/getIndexState. Авто-индексация на старте отключена (fail-closed), используйте Build Index вручную.'
            );
            return {
                kind: 'legacy-fail-closed',
                reason: 'getIndexState method not found',
            };
        }

        deps.log(`❌ Failed to query bsl/getIndexState: ${error}`);
        deps.setStatus('$(warning) BSL: Index state unavailable');
        return {
            kind: 'index-state-unavailable',
            reason: 'getIndexState request failed',
        };
    }

    const decision = decideStartupIndexAction(indexState);
    if (decision.action === 'skip') {
        deps.log('✅ LSP reports ready index state, startup build skipped');
        deps.setStatus('$(check) BSL: Index Ready');
        return { kind: 'ready-skip', reason: decision.reason };
    }

    if (decision.action === 'attach') {
        const operation = indexState.active_operation || 'unknown';
        const operationSuffix = indexState.operation_id
            ? `, operation_id=${indexState.operation_id}`
            : '';
        deps.log(
            `ℹ️ LSP reports running full-index (${operation}${operationSuffix}), startup build attached`
        );
        deps.setStatus('$(sync~spin) BSL: Index already running');
        return { kind: 'running-attach', reason: decision.reason };
    }

    if (!deps.platformDocsArchive) {
        deps.log('❌ Platform documentation not configured - cannot build full index');
        deps.log('💡 User must specify platform documentation archive in settings');
        deps.setStatus('BSL Analyzer: No Platform Docs');
        return { kind: 'no-platform-docs', reason: 'platform docs missing' };
    }

    deps.log(`🚀 Building BSL index (reason=${decision.reason})...`);
    deps.setStatus('$(sync~spin) BSL: Building index...');
    deps.log(`📁 Configuration: ${deps.configPath}`);
    deps.log(`📚 Platform docs: ${deps.platformDocsArchive}`);
    deps.log(`🔢 Platform version: ${deps.platformVersion}`);

    try {
        const response = await deps.buildIndex({ workspace_path: deps.workspacePath });
        if (isAttachedBuildIndexResponse(response)) {
            deps.log(`ℹ️ ${response.message}`);
            deps.setStatus('$(sync~spin) BSL: Index already running');
            return { kind: 'build-attached', reason: decision.reason };
        }

        if (!response.success) {
            deps.setStatus(`$(error) BSL: Index build failed: ${response.message}`);
            deps.log(`❌ Index build failed: ${response.message}`);
            return { kind: 'build-failed', reason: response.message };
        }

        deps.setStatus('$(check) BSL: Index Ready');
        deps.log(`✅ Index build completed successfully: ${response.message}`);
        return { kind: 'build-success', reason: decision.reason };
    } catch (error) {
        deps.setStatus(`$(error) BSL: Index build failed: ${error}`);
        deps.log(`❌ Index build failed: ${error}`);
        return { kind: 'build-failed', reason: String(error) };
    }
}
