import * as vscode from 'vscode';
export declare function setOutputChannel(channel: vscode.OutputChannel): void;
/**
 * Выполняет команду BSL Analyzer и возвращает результат
 * @param command Имя команды (бинарного файла)
 * @param args Аргументы командной строки
 * @param extensionContext Контекст расширения
 * @returns Promise с результатом выполнения
 */
export declare function executeBslCommand(command: string, args: string[], extensionContext?: vscode.ExtensionContext): Promise<string>;
//# sourceMappingURL=executor.d.ts.map