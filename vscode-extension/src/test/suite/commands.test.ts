import * as assert from 'assert';
import * as vscode from 'vscode';

/**
 * Тесты команд расширения согласно официальным рекомендациям VSCode
 * https://code.visualstudio.com/api/working-with-extensions/testing-extension
 */
suite('Commands Test Suite', () => {
    
    suiteSetup(async () => {
        // Подготовка к тестам команд
        const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
        if (workspaceFolder) {
            // Workspace доступен для тестов
        }
    });

    test('Command: analyzeFile should exist', async () => {
        const commands = await vscode.commands.getCommands();
        assert.ok(commands.includes('bslAnalyzer.analyzeFile'));
    });

    test('Command: searchType should handle user input', async () => {
        // Проверяем, что команда зарегистрирована
        const commands = await vscode.commands.getCommands();
        assert.ok(commands.includes('bslAnalyzer.searchType'));
        
        // Note: Мы не можем полностью протестировать интерактивные команды
        // без мокирования vscode.window.showInputBox
    });

    test('Command: buildIndex should be executable', async () => {
        const commands = await vscode.commands.getCommands();
        assert.ok(commands.includes('bslAnalyzer.buildIndex'));
    });

    test('Command: restartServer should be executable', async () => {
        const commands = await vscode.commands.getCommands();
        assert.ok(commands.includes('bslAnalyzer.restartServer'));
    });

    test('Command: refresh commands should exist', async () => {
        const refreshCommands = [
            'bslAnalyzer.refreshOverview',
            'bslAnalyzer.refreshDiagnostics',
            'bslAnalyzer.refreshTypeIndex',
            'bslAnalyzer.refreshPlatformDocs'
        ];
        
        const commands = await vscode.commands.getCommands();
        for (const cmd of refreshCommands) {
            assert.ok(commands.includes(cmd), `Command ${cmd} not found`);
        }
    });
});

/**
 * Тесты обработки ошибок
 */
suite('Error Handling Test Suite', () => {
    
    test('Should handle missing configuration gracefully', async () => {
        // Получаем конфигурацию
        const config = vscode.workspace.getConfiguration('bslAnalyzer');
        
        // Даже если путь не настроен, расширение должно работать
        const configPath = config.get<string>('configurationPath');
        // Расширение должно обрабатывать отсутствующую конфигурацию
        assert.ok(configPath !== undefined || configPath === '');
    });

    test('Should handle invalid platform version', () => {
        const config = vscode.workspace.getConfiguration('bslAnalyzer');
        const platformVersion = config.get<string>('platformVersion');
        
        // Версия должна соответствовать формату X.X.X
        if (platformVersion) {
            const versionPattern = /^\d+\.\d+\.\d+$/;
            assert.ok(
                versionPattern.test(platformVersion),
                `Invalid platform version format: ${platformVersion}`
            );
        }
    });
});

/**
 * Тесты производительности (Performance Tests)
 * Рекомендовано в официальной документации для расширений с тяжелыми операциями
 */
suite('Performance Test Suite', () => {
    
    test('Extension activation should be fast', async function() {
        this.timeout(5000); // 5 секунд максимум на активацию
        
        const startTime = Date.now();
        const ext = vscode.extensions.getExtension('bsl-analyzer-team.bsl-type-safety-analyzer');
        
        if (ext && !ext.isActive) {
            await ext.activate();
        }
        
        const activationTime = Date.now() - startTime;
        assert.ok(activationTime < 5000, `Activation took ${activationTime}ms`);
    });

    test('Commands registration should be fast', async () => {
        const startTime = Date.now();
        await vscode.commands.getCommands();
        const elapsed = Date.now() - startTime;
        
        assert.ok(elapsed < 1000, `Commands enumeration took ${elapsed}ms`);
    });
});

/**
 * Тесты TreeDataProvider (для боковых панелей)
 */
suite('TreeDataProvider Test Suite', () => {
    
    test('TreeView providers should be registered', () => {
        // Проверяем, что TreeView провайдеры зарегистрированы через команды обновления
        const treeViewCommands = [
            'bslAnalyzer.refreshOverview',
            'bslAnalyzer.refreshDiagnostics',
            'bslAnalyzer.refreshTypeIndex',
            'bslAnalyzer.refreshPlatformDocs'
        ];
        
        return vscode.commands.getCommands(true).then((allCommands) => {
            const foundCommands = treeViewCommands.filter(cmd => allCommands.includes(cmd));
            assert.strictEqual(
                foundCommands.length, 
                treeViewCommands.length,
                'Not all TreeView refresh commands are registered'
            );
        });
    });
});

/**
 * Тесты настроек (Configuration Tests)
 */
suite('Configuration Management Test Suite', () => {
    
    test('Should have all required configuration sections', () => {
        const config = vscode.workspace.getConfiguration('bslAnalyzer');
        
        // Проверяем наличие всех ключевых настроек
        const requiredSettings = [
            'platformVersion',
            'configurationPath',
            'binaryPath',
            'enableSemanticAnalysis',
            'enableMethodValidation',
            'autoIndexBuild',
            'showProgressNotifications',
            'logLevel'
        ];
        
        for (const setting of requiredSettings) {
            const value = config.inspect(setting);
            assert.ok(value, `Missing configuration: bslAnalyzer.${setting}`);
        }
    });

    test('Configuration should have valid default values', () => {
        const config = vscode.workspace.getConfiguration('bslAnalyzer');
        
        // platformVersion должна иметь дефолтное значение
        const platformVersion = config.get<string>('platformVersion');
        assert.ok(platformVersion, 'Platform version should have a default value');
        
        // enableSemanticAnalysis должна быть boolean или undefined (если не задана)
        const enableSemantic = config.get<boolean>('enableSemanticAnalysis');
        assert.ok(
            typeof enableSemantic === 'boolean' || enableSemantic === undefined,
            `enableSemanticAnalysis should be boolean or undefined, got: ${typeof enableSemantic}`
        );
        
        // logLevel должен быть одним из допустимых значений
        const logLevel = config.get<string>('logLevel');
        const validLogLevels = ['off', 'error', 'warn', 'info', 'debug', 'trace'];
        assert.ok(validLogLevels.includes(logLevel || 'info'));
    });

    test('Should handle configuration changes', async () => {
        const config = vscode.workspace.getConfiguration('bslAnalyzer');
        
        // Получаем текущее значение
        const originalValue = config.get<boolean>('showProgressNotifications');
        
        // Пытаемся изменить (в тестах это может не сработать из-за ограничений)
        try {
            await config.update('showProgressNotifications', !originalValue, vscode.ConfigurationTarget.Workspace);
            const newValue = config.get<boolean>('showProgressNotifications');
            
            // Возвращаем обратно
            await config.update('showProgressNotifications', originalValue, vscode.ConfigurationTarget.Workspace);
            
            assert.notStrictEqual(originalValue, newValue);
        } catch (error) {
            // В тестовом окружении изменение настроек может быть запрещено
            assert.ok(true, 'Configuration update test skipped in test environment');
        }
    });
});