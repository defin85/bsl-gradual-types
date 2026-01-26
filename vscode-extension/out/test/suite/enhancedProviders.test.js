"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
Object.defineProperty(exports, "__esModule", { value: true });
const assert = __importStar(require("assert"));
const vscode = __importStar(require("vscode"));
const sinon = __importStar(require("sinon"));
const providers_1 = require("../../setup/providers");
const enhancedDiagnosticsProvider_1 = require("../../providers/enhancedDiagnosticsProvider");
suite('Enhanced Providers (no stubs) Test Suite', () => {
    teardown(() => {
        sinon.restore();
    });
    test('registerEnhancedProviders should not register stub VSCode providers', async () => {
        const registerInlayStub = sinon.stub(vscode.languages, 'registerInlayHintsProvider');
        const registerCodeActionsStub = sinon.stub(vscode.languages, 'registerCodeActionsProvider');
        const context = { subscriptions: [] };
        const outputChannel = {
            appendLine: sinon.stub(),
            show: sinon.stub(),
            dispose: sinon.stub(),
        };
        await (0, providers_1.registerEnhancedProviders)(context, {}, outputChannel);
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
        ]);
        const outputChannel = {
            appendLine: sinon.stub(),
            show: sinon.stub(),
            dispose: sinon.stub(),
        };
        const provider = new enhancedDiagnosticsProvider_1.EnhancedDiagnosticsProvider({}, outputChannel);
        const stats = provider.getDiagnosticsStats();
        assert.deepStrictEqual(stats, {
            errors: 1,
            warnings: 1,
            infos: 1,
            hints: 1,
        });
    });
});
//# sourceMappingURL=enhancedProviders.test.js.map