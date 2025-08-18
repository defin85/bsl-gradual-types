import * as assert from 'assert';
import * as vscode from 'vscode';
import { parseMethodCall, extractTypeName } from '../../utils/parser';

suite('Extension Test Suite', () => {
    vscode.window.showInformationMessage('Start all tests.');

    test('Extension should be present', () => {
        assert.ok(vscode.extensions.getExtension('bsl-analyzer-team.bsl-type-safety-analyzer'));
    });

    test('Should activate extension', async () => {
        const ext = vscode.extensions.getExtension('bsl-analyzer-team.bsl-type-safety-analyzer');
        if (ext) {
            await ext.activate();
            assert.ok(ext.isActive);
        }
    });

    test('Should register all commands', () => {
        const commands = [
            'bslAnalyzer.analyzeFile',
            'bslAnalyzer.analyzeWorkspace',
            'bslAnalyzer.generateReports',
            'bslAnalyzer.showMetrics',
            'bslAnalyzer.configureRules',
            'bslAnalyzer.searchType',
            'bslAnalyzer.searchMethod',
            'bslAnalyzer.buildIndex',
            'bslAnalyzer.showIndexStats',
            'bslAnalyzer.incrementalUpdate',
            'bslAnalyzer.exploreType',
            'bslAnalyzer.validateMethodCall',
            'bslAnalyzer.checkTypeCompatibility',
            'bslAnalyzer.restartServer',
            'bslAnalyzer.refreshOverview',
            'bslAnalyzer.refreshDiagnostics',
            'bslAnalyzer.refreshTypeIndex',
            'bslAnalyzer.refreshPlatformDocs',
            'bslAnalyzer.addPlatformDocs',
            'bslAnalyzer.removePlatformDocs',
            'bslAnalyzer.parsePlatformDocs'
        ];

        return vscode.commands.getCommands(true).then((allCommands) => {
            const foundCommands = commands.filter(cmd => allCommands.includes(cmd));
            assert.strictEqual(foundCommands.length, commands.length, 
                `Missing commands: ${commands.filter(cmd => !foundCommands.includes(cmd)).join(', ')}`);
        });
    });
});

suite('Parser Test Suite', () => {
    test('parseMethodCall should extract method info', () => {
        const result = parseMethodCall('Справочники.Номенклатура.НайтиПоКоду("123")');
        assert.ok(result);
        assert.strictEqual(result?.objectName, 'Справочники.Номенклатура');
        assert.strictEqual(result?.methodName, 'НайтиПоКоду');
    });

    test('parseMethodCall should handle simple calls', () => {
        const result = parseMethodCall('Массив.Добавить(');
        assert.ok(result);
        assert.strictEqual(result?.objectName, 'Массив');
        assert.strictEqual(result?.methodName, 'Добавить');
    });

    test('parseMethodCall should return null for invalid input', () => {
        const result = parseMethodCall('НеМетод');
        assert.strictEqual(result, null);
    });

    test('extractTypeName should extract variable name', () => {
        const result = extractTypeName('Перем МояПеременная');
        assert.strictEqual(result, 'МояПеременная');
    });

    test('extractTypeName should extract variable name with Var', () => {
        const result = extractTypeName('Var МояПеременная');
        assert.strictEqual(result, 'МояПеременная');
    });

    test('extractTypeName should extract from assignment', () => {
        const result = extractTypeName('Результат = НовыйМассив()');
        assert.strictEqual(result, 'Результат');
    });

    test('extractTypeName should return first word as fallback', () => {
        const result = extractTypeName('СложныйТекст без паттернов');
        assert.strictEqual(result, 'СложныйТекст');
    });
});

suite('Configuration Test Suite', () => {
    test('Should have default configuration values', () => {
        const config = vscode.workspace.getConfiguration('bslAnalyzer');
        
        // Check that configuration exists
        assert.ok(config);
        
        // Check default values
        const platformVersion = config.get<string>('platformVersion');
        assert.ok(platformVersion, 'Platform version should be set');
        
        const autoIndexBuild = config.get<boolean>('autoIndexBuild');
        assert.strictEqual(typeof autoIndexBuild, 'boolean', 'autoIndexBuild should be boolean');
    });
});