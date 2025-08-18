/**
 * Экспорт всех утилит из одного места
 */
export { getBinaryPath, setOutputChannel as setBinaryPathOutputChannel } from './binaryPath';
export { executeBslCommand, setOutputChannel as setExecutorOutputChannel } from './executor';
export { parseMethodCall, extractTypeName, type MethodCallInfo } from './parser';
export { 
    getConfigurationPath, 
    getPlatformVersion, 
    getPlatformDocsArchive,
    setOutputChannel as setConfigOutputChannel 
} from './config';

import * as vscode from 'vscode';

/**
 * Инициализирует output channel для всех утилит
 */
export function initializeUtils(outputChannel: vscode.OutputChannel) {
    require('./binaryPath').setOutputChannel(outputChannel);
    require('./executor').setOutputChannel(outputChannel);
    require('./config').setOutputChannel(outputChannel);
}

// Export configuration finder utilities
export { 
    findConfigurations, 
    findMainConfiguration, 
    selectConfiguration, 
    autoDetectConfiguration,
    ConfigurationInfo 
} from './configurationFinder';