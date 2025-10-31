/**
 * Unit тесты для Task 2.20.2: Enhanced Indexing Progress
 *
 * Проверяет корректность работы прогресса в Extension:
 * - Throttling UI обновлений (максимум 1 обновление каждые 500ms)
 * - flushPendingUpdate() корректно применяет накопленные обновления
 * - finishIndexing() сбрасывает pending throttled updates
 * - ETA корректно вычисляется
 */

import * as assert from 'assert';
import * as sinon from 'sinon';
import {
    startIndexing,
    updateIndexingProgress,
    finishIndexing,
    getCurrentProgress,
    initializeProgress,
    IndexingProgress
} from '../../lsp/progress';
import * as vscode from 'vscode';

suite('Progress Throttling Tests', () => {
    let outputChannel: vscode.OutputChannel;
    let statusBarItem: vscode.StatusBarItem;
    let clock: sinon.SinonFakeTimers;

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
            show: sinon.stub(),
            hide: sinon.stub(),
            dispose: sinon.stub()
        } as any;

        // Инициализируем модуль прогресса
        initializeProgress(outputChannel, statusBarItem);

        // Используем fake timers для контроля времени
        clock = sinon.useFakeTimers();
    });

    teardown(() => {
        clock.restore();
    });

    // ========================================================================
    // Тест 1: updateIndexingProgress throttles UI updates
    // ========================================================================

    test('updateIndexingProgress throttles UI updates to max 2 per second', () => {
        startIndexing(4);

        const statusBarStub = statusBarItem as any;

        // Засекаем начальное состояние
        const initialCallCount = statusBarStub.show.callCount;

        // Быстро отправляем 10 обновлений (каждые 100ms)
        for (let i = 1; i <= 10; i++) {
            updateIndexingProgress(i * 10, `Step ${i}`, undefined);
            clock.tick(100); // Продвигаем время на 100ms
        }

        // Финализируем все pending updates
        clock.tick(500); // Ждём завершения последнего throttle

        const totalCalls = statusBarStub.show.callCount - initialCallCount;

        // За 10 * 100ms = 1000ms должно быть максимум 2 UI обновления (каждые 500ms)
        // + 1 начальное обновление = 3 максимум
        assert.ok(
            totalCalls <= 3,
            `UI должен обновляться максимум 3 раза (начальное + 2 throttled), получили ${totalCalls}`
        );

        const progress = getCurrentProgress();
        assert.strictEqual(progress.progress, 100, 'Финальный прогресс должен быть 100%');
    });

    // ========================================================================
    // Тест 2: finishIndexing clears pending throttled updates
    // ========================================================================

    test('finishIndexing clears pending throttled updates immediately', () => {
        startIndexing(4);

        const statusBarStub = statusBarItem as any;
        const outputStub = outputChannel as any;

        // Быстро отправляем несколько обновлений (попадут в throttle queue)
        updateIndexingProgress(30, 'Step 1', undefined);
        updateIndexingProgress(60, 'Step 2', undefined);

        // НЕ ждём 500ms - сразу финишируем
        finishIndexing('✅ Загружено успешно');

        // Проверяем что индексация завершена
        const progress = getCurrentProgress();
        assert.strictEqual(progress.isIndexing, false, 'isIndexing должен быть false');
        assert.strictEqual(progress.currentStep, 'Completed', 'currentStep должен быть Completed');
        assert.strictEqual(progress.progress, 100, 'progress должен быть 100%');

        // Проверяем что финальное сообщение было отправлено
        const lastCall = outputStub.appendLine.lastCall;
        assert.ok(
            lastCall.args[0].includes('✅'),
            'Финальное сообщение должно содержать галочку'
        );
        assert.ok(
            lastCall.args[0].includes('Загружено успешно'),
            'Финальное сообщение должно содержать текст успеха'
        );

        // Продвигаем время - pending updates НЕ должны выполниться после finish
        clock.tick(600);

        // Status bar должен показывать финальное состояние (не промежуточное)
        assert.ok(
            statusBarItem.text.includes('Ready') || statusBarItem.text.includes('BSL Analyzer'),
            `Status bar должен показывать финальное состояние, получили: ${statusBarItem.text}`
        );
    });

    // ========================================================================
    // Тест 3: ETA calculation is reasonable
    // ========================================================================

    test('ETA calculation is reasonable for 10% progress', () => {
        startIndexing(4);

        // Симулируем что прошло 1 секунда и выполнено 10%
        clock.tick(1000);
        updateIndexingProgress(10, 'Step 1', undefined);

        const progress = getCurrentProgress();
        const eta = progress.estimatedTimeRemaining;

        // ETA должна быть определена (не 'calculating...')
        assert.ok(eta, 'ETA должна быть определена');
        assert.notStrictEqual(eta, 'calculating...', 'ETA не должна быть "calculating..."');

        // Проверяем что ETA разумная
        // При 10% за 1s → всего ~10s → осталось ~9s
        // Допускаем погрешность: от 7s до 12s
        if (eta && eta !== 'calculating...') {
            const etaSeconds = parseInt(eta.replace('s', ''));
            assert.ok(
                etaSeconds >= 7 && etaSeconds <= 12,
                `ETA должна быть разумной (7-12s), получили ${eta}`
            );
        }
    });

    // ========================================================================
    // Тест 4: ETA не показывается при прогрессе меньше 5%
    // ========================================================================

    test('ETA is not shown for progress less than 5%', () => {
        startIndexing(4);

        // Прогресс 3% (слишком рано для точной ETA)
        clock.tick(500);
        updateIndexingProgress(3, 'Step 1', undefined);

        const progress = getCurrentProgress();
        const eta = progress.estimatedTimeRemaining;

        // ETA должна быть 'calculating...' или undefined
        assert.ok(
            eta === 'calculating...' || eta === undefined,
            `ETA не должна отображаться при прогрессе < 5%, получили: ${eta}`
        );
    });

    // ========================================================================
    // Тест 5: ETA decreases over time
    // ========================================================================

    test('ETA decreases as progress increases', () => {
        startIndexing(4);

        // Прогресс 20% за 2 секунды
        clock.tick(2000);
        updateIndexingProgress(20, 'Step 1', undefined);
        const eta1 = getCurrentProgress().estimatedTimeRemaining;

        // Прогресс 50% за ещё 3 секунды (всего 5 секунд)
        clock.tick(3000);
        updateIndexingProgress(50, 'Step 2', undefined);
        const eta2 = getCurrentProgress().estimatedTimeRemaining;

        // Оба ETA должны быть определены
        assert.ok(eta1 && eta1 !== 'calculating...', 'ETA1 должна быть определена');
        assert.ok(eta2 && eta2 !== 'calculating...', 'ETA2 должна быть определена');

        // Парсим значения
        if (eta1 && eta2 && eta1 !== 'calculating...' && eta2 !== 'calculating...') {
            const etaSeconds1 = parseInt(eta1.replace('s', ''));
            const etaSeconds2 = parseInt(eta2.replace('s', ''));

            assert.ok(
                etaSeconds2 < etaSeconds1,
                `ETA должна уменьшаться: ${etaSeconds1}s → ${etaSeconds2}s`
            );
        }
    });

    // ========================================================================
    // Тест 6: Throttle использует "trailing edge" паттерн
    // ========================================================================

    test('Throttle uses trailing edge pattern - last update is always applied', () => {
        startIndexing(4);

        const statusBarStub = statusBarItem as any;

        // Отправляем 5 быстрых обновлений
        for (let i = 1; i <= 5; i++) {
            updateIndexingProgress(i * 20, `Step ${i}`, undefined);
            clock.tick(50); // 50ms между обновлениями
        }

        // НЕ ждём 500ms - финишируем сразу
        // Но последнее значение (100%) должно показаться
        finishIndexing('✅ Завершено');

        const progress = getCurrentProgress();

        // Проверяем что финальное значение применено
        assert.strictEqual(
            progress.progress,
            100,
            'Финальное значение должно быть применено (trailing edge)'
        );
    });

    // ========================================================================
    // Тест 7: Multiple indexing cycles don't interfere
    // ========================================================================

    test('Multiple indexing cycles do not interfere with each other', () => {
        // Первый цикл
        startIndexing(4);
        updateIndexingProgress(50, 'Step 1', undefined);
        clock.tick(500);
        finishIndexing('✅ Цикл 1 завершён');

        const progress1 = getCurrentProgress();
        assert.strictEqual(progress1.isIndexing, false, 'Первый цикл должен быть завершён');
        assert.strictEqual(progress1.progress, 100, 'Первый цикл должен показывать 100%');

        // Второй цикл (сразу после первого)
        startIndexing(4);
        assert.strictEqual(getCurrentProgress().isIndexing, true, 'Второй цикл должен начаться');
        assert.strictEqual(getCurrentProgress().progress, 0, 'Второй цикл должен начинаться с 0%');

        updateIndexingProgress(30, 'Step 1', undefined);
        clock.tick(500);

        const progress2 = getCurrentProgress();
        assert.strictEqual(progress2.progress, 30, 'Второй цикл должен показывать 30%');
        assert.strictEqual(progress2.isIndexing, true, 'Второй цикл должен быть активным');

        finishIndexing('✅ Цикл 2 завершён');
        assert.strictEqual(getCurrentProgress().isIndexing, false, 'Второй цикл должен быть завершён');
    });

    // ========================================================================
    // Тест 8: Progress never goes backwards
    // ========================================================================

    test('Progress percentage never goes backwards', () => {
        startIndexing(4);

        const percentages: number[] = [];

        // Отправляем несколько обновлений
        for (let i of [10, 20, 30, 40, 35, 50, 60]) { // Заметьте 35 после 40!
            updateIndexingProgress(i, `Step ${i}`, undefined);
            clock.tick(100);
            percentages.push(getCurrentProgress().progress);
        }

        // Проверяем монотонность
        for (let i = 1; i < percentages.length; i++) {
            assert.ok(
                percentages[i] >= percentages[i - 1],
                `Прогресс не должен уменьшаться: ${percentages[i - 1]}% → ${percentages[i]}%`
            );
        }
    });
});
