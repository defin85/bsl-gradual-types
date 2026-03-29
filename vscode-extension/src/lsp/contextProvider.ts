import * as vscode from 'vscode';
import { getLanguageClient } from './client/index';
import { State } from 'vscode-languageclient/node';
import { logger } from './logger';

export interface CurrentContext {
    functionName?: string;      // Имя функции/процедуры
    functionKind: 'function' | 'procedure' | 'none';
    params?: string[];          // Параметры (опционально для будущего)
    returnType?: string;        // Возвращаемый тип (опционально)
}

let debounceTimer: NodeJS.Timeout | undefined;
let statusBarItem: vscode.StatusBarItem | undefined;

// ✅ ИСПРАВЛЕНИЕ: Уникальные маркеры для секции контекста
const CONTEXT_MARKER_START = '<!-- BSL_CONTEXT_START -->';
const CONTEXT_MARKER_END = '<!-- BSL_CONTEXT_END -->';

/**
 * Инициализирует отслеживание текущего контекста в редакторе
 *
 * @param context - VSCode extension context
 * @param statusBar - Status bar item для обновления tooltip
 */
export function initializeContextProvider(
    context: vscode.ExtensionContext,
    statusBar: vscode.StatusBarItem
): void {
    statusBarItem = statusBar;

    // Обработчик изменения позиции курсора
    context.subscriptions.push(
        vscode.window.onDidChangeTextEditorSelection((event) => {
            handleCursorMove(event.textEditor);
        })
    );

    // Обработчик изменения активного редактора
    context.subscriptions.push(
        vscode.window.onDidChangeActiveTextEditor((editor) => {
            if (editor) {
                handleCursorMove(editor);
            }
        })
    );

    // Начальное обновление контекста (если есть активный редактор)
    if (vscode.window.activeTextEditor) {
        handleCursorMove(vscode.window.activeTextEditor);
    }
}

/**
 * Обрабатывает движение курсора с debouncing
 */
function handleCursorMove(editor: vscode.TextEditor): void {
    // Обрабатываем только .bsl файлы
    if (editor.document.languageId !== 'bsl') {
        return;
    }

    // Debouncing: обновляем не чаще 1 раза в 200ms
    if (debounceTimer) {
        clearTimeout(debounceTimer);
        debounceTimer = undefined;
    }

    debounceTimer = setTimeout(() => {
        debounceTimer = undefined;
        void updateCurrentContext(editor);
    }, 200);
}

/**
 * Запрашивает текущий контекст через LSP и обновляет tooltip
 */
async function updateCurrentContext(editor: vscode.TextEditor): Promise<void> {
    const client = getLanguageClient();

    // Проверяем что LSP готов
    if (!client || client.state !== State.Running) {
        return; // Молча игнорируем, пока LSP не готов
    }

    const uri = editor.document.uri.toString();
    const position = editor.selection.active;

    try {
        // Вызываем Custom Command через executeCommand
        const context = await vscode.commands.executeCommand<CurrentContext>(
            'bsl.getCurrentContext',
            {
                uri,
                line: position.line,
                character: position.character,
            }
        );

        if (context) {
            updateStatusBarTooltip(context);
        }
    } catch (error) {
        logger.error('[Context Provider] Failed to get current context', error);
        // НЕ показываем ошибку пользователю (graceful degradation)
    }
}

/**
 * Обновляет tooltip статус-бара с информацией о текущем контексте
 */
function updateStatusBarTooltip(context: CurrentContext): void {
    if (!statusBarItem) {
        return;
    }

    // ✅ ИСПРАВЛЕНИЕ: Получаем текущий tooltip, удаляем старую секцию контекста
    let currentTooltip = statusBarItem.tooltip as string || '';
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
    } else {
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
