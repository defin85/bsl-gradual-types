import * as assert from 'assert';
import * as sinon from 'sinon';
import * as vscode from 'vscode';
import { buildServerOptions } from '../../lsp/client/server-options';

type FakeConfigValues = Record<string, unknown>;

function stubBslAnalyzerConfiguration(values: FakeConfigValues): sinon.SinonStub {
    return sinon.stub(vscode.workspace, 'getConfiguration').callsFake((section?: string) => {
        if (section !== 'bslAnalyzer') {
            throw new Error(`Unexpected configuration section: ${section}`);
        }
        return {
            get<T>(key: string, defaultValue?: T): T {
                return (values[key] as T | undefined) ?? (defaultValue as T);
            },
        } as vscode.WorkspaceConfiguration;
    });
}

suite('Server Options Test Suite', () => {
    teardown(() => {
        delete process.env.BSL_DEBUG_DIAGNOSTICS_SAVE_COHERENCE;
        sinon.restore();
    });

    test('stdio server options should inject diagnostics-save coherence debug env when enabled', () => {
        stubBslAnalyzerConfiguration({
            serverMode: 'stdio',
            serverTcpPort: 8080,
            slowClientLogMs: 2000,
            diagnosticsDebounceMs: 250,
            debugDiagnosticsSaveCoherence: true,
        });
        const outputChannel = {
            appendLine: sinon.stub(),
        } as unknown as vscode.OutputChannel;

        const serverOptions = buildServerOptions('/tmp/bsl-lsp-server', outputChannel) as {
            run: { options?: { env?: NodeJS.ProcessEnv } };
        };

        assert.strictEqual(
            serverOptions.run.options?.env?.BSL_DEBUG_DIAGNOSTICS_SAVE_COHERENCE,
            '1'
        );
        assert.ok(
            (outputChannel.appendLine as sinon.SinonStub).calledWith(
                'STDIO mode: BSL_DEBUG_DIAGNOSTICS_SAVE_COHERENCE=1'
            )
        );
    });

    test('stdio server options should clear inherited diagnostics-save coherence debug env when disabled', () => {
        process.env.BSL_DEBUG_DIAGNOSTICS_SAVE_COHERENCE = '1';
        stubBslAnalyzerConfiguration({
            serverMode: 'stdio',
            serverTcpPort: 8080,
            slowClientLogMs: 2000,
            diagnosticsDebounceMs: 250,
            debugDiagnosticsSaveCoherence: false,
        });
        const outputChannel = {
            appendLine: sinon.stub(),
        } as unknown as vscode.OutputChannel;

        const serverOptions = buildServerOptions('/tmp/bsl-lsp-server', outputChannel) as {
            run: { options?: { env?: NodeJS.ProcessEnv } };
        };

        assert.strictEqual(
            serverOptions.run.options?.env?.BSL_DEBUG_DIAGNOSTICS_SAVE_COHERENCE,
            undefined
        );
        assert.ok(
            !(outputChannel.appendLine as sinon.SinonStub).calledWith(
                'STDIO mode: BSL_DEBUG_DIAGNOSTICS_SAVE_COHERENCE=1'
            )
        );
    });
});
