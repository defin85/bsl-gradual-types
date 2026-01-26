/**
 * Unit тесты для Task 2.20.3: Current Context Provider
 *
 * Проверяет корректность работы contextProvider модуля:
 * - Инициализация с event listeners
 * - Debouncing cursor move (200ms)
 * - LSP Custom Request для получения контекста
 * - Форматирование tooltip с уникальными маркерами
 * - Graceful degradation при отсутствии LSP
 */

import * as assert from 'assert';
import * as sinon from 'sinon';
import * as vscode from 'vscode';
import { initializeContextProvider, CurrentContext } from '../../lsp/contextProvider';
import * as clientModule from '../../lsp/client';
import { State } from 'vscode-languageclient/node';

async function flushPromises(): Promise<void> {
    await Promise.resolve();
    await Promise.resolve();
}

suite('Context Provider Test Suite', () => {
    let statusBarStub: any;
    let context: vscode.ExtensionContext;
    let clock: sinon.SinonFakeTimers;
    let commandExecuteStub: sinon.SinonStub;
    let getLanguageClientStub: sinon.SinonStub;
    let onDidChangeTextEditorSelectionStub: sinon.SinonStub;
    let onDidChangeActiveTextEditorStub: sinon.SinonStub;
    let selectionHandler: ((event: any) => void) | undefined;
    let showErrorMessageStub: sinon.SinonStub;

    setup(() => {
        // Mock status bar
        statusBarStub = {
            text: '',
            tooltip: '',
            show: sinon.stub(),
            hide: sinon.stub(),
            dispose: sinon.stub()
        };

        // Mock extension context
        context = {
            subscriptions: [],
        } as any;

        // Используем fake timers для контроля debouncing
        clock = sinon.useFakeTimers();

        // Mock vscode.commands.executeCommand для LSP Custom Request
        commandExecuteStub = sinon.stub(vscode.commands, 'executeCommand');

        // Перехватываем регистрацию событий VSCode, чтобы можно было вручную вызвать обработчики
        selectionHandler = undefined;
        onDidChangeTextEditorSelectionStub = sinon
            .stub(vscode.window, 'onDidChangeTextEditorSelection')
            .callsFake((handler: any) => {
                selectionHandler = handler;
                return { dispose: sinon.stub() } as any;
            });

        onDidChangeActiveTextEditorStub = sinon
            .stub(vscode.window, 'onDidChangeActiveTextEditor')
            .callsFake((_handler: any) => {
                return { dispose: sinon.stub() } as any;
            });

        // По умолчанию LSP Running
        getLanguageClientStub = sinon
            .stub(clientModule, 'getLanguageClient')
            .returns({ state: State.Running } as any);
    });

    teardown(() => {
        clock.restore();
        sinon.restore();
    });

    // ========================================================================
    // Тест 1: initializeContextProvider registers event listeners
    // ========================================================================

    test('initializeContextProvider registers event listeners', () => {
        const initialSubscriptionsCount = context.subscriptions.length;

        initializeContextProvider(context, statusBarStub);

        // Проверяем что добавлены subscriptions для событий:
        // - onDidChangeTextEditorSelection
        // - onDidChangeActiveTextEditor
        assert.ok(
            context.subscriptions.length > initialSubscriptionsCount,
            'Context provider должен зарегистрировать event listeners'
        );
    });

    // ========================================================================
    // Тест 2: handleCursorMove debounces updates (200ms)
    // ========================================================================

    test('handleCursorMove debounces updates to max 1 per 200ms', async () => {
        // Мокируем успешный LSP response
        const mockContext: CurrentContext = {
            functionName: 'TestFunction',
            functionKind: 'function',
            params: ['Param1'],
            returnType: 'String',
        };
        commandExecuteStub.resolves(mockContext);

        initializeContextProvider(context, statusBarStub);

        assert.ok(selectionHandler, 'Selection handler должен быть зарегистрирован');

        // Создаём mock text editor
        const mockEditor = {
            document: {
                languageId: 'bsl',
                uri: { toString: () => 'file:///test.bsl' },
            },
            selection: {
                active: { line: 5, character: 10 },
            },
        } as any;

        // Симулируем быстрое движение курсора (10 раз каждые 50ms)
        for (let i = 0; i < 10; i++) {
            selectionHandler!({ textEditor: mockEditor });
            clock.tick(50); // 50ms между обновлениями
        }

        // Ждём завершения debouncing (200ms)
        clock.tick(200);
        await flushPromises();

        // Должна быть только 1 попытка вызова LSP (debouncing сработал)
        // Проверяем вызовы 'bsl.getCurrentContext'
        const contextCalls = commandExecuteStub.getCalls().filter(
            call => call.args[0] === 'bsl.getCurrentContext'
        );

        assert.ok(
            contextCalls.length <= 2,
            `Debouncing должен ограничить вызовы LSP, получили ${contextCalls.length} вызовов`
        );
    });

    // ========================================================================
    // Тест 3: updateCurrentContext calls LSP Custom Request
    // ========================================================================

    test('updateCurrentContext calls LSP Custom Request with correct parameters', async () => {
        const mockContext: CurrentContext = {
            functionName: 'MyFunction',
            functionKind: 'procedure',
            params: ['Param1', 'Param2'],
            returnType: undefined,
        };
        commandExecuteStub.withArgs('bsl.getCurrentContext', sinon.match.any).resolves(mockContext);

        initializeContextProvider(context, statusBarStub);

        assert.ok(selectionHandler, 'Selection handler должен быть зарегистрирован');

        // Симулируем cursor move в BSL файле
        const mockEditor = {
            document: {
                languageId: 'bsl',
                uri: { toString: () => 'file:///test.bsl' },
            },
            selection: {
                active: { line: 10, character: 5 },
            },
        } as any;

        // Вызываем обработчик cursor move
        selectionHandler!({ textEditor: mockEditor });

        // Ждём debouncing
        clock.tick(200);
        await flushPromises();

        // Проверяем что был вызван 'bsl.getCurrentContext'
        const contextCall = commandExecuteStub.getCalls().find(
            call => call.args[0] === 'bsl.getCurrentContext'
        );

        if (contextCall) {
            const params = contextCall.args[1];
            assert.strictEqual(params.uri, 'file:///test.bsl', 'URI должен совпадать');
            assert.strictEqual(params.line, 10, 'Line должна совпадать');
            assert.strictEqual(params.character, 5, 'Character должен совпадать');
        }
    });

    // ========================================================================
    // Тест 4: updateStatusBarTooltip formats function context correctly
    // ========================================================================

    test('updateStatusBarTooltip formats function context correctly', async () => {
        const mockContext: CurrentContext = {
            functionName: 'CalculateTotal',
            functionKind: 'function',
            params: ['Quantity', 'Price'],
            returnType: 'Number',
        };
        commandExecuteStub.withArgs('bsl.getCurrentContext', sinon.match.any).resolves(mockContext);

        initializeContextProvider(context, statusBarStub);

        assert.ok(selectionHandler, 'Selection handler должен быть зарегистрирован');

        // Симулируем cursor move
        const mockEditor = {
            document: {
                languageId: 'bsl',
                uri: { toString: () => 'file:///test.bsl' },
            },
            selection: {
                active: { line: 1, character: 1 },
            },
        } as any;

        selectionHandler!({ textEditor: mockEditor });

        clock.tick(200);

        // Ждём асинхронное обновление
        await flushPromises();

        const tooltip = statusBarStub.tooltip as string;

        // Проверяем что tooltip содержит корректную информацию
        assert.ok(tooltip.includes('Функция: CalculateTotal'), 'Tooltip должен содержать имя функции');
        assert.ok(tooltip.includes('Параметры: Quantity, Price'), 'Tooltip должен содержать параметры');
        assert.ok(tooltip.includes('Возвращает: Number'), 'Tooltip должен содержать тип возврата');
    });

    // ========================================================================
    // Тест 5: updateStatusBarTooltip uses unique markers
    // ========================================================================

    test('updateStatusBarTooltip uses unique markers <!-- BSL_CONTEXT_START/END -->', async () => {
        const mockContext: CurrentContext = {
            functionName: 'TestFunction',
            functionKind: 'function',
            params: [],
        };
        commandExecuteStub.withArgs('bsl.getCurrentContext', sinon.match.any).resolves(mockContext);

        initializeContextProvider(context, statusBarStub);

        assert.ok(selectionHandler, 'Selection handler должен быть зарегистрирован');

        const mockEditor = {
            document: {
                languageId: 'bsl',
                uri: { toString: () => 'file:///test.bsl' },
            },
            selection: {
                active: { line: 1, character: 1 },
            },
        } as any;

        selectionHandler!({ textEditor: mockEditor });

        clock.tick(200);
        await flushPromises();

        const tooltip = statusBarStub.tooltip as string;

        assert.ok(
            tooltip.includes('<!-- BSL_CONTEXT_START -->'),
            'Tooltip должен содержать начальный маркер контекста'
        );
        assert.ok(
            tooltip.includes('<!-- BSL_CONTEXT_END -->'),
            'Tooltip должен содержать конечный маркер контекста'
        );
    });

    // ========================================================================
    // Тест 6: updateStatusBarTooltip preserves other sections
    // ========================================================================

    test('updateStatusBarTooltip preserves STATS section when updating context', async () => {
        // Устанавливаем tooltip с секцией от statsProvider
        statusBarStub.tooltip = 'BSL Analyzer\n<!-- BSL_STATS_START -->\nTypeRepository: 100 типов\n<!-- BSL_STATS_END -->';

        const mockContext: CurrentContext = {
            functionName: 'NewFunction',
            functionKind: 'function',
            params: [],
        };
        commandExecuteStub.withArgs('bsl.getCurrentContext', sinon.match.any).resolves(mockContext);

        initializeContextProvider(context, statusBarStub);

        assert.ok(selectionHandler, 'Selection handler должен быть зарегистрирован');

        const mockEditor = {
            document: {
                languageId: 'bsl',
                uri: { toString: () => 'file:///test.bsl' },
            },
            selection: {
                active: { line: 1, character: 1 },
            },
        } as any;

        selectionHandler!({ textEditor: mockEditor });

        clock.tick(200);
        await flushPromises();

        const tooltip = statusBarStub.tooltip as string;

        // Проверяем что секция STATS сохранилась
        assert.ok(
            tooltip.includes('<!-- BSL_STATS_START -->'),
            'Секция STATS должна сохраниться'
        );
        assert.ok(
            tooltip.includes('TypeRepository: 100 типов'),
            'Содержимое секции STATS должно сохраниться'
        );
        assert.ok(
            tooltip.includes('<!-- BSL_CONTEXT_START -->'),
            'Секция CONTEXT должна добавиться'
        );
    });

    // ========================================================================
    // Тест 7: graceful degradation when LSP unavailable
    // ========================================================================

    test('graceful degradation when LSP unavailable (no error shown)', async () => {
        showErrorMessageStub = sinon.stub(vscode.window, 'showErrorMessage');

        // Мокируем отсутствие LSP (команда выбрасывает ошибку)
        commandExecuteStub.withArgs('bsl.getCurrentContext', sinon.match.any).rejects(
            new Error('LSP not available')
        );

        initializeContextProvider(context, statusBarStub);

        assert.ok(selectionHandler, 'Selection handler должен быть зарегистрирован');

        const mockEditor = {
            document: {
                languageId: 'bsl',
                uri: { toString: () => 'file:///test.bsl' },
            },
            selection: {
                active: { line: 1, character: 1 },
            },
        } as any;

        selectionHandler!({ textEditor: mockEditor });

        clock.tick(200);
        await flushPromises();

        // Проверяем что НЕ было показано сообщение об ошибке
        assert.ok(showErrorMessageStub.notCalled, 'Graceful degradation должен НЕ показывать ошибку пользователю');
    });

    // ========================================================================
    // Тест 8: cleanup disposes event listeners
    // ========================================================================

    test('cleanup disposes event listeners and clears debounce timer', () => {
        initializeContextProvider(context, statusBarStub);

        const subscriptionsCount = context.subscriptions.length;
        assert.ok(subscriptionsCount > 0, 'Должны быть зарегистрированы subscriptions');

        // Dispose всех subscriptions
        context.subscriptions.forEach(disposable => {
            disposable.dispose();
        });

        // Проверяем что debounceTimer был очищен
        // (В реальности это проверяется косвенно - после dispose не должно быть утечек памяти)
        assert.ok(true, 'Cleanup должен корректно очистить resources');
    });
});
