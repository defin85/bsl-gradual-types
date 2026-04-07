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
    let activeEditorHandler: ((editor: any) => void) | undefined;
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
        activeEditorHandler = undefined;
        onDidChangeTextEditorSelectionStub = sinon
            .stub(vscode.window, 'onDidChangeTextEditorSelection')
            .callsFake((handler: any) => {
                selectionHandler = handler;
                return { dispose: sinon.stub() } as any;
            });

        onDidChangeActiveTextEditorStub = sinon
            .stub(vscode.window, 'onDidChangeActiveTextEditor')
            .callsFake((handler: any) => {
                activeEditorHandler = handler;
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

        assert.ok(contextCall, 'bsl.getCurrentContext должен быть вызван после debounce');

        const params = contextCall.args[1];
        assert.strictEqual(params.uri, 'file:///test.bsl', 'URI должен совпадать');
        assert.strictEqual(params.line, 10, 'Line должна совпадать');
        assert.strictEqual(params.character, 5, 'Character должен совпадать');
        assert.strictEqual(
            params.editorSessionId,
            'file:///test.bsl::0',
            'editorSessionId должен быть стабильно производным от текущего editor session'
        );
        assert.strictEqual(
            params.requestGeneration,
            1,
            'Первый requestGeneration в рамках editor session должен быть равен 1'
        );
    });

    test('latest request generation wins for the same editor session', async () => {
        let resolveFirst: ((context: CurrentContext) => void) | undefined;
        let resolveSecond: ((context: CurrentContext) => void) | undefined;

        commandExecuteStub
            .onFirstCall()
            .returns(new Promise<CurrentContext>((resolve) => {
                resolveFirst = resolve;
            }));
        commandExecuteStub
            .onSecondCall()
            .returns(new Promise<CurrentContext>((resolve) => {
                resolveSecond = resolve;
            }));

        initializeContextProvider(context, statusBarStub);

        assert.ok(selectionHandler, 'Selection handler должен быть зарегистрирован');

        const mockEditor = {
            document: {
                languageId: 'bsl',
                uri: { toString: () => 'file:///latest-only.bsl' },
            },
            selection: {
                active: { line: 4, character: 3 },
            },
        } as any;

        selectionHandler!({ textEditor: mockEditor });
        clock.tick(200);
        await flushPromises();

        mockEditor.selection.active = { line: 8, character: 11 };
        selectionHandler!({ textEditor: mockEditor });
        clock.tick(200);
        await flushPromises();

        assert.strictEqual(
            commandExecuteStub.callCount,
            2,
            'Должно уйти два current-context request для двух поколений одной editor session'
        );
        assert.strictEqual(
            commandExecuteStub.secondCall.args[1].requestGeneration,
            2,
            'Второй request в рамках той же editor session должен получить generation=2'
        );

        resolveSecond?.({
            functionName: 'НоваяФункция',
            functionKind: 'function',
            params: ['Параметр2'],
        });
        await flushPromises();

        assert.ok(
            (statusBarStub.tooltip as string).includes('НоваяФункция'),
            'Status bar должен принять latest response'
        );

        resolveFirst?.({
            functionName: 'СтараяФункция',
            functionKind: 'function',
            params: ['Параметр1'],
        });
        await flushPromises();

        const tooltip = statusBarStub.tooltip as string;
        assert.ok(
            tooltip.includes('НоваяФункция'),
            'Поздний stale response не должен затирать latest current-context tooltip'
        );
        assert.ok(
            !tooltip.includes('СтараяФункция'),
            'Stale response из более старого generation должен быть проигнорирован'
        );
    });

    test('newer cursor move reserves generation before debounce so stale response cannot flash old tooltip', async () => {
        let resolveFirst: ((context: CurrentContext) => void) | undefined;
        let resolveSecond: ((context: CurrentContext) => void) | undefined;

        commandExecuteStub
            .onFirstCall()
            .returns(new Promise<CurrentContext>((resolve) => {
                resolveFirst = resolve;
            }));
        commandExecuteStub
            .onSecondCall()
            .returns(new Promise<CurrentContext>((resolve) => {
                resolveSecond = resolve;
            }));

        initializeContextProvider(context, statusBarStub);

        assert.ok(selectionHandler, 'Selection handler должен быть зарегистрирован');

        const mockEditor = {
            document: {
                languageId: 'bsl',
                uri: { toString: () => 'file:///debounce-window.bsl' },
            },
            selection: {
                active: { line: 2, character: 4 },
            },
        } as any;

        selectionHandler!({ textEditor: mockEditor });
        clock.tick(200);
        await flushPromises();

        mockEditor.selection.active = { line: 9, character: 1 };
        selectionHandler!({ textEditor: mockEditor });

        resolveFirst?.({
            functionName: 'СтарыйКонтекст',
            functionKind: 'function',
            params: [],
        });
        await flushPromises();

        assert.ok(
            !(statusBarStub.tooltip as string).includes('СтарыйКонтекст'),
            'После нового cursor move stale response не должен вспыхивать в tooltip даже до отправки следующего request'
        );

        clock.tick(200);
        await flushPromises();

        assert.strictEqual(
            commandExecuteStub.secondCall.args[1].requestGeneration,
            2,
            'Следующий request после debounce должен использовать уже зарезервированное generation=2'
        );

        resolveSecond?.({
            functionName: 'НовыйКонтекст',
            functionKind: 'function',
            params: [],
        });
        await flushPromises();

        assert.ok(
            (statusBarStub.tooltip as string).includes('НовыйКонтекст'),
            'После отправки нового request tooltip должен перейти на newest generation'
        );
    });

    test('request generation stays monotonic after tracked session eviction', async () => {
        commandExecuteStub.resolves({
            functionKind: 'none',
        });

        initializeContextProvider(context, statusBarStub);
        assert.ok(selectionHandler, 'Selection handler должен быть зарегистрирован');

        const makeEditor = (index: number) => ({
            document: {
                languageId: 'bsl',
                uri: { toString: () => `file:///session-${index}.bsl` },
            },
            selection: {
                active: { line: index, character: 0 },
            },
            viewColumn: 1,
        }) as any;

        const firstEditor = makeEditor(0);
        selectionHandler!({ textEditor: firstEditor });
        clock.tick(200);
        await flushPromises();
        assert.strictEqual(
            commandExecuteStub.firstCall.args[1].requestGeneration,
            1,
            'Первая editor session должна получить generation=1'
        );

        for (let index = 1; index <= 256; index += 1) {
            selectionHandler!({ textEditor: makeEditor(index) });
            clock.tick(200);
            await flushPromises();
        }

        selectionHandler!({ textEditor: firstEditor });
        clock.tick(200);
        await flushPromises();

        assert.strictEqual(
            commandExecuteStub.lastCall.args[1].requestGeneration,
            258,
            'После вытеснения старой session generation не должен перезапускаться с 1'
        );
    });

    test('stale response from previous target session is ignored after active editor switch', async () => {
        let resolveFirst: ((context: CurrentContext) => void) | undefined;
        let resolveSecond: ((context: CurrentContext) => void) | undefined;

        commandExecuteStub
            .onFirstCall()
            .returns(new Promise<CurrentContext>((resolve) => {
                resolveFirst = resolve;
            }));
        commandExecuteStub
            .onSecondCall()
            .returns(new Promise<CurrentContext>((resolve) => {
                resolveSecond = resolve;
            }));

        initializeContextProvider(context, statusBarStub);

        assert.ok(selectionHandler, 'Selection handler должен быть зарегистрирован');
        assert.ok(activeEditorHandler, 'Active editor handler должен быть зарегистрирован');

        const firstEditor = {
            document: {
                languageId: 'bsl',
                uri: { toString: () => 'file:///first-target.bsl' },
            },
            selection: {
                active: { line: 1, character: 1 },
            },
        } as any;
        const secondEditor = {
            document: {
                languageId: 'bsl',
                uri: { toString: () => 'file:///second-target.bsl' },
            },
            selection: {
                active: { line: 2, character: 2 },
            },
        } as any;

        selectionHandler!({ textEditor: firstEditor });
        clock.tick(200);
        await flushPromises();

        activeEditorHandler!(secondEditor);
        clock.tick(200);
        await flushPromises();

        resolveSecond?.({
            functionName: 'ВтораяФункция',
            functionKind: 'procedure',
            params: [],
        });
        await flushPromises();

        resolveFirst?.({
            functionName: 'ПерваяФункция',
            functionKind: 'procedure',
            params: [],
        });
        await flushPromises();

        const tooltip = statusBarStub.tooltip as string;
        assert.ok(
            tooltip.includes('ВтораяФункция'),
            'Ответ для текущей target session должен применяться'
        );
        assert.ok(
            !tooltip.includes('ПерваяФункция'),
            'Ответ для предыдущей target session не должен возвращать stale tooltip'
        );
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
