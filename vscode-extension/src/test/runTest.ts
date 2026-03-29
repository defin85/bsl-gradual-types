import * as path from 'path';
import * as fs from 'fs';
import { runTests } from '@vscode/test-electron';

function addLaunchArgIfMissing(launchArgs: string[], flag: string): void {
    if (!launchArgs.includes(flag)) {
        launchArgs.push(flag);
    }
}

function collectLaunchArgs(): string[] {
    const launchArgs = (process.env.BSL_TEST_ELECTRON_LAUNCH_ARGS ?? '')
        .split(/\s+/)
        .filter(Boolean);

    // act/docker jobs run the VS Code test host as root, so Electron must disable sandboxing.
    if (typeof process.getuid === 'function' && process.getuid() === 0) {
        addLaunchArgIfMissing(launchArgs, '--no-sandbox');
    }

    return launchArgs;
}

function ensureTestSettings(
    extensionDevelopmentPath: string,
): void {
    // test-electron использует `.vscode-test` рядом с extensionDevelopmentPath по умолчанию.
    const userDataDir = path.resolve(extensionDevelopmentPath, '.vscode-test', 'user-data');
    const settingsPath = path.resolve(userDataDir, 'User', 'settings.json');

    fs.mkdirSync(path.dirname(settingsPath), { recursive: true });

    const repoRoot = path.resolve(extensionDevelopmentPath, '..');
    const defaultPlatformDocs = path.resolve(repoRoot, 'examples', 'syntax_helper');
    const defaultConfigPath = path.resolve(repoRoot, 'examples', 'conf', 'conf_test');

    const platformDocsArchive = process.env.BSL_TEST_PLATFORM_DOCS_ARCHIVE || defaultPlatformDocs;
    const configurationPath = process.env.BSL_TEST_CONFIGURATION_PATH || defaultConfigPath;
    const useRealFixtures = process.env.BSL_TEST_USE_REAL_FIXTURES === '1';

    // В тестах по умолчанию стараемся НЕ грузить реальные данные (быстро и детерминированно).
    // Если нужно посмотреть реальный прогресс парсинга docs/config — запускай с BSL_TEST_USE_REAL_FIXTURES=1
    // (и при необходимости переопредели пути через env).
    const settings: Record<string, unknown> = useRealFixtures
        ? {
              'bslAnalyzer.platformDocsArchive': platformDocsArchive,
              'bslAnalyzer.configurationPath': configurationPath,
              // Чтобы не запускать buildIndex автоматически в тестовом инстансе:
              'bslAnalyzer.autoIndexBuild': false,
              'bslAnalyzer.serverTrace': 'verbose',
          }
        : {
              'bslAnalyzer.platformDocsArchive': '',
              'bslAnalyzer.configurationPath': '',
              'bslAnalyzer.autoIndexBuild': false,
          };

    fs.writeFileSync(settingsPath, JSON.stringify(settings, null, 2), 'utf-8');
}

async function main() {
    try {
        // Установить переменные окружения для тестового режима
        process.env.NODE_ENV = 'test';
        process.env.VSCODE_TEST_MODE = '1';

        // The folder containing the Extension Manifest package.json
        // Passed to `--extensionDevelopmentPath`
        const extensionDevelopmentPath = path.resolve(__dirname, '../../');

        ensureTestSettings(extensionDevelopmentPath);

        // The path to test runner
        // Passed to --extensionTestsPath
        const extensionTestsPath = path.resolve(__dirname, './suite/index');
        const launchArgs = collectLaunchArgs();

        // Download VS Code, unzip it and run the integration test
        await runTests({ extensionDevelopmentPath, extensionTestsPath, launchArgs });
    } catch (err) {
        console.error('Failed to run tests');
        process.exit(1);
    }
}

main();
