import * as path from 'path';
import * as vscode from 'vscode';
const glob = require('glob');

export async function run(): Promise<void> {
    // Настроить mock конфигурацию для тестов
    try {
        const config = vscode.workspace.getConfiguration('bslAnalyzer');

        // Если runTest.ts уже положил реальные пути в user settings (BSL_TEST_USE_REAL_FIXTURES=1),
        // не перетираем их пустыми значениями.
        const useRealFixtures = process.env.BSL_TEST_USE_REAL_FIXTURES === '1';
        if (!useRealFixtures) {
            // Установить пустую строку для platformDocsArchive (тесты используют mocks)
            await config.update(
                'platformDocsArchive',
                '', // Пустая строка - тесты работают с mocks
                vscode.ConfigurationTarget.Global
            );
        }

        console.log('[Test Setup] Mock configuration applied');
    } catch (error) {
        console.warn('[Test Setup] Failed to apply mock configuration:', error);
    }

    // Create the mocha test
    const Mocha = require('mocha');
    const mocha = new Mocha({
        ui: 'tdd',
        color: true
    });
    const grepPattern = process.env.BSL_TEST_GREP?.trim();

    if (grepPattern) {
        mocha.grep(new RegExp(grepPattern));
        console.log(`[Test Setup] Applying BSL_TEST_GREP=${grepPattern}`);
    }

    const testsRoot = path.resolve(__dirname, '..');

    return new Promise((resolve, reject) => {
        glob('**/**.test.js', { cwd: testsRoot }, (err: Error | null, files: string[]) => {
            if (err) {
                return reject(err);
            }

            // Add files to the test suite
            files.forEach((f: string) => mocha.addFile(path.resolve(testsRoot, f)));

            try {
                // Run the mocha test
                mocha.run((failures: number) => {
                    if (failures > 0) {
                        reject(new Error(`${failures} tests failed.`));
                    } else {
                        resolve();
                    }
                });
            } catch (err) {
                console.error(err);
                reject(err);
            }
        });
    });
}
