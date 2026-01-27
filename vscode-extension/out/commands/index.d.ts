import * as vscode from 'vscode';
export declare function initializeCommands(channel: vscode.OutputChannel): void;
export declare function registerCommands(context: vscode.ExtensionContext): Promise<void>;
export { registerSemanticVisualization } from './semanticVisualization';
export { registerParseConfigurationCommand } from './parseConfiguration';
export { registerAnalysisCommands } from './analysis';
export { registerSearchCommands } from './search';
export { registerIndexCommands } from './index-commands';
export { registerConfigurationCommands } from './configuration';
export { registerDebugCommands } from './debug';
export { registerCacheCommands } from './cache';
export { registerObservabilityCommands } from './observability';
//# sourceMappingURL=index.d.ts.map