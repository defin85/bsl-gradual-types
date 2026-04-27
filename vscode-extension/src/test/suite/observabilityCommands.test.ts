import * as assert from 'assert';
import * as fs from 'fs/promises';
import * as os from 'os';
import * as path from 'path';
import * as sinon from 'sinon';
import * as vscode from 'vscode';
import { registerObservabilityCommands } from '../../commands/observability';
import * as customRequestsModule from '../../lsp/customRequests';
import {
    getSharedCompletionProbeRecorder,
    resetSharedCompletionProbeRecorderForTests,
} from '../../providers/completionProbeRecorder';
import {
    clearSharedCompletionTimelineExportCaptureForTests,
    setSharedCompletionTimelineExportCaptureForTests,
} from '../../providers/completionTimelineExportCapture';

suite('Observability Commands Test Suite', () => {
    const registeredCommands = new Map<string, (...args: unknown[]) => Promise<unknown> | unknown>();
    let tempRootDir: string | null = null;

    function safeRegisterCommand(
        commandId: string,
        callback: (...args: unknown[]) => Promise<unknown> | unknown
    ): Promise<vscode.Disposable | null> {
        registeredCommands.set(commandId, callback);
        return Promise.resolve({ dispose() {} });
    }

    setup(() => {
        registeredCommands.clear();
        resetSharedCompletionProbeRecorderForTests();
        getSharedCompletionProbeRecorder().clear();
        clearSharedCompletionTimelineExportCaptureForTests();
    });

    teardown(() => {
        clearSharedCompletionTimelineExportCaptureForTests();
        resetSharedCompletionProbeRecorderForTests();
        sinon.restore();
        const cleanup = tempRootDir ? fs.rm(tempRootDir, { recursive: true, force: true }) : Promise.resolve();
        tempRootDir = null;
        return cleanup;
    });

    test('exportObservabilityIncidentBundle should write bundle files via command callback', async () => {
        const outputChannel = {
            appendLine: sinon.stub(),
        } as unknown as vscode.OutputChannel;

        sinon.stub(customRequestsModule, 'getCompletionTimeline').resolves({
            kind: 'ok',
            response: {
                version: 8,
                traces: [],
            },
        });
        sinon.stub(customRequestsModule, 'getCurrentContextTimeline').resolves({
            kind: 'ok',
            response: {
                version: 1,
                traces: [],
            },
        });
        sinon.stub(customRequestsModule, 'getDiagnosticsSaveTimeline').resolves({
            kind: 'ok',
            response: {
                version: 7,
                traces: [],
            },
        });
        sinon.stub(customRequestsModule, 'getObservabilityMetricsFetchResult').resolves({
            kind: 'ok',
            response: {
                metrics: {
                    uptime_seconds: 42,
                },
            },
        });
        tempRootDir = await fs.mkdtemp(path.join(os.tmpdir(), 'bsl-incident-export-'));
        sinon.stub(vscode.window, 'showOpenDialog').resolves([vscode.Uri.file(tempRootDir)]);
        sinon.stub(vscode.window, 'showInformationMessage').resolves(undefined);

        registerObservabilityCommands(
            {} as vscode.ExtensionContext,
            safeRegisterCommand,
            outputChannel
        );

        const command = registeredCommands.get('bslAnalyzer.exportObservabilityIncidentBundle');
        assert.ok(command, 'export command should be registered');

        await command!();

        const bundleFolders = await fs.readdir(tempRootDir);
        assert.strictEqual(bundleFolders.length, 1, 'export should create exactly one bundle folder');
        const bundleRoot = path.join(tempRootDir, bundleFolders[0]);
        const incidentJson = await fs.readFile(path.join(bundleRoot, 'incident.json'), 'utf8');
        const incident = JSON.parse(incidentJson);
        const rawDirEntries = await fs.readdir(path.join(bundleRoot, 'raw'));
        assert.deepStrictEqual(
            rawDirEntries.sort(),
            [
                'client_probes.json',
                'completion_timeline.json',
                'current_context_timeline.json',
                'diagnostics_save_timeline.json',
                'observability_metrics.json',
            ]
        );
        assert.deepStrictEqual(incident.capture_scope, { kind: 'empty' });
        assert.strictEqual(incident.request_window.request_count, 0);
        assert.deepStrictEqual(incident.requests, []);
        assert.strictEqual(incident.sources.completion_timeline.status, 'available');
        assert.strictEqual(incident.sources.current_context_timeline.status, 'available');
        assert.strictEqual(incident.sources.diagnostics_save_timeline.status, 'available');
        assert.strictEqual(incident.sources.observability_metrics.status, 'available');
    });

    test('exportObservabilityIncidentBundle should include extension build identity when context is available', async () => {
        const outputChannel = {
            appendLine: sinon.stub(),
        } as unknown as vscode.OutputChannel;

        sinon.stub(customRequestsModule, 'getCompletionTimeline').resolves({
            kind: 'ok',
            response: {
                version: 8,
                traces: [],
            },
        });
        sinon.stub(customRequestsModule, 'getCurrentContextTimeline').resolves({
            kind: 'ok',
            response: {
                version: 1,
                traces: [],
            },
        });
        sinon.stub(customRequestsModule, 'getDiagnosticsSaveTimeline').resolves({
            kind: 'ok',
            response: {
                version: 7,
                traces: [],
            },
        });
        sinon.stub(customRequestsModule, 'getObservabilityMetricsFetchResult').resolves({
            kind: 'ok',
            response: {
                metrics: {
                    uptime_seconds: 42,
                },
            },
        });
        tempRootDir = await fs.mkdtemp(path.join(os.tmpdir(), 'bsl-incident-export-'));
        sinon.stub(vscode.window, 'showOpenDialog').resolves([vscode.Uri.file(tempRootDir)]);
        sinon.stub(vscode.window, 'showInformationMessage').resolves(undefined);

        registerObservabilityCommands(
            {
                extensionPath: '/tmp/fake-extension',
                extension: {
                    id: 'bsl-gradual-types-team.bsl-gradual-types',
                    packageJSON: {
                        displayName: 'BSL Gradual Type System',
                        version: '0.4.142',
                    },
                },
            } as unknown as vscode.ExtensionContext,
            safeRegisterCommand,
            outputChannel
        );

        const command = registeredCommands.get('bslAnalyzer.exportObservabilityIncidentBundle');
        assert.ok(command, 'export command should be registered');

        await command!();

        const bundleFolders = await fs.readdir(tempRootDir);
        assert.strictEqual(bundleFolders.length, 1, 'export should create exactly one bundle folder');
        const bundleRoot = path.join(tempRootDir, bundleFolders[0]);
        const incident = JSON.parse(await fs.readFile(path.join(bundleRoot, 'incident.json'), 'utf8'));
        const rawDirEntries = await fs.readdir(path.join(bundleRoot, 'raw'));
        assert.ok(
            rawDirEntries.includes('build_identity.json'),
            'build identity raw attachment must be exported when extension identity is available'
        );
        assert.strictEqual(incident.build_identity.extension.display_name, 'BSL Gradual Type System');
        assert.strictEqual(incident.build_identity.extension.version, '0.4.142');
        assert.strictEqual(
            incident.build_identity.extension.id,
            'bsl-gradual-types-team.bsl-gradual-types'
        );
    });

    test('exportObservabilityIncidentBundle should honor provided capture overrides without refetching timeline', async () => {
        const outputChannel = {
            appendLine: sinon.stub(),
        } as unknown as vscode.OutputChannel;

        const getCompletionTimelineStub = sinon
            .stub(customRequestsModule, 'getCompletionTimeline')
            .rejects(new Error('unexpected refetch'));
        const getCurrentContextTimelineStub = sinon
            .stub(customRequestsModule, 'getCurrentContextTimeline')
            .rejects(new Error('unexpected refetch'));
        const getDiagnosticsSaveTimelineStub = sinon
            .stub(customRequestsModule, 'getDiagnosticsSaveTimeline')
            .rejects(new Error('unexpected refetch'));
        const getMetricsStub = sinon
            .stub(customRequestsModule, 'getObservabilityMetricsFetchResult')
            .rejects(new Error('unexpected refetch'));
        tempRootDir = await fs.mkdtemp(path.join(os.tmpdir(), 'bsl-incident-export-'));
        sinon.stub(vscode.window, 'showOpenDialog').resolves([vscode.Uri.file(tempRootDir)]);
        sinon.stub(vscode.window, 'showInformationMessage').resolves(undefined);

        registerObservabilityCommands(
            {} as vscode.ExtensionContext,
            safeRegisterCommand,
            outputChannel
        );

        const command = registeredCommands.get('bslAnalyzer.exportObservabilityIncidentBundle');
        assert.ok(command, 'export command should be registered');

        await command!({
            capturedAtMs: Date.parse('2026-03-19T13:21:28.000Z'),
            completionTimeline: {
                kind: 'unsupported',
            },
            currentContextTimeline: {
                kind: 'unsupported',
            },
            diagnosticsSaveTimeline: {
                kind: 'unsupported',
            },
            clientProbes: [],
            observabilityMetrics: {
                kind: 'unsupported',
            },
        });

        assert.strictEqual(getCompletionTimelineStub.callCount, 0);
        assert.strictEqual(getCurrentContextTimelineStub.callCount, 0);
        assert.strictEqual(getDiagnosticsSaveTimelineStub.callCount, 0);
        assert.strictEqual(getMetricsStub.callCount, 0);

        const bundleFolders = await fs.readdir(tempRootDir);
        assert.strictEqual(bundleFolders.length, 1, 'export should create exactly one bundle folder');
        const bundleRoot = path.join(tempRootDir, bundleFolders[0]);
        const incident = JSON.parse(await fs.readFile(path.join(bundleRoot, 'incident.json'), 'utf8'));
        assert.deepStrictEqual(incident.capture_scope, { kind: 'unavailable' });
        assert.strictEqual(incident.request_window.request_count, 0);
        assert.deepStrictEqual(incident.requests, []);
        assert.strictEqual(incident.sources.completion_timeline.status, 'unsupported');
        assert.strictEqual(incident.sources.current_context_timeline.status, 'unsupported');
        assert.strictEqual(incident.sources.diagnostics_save_timeline.status, 'unsupported');
        assert.strictEqual(incident.sources.observability_metrics.status, 'unsupported');
    });

    test('exportObservabilityIncidentBundle should reuse shared webview capture before refetching timeline', async () => {
        const outputChannel = {
            appendLine: sinon.stub(),
        } as unknown as vscode.OutputChannel;

        const getCompletionTimelineStub = sinon
            .stub(customRequestsModule, 'getCompletionTimeline')
            .rejects(new Error('unexpected refetch'));
        const getCurrentContextTimelineStub = sinon
            .stub(customRequestsModule, 'getCurrentContextTimeline')
            .rejects(new Error('unexpected refetch'));
        const getDiagnosticsSaveTimelineStub = sinon
            .stub(customRequestsModule, 'getDiagnosticsSaveTimeline')
            .rejects(new Error('unexpected refetch'));
        const getMetricsStub = sinon
            .stub(customRequestsModule, 'getObservabilityMetricsFetchResult')
            .rejects(new Error('unexpected refetch'));
        tempRootDir = await fs.mkdtemp(path.join(os.tmpdir(), 'bsl-incident-export-'));
        sinon.stub(vscode.window, 'showOpenDialog').resolves([vscode.Uri.file(tempRootDir)]);
        sinon.stub(vscode.window, 'showInformationMessage').resolves(undefined);

        setSharedCompletionTimelineExportCaptureForTests({
            capturedAtMs: Date.parse('2026-03-27T18:01:00.000Z'),
            completionTimeline: {
                kind: 'ok',
                response: {
                    version: 20,
                    traces: [],
                },
            },
            currentContextTimeline: {
                kind: 'unsupported',
            },
            diagnosticsSaveTimeline: {
                kind: 'unsupported',
            },
            clientProbes: [],
            observabilityMetrics: {
                kind: 'unsupported',
            },
        });

        registerObservabilityCommands(
            {} as vscode.ExtensionContext,
            safeRegisterCommand,
            outputChannel
        );

        const command = registeredCommands.get('bslAnalyzer.exportObservabilityIncidentBundle');
        assert.ok(command, 'export command should be registered');

        await command!();

        assert.strictEqual(getCompletionTimelineStub.callCount, 0);
        assert.strictEqual(getCurrentContextTimelineStub.callCount, 0);
        assert.strictEqual(getDiagnosticsSaveTimelineStub.callCount, 0);
        assert.strictEqual(getMetricsStub.callCount, 0);

        const bundleFolders = await fs.readdir(tempRootDir);
        assert.strictEqual(bundleFolders.length, 1, 'export should create exactly one bundle folder');
        const bundleRoot = path.join(tempRootDir, bundleFolders[0]);
        const incident = JSON.parse(await fs.readFile(path.join(bundleRoot, 'incident.json'), 'utf8'));
        assert.strictEqual(incident.sources.completion_timeline.status, 'available');
        assert.strictEqual(incident.sources.current_context_timeline.status, 'unsupported');
        assert.strictEqual(incident.sources.diagnostics_save_timeline.status, 'unsupported');
        assert.strictEqual(incident.sources.observability_metrics.status, 'unsupported');
    });
});
