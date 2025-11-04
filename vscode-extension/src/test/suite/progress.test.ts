/**
 * Unit тесты для Progress Status Bar
 *
 * Проверяет базовую функциональность Status Bar после рефакторинга.
 * Детальное управление прогрессом теперь делегировано vscode-languageclient.
 */

import * as assert from 'assert';
import * as sinon from 'sinon';
import {
    getCurrentProgress,
    initializeProgress,
    updateStatusBar,
    updateLspStatus
} from '../../lsp/progress';
import { State } from 'vscode-languageclient/node';
import * as vscode from 'vscode';

suite('Progress Status Bar Tests', () => {
    let outputChannel: vscode.OutputChannel;
    let statusBarItem: vscode.StatusBarItem;

    setup(() => {
        // Создаём моки для VSCode компонентов
        outputChannel = {
            appendLine: sinon.stub(),
            show: sinon.stub(),
            dispose: sinon.stub()
        } as any;

        statusBarItem = {
            text: '',
            tooltip: '',
            backgroundColor: undefined,
            show: sinon.stub(),
            hide: sinon.stub(),
            dispose: sinon.stub()
        } as any;

        // Инициализируем модуль прогресса
        initializeProgress(outputChannel, statusBarItem);
    });

    test('updateStatusBar updates text correctly', () => {
        updateStatusBar('Test message');
        assert.strictEqual(statusBarItem.text, 'Test message', 'Status bar text должен обновиться');
    });

    test('updateLspStatus changes icon based on state - Running', () => {
        updateLspStatus(State.Running);
        assert.ok(
            statusBarItem.text.includes('$(check)'),
            'Status bar должен показывать $(check) icon для Running state'
        );
        assert.strictEqual(
            statusBarItem.backgroundColor,
            undefined,
            'Background должен быть undefined для Running'
        );
    });

    test('updateLspStatus changes icon based on state - Stopped', () => {
        updateLspStatus(State.Stopped);
        assert.ok(
            statusBarItem.text.includes('$(error)'),
            'Status bar должен показывать $(error) icon для Stopped state'
        );
        assert.ok(
            statusBarItem.backgroundColor,
            'Background должен быть установлен для Stopped state'
        );
    });

    test('updateLspStatus changes icon based on state - Starting', () => {
        updateLspStatus(State.Starting);
        assert.ok(
            statusBarItem.text.includes('$(sync~spin)'),
            'Status bar должен показывать $(sync~spin) icon для Starting state'
        );
    });

    test('getCurrentProgress returns current state', () => {
        const progress = getCurrentProgress();
        assert.ok(progress, 'Progress должен быть определён');
        assert.strictEqual(typeof progress.isIndexing, 'boolean', 'isIndexing должен быть boolean');
        assert.strictEqual(typeof progress.progress, 'number', 'progress должен быть number');
        assert.strictEqual(typeof progress.currentStep, 'string', 'currentStep должен быть string');
    });
});
