import * as assert from 'assert';
import * as vscode from 'vscode';
import * as sinon from 'sinon';
import {
    queryType,
    buildIndex,
    getIndexState,
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
    let getLanguageClientStub: sinon.SinonStub;
    let sendRequestStub: sinon.SinonStub;
    let sendCustomRequestStub: sinon.SinonStub;

    suiteSetup(async function() {
        this.timeout(15000);

        // Активируем расширение перед тестами
        const ext = vscode.extensions.getExtension('bsl-gradual-types-team.bsl-gradual-types');
        if (ext && !ext.isActive) {
            await ext.activate();
        }

        // Даем LSP серверу время на запуск
        await new Promise(resolve => setTimeout(resolve, 3000));
    });

    setup(async () => {
        // Применить mock конфигурацию
        const config = vscode.workspace.getConfiguration('bslAnalyzer');
        await config.update('platformDocsArchive', '', vscode.ConfigurationTarget.Global);

        // Mock данные типов платформы 1С
        const mockPlatformTypes: Record<string, any> = {
            'Массив': {
                name: 'Массив',
                facet: 'Object',
                certainty: 'Known',
                description: 'Универсальная коллекция произвольных значений',
                methods: [
                    { name: 'Добавить', params: ['Значение'], returns: 'Число' },
                    { name: 'Количество', params: [], returns: 'Число' },
                    { name: 'Очистить', params: [], returns: 'void' }
                ],
                properties: [
                    { name: 'ВГраница', type: 'Число', readonly: true }
                ]
            },
            'Строка': {
                name: 'Строка',
                facet: 'Object',
                certainty: 'Known',
                description: 'Строковое значение произвольной длины',
                methods: []
            },
            'Число': {
                name: 'Число',
                facet: 'Object',
                certainty: 'Known',
                description: 'Числовое значение',
                methods: []
            }
        };

        // Mock sendRequest для эмуляции ответов LSP Server
        sendRequestStub = sinon.stub().callsFake((method: string, params: any) => {
            if (method === 'workspace/executeCommand') {
                const command = params.command;
                const args = params.arguments || [];

                // Mock команды
                if (command === 'bsl.queryType') {
                    const typeName = args[0]?.type_name;
                    const type = mockPlatformTypes[typeName];
                    if (type) {
                        return Promise.resolve({
                            typeName: typeName,
                            found: true,
                            certainty: type.certainty,
                            facet: type.facet,
                            details: type.description,
                            methods: type.methods,
                            properties: type.properties
                        });
                    }
                    return Promise.resolve({
                        typeName: typeName,
                        found: false
                    });
                }

                if (command === 'bsl.searchTypes') {
                    const query = args[0]?.query || '';
                    const limit = args[0]?.limit || 100;

                    if (!query) {
                        return Promise.resolve({ types: [], total: 0 });
                    }

                    // Поиск типов по query
                    const matchedTypes = Object.values(mockPlatformTypes)
                        .filter((type: any) =>
                            type.name.toLowerCase().includes(query.toLowerCase())
                        )
                        .slice(0, limit);

                    return Promise.resolve({
                        types: matchedTypes,
                        total: matchedTypes.length
                    });
                }

                if (command === 'bsl.buildIndex') {
                    return Promise.resolve({
                        success: true,
                        types_count: Object.keys(mockPlatformTypes).length
                    });
                }

                if (command === 'bsl.validateMethod') {
                    const objectType = args[0]?.object_type;
                    const methodName = args[0]?.method_name;

                    const type = mockPlatformTypes[objectType];
                    if (!type) {
                        return Promise.resolve({ valid: false, error: 'Type not found' });
                    }

                    const method = type.methods?.find((m: any) => m.name === methodName);
                    if (!method) {
                        return Promise.resolve({ valid: false, error: 'Method not found' });
                    }

                    return Promise.resolve({ valid: true, signature: method });
                }

                if (command === 'bsl.checkTypeCompatibility') {
                    const sourceType = args[0]?.source_type;
                    const targetType = args[0]?.target_type;

                    // Простая проверка совместимости
                    const compatible = sourceType === targetType ||
                                     targetType === 'Произвольный';

                    return Promise.resolve({ compatible, reason: compatible ? null : 'Types mismatch' });
                }

                if (command === 'bsl.incrementalUpdate') {
                    return Promise.resolve({ success: true, updated_types: 0 });
                }

                if (command === 'bsl.extractPlatformDocs') {
                    return Promise.resolve({ success: true, extracted_types: Object.keys(mockPlatformTypes).length });
                }

                if (command === 'bsl.getTypeStats') {
                    return Promise.resolve({
                        total_types: Object.keys(mockPlatformTypes).length,
                        by_certainty: {
                            Known: Object.keys(mockPlatformTypes).length,
                            Inferred: 0,
                            Unknown: 0
                        },
                        by_facet: {
                            Object: Object.keys(mockPlatformTypes).length
                        }
                    });
                }
            }

            return Promise.resolve(null);
        });

        // Mock sendCustomRequest для других запросов
        sendCustomRequestStub = sinon.stub().callsFake((method: string, params: any) => {
            if (method === 'bsl/buildIndex') {
                return Promise.resolve({
                    success: true,
                    types_count: 100,
                    message: 'Mock: Index built successfully'
                });
            } else if (method === 'bsl/getIndexState') {
                return Promise.resolve({
                    version: 1,
                    state: 'ready',
                    ready: true,
                    active_operation: null,
                    operation_id: null,
                    message: null,
                    updated_at_ms: Date.now()
                });
            } else if (method === 'bsl/validateMethod') {
                return Promise.resolve({
                    valid: true,
                    message: 'Mock: Method is valid'
                });
            } else if (method === 'bsl/checkTypeCompatibility') {
                return Promise.resolve({
                    compatible: false,
                    message: 'Mock: Types are not compatible'
                });
            } else if (method === 'bsl/incrementalUpdate') {
                return Promise.resolve({
                    success: true,
                    message: 'Mock: Incremental update completed'
                });
            } else if (method === 'bsl/extractPlatformDocs') {
                return Promise.resolve({
                    success: true,
                    types_count: 500,
                    message: 'Mock: Platform docs extracted'
                });
            }
            return Promise.resolve(null);
        });

        // Mock LSP Client
        const mockClient = {
            isRunning: () => true,
            sendRequest: sendRequestStub,
            state: 2 // Running
        };

        // Stub getLanguageClient
        const clientModule = await import('../../lsp/client');
        getLanguageClientStub = sinon.stub(clientModule, 'getLanguageClient');
        getLanguageClientStub.returns(mockClient as any);

        // Stub sendCustomRequest
        const sendCustomRequestOriginal = (clientModule as any).sendCustomRequest;
        if (sendCustomRequestOriginal) {
            sinon.stub(clientModule as any, 'sendCustomRequest').callsFake(sendCustomRequestStub);
        }
    });

    teardown(() => {
        // Восстановить все stubs
        sinon.restore();
    });

    /**
     * Тест custom request: bsl/queryType
     * Заменяет CLI: query_type
     */
    test('queryType should work via LSP', async function() {
        this.timeout(5000);

        const result = await queryType('Массив');

        assert.ok(result, 'Query result should not be null');
        assert.strictEqual(result.typeName, 'Массив', 'Type name should match');
        assert.strictEqual(typeof result.found, 'boolean', 'Found should be boolean');

        if (result.details) {
            assert.strictEqual(typeof result.details, 'string', 'Details should be string');
        }
    });

    /**
     * Тест custom request: bsl/buildIndex
     * Заменяет CLI: build_unified_index
     */
    test('buildIndex should work via LSP', async function() {
        this.timeout(5000);

        // Mock workspace path - тест работает с mocks
        const result = await buildIndex({ workspace_path: '/tmp/test-workspace' });

        assert.ok(result, 'Build index result should not be null');
        assert.strictEqual(typeof result.success, 'boolean', 'Success should be boolean');
        assert.strictEqual(typeof result.types_count, 'number', 'Types count should be number');
        assert.strictEqual(typeof result.message, 'string', 'Message should be string');

        // types_count может быть 0 для stub реализации
        assert.ok(result.types_count >= 0, 'Types count should be non-negative');
    });

    test('getIndexState should work via LSP', async function() {
        this.timeout(5000);

        const result = await getIndexState({});
        assert.ok(result, 'Get index state result should not be null');
        assert.strictEqual(result.version, 1, 'Version should be v1');
        assert.strictEqual(typeof result.state, 'string', 'State should be string');
        assert.strictEqual(typeof result.ready, 'boolean', 'Ready should be boolean');
        assert.strictEqual(result.active_operation, null, 'active_operation should be nullable');
        assert.strictEqual(result.operation_id, null, 'operation_id should be nullable');
        assert.strictEqual(result.message, null, 'message should be nullable');
        assert.strictEqual(typeof result.updated_at_ms, 'number', 'updated_at_ms should be number');
    });

    /**
     * Тест custom request: bsl/validateMethod
     * Заменяет CLI: check_type_compatibility (для методов)
     */
    test('validateMethod should work via LSP', async function() {
        this.timeout(5000);

        const result = await validateMethod('Массив', 'Добавить', ['Элемент']);

        assert.ok(result, 'Validate method result should not be null');
        assert.strictEqual(typeof result.valid, 'boolean', 'Valid should be boolean');
        assert.strictEqual(typeof result.message, 'string', 'Message should be string');
    });

    /**
     * Тест custom request: bsl/checkTypeCompatibility
     * Заменяет CLI: check_type_compatibility
     */
    test('checkTypeCompatibility should work via LSP', async function() {
        this.timeout(5000);

        const result = await checkTypeCompatibility('Число', 'Строка');

        assert.ok(result, 'Compatibility result should not be null');
        assert.strictEqual(typeof result.compatible, 'boolean', 'Compatible should be boolean');
        assert.strictEqual(typeof result.message, 'string', 'Message should be string');
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

        const result = await incrementalUpdate(configPath, platformVersion);

        assert.ok(result, 'Incremental update result should not be null');
        assert.strictEqual(typeof result.success, 'boolean', 'Success should be boolean');
        assert.strictEqual(typeof result.message, 'string', 'Message should be string');
    });

    /**
     * Тест custom request: bsl/extractPlatformDocs
     * Заменяет CLI: extract_platform_docs
     */
    test('extractPlatformDocs should work via LSP', async function() {
        this.timeout(5000);

        const testArchivePath = '/tmp/test_archive.zip';
        const platformVersion = '8.3.25';

        const result = await extractPlatformDocs(testArchivePath, platformVersion, true);

        assert.ok(result, 'Extract platform docs result should not be null');
        assert.strictEqual(typeof result.success, 'boolean', 'Success should be boolean');
        assert.strictEqual(typeof result.types_count, 'number', 'Types count should be number');
        assert.strictEqual(typeof result.message, 'string', 'Message should be string');

        assert.ok(result.types_count >= 0, 'Types count should be non-negative');
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
                // LSP не доступен, пропускаем тест
                return;
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
                // LSP не доступен, пропускаем тест
                return;
            } else {
                throw error;
            }
        }
    });
});
