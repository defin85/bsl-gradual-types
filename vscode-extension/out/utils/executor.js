"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.executeBslCommand = exports.setOutputChannel = void 0;
const child_process_1 = require("child_process");
const binaryPath_1 = require("./binaryPath");
let outputChannel;
function setOutputChannel(channel) {
    outputChannel = channel;
}
exports.setOutputChannel = setOutputChannel;
/**
 * Выполняет команду BSL Analyzer и возвращает результат
 * @param command Имя команды (бинарного файла)
 * @param args Аргументы командной строки
 * @param extensionContext Контекст расширения
 * @returns Promise с результатом выполнения
 */
function executeBslCommand(command, args, extensionContext) {
    return new Promise((resolve, reject) => {
        const binaryPath = (0, binaryPath_1.getBinaryPath)(command, extensionContext);
        outputChannel?.appendLine(`Executing: ${binaryPath} ${args.join(' ')}`);
        const child = (0, child_process_1.spawn)(binaryPath, args, {
            shell: true,
            env: { ...process.env, RUST_LOG: 'info' }
        });
        let stdout = '';
        let stderr = '';
        child.stdout.on('data', (data) => {
            const text = data.toString();
            stdout += text;
            // Показываем прогресс в output channel
            if (text.includes('Processing') || text.includes('Extracted')) {
                outputChannel?.appendLine(text.trim());
            }
        });
        child.stderr.on('data', (data) => {
            stderr += data.toString();
        });
        child.on('close', (code) => {
            outputChannel?.appendLine(`Command completed with code: ${code}`);
            if (code === 0) {
                outputChannel?.appendLine(`Output: ${stdout.substring(0, 500)}...`);
                resolve(stdout);
            }
            else {
                const errorMsg = stderr || stdout || `Command failed with code ${code}`;
                outputChannel?.appendLine(`Error: ${errorMsg}`);
                reject(new Error(errorMsg));
            }
        });
        child.on('error', (err) => {
            outputChannel?.appendLine(`Failed to execute command: ${err.message}`);
            reject(err);
        });
    });
}
exports.executeBslCommand = executeBslCommand;
//# sourceMappingURL=executor.js.map