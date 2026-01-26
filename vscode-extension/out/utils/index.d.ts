/**
 * Экспорт всех утилит из одного места
 */
export { getBinaryPath, setOutputChannel as setBinaryPathOutputChannel } from './binaryPath';
export { parseMethodCall, extractTypeName, type MethodCallInfo } from './parser';
export { getConfigurationPath, getPlatformVersion, getPlatformDocsArchive, getAutoReindexEnabled, setOutputChannel as setConfigOutputChannel } from './config';
import * as vscode from 'vscode';
/**
 * Инициализирует output channel для всех утилит
 */
export declare function initializeUtils(outputChannel: vscode.OutputChannel): void;
export { findConfigurations, findMainConfiguration, selectConfiguration, autoDetectConfiguration, ConfigurationInfo } from './configurationFinder';
//# sourceMappingURL=index.d.ts.map