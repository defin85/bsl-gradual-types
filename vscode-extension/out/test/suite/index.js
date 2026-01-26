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
exports.run = void 0;
const path = __importStar(require("path"));
const vscode = __importStar(require("vscode"));
const glob = require('glob');
async function run() {
    // Настроить mock конфигурацию для тестов
    try {
        const config = vscode.workspace.getConfiguration('bslAnalyzer');
        // Если runTest.ts уже положил реальные пути в user settings (BSL_TEST_USE_REAL_FIXTURES=1),
        // не перетираем их пустыми значениями.
        const useRealFixtures = process.env.BSL_TEST_USE_REAL_FIXTURES === '1';
        if (!useRealFixtures) {
            // Установить пустую строку для platformDocsArchive (тесты используют mocks)
            await config.update('platformDocsArchive', '', // Пустая строка - тесты работают с mocks
            vscode.ConfigurationTarget.Global);
        }
        console.log('[Test Setup] Mock configuration applied');
    }
    catch (error) {
        console.warn('[Test Setup] Failed to apply mock configuration:', error);
    }
    // Create the mocha test
    const Mocha = require('mocha');
    const mocha = new Mocha({
        ui: 'tdd',
        color: true
    });
    const testsRoot = path.resolve(__dirname, '..');
    return new Promise((resolve, reject) => {
        glob('**/**.test.js', { cwd: testsRoot }, (err, files) => {
            if (err) {
                return reject(err);
            }
            // Add files to the test suite
            files.forEach((f) => mocha.addFile(path.resolve(testsRoot, f)));
            try {
                // Run the mocha test
                mocha.run((failures) => {
                    if (failures > 0) {
                        reject(new Error(`${failures} tests failed.`));
                    }
                    else {
                        resolve();
                    }
                });
            }
            catch (err) {
                console.error(err);
                reject(err);
            }
        });
    });
}
exports.run = run;
//# sourceMappingURL=index.js.map