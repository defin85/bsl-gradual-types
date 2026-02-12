import * as assert from 'assert';
import * as fs from 'fs';
import * as path from 'path';
import * as vscode from 'vscode';

const EXTENSION_ID = 'bsl-gradual-types-team.bsl-gradual-types';

suite('Sidebar Consistency Test Suite', () => {
    let extensionPath: string;

    suiteSetup(async function() {
        this.timeout(10000);
        const extension = vscode.extensions.getExtension(EXTENSION_ID);
        assert.ok(extension, `Extension ${EXTENSION_ID} should be installed`);
        if (!extension) {
            throw new Error(`Extension ${EXTENSION_ID} not found`);
        }
        if (!extension.isActive) {
            await extension.activate();
        }
        extensionPath = extension.extensionPath;
    });

    test('package.json should define a single BSL Analyzer activity bar container', () => {
        const packageJsonPath = path.join(extensionPath, 'package.json');
        const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
        const activitybar = packageJson?.contributes?.viewsContainers?.activitybar ?? [];
        const ids = activitybar.map((item: { id: string }) => item.id);

        assert.ok(ids.includes('bslAnalyzer'), 'bslAnalyzer container should exist');
        assert.ok(!ids.includes('bslAnalyzerCache'), 'legacy bslAnalyzerCache container should not exist');
        assert.strictEqual(ids.filter((id: string) => id.startsWith('bslAnalyzer')).length, 1);
    });

    test('package.json should keep all sidebar views under bslAnalyzer', () => {
        const packageJsonPath = path.join(extensionPath, 'package.json');
        const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
        const viewEntries = packageJson?.contributes?.views?.bslAnalyzer ?? [];
        const viewIds = new Set(viewEntries.map((item: { id: string }) => item.id));

        const required = [
            'bslAnalyzer.overview',
            'bslAnalyzer.diagnostics',
            'bslAnalyzer.typeRepository',
            'bslAnalyzer.actions',
            'bslAnalyzer.cacheDashboard',
            'bslAnalyzer.observability',
        ];

        for (const id of required) {
            assert.ok(viewIds.has(id), `Sidebar view ${id} should exist in bslAnalyzer container`);
        }
    });

    test('refresh command wiring should use refreshTypeRepository only', async () => {
        const commands = await vscode.commands.getCommands();
        assert.ok(commands.includes('bslAnalyzer.refreshTypeRepository'));
        assert.ok(!commands.includes('bslAnalyzer.refreshTypeIndex'));
    });

    test('diagnostics navigation command should be registered', async () => {
        const commands = await vscode.commands.getCommands();
        assert.ok(commands.includes('bslAnalyzer.goToDiagnostic'));
    });

    test('overview and quick actions should not contain hardcoded/raw token labels', () => {
        const overviewProviderPath = path.join(extensionPath, 'src', 'providers', 'overviewProvider.ts');
        const quickActionsPanelPath = path.resolve(extensionPath, '..', 'frontend', 'src', 'vscode', 'quick_actions_panel.rs');

        const overviewSource = fs.readFileSync(overviewProviderPath, 'utf8');
        const quickActionsSource = fs.readFileSync(quickActionsPanelPath, 'utf8');

        assert.ok(!overviewSource.includes('$(check) Status'));
        assert.ok(!overviewSource.includes('$(error) Status'));
        assert.ok(!overviewSource.includes('$(loading~spin)'));
        assert.ok(!quickActionsSource.includes('3927 типов'));
    });

    test('providers should use unified sidebar snapshot contract', () => {
        const overviewProviderPath = path.join(extensionPath, 'src', 'providers', 'overviewProvider.ts');
        const diagnosticsProviderPath = path.join(extensionPath, 'src', 'providers', 'diagnosticsProvider.ts');
        const typeRepositoryProviderPath = path.join(extensionPath, 'src', 'providers', 'hierarchicalTypeProvider.ts');
        const cacheProviderPath = path.join(extensionPath, 'src', 'providers', 'cacheDashboardProvider.ts');
        const actionsProviderPath = path.join(extensionPath, 'src', 'providers', 'actionsWebview.ts');

        const overviewSource = fs.readFileSync(overviewProviderPath, 'utf8');
        const diagnosticsSource = fs.readFileSync(diagnosticsProviderPath, 'utf8');
        const typeRepositorySource = fs.readFileSync(typeRepositoryProviderPath, 'utf8');
        const cacheSource = fs.readFileSync(cacheProviderPath, 'utf8');
        const actionsSource = fs.readFileSync(actionsProviderPath, 'utf8');

        assert.ok(overviewSource.includes('getSidebarSnapshot'));
        assert.ok(typeRepositorySource.includes('getSidebarSnapshot'));
        assert.ok(actionsSource.includes('getSidebarSnapshot'));

        assert.ok(diagnosticsSource.includes('getSidebarSnapshot'));
        assert.ok(!diagnosticsSource.includes('collectBslDiagnosticsSnapshot'));

        assert.ok(cacheSource.includes('getSidebarSnapshot'));
        assert.ok(!cacheSource.includes('getCacheStats('));
    });
});
