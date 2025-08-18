import * as vscode from 'vscode';
import { spawn } from 'child_process';
import { getBinaryPath } from './binaryPath';

let outputChannel: vscode.OutputChannel;

export function setOutputChannel(channel: vscode.OutputChannel) {
    outputChannel = channel;
}

/**
 * Выполняет команду BSL Analyzer и возвращает результат
 * @param command Имя команды (бинарного файла)
 * @param args Аргументы командной строки
 * @param extensionContext Контекст расширения
 * @returns Promise с результатом выполнения
 */
export function executeBslCommand(
    command: string, 
    args: string[], 
    extensionContext?: vscode.ExtensionContext
): Promise<string> {
    return new Promise((resolve, reject) => {
        const binaryPath = getBinaryPath(command, extensionContext);
        
        outputChannel?.appendLine(`Executing: ${binaryPath} ${args.join(' ')}`);
        
        const child = spawn(binaryPath, args, {
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
            } else {
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