/**
 * Setup Module - Re-exports
 *
 * Точка входа для всех модулей инициализации расширения
 */

// LSP Setup
export { loadConfiguration, initializeEnhancedLsp } from './lsp';

// Providers Setup
export { registerEnhancedProviders } from './providers';
export type { ProvidersResult } from './providers';

// Commands Setup
export { registerEnhancedCommands, getEnhancedPackageContributions } from './commands';

// UI Setup
export {
    createStatusBarItem,
    showWelcomeMessage,
    showFeaturesOverview,
    showProjectAnalysisResults,
    generateTypeInfoHtml
} from './ui';
export type { ProjectAnalysisResult, HoverInfo } from './ui';
