import * as vscode from 'vscode';
import { CommandHandler } from '../types';
/**
 * Register index-related commands (build, stats, incremental update)
 */
export declare function registerIndexCommands(context: vscode.ExtensionContext, safeRegisterCommand: (commandId: string, callback: CommandHandler) => Promise<vscode.Disposable | null>, outputChannel: vscode.OutputChannel): void;
//# sourceMappingURL=index-commands.d.ts.map