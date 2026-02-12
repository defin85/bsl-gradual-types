import * as assert from 'assert';
import * as vscode from 'vscode';
import { countDiagnosticsBySeverity } from '../../providers/sidebarSnapshot';

suite('Sidebar Snapshot Test Suite', () => {
    function makeDiagnostic(severity: vscode.DiagnosticSeverity, message: string): vscode.Diagnostic {
        return new vscode.Diagnostic(
            new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 1)),
            message,
            severity
        );
    }

    test('countDiagnosticsBySeverity should aggregate all severities', () => {
        const diagnostics: vscode.Diagnostic[] = [
            makeDiagnostic(vscode.DiagnosticSeverity.Error, 'e1'),
            makeDiagnostic(vscode.DiagnosticSeverity.Error, 'e2'),
            makeDiagnostic(vscode.DiagnosticSeverity.Warning, 'w1'),
            makeDiagnostic(vscode.DiagnosticSeverity.Information, 'i1'),
            makeDiagnostic(vscode.DiagnosticSeverity.Hint, 'h1'),
        ];

        const stats = countDiagnosticsBySeverity(diagnostics);
        assert.strictEqual(stats.total, 5);
        assert.strictEqual(stats.errors, 2);
        assert.strictEqual(stats.warnings, 1);
        assert.strictEqual(stats.infos, 1);
        assert.strictEqual(stats.hints, 1);
    });

    test('countDiagnosticsBySeverity should return zeros for empty list', () => {
        const stats = countDiagnosticsBySeverity([]);
        assert.strictEqual(stats.total, 0);
        assert.strictEqual(stats.errors, 0);
        assert.strictEqual(stats.warnings, 0);
        assert.strictEqual(stats.infos, 0);
        assert.strictEqual(stats.hints, 0);
    });
});
