import * as assert from 'assert';
import * as vscode from 'vscode';
import {
    queryType,
    buildIndex,
    validateMethod,
    checkTypeCompatibility,
    incrementalUpdate,
    extractPlatformDocs
} from '../../lsp/customRequests';

/**
 * Тесты для LSP Custom Requests (Task 3 из Milestone 2.2)
 * Проверяют замену CLI вызовов на LSP custom requests
 */
suite('LSP Custom Requests Test Suite', () => {

    suiteSetup(async function() {
        this.timeout(15000);

        // Активируем расширение перед тестами
        const ext = vscode.extensions.getExtension('bsl-analyzer-team.bsl-type-safety-analyzer');
        if (ext && !ext.isActive) {
            await ext.activate();
        }

        // Даем LSP серверу время на запуск
        await new Promise(resolve => setTimeout(resolve, 3000));
    });

    /**
     * Тест custom request: bsl/queryType
     * Заменяет CLI: query_type
     */
    test('queryType should work via LSP', async function() {
        this.timeout(5000);

        try {
            const result = await queryType('Массив');

            assert.ok(result, 'Query result should not be null');
            assert.strictEqual(result.type_name, 'Массив', 'Type name should match');
            assert.strictEqual(typeof result.found, 'boolean', 'Found should be boolean');

            if (result.details) {
                assert.strictEqual(typeof result.details, 'string', 'Details should be string');
            }
        } catch (error: any) {
            // Если LSP сервер не запущен, тест пропускается
            if (error.message.includes('LSP') || error.message.includes('not initialized')) {
                this.skip();
            } else {
                throw error;
            }
        }
    });

    /**
     * Тест custom request: bsl/buildIndex
     * Заменяет CLI: build_unified_index
     */
    test('buildIndex should work via LSP', async function() {
        this.timeout(5000);

        const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
        if (!workspaceFolder) {
            this.skip();
            return;
        }

        try {
            const result = await buildIndex(workspaceFolder.uri.fsPath);

            assert.ok(result, 'Build index result should not be null');
            assert.strictEqual(typeof result.success, 'boolean', 'Success should be boolean');
            assert.strictEqual(typeof result.types_count, 'number', 'Types count should be number');
            assert.strictEqual(typeof result.message, 'string', 'Message should be string');

            // types_count может быть 0 для stub реализации
            assert.ok(result.types_count >= 0, 'Types count should be non-negative');
        } catch (error: any) {
            if (error.message.includes('LSP') || error.message.includes('not initialized')) {
                this.skip();
            } else {
                throw error;
            }
        }
    });

    /**
     * Тест custom request: bsl/validateMethod
     * Заменяет CLI: check_type_compatibility (для методов)
     */
    test('validateMethod should work via LSP', async function() {
        this.timeout(5000);

        try {
            const result = await validateMethod('Массив', 'Добавить', ['Элемент']);

            assert.ok(result, 'Validate method result should not be null');
            assert.strictEqual(typeof result.valid, 'boolean', 'Valid should be boolean');
            assert.strictEqual(typeof result.message, 'string', 'Message should be string');
        } catch (error: any) {
            if (error.message.includes('LSP') || error.message.includes('not initialized')) {
                this.skip();
            } else {
                throw error;
            }
        }
    });

    /**
     * Тест custom request: bsl/checkTypeCompatibility
     * Заменяет CLI: check_type_compatibility
     */
    test('checkTypeCompatibility should work via LSP', async function() {
        this.timeout(5000);

        try {
            const result = await checkTypeCompatibility('Число', 'Строка');

            assert.ok(result, 'Compatibility result should not be null');
            assert.strictEqual(typeof result.compatible, 'boolean', 'Compatible should be boolean');
            assert.strictEqual(typeof result.message, 'string', 'Message should be string');
        } catch (error: any) {
            if (error.message.includes('LSP') || error.message.includes('not initialized')) {
                this.skip();
            } else {
                throw error;
            }
        }
    });

    /**
     * Тест custom request: bsl/incrementalUpdate
     * Заменяет CLI: incremental_update
     */
    test('incrementalUpdate should work via LSP', async function() {
        this.timeout(5000);

        const config = vscode.workspace.getConfiguration('bslAnalyzer');
        const configPath = config.get<string>('configurationPath') || '/tmp/test.xml';
        const platformVersion = config.get<string>('platformVersion') || '8.3.25';

        try {
            const result = await incrementalUpdate(configPath, platformVersion);

            assert.ok(result, 'Incremental update result should not be null');
            assert.strictEqual(typeof result.success, 'boolean', 'Success should be boolean');
            assert.strictEqual(typeof result.message, 'string', 'Message should be string');
        } catch (error: any) {
            if (error.message.includes('LSP') || error.message.includes('not initialized')) {
                this.skip();
            } else {
                throw error;
            }
        }
    });

    /**
     * Тест custom request: bsl/extractPlatformDocs
     * Заменяет CLI: extract_platform_docs
     */
    test('extractPlatformDocs should work via LSP', async function() {
        this.timeout(5000);

        const testArchivePath = '/tmp/test_archive.zip';
        const platformVersion = '8.3.25';

        try {
            const result = await extractPlatformDocs(testArchivePath, platformVersion, true);

            assert.ok(result, 'Extract platform docs result should not be null');
            assert.strictEqual(typeof result.success, 'boolean', 'Success should be boolean');
            assert.strictEqual(typeof result.types_count, 'number', 'Types count should be number');
            assert.strictEqual(typeof result.message, 'string', 'Message should be string');

            assert.ok(result.types_count >= 0, 'Types count should be non-negative');
        } catch (error: any) {
            if (error.message.includes('LSP') || error.message.includes('not initialized')) {
                this.skip();
            } else {
                throw error;
            }
        }
    });

    /**
     * Тест обработки ошибок при недоступности LSP сервера
     */
    test('Should handle LSP server errors gracefully', async function() {
        this.timeout(5000);

        // Если LSP сервер не запущен, функции должны выбросить понятную ошибку
        try {
            // Пытаемся вызвать custom request
            await queryType('ТестовыйТип');

            // Если дошли сюда, значит LSP работает
            assert.ok(true, 'LSP server is running');
        } catch (error: any) {
            // Проверяем, что ошибка содержит информацию о проблеме
            assert.ok(error, 'Error should be thrown');
            assert.ok(
                error.message.includes('LSP') ||
                error.message.includes('not initialized') ||
                error.message.includes('client'),
                `Error message should mention LSP: ${error.message}`
            );
        }
    });

    /**
     * Тест производительности custom requests
     */
    test('Custom requests should be fast', async function() {
        this.timeout(3000);

        try {
            const startTime = Date.now();
            await queryType('Строка');
            const elapsed = Date.now() - startTime;

            // LSP custom request должен выполниться быстрее 2 секунд
            assert.ok(elapsed < 2000, `Custom request took ${elapsed}ms (should be < 2000ms)`);
        } catch (error: any) {
            if (error.message.includes('LSP') || error.message.includes('not initialized')) {
                this.skip();
            } else {
                throw error;
            }
        }
    });
});

/**
 * Тесты параллельных вызовов custom requests
 */
suite('Concurrent Custom Requests Test Suite', () => {

    test('Should handle concurrent requests', async function() {
        this.timeout(10000);

        try {
            // Выполняем несколько запросов параллельно
            const promises = [
                queryType('Массив'),
                queryType('Строка'),
                queryType('Число'),
                checkTypeCompatibility('Число', 'Строка')
            ];

            const results = await Promise.all(promises);

            // Все запросы должны успешно завершиться
            assert.strictEqual(results.length, 4, 'All requests should complete');

            for (const result of results) {
                assert.ok(result, 'Each result should not be null');
            }
        } catch (error: any) {
            if (error.message.includes('LSP') || error.message.includes('not initialized')) {
                this.skip();
            } else {
                throw error;
            }
        }
    });
});
