/**
 * Экспорт всех LSP модулей
 */

// Модуль управления прогрессом
export {
    IndexingProgress,
    progressEmitter,
    initializeProgress,
    updateStatusBar,
    getCurrentProgress
} from './progress';

// Модуль LSP клиента
export {
    initializeLspClient,
    startLanguageClient,
    stopLanguageClient,
    restartLanguageClient,
    getLanguageClient,
    getActiveServerLaunchInfo,
    isClientRunning,
    sendCustomRequest,
    sendCustomNotification
} from './client/index';

// UX: auto-trigger signature help when moving cursor between arguments (snippets/Tab).
export { initializeAutoSignatureHelpOnCursorMove } from './autoSignatureHelp';
