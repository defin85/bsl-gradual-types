"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.initializeContextProvider = void 0;
const vscode = __importStar(require("vscode"));
const client_1 = require("./client");
const node_1 = require("vscode-languageclient/node");
let debounceTimer;
let statusBarItem;
// ✅ ИСПРАВЛЕНИЕ: Уникальные маркеры для секции контекста
const CONTEXT_MARKER_START = '<!-- BSL_CONTEXT_START -->';
const CONTEXT_MARKER_END = '<!-- BSL_CONTEXT_END -->';
/**
 * Инициализирует отслеживание текущего контекста в редакторе
 *
 * @param context - VSCode extension context
 * @param statusBar - Status bar item для обновления tooltip
 */
function initializeContextProvider(context, statusBar) {
    statusBarItem = statusBar;
    // Обработчик изменения позиции курсора
    context.subscriptions.push(vscode.window.onDidChangeTextEditorSelection((event) => {
        handleCursorMove(event.textEditor);
    }));
    // Обработчик изменения активного редактора
    context.subscriptions.push(vscode.window.onDidChangeActiveTextEditor((editor) => {
        if (editor) {
            handleCursorMove(editor);
        }
    }));
    // Начальное обновление контекста (если есть активный редактор)
    if (vscode.window.activeTextEditor) {
        handleCursorMove(vscode.window.activeTextEditor);
    }
}
exports.initializeContextProvider = initializeContextProvider;
/**
 * Обрабатывает движение курсора с debouncing
 */
function handleCursorMove(editor) {
    // Обрабатываем только .bsl файлы
    if (editor.document.languageId !== 'bsl') {
        return;
    }
    // Debouncing: обновляем не чаще 1 раза в 200ms
    if (debounceTimer) {
        clearTimeout(debounceTimer);
    }
    debounceTimer = setTimeout(() => {
        updateCurrentContext(editor);
    }, 200);
}
/**
 * Запрашивает текущий контекст через LSP и обновляет tooltip
 */
async function updateCurrentContext(editor) {
    const client = (0, client_1.getLanguageClient)();
    // Проверяем что LSP готов
    if (!client || client.state !== node_1.State.Running) {
        return; // Молча игнорируем, пока LSP не готов
    }
    const uri = editor.document.uri.toString();
    const position = editor.selection.active;
    try {
        // Вызываем Custom Command через executeCommand
        const context = await vscode.commands.executeCommand('bsl.getCurrentContext', {
            uri,
            line: position.line,
            character: position.character,
        });
        if (context) {
            updateStatusBarTooltip(context);
        }
    }
    catch (error) {
        console.error('[Context Provider] Failed to get current context:', error);
        // НЕ показываем ошибку пользователю (graceful degradation)
    }
}
/**
 * Обновляет tooltip статус-бара с информацией о текущем контексте
 */
function updateStatusBarTooltip(context) {
    if (!statusBarItem) {
        return;
    }
    // ✅ ИСПРАВЛЕНИЕ: Получаем текущий tooltip, удаляем старую секцию контекста
    let currentTooltip = statusBarItem.tooltip || '';
    const markerRegex = /<!-- BSL_CONTEXT_START -->[\s\S]*?<!-- BSL_CONTEXT_END -->/;
    currentTooltip = currentTooltip.replace(markerRegex, '');
    // ✅ ИСПРАВЛЕНИЕ: Формируем ТОЛЬКО секцию контекста (не перезаписываем base)
    let contextSection = '';
    if (context.functionKind !== 'none' && context.functionName) {
        const kindRu = context.functionKind === 'function' ? 'Функция' : 'Процедура';
        contextSection += `\n${kindRu}: ${context.functionName}`;
        if (context.params && context.params.length > 0) {
            contextSection += `\n  - Параметры: ${context.params.join(', ')}`;
        }
        if (context.returnType) {
            contextSection += `\n  - Возвращает: ${context.returnType}`;
        }
    }
    else {
        contextSection += '\nГлобальная область видимости';
    }
    // ✅ ИСПРАВЛЕНИЕ: Оборачиваем секцию в маркеры
    const wrappedSection = `${CONTEXT_MARKER_START}${contextSection}${CONTEXT_MARKER_END}`;
    // ✅ ИСПРАВЛЕНИЕ: Если tooltip пуст, добавляем базовую часть
    if (!currentTooltip || currentTooltip.trim() === '') {
        currentTooltip = 'BSL Type Safety Analyzer\nLSP Server активен\n';
        currentTooltip += '━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━';
    }
    statusBarItem.tooltip = currentTooltip + '\n' + wrappedSection;
}
//# sourceMappingURL=contextProvider.js.map