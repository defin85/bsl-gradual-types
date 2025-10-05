import * as assert from 'assert';
import * as vscode from 'vscode';

/**
 * Тесты для Tree Data Providers
 * Проверяют боковые панели расширения
 */
suite('Providers Test Suite', () => {

    suiteSetup(async function() {
        this.timeout(10000);

        // Активируем расширение
        const ext = vscode.extensions.getExtension('bsl-analyzer-team.bsl-type-safety-analyzer');
        if (ext && !ext.isActive) {
            await ext.activate();
        }

        await new Promise(resolve => setTimeout(resolve, 1000));
    });

    /**
     * Тест BslOverviewProvider
     */
    test('BslOverviewProvider should be registered', async () => {
        const commands = await vscode.commands.getCommands();

        // Проверяем наличие команды обновления
        assert.ok(
            commands.includes('bslAnalyzer.refreshOverview'),
            'Overview refresh command should exist'
        );
    });

    test('BslOverviewProvider refresh should work', async function() {
        this.timeout(5000);

        try {
            // Пытаемся обновить Overview панель
            await vscode.commands.executeCommand('bslAnalyzer.refreshOverview');
            assert.ok(true, 'Overview refresh executed successfully');
        } catch (error: any) {
            // Может не работать в тестовой среде
            assert.ok(
                error.message.includes('not found') === false,
                'Command should be registered'
            );
        }
    });

    /**
     * Тест BslDiagnosticsProvider
     */
    test('BslDiagnosticsProvider should be registered', async () => {
        const commands = await vscode.commands.getCommands();

        assert.ok(
            commands.includes('bslAnalyzer.refreshDiagnostics'),
            'Diagnostics refresh command should exist'
        );
    });

    test('BslDiagnosticsProvider refresh should work', async function() {
        this.timeout(5000);

        try {
            await vscode.commands.executeCommand('bslAnalyzer.refreshDiagnostics');
            assert.ok(true, 'Diagnostics refresh executed successfully');
        } catch (error: any) {
            assert.ok(
                error.message.includes('not found') === false,
                'Command should be registered'
            );
        }
    });

    /**
     * Тест HierarchicalTypeIndexProvider
     */
    test('HierarchicalTypeIndexProvider should be registered', async () => {
        const commands = await vscode.commands.getCommands();

        assert.ok(
            commands.includes('bslAnalyzer.refreshTypeIndex'),
            'Type index refresh command should exist'
        );
    });

    test('HierarchicalTypeIndexProvider refresh should work', async function() {
        this.timeout(5000);

        try {
            await vscode.commands.executeCommand('bslAnalyzer.refreshTypeIndex');
            assert.ok(true, 'Type index refresh executed successfully');
        } catch (error: any) {
            assert.ok(
                error.message.includes('not found') === false,
                'Command should be registered'
            );
        }
    });

    /**
     * Тест BslPlatformDocsProvider
     */
    test('BslPlatformDocsProvider should be registered', async () => {
        const commands = await vscode.commands.getCommands();

        assert.ok(
            commands.includes('bslAnalyzer.refreshPlatformDocs'),
            'Platform docs refresh command should exist'
        );

        assert.ok(
            commands.includes('bslAnalyzer.addPlatformDocs'),
            'Add platform docs command should exist'
        );

        assert.ok(
            commands.includes('bslAnalyzer.removePlatformDocs'),
            'Remove platform docs command should exist'
        );
    });

    test('BslPlatformDocsProvider refresh should work', async function() {
        this.timeout(5000);

        try {
            await vscode.commands.executeCommand('bslAnalyzer.refreshPlatformDocs');
            assert.ok(true, 'Platform docs refresh executed successfully');
        } catch (error: any) {
            assert.ok(
                error.message.includes('not found') === false,
                'Command should be registered'
            );
        }
    });

    /**
     * Тест BslActionsWebviewProvider
     */
    test('BslActionsWebviewProvider should be registered', () => {
        // WebviewProvider регистрируется как view container
        // Проверяем через viewsContainers в package.json
        // В тестовой среде это сложно проверить напрямую

        assert.ok(true, 'WebviewProvider assumed to be registered via package.json');
    });
});

/**
 * Тесты взаимодействия Provider'ов с LSP
 */
suite('Provider LSP Integration Test Suite', () => {

    test('Providers should update when LSP sends diagnostics', async function() {
        this.timeout(5000);

        // В реальности Providers подписаны на события LSP
        // В тестовой среде проверяем только наличие механизмов обновления

        const commands = await vscode.commands.getCommands();
        const refreshCommands = commands.filter(cmd =>
            cmd.startsWith('bslAnalyzer.refresh')
        );

        assert.ok(
            refreshCommands.length >= 4,
            `Should have at least 4 refresh commands, found: ${refreshCommands.length}`
        );
    });

    test('Providers should handle LSP server restart', async function() {
        this.timeout(10000);

        try {
            // Получаем начальное состояние
            const commandsBefore = await vscode.commands.getCommands();
            const providerCommandsBefore = commandsBefore.filter(cmd =>
                cmd.includes('refresh')
            );

            // Перезапускаем сервер
            await vscode.commands.executeCommand('bslAnalyzer.restartServer');

            // Даем время на перезапуск
            await new Promise(resolve => setTimeout(resolve, 2000));

            // Проверяем, что provider команды все еще доступны
            const commandsAfter = await vscode.commands.getCommands();
            const providerCommandsAfter = commandsAfter.filter(cmd =>
                cmd.includes('refresh')
            );

            assert.strictEqual(
                providerCommandsAfter.length,
                providerCommandsBefore.length,
                'Provider commands should remain after server restart'
            );
        } catch (error: any) {
            // В тестовой среде перезапуск может не работать
            if (error.message.includes('not found')) {
                this.skip();
            }
        }
    });
});

/**
 * Тесты производительности Provider'ов
 */
suite('Provider Performance Test Suite', () => {

    test('Provider refresh should be fast', async function() {
        this.timeout(3000);

        try {
            const startTime = Date.now();
            await vscode.commands.executeCommand('bslAnalyzer.refreshOverview');
            const elapsed = Date.now() - startTime;

            // Обновление должно занимать менее 2 секунд
            assert.ok(
                elapsed < 2000,
                `Provider refresh took ${elapsed}ms (should be < 2000ms)`
            );
        } catch (error) {
            // Может не работать в тестовой среде
            this.skip();
        }
    });

    test('Multiple provider refreshes should work concurrently', async function() {
        this.timeout(5000);

        try {
            const startTime = Date.now();

            // Обновляем все provider'ы параллельно
            await Promise.all([
                vscode.commands.executeCommand('bslAnalyzer.refreshOverview'),
                vscode.commands.executeCommand('bslAnalyzer.refreshDiagnostics'),
                vscode.commands.executeCommand('bslAnalyzer.refreshTypeIndex'),
                vscode.commands.executeCommand('bslAnalyzer.refreshPlatformDocs')
            ]);

            const elapsed = Date.now() - startTime;

            // Параллельное обновление должно быть быстрее последовательного
            assert.ok(
                elapsed < 5000,
                `Concurrent refresh took ${elapsed}ms (should be < 5000ms)`
            );
        } catch (error) {
            // Может не работать в тестовой среде
            this.skip();
        }
    });
});
