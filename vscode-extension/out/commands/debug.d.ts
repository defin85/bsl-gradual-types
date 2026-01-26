import * as vscode from 'vscode';
import { CommandHandler } from '../types';
/**
 * Register debug and utility commands
 */
export declare function registerDebugCommands(context: vscode.ExtensionContext, safeRegisterCommand: (commandId: string, callback: CommandHandler) => Promise<vscode.Disposable | null>, outputChannel: vscode.OutputChannel): void;
//# sourceMappingURL=debug.d.ts.map