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
const parser_1 = require("../../utils/parser");
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
            assert.strictEqual(foundCommands.length, commands.length, `Missing commands: ${commands.filter(cmd => !foundCommands.includes(cmd)).join(', ')}`);
        });
    });
});
suite('Parser Test Suite', () => {
    test('parseMethodCall should extract method info', () => {
        const result = (0, parser_1.parseMethodCall)('Справочники.Номенклатура.НайтиПоКоду("123")');
        assert.ok(result);
        assert.strictEqual(result?.objectName, 'Справочники.Номенклатура');
        assert.strictEqual(result?.methodName, 'НайтиПоКоду');
    });
    test('parseMethodCall should handle simple calls', () => {
        const result = (0, parser_1.parseMethodCall)('Массив.Добавить(');
        assert.ok(result);
        assert.strictEqual(result?.objectName, 'Массив');
        assert.strictEqual(result?.methodName, 'Добавить');
    });
    test('parseMethodCall should return null for invalid input', () => {
        const result = (0, parser_1.parseMethodCall)('НеМетод');
        assert.strictEqual(result, null);
    });
    test('extractTypeName should extract variable name', () => {
        const result = (0, parser_1.extractTypeName)('Перем МояПеременная');
        assert.strictEqual(result, 'МояПеременная');
    });
    test('extractTypeName should extract variable name with Var', () => {
        const result = (0, parser_1.extractTypeName)('Var МояПеременная');
        assert.strictEqual(result, 'МояПеременная');
    });
    test('extractTypeName should extract from assignment', () => {
        const result = (0, parser_1.extractTypeName)('Результат = НовыйМассив()');
        assert.strictEqual(result, 'Результат');
    });
    test('extractTypeName should return first word as fallback', () => {
        const result = (0, parser_1.extractTypeName)('СложныйТекст без паттернов');
        assert.strictEqual(result, 'СложныйТекст');
    });
});
suite('Configuration Test Suite', () => {
    test('Should have default configuration values', () => {
        const config = vscode.workspace.getConfiguration('bslAnalyzer');
        // Check that configuration exists
        assert.ok(config);
        // Check default values
        const platformVersion = config.get('platformVersion');
        assert.ok(platformVersion, 'Platform version should be set');
        const autoIndexBuild = config.get('autoIndexBuild');
        assert.strictEqual(typeof autoIndexBuild, 'boolean', 'autoIndexBuild should be boolean');
    });
});
//# sourceMappingURL=extension.test.js.map