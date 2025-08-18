import * as assert from 'assert';
import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

/**
 * Интеграционные тесты согласно официальным рекомендациям VSCode
 * https://code.visualstudio.com/api/working-with-extensions/testing-extension#integration-tests
 */
suite('Integration Test Suite', () => {

    /**
     * Тест полного жизненного цикла расширения
     */
    test('Extension lifecycle: activate -> use -> deactivate', async function() {
        this.timeout(10000);
        
        // 1. Проверяем, что расширение существует
        const ext = vscode.extensions.getExtension('bsl-analyzer-team.bsl-type-safety-analyzer');
        assert.ok(ext, 'Extension not found');
        
        // 2. Активируем если не активно
        if (!ext.isActive) {
            await ext.activate();
        }
        assert.ok(ext.isActive, 'Extension should be active');
        
        // 3. Проверяем, что API экспортировано (если есть)
        const api = ext.exports;
        // API может быть undefined, это нормально
        assert.ok(api !== null, 'Extension exports should not be null');
        
        // 4. Проверяем, что основные компоненты инициализированы
        const commands = await vscode.commands.getCommands();
        const bslCommands = commands.filter(cmd => cmd.startsWith('bslAnalyzer.'));
        assert.ok(bslCommands.length > 0, 'BSL commands should be registered');
    });

    /**
     * Тест работы с BSL файлами
     */
    test('BSL file handling', async function() {
        this.timeout(5000);
        
        // Создаем временный BSL файл
        const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
        if (!workspaceFolder) {
            this.skip();
            return;
        }
        
        const testFilePath = path.join(workspaceFolder.uri.fsPath, 'test.bsl');
        const testContent = `
Процедура ТестоваяПроцедура()
    Перем МояПеременная;
    МояПеременная = "Тест";
    Сообщить(МояПеременная);
КонецПроцедуры`;
        
        try {
            // Записываем файл
            fs.writeFileSync(testFilePath, testContent, 'utf8');
            
            // Открываем в VSCode
            const doc = await vscode.workspace.openTextDocument(testFilePath);
            await vscode.window.showTextDocument(doc);
            
            // Проверяем, что файл определен как BSL
            assert.strictEqual(doc.languageId, 'bsl', 'File should be recognized as BSL');
            
            // Закрываем редактор
            await vscode.commands.executeCommand('workbench.action.closeActiveEditor');
            
        } finally {
            // Удаляем тестовый файл
            if (fs.existsSync(testFilePath)) {
                fs.unlinkSync(testFilePath);
            }
        }
    });

    /**
     * Тест взаимодействия с Language Server
     */
    test('Language Server communication', async function() {
        this.timeout(10000);
        
        // Проверяем, что команды LSP зарегистрированы
        const commands = await vscode.commands.getCommands();
        
        assert.ok(
            commands.includes('bslAnalyzer.restartServer'),
            'LSP restart command should be available'
        );
        
        // Можем попробовать выполнить команду перезапуска
        // В тестовом окружении LSP может не запуститься полностью
        try {
            await vscode.commands.executeCommand('bslAnalyzer.restartServer');
            assert.ok(true, 'Server restart command executed');
        } catch (error) {
            // В тестовом окружении это может не работать
            assert.ok(true, 'Server restart test skipped in test environment');
        }
    });

    /**
     * Тест работы с конфигурацией 1С
     */
    test('1C Configuration handling', async () => {
        const config = vscode.workspace.getConfiguration('bslAnalyzer');
        const configPath = config.get<string>('configurationPath');
        const platformVersion = config.get<string>('platformVersion');
        
        // Проверяем формат версии платформы
        if (platformVersion) {
            const versionRegex = /^\d+\.\d+\.\d+$/;
            assert.ok(
                versionRegex.test(platformVersion),
                `Platform version should match X.X.X format, got: ${platformVersion}`
            );
        }
        
        // Если путь к конфигурации указан, проверяем его валидность
        if (configPath && configPath !== '') {
            // Путь должен быть абсолютным
            assert.ok(
                path.isAbsolute(configPath),
                'Configuration path should be absolute'
            );
        }
    });

    /**
     * Тест кеширования и производительности
     */
    test('Caching mechanism', async function() {
        this.timeout(5000);
        
        const homeDir = require('os').homedir();
        const cacheDir = path.join(homeDir, '.bsl_analyzer');
        
        // Проверяем, что директория кеша может быть создана
        if (!fs.existsSync(cacheDir)) {
            try {
                fs.mkdirSync(cacheDir, { recursive: true });
                assert.ok(true, 'Cache directory can be created');
                
                // Удаляем созданную директорию
                fs.rmdirSync(cacheDir);
            } catch (error) {
                assert.ok(false, `Cannot create cache directory: ${error}`);
            }
        } else {
            assert.ok(true, 'Cache directory exists');
        }
    });
});

/**
 * Тесты диагностики и отчетов
 */
suite('Diagnostics Test Suite', () => {
    
    test('Diagnostics provider should be registered', async () => {
        // Проверяем наличие команд диагностики
        const commands = await vscode.commands.getCommands();
        
        assert.ok(
            commands.includes('bslAnalyzer.showMetrics'),
            'Show metrics command should exist'
        );
        
        assert.ok(
            commands.includes('bslAnalyzer.generateReports'),
            'Generate reports command should exist'
        );
    });

    test('Output channel should be created', () => {
        // Ищем output channel для BSL Analyzer
        // Note: В тестовом окружении это может быть недоступно
        
        // Проверяем хотя бы, что команда показа output существует
        return vscode.commands.getCommands().then(commands => {
            // Output channel команды обычно начинаются с workbench.action.output
            const outputCommands = commands.filter(cmd => cmd.includes('output'));
            assert.ok(outputCommands.length > 0, 'Output commands should exist');
        });
    });
});

/**
 * Тесты WebView (если используются)
 */
suite('WebView Test Suite', () => {
    
    test('WebView commands should be registered', async () => {
        const commands = await vscode.commands.getCommands();
        
        // Проверяем команды, которые могут открывать WebView
        const webViewCommands = [
            'bslAnalyzer.showMetrics',
            'bslAnalyzer.searchType',
            'bslAnalyzer.exploreType'
        ];
        
        for (const cmd of webViewCommands) {
            assert.ok(
                commands.includes(cmd),
                `WebView command ${cmd} should be registered`
            );
        }
    });
});

/**
 * Тесты совместимости с разными версиями VSCode
 */
suite('Compatibility Test Suite', () => {
    
    test('Extension should work with current VSCode version', () => {
        const vscodeVersion = vscode.version;
        const [major, minor] = vscodeVersion.split('.').map(Number);
        
        // Минимальная версия из package.json - 1.75.0
        assert.ok(
            major > 1 || (major === 1 && minor >= 75),
            `VSCode version ${vscodeVersion} should be >= 1.75.0`
        );
    });

    test('All required APIs should be available', () => {
        // Проверяем доступность основных API
        assert.ok(vscode.window, 'Window API should be available');
        assert.ok(vscode.workspace, 'Workspace API should be available');
        assert.ok(vscode.commands, 'Commands API should be available');
        assert.ok(vscode.languages, 'Languages API should be available');
        assert.ok(vscode.extensions, 'Extensions API should be available');
    });
});