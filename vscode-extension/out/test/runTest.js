"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
Object.defineProperty(exports, "__esModule", { value: true });
const path = __importStar(require("path"));
const fs = __importStar(require("fs"));
const test_electron_1 = require("@vscode/test-electron");
function ensureTestSettings(extensionDevelopmentPath) {
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
    const settings = useRealFixtures
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
        // Download VS Code, unzip it and run the integration test
        await (0, test_electron_1.runTests)({ extensionDevelopmentPath, extensionTestsPath });
    }
    catch (err) {
        console.error('Failed to run tests');
        process.exit(1);
    }
}
main();
//# sourceMappingURL=runTest.js.map