import * as assert from 'assert';
import * as vscode from 'vscode';
import * as sinon from 'sinon';

import { registerEnhancedProviders } from '../../setup/providers';
import { EnhancedDiagnosticsProvider } from '../../providers/enhancedDiagnosticsProvider';

suite('Enhanced Providers (no stubs) Test Suite', () => {
    teardown(() => {
        sinon.restore();
    });

    test('registerEnhancedProviders should not register stub VSCode providers', async () => {
        const registerInlayStub = sinon.stub(vscode.languages, 'registerInlayHintsProvider');
        const registerCodeActionsStub = sinon.stub(vscode.languages, 'registerCodeActionsProvider');

        const context = { subscriptions: [] } as unknown as vscode.ExtensionContext;
        const outputChannel = {
            appendLine: sinon.stub(),
            show: sinon.stub(),
            dispose: sinon.stub(),
        } as unknown as vscode.OutputChannel;

        await registerEnhancedProviders(context, {} as any, outputChannel);

        assert.strictEqual(registerInlayStub.called, false);
        assert.strictEqual(registerCodeActionsStub.called, false);
    });

    test('EnhancedDiagnosticsProvider.getDiagnosticsStats should count BSL diagnostics', () => {
        const getDiagnosticsStub = sinon.stub(vscode.languages, 'getDiagnostics');

        const diagRange = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 1));
        const error = new vscode.Diagnostic(diagRange, 'E', vscode.DiagnosticSeverity.Error);
        const warning = new vscode.Diagnostic(diagRange, 'W', vscode.DiagnosticSeverity.Warning);
        const info = new vscode.Diagnostic(diagRange, 'I', vscode.DiagnosticSeverity.Information);
        const hint = new vscode.Diagnostic(diagRange, 'H', vscode.DiagnosticSeverity.Hint);

        getDiagnosticsStub.returns([
            [vscode.Uri.file('/tmp/test.bsl'), [error, warning]],
            [vscode.Uri.file('/tmp/test.os'), [info, hint]],
            [vscode.Uri.file('/tmp/ignore.txt'), [error]],
        ] as any);

        const outputChannel = {
            appendLine: sinon.stub(),
            show: sinon.stub(),
            dispose: sinon.stub(),
        } as unknown as vscode.OutputChannel;

        const provider = new EnhancedDiagnosticsProvider({} as any, outputChannel);
        const stats = provider.getDiagnosticsStats();

        assert.deepStrictEqual(stats, {
            errors: 1,
            warnings: 1,
            infos: 1,
            hints: 1,
        });
    });
});

