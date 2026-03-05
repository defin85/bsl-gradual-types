import { BuildIndexResponse, GetIndexStateResponse } from './lsp/customRequests';

export const ATTACHED_BUILD_INDEX_MESSAGE = 'already running (attached)';

export type StartupIndexAction = 'skip' | 'attach' | 'build';

export interface StartupIndexDecision {
    action: StartupIndexAction;
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
