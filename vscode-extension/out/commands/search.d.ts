import * as vscode from 'vscode';
import { CommandHandler } from '../types';
/**
 * Register type search related commands
 */
export declare function registerSearchCommands(context: vscode.ExtensionContext, safeRegisterCommand: (commandId: string, callback: CommandHandler) => Promise<vscode.Disposable | null>, _outputChannel: vscode.OutputChannel): void;
//# sourceMappingURL=search.d.ts.map