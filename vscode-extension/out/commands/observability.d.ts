import * as vscode from 'vscode';
import { CommandHandler } from '../types';
/**
 * Register observability/diagnostic commands.
 */
export declare function registerObservabilityCommands(context: vscode.ExtensionContext, safeRegisterCommand: (commandId: string, callback: CommandHandler) => Promise<vscode.Disposable | null>, outputChannel: vscode.OutputChannel): void;
//# sourceMappingURL=observability.d.ts.map
