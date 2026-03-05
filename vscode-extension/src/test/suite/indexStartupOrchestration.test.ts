import * as assert from 'assert';
import {
    decideStartupIndexAction,
    isAttachedBuildIndexResponse,
} from '../../indexStartupOrchestration';
import {
    GetIndexStateResponse,
    isMethodNotFoundError,
} from '../../lsp/customRequests';

function makeState(
    overrides: Partial<GetIndexStateResponse>
): GetIndexStateResponse {
    return {
        version: 1,
        state: 'idle',
        ready: false,
        updated_at_ms: 1,
        ...overrides,
    };
}

suite('Index Startup Orchestration Test Suite', () => {
    test('decideStartupIndexAction: ready => skip', () => {
        const decision = decideStartupIndexAction(
            makeState({ state: 'ready', ready: true })
        );
        assert.strictEqual(decision.action, 'skip');
        assert.strictEqual(decision.reason, 'ready');
    });

    test('decideStartupIndexAction: running => attach', () => {
        const decision = decideStartupIndexAction(
            makeState({
                state: 'running',
                active_operation: 'startup',
                operation_id: 'startup-1',
            })
        );
        assert.strictEqual(decision.action, 'attach');
        assert.strictEqual(decision.reason, 'running');
    });

    test('decideStartupIndexAction: failed => build', () => {
        const decision = decideStartupIndexAction(
            makeState({ state: 'failed' })
        );
        assert.strictEqual(decision.action, 'build');
        assert.strictEqual(decision.reason, 'failed');
    });

    test('decideStartupIndexAction: idle => build', () => {
        const decision = decideStartupIndexAction(
            makeState({ state: 'idle' })
        );
        assert.strictEqual(decision.action, 'build');
        assert.strictEqual(decision.reason, 'idle');
    });

    test('isAttachedBuildIndexResponse detects attached server response', () => {
        assert.ok(
            isAttachedBuildIndexResponse({
                success: true,
                types_count: 0,
                message: 'already running (attached): active_operation=startup',
            })
        );
        assert.ok(
            !isAttachedBuildIndexResponse({
                success: true,
                types_count: 42,
                message: 'Index build completed',
            })
        );
    });

    test('isMethodNotFoundError handles legacy method-not-found responses', () => {
        assert.ok(isMethodNotFoundError({ code: -32601 }));
        assert.ok(isMethodNotFoundError({ error: { code: -32601 } }));
        assert.ok(
            isMethodNotFoundError(
                new Error('Request failed: Method not found (-32601)')
            )
        );
        assert.ok(!isMethodNotFoundError(new Error('Internal error')));
    });
});
