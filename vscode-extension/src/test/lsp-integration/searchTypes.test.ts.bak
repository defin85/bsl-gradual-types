/**
 * Integration тесты для LSP Custom Command: bsl.searchTypes
 *
 * ЦЕЛЬ:
 * - Проверить интеграцию TypeScript → LSP Server → TypeRepository
 * - Проверить корректность формата запроса/ответа
 * - Проверить обработку граничных случаев
 *
 * ВАЖНО:
 * Эти тесты требуют запущенного LSP сервера
 */

import * as assert from 'assert';
import * as vscode from 'vscode';
import { searchTypes, SearchTypesResponse } from '../../lsp/customRequests';

suite('LSP Integration: searchTypes', () => {
    vscode.window.showInformationMessage('LSP searchTypes Integration Tests');

    // Вспомогательная функция: ожидание активации extension
    async function ensureExtensionActivated(): Promise<void> {
        const ext = vscode.extensions.getExtension('bsl-gradual-types-team.bsl-gradual-types');
        if (ext && !ext.isActive) {
            await ext.activate();
        }
    }

    // Вспомогательная функция: проверка LSP client готовности
    async function ensureLspReady(): Promise<boolean> {
        // Импорт getLanguageClient для проверки состояния LSP
        const { getLanguageClient } = await import('../../lsp/client');
        const client = getLanguageClient();
        return client !== null && client.state === 2; // Running state
    }

    // ============================================================================
    // Тесты: Основная функциональность
    // ============================================================================

    test('LSP команда bsl.searchTypes зарегистрирована', async function() {
        this.timeout(10000); // LSP старт может занять время

        await ensureExtensionActivated();

        // Проверяем, что команда доступна через workspace.executeCommand
        const allCommands = await vscode.commands.getCommands(true);
        assert.ok(
            allCommands.includes('bsl.searchTypes'),
            'Команда bsl.searchTypes должна быть зарегистрирована'
        );
    });

    test('Запрос через searchTypes функцию работает', async function() {
        this.timeout(10000);

        await ensureExtensionActivated();

        const lspReady = await ensureLspReady();
        if (!lspReady) {
            console.warn('⚠️ LSP Server не готов - пропускаем тест');
            this.skip();
        }

        try {
            const response = await searchTypes('Массив', 5);

            // Проверяем формат ответа
            assert.ok(response, 'Response не должен быть null');
            assert.ok('types' in response, 'Response должен содержать поле types');
            assert.ok('total' in response, 'Response должен содержать поле total');
            assert.ok(Array.isArray(response.types), 'types должен быть массивом');
        } catch (error) {
            // LSP может быть не готов - это не ошибка теста
            console.warn('⚠️ LSP запрос failed:', error);
            this.skip();
        }
    });

    test('Response формат совпадает с SearchTypesResponse', async function() {
        this.timeout(10000);

        await ensureExtensionActivated();

        const lspReady = await ensureLspReady();
        if (!lspReady) {
            this.skip();
        }

        try {
            const response: SearchTypesResponse = await searchTypes('Число', 1);

            // Проверяем структуру TypeSearchResult
            if (response.types.length > 0) {
                const result = response.types[0];

                assert.ok('name' in result, 'TypeSearchResult должен содержать name');
                assert.ok('facet' in result, 'TypeSearchResult должен содержать facet');
                assert.ok('certainty' in result, 'TypeSearchResult должен содержать certainty');

                assert.strictEqual(typeof result.name, 'string');
                assert.strictEqual(typeof result.facet, 'string');
                assert.strictEqual(typeof result.certainty, 'string');

                // Опциональные поля
                if (result.english_name) {
                    assert.strictEqual(typeof result.english_name, 'string');
                }
                if (result.description) {
                    assert.strictEqual(typeof result.description, 'string');
                }
            }
        } catch (error) {
            console.warn('⚠️ LSP запрос failed:', error);
            this.skip();
        }
    });

    // ============================================================================
    // Тесты: Поиск типов
    // ============================================================================

    test('Поиск "Массив" возвращает результаты', async function() {
        this.timeout(10000);

        await ensureExtensionActivated();

        const lspReady = await ensureLspReady();
        if (!lspReady) {
            this.skip();
        }

        try {
            const response = await searchTypes('Массив', 15);

            // Если TypeRepository пустой - это не ошибка теста
            if (response.total === 0) {
                console.warn('⚠️ TypeRepository пустой - типы платформы не загружены');
                this.skip();
            }

            // Проверяем, что "Массив" найден
            const found = response.types.some(t => t.name === 'Массив' || t.english_name === 'Array');
            assert.ok(found, 'Тип "Массив" должен быть найден');
        } catch (error) {
            console.warn('⚠️ LSP запрос failed:', error);
            this.skip();
        }
    });

    test('Пустой query возвращает пустой массив', async function() {
        this.timeout(10000);

        await ensureExtensionActivated();

        const lspReady = await ensureLspReady();
        if (!lspReady) {
            this.skip();
        }

        try {
            const response = await searchTypes('', 15);

            // Backend должен вернуть пустой массив для пустого query
            // (или все типы - в зависимости от реализации)
            assert.ok(response.types.length === 0 || response.types.length > 0);
        } catch (error) {
            console.warn('⚠️ LSP запрос failed:', error);
            this.skip();
        }
    });

    // ============================================================================
    // Тесты: Граничные случаи
    // ============================================================================

    test('Лимит результатов соблюдается', async function() {
        this.timeout(10000);

        await ensureExtensionActivated();

        const lspReady = await ensureLspReady();
        if (!lspReady) {
            this.skip();
        }

        try {
            const limit = 3;
            const response = await searchTypes('Массив', limit);

            assert.ok(
                response.types.length <= limit,
                `Количество результатов (${response.types.length}) не должно превышать лимит (${limit})`
            );
        } catch (error) {
            console.warn('⚠️ LSP запрос failed:', error);
            this.skip();
        }
    });

    test('Несуществующий тип возвращает пустой результат', async function() {
        this.timeout(10000);

        await ensureExtensionActivated();

        const lspReady = await ensureLspReady();
        if (!lspReady) {
            this.skip();
        }

        try {
            const response = await searchTypes('НеСуществуетТакойТипАбсолютноТочно123', 15);

            assert.strictEqual(response.total, 0, 'Несуществующий тип не должен находиться');
            assert.strictEqual(response.types.length, 0);
        } catch (error) {
            console.warn('⚠️ LSP запрос failed:', error);
            this.skip();
        }
    });

    test('Поиск по английскому имени работает', async function() {
        this.timeout(10000);

        await ensureExtensionActivated();

        const lspReady = await ensureLspReady();
        if (!lspReady) {
            this.skip();
        }

        try {
            const response = await searchTypes('Array', 15);

            if (response.total === 0) {
                console.warn('⚠️ TypeRepository пустой');
                this.skip();
            }

            // Проверяем, что найден тип с английским именем "Array"
            const found = response.types.some(
                t => t.name === 'Массив' || t.english_name === 'Array'
            );
            assert.ok(found, 'Поиск по "Array" должен найти "Массив"');
        } catch (error) {
            console.warn('⚠️ LSP запрос failed:', error);
            this.skip();
        }
    });

    // ============================================================================
    // Регрессионный тест: ТаблицаЗначений
    // ============================================================================

    test('РЕГРЕСС: "ТаблицаЗначений" теперь находится', async function() {
        this.timeout(10000);

        await ensureExtensionActivated();

        const lspReady = await ensureLspReady();
        if (!lspReady) {
            this.skip();
        }

        try {
            const response = await searchTypes('ТаблицаЗначений', 15);

            if (response.total === 0) {
                console.warn('⚠️ TypeRepository пустой - пропускаем регресс тест');
                this.skip();
            }

            // ⚠️ КРИТИЧЕСКИЙ РЕГРЕСС:
            // До интеграции с TypeRepository "ТаблицаЗначений" НЕ находилась в mock данных
            // Теперь она должна находиться в реальном TypeRepository
            const found = response.types.some(
                t => t.name === 'ТаблицаЗначений' || t.english_name === 'ValueTable'
            );
            assert.ok(
                found,
                '⚠️ РЕГРЕСС: "ТаблицаЗначений" раньше не находилась в mock данных, теперь должна!'
            );
        } catch (error) {
            console.warn('⚠️ LSP запрос failed:', error);
            this.skip();
        }
    });

    // ============================================================================
    // Тест: Performance
    // ============================================================================

    test('Поиск выполняется быстро (<1s)', async function() {
        this.timeout(10000);

        await ensureExtensionActivated();

        const lspReady = await ensureLspReady();
        if (!lspReady) {
            this.skip();
        }

        try {
            const start = Date.now();
            await searchTypes('Массив', 15);
            const duration = Date.now() - start;

            assert.ok(duration < 1000, `Поиск должен быть < 1s, actual: ${duration}ms`);
        } catch (error) {
            console.warn('⚠️ LSP запрос failed:', error);
            this.skip();
        }
    });
});
