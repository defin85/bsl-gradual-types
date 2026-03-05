import * as assert from 'assert';
import {
    decideStartupIndexAction,
    orchestrateStartupIndex,
    isAttachedBuildIndexResponse,
} from '../../indexStartupOrchestration';
import {
    BuildIndexResponse,
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
        active_operation: null,
        operation_id: null,
        message: null,
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

    test('orchestrateStartupIndex: ready state skips build', async () => {
        let buildCalls = 0;
        const logs: string[] = [];

        const outcome = await orchestrateStartupIndex({
            autoIndexBuild: true,
            configPath: '/tmp/cfg',
            platformVersion: '8.3.25',
            platformDocsArchive: '/tmp/docs',
            workspacePath: '/tmp/ws',
            getIndexState: async () => makeState({ state: 'ready', ready: true }),
            buildIndex: async () => {
                buildCalls += 1;
                return {
                    success: true,
                    types_count: 0,
                    message: 'should not be called',
                };
            },
            isMethodNotFoundError,
            log: (line) => logs.push(line),
            setStatus: () => {},
            showWarning: () => {},
        });

        assert.strictEqual(outcome.kind, 'ready-skip');
        assert.strictEqual(buildCalls, 0);
        assert.ok(logs.some((line) => line.includes('startup build skipped')));
    });

    test('orchestrateStartupIndex: running state attaches without build', async () => {
        let buildCalls = 0;
        const statuses: string[] = [];

        const outcome = await orchestrateStartupIndex({
            autoIndexBuild: true,
            configPath: '/tmp/cfg',
            platformVersion: '8.3.25',
            platformDocsArchive: '/tmp/docs',
            workspacePath: '/tmp/ws',
            getIndexState: async () =>
                makeState({
                    state: 'running',
                    active_operation: 'startup',
                    operation_id: 'startup-1',
                }),
            buildIndex: async () => {
                buildCalls += 1;
                return {
                    success: true,
                    types_count: 0,
                    message: 'should not be called',
                };
            },
            isMethodNotFoundError,
            log: () => {},
            setStatus: (line) => statuses.push(line),
            showWarning: () => {},
        });

        assert.strictEqual(outcome.kind, 'running-attach');
        assert.strictEqual(buildCalls, 0);
        assert.ok(
            statuses.includes('$(sync~spin) BSL: Index already running'),
            `statuses=${JSON.stringify(statuses)}`
        );
    });

    test('orchestrateStartupIndex: failed state triggers exactly one build', async () => {
        let buildCalls = 0;

        const outcome = await orchestrateStartupIndex({
            autoIndexBuild: true,
            configPath: '/tmp/cfg',
            platformVersion: '8.3.25',
            platformDocsArchive: '/tmp/docs',
            workspacePath: '/tmp/ws',
            getIndexState: async () => makeState({ state: 'failed' }),
            buildIndex: async (): Promise<BuildIndexResponse> => {
                buildCalls += 1;
                return {
                    success: true,
                    types_count: 12,
                    message: 'Index build completed',
                };
            },
            isMethodNotFoundError,
            log: () => {},
            setStatus: () => {},
            showWarning: () => {},
        });

        assert.strictEqual(outcome.kind, 'build-success');
        assert.strictEqual(buildCalls, 1);
    });

    test('orchestrateStartupIndex: idle state triggers exactly one build', async () => {
        let buildCalls = 0;

        const outcome = await orchestrateStartupIndex({
            autoIndexBuild: true,
            configPath: '/tmp/cfg',
            platformVersion: '8.3.25',
            platformDocsArchive: '/tmp/docs',
            workspacePath: '/tmp/ws',
            getIndexState: async () => makeState({ state: 'idle' }),
            buildIndex: async (): Promise<BuildIndexResponse> => {
                buildCalls += 1;
                return {
                    success: true,
                    types_count: 9,
                    message: 'Index build completed',
                };
            },
            isMethodNotFoundError,
            log: () => {},
            setStatus: () => {},
            showWarning: () => {},
        });

        assert.strictEqual(outcome.kind, 'build-success');
        assert.strictEqual(buildCalls, 1);
    });

    test('orchestrateStartupIndex: legacy Method not found is fail-closed', async () => {
        let buildCalls = 0;
        let warningCalls = 0;

        const outcome = await orchestrateStartupIndex({
            autoIndexBuild: true,
            configPath: '/tmp/cfg',
            platformVersion: '8.3.25',
            platformDocsArchive: '/tmp/docs',
            workspacePath: '/tmp/ws',
            getIndexState: async () => {
                throw { code: -32601, message: 'Method not found' };
            },
            buildIndex: async () => {
                buildCalls += 1;
                return {
                    success: true,
                    types_count: 1,
                    message: 'unexpected build',
                };
            },
            isMethodNotFoundError,
            log: () => {},
            setStatus: () => {},
            showWarning: () => {
                warningCalls += 1;
            },
        });

        assert.strictEqual(outcome.kind, 'legacy-fail-closed');
        assert.strictEqual(buildCalls, 0);
        assert.strictEqual(warningCalls, 1);
    });
});
