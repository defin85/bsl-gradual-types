import * as vscode from 'vscode';
import { CommandHandler } from '../types';
/**
 * Register analysis-related commands
 */
export declare function registerAnalysisCommands(context: vscode.ExtensionContext, safeRegisterCommand: (commandId: string, callback: CommandHandler) => Promise<vscode.Disposable | null>, outputChannel: vscode.OutputChannel): void;
//# sourceMappingURL=analysis.d.ts.map