"use strict";
/**
 * Unit тесты для Progress Status Bar
 *
 * Проверяет базовую функциональность Status Bar после рефакторинга.
 * Детальное управление прогрессом теперь делегировано vscode-languageclient.
 */
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
const sinon = __importStar(require("sinon"));
const progress_1 = require("../../lsp/progress");
const node_1 = require("vscode-languageclient/node");
suite('Progress Status Bar Tests', () => {
    let outputChannel;
    let statusBarItem;
    setup(() => {
        // Создаём моки для VSCode компонентов
        outputChannel = {
            appendLine: sinon.stub(),
            show: sinon.stub(),
            dispose: sinon.stub()
        };
        statusBarItem = {
            text: '',
            tooltip: '',
            backgroundColor: undefined,
            show: sinon.stub(),
            hide: sinon.stub(),
            dispose: sinon.stub()
        };
        // Инициализируем модуль прогресса
        (0, progress_1.initializeProgress)(outputChannel, statusBarItem);
    });
    test('updateStatusBar updates text correctly', () => {
        (0, progress_1.updateStatusBar)('Test message');
        assert.strictEqual(statusBarItem.text, 'Test message', 'Status bar text должен обновиться');
    });
    test('updateLspStatus changes icon based on state - Running', () => {
        (0, progress_1.updateLspStatus)(node_1.State.Running);
        assert.ok(statusBarItem.text.includes('$(check)'), 'Status bar должен показывать $(check) icon для Running state');
        assert.strictEqual(statusBarItem.backgroundColor, undefined, 'Background должен быть undefined для Running');
    });
    test('updateLspStatus changes icon based on state - Stopped', () => {
        (0, progress_1.updateLspStatus)(node_1.State.Stopped);
        assert.ok(statusBarItem.text.includes('$(error)'), 'Status bar должен показывать $(error) icon для Stopped state');
        assert.ok(statusBarItem.backgroundColor, 'Background должен быть установлен для Stopped state');
    });
    test('updateLspStatus changes icon based on state - Starting', () => {
        (0, progress_1.updateLspStatus)(node_1.State.Starting);
        assert.ok(statusBarItem.text.includes('$(sync~spin)'), 'Status bar должен показывать $(sync~spin) icon для Starting state');
    });
    test('getCurrentProgress returns current state', () => {
        const progress = (0, progress_1.getCurrentProgress)();
        assert.ok(progress, 'Progress должен быть определён');
        assert.strictEqual(typeof progress.isIndexing, 'boolean', 'isIndexing должен быть boolean');
        assert.strictEqual(typeof progress.progress, 'number', 'progress должен быть number');
        assert.strictEqual(typeof progress.currentStep, 'string', 'currentStep должен быть string');
    });
});
//# sourceMappingURL=progress.test.js.map