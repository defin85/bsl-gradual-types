import * as vscode from 'vscode';
import { CommandHandler } from '../types';
export declare function registerCacheCommands(context: vscode.ExtensionContext, safeRegisterCommand: (commandId: string, callback: CommandHandler) => Promise<vscode.Disposable | null>, outputChannel: vscode.OutputChannel): void;
//# sourceMappingURL=cache.d.ts.map