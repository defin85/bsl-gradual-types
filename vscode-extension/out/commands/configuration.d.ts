import * as vscode from 'vscode';
import { CommandHandler } from '../types';
/**
 * Register configuration-related commands
 */
export declare function registerConfigurationCommands(context: vscode.ExtensionContext, safeRegisterCommand: (commandId: string, callback: CommandHandler) => Promise<vscode.Disposable | null>, _outputChannel: vscode.OutputChannel): void;
//# sourceMappingURL=configuration.d.ts.map