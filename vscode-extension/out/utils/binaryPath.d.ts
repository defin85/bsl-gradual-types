import * as vscode from 'vscode';
export declare function setOutputChannel(channel: vscode.OutputChannel): void;
/**
 * Получает путь к бинарному файлу BSL Analyzer
 * @param binaryName Имя бинарного файла (без расширения)
 * @param extensionContext Контекст расширения
 * @returns Полный путь к бинарному файлу
 */
export declare function getBinaryPath(binaryName: string, extensionContext?: vscode.ExtensionContext): string;
//# sourceMappingURL=binaryPath.d.ts.map