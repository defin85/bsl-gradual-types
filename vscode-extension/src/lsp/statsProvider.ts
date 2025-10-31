// vscode-extension/src/lsp/statsProvider.ts

import * as vscode from 'vscode';
import { getTypeRepositoryStats, TypeRepositoryStats } from './customRequests';
import { State } from 'vscode-languageclient/node';
import { getLanguageClient } from './client';

let statusBarItem: vscode.StatusBarItem | undefined;
let updateInterval: NodeJS.Timeout | undefined;

const UPDATE_INTERVAL_MS = 5000;  // 5 секунд

// ✅ ИСПРАВЛЕНИЕ: Уникальные маркеры для секции статистики
const STATS_MARKER_START = '<!-- BSL_STATS_START -->';
const STATS_MARKER_END = '<!-- BSL_STATS_END -->';

/**
 * Инициализирует отслеживание статистики TypeRepository
 */
export function initializeStatsProvider(
    context: vscode.ExtensionContext,
    statusBar: vscode.StatusBarItem
): void {
    statusBarItem = statusBar;

    // Начальное обновление
    updateTypeStats();

    // Периодическое обновление каждые 5 секунд
    updateInterval = setInterval(() => {
        const client = getLanguageClient();
        if (client && client.state === State.Running) {
            updateTypeStats();
        }
    }, UPDATE_INTERVAL_MS);

    // Cleanup
    context.subscriptions.push({
        dispose: () => {
            if (updateInterval) {
                clearInterval(updateInterval);
                updateInterval = undefined;
            }
        }
    });
}

/**
 * Запрашивает статистику TypeRepository и обновляет tooltip
 */
async function updateTypeStats(): Promise<void> {
    if (!statusBarItem) {
        return;
    }

    try {
        const stats = await getTypeRepositoryStats();

        if (stats) {
            updateTooltipWithStats(stats);
        } else {
            updateTooltipWithoutStats();
        }
    } catch (error) {
        console.error('[Stats Provider] Failed to update type stats:', error);
        updateTooltipWithoutStats();
    }
}

/**
 * Обновляет tooltip с актуальной статистикой TypeRepository
 */
function updateTooltipWithStats(stats: TypeRepositoryStats): void {
    if (!statusBarItem) {
        return;
    }

    let currentTooltip = statusBarItem.tooltip as string || '';

    // ✅ ИСПРАВЛЕНИЕ: Удаляем старую секцию по уникальным маркерам
    const markerRegex = /<!-- BSL_STATS_START -->[\s\S]*?<!-- BSL_STATS_END -->/;
    currentTooltip = currentTooltip.replace(markerRegex, '');

    // Форматируем новую статистику
    const statsSection = formatStatsSection(stats);

    // ✅ ИСПРАВЛЕНИЕ: Добавляем секцию с маркерами
    const wrappedSection = `\n${STATS_MARKER_START}${statsSection}${STATS_MARKER_END}`;
    statusBarItem.tooltip = currentTooltip + wrappedSection;
}

/**
 * Форматирует секцию статистики для tooltip
 */
function formatStatsSection(stats: TypeRepositoryStats): string {
    let result = '\n\nTypeRepository: ';

    if (stats.totalTypes === 0) {
        result += '⚠️ Типы не загружены';
    } else {
        result += `${stats.totalTypes} типов`;
        result += `\n- Платформа: ${stats.platformTypes}`;
        result += `\n- Конфигурация: ${stats.configurationTypes}`;

        if (stats.lastUpdateTime) {
            const updateTime = formatUpdateTime(stats.lastUpdateTime);
            result += `\n- Обновлено: ${updateTime}`;
        }
    }

    return result;
}

/**
 * Форматирует ISO 8601 timestamp в читаемый формат
 */
function formatUpdateTime(isoTimestamp: string): string {
    try {
        const date = new Date(isoTimestamp);
        const now = new Date();
        const diffMs = now.getTime() - date.getTime();
        const diffMinutes = Math.floor(diffMs / 60000);

        if (diffMinutes < 1) {
            return 'только что';
        } else if (diffMinutes < 60) {
            return `${diffMinutes} мин назад`;
        } else {
            const hours = Math.floor(diffMinutes / 60);
            return `${hours} ч назад`;
        }
    } catch (error) {
        return 'неизвестно';
    }
}

/**
 * Обновляет tooltip без статистики (graceful degradation)
 */
function updateTooltipWithoutStats(): void {
    if (!statusBarItem) {
        return;
    }

    let currentTooltip = statusBarItem.tooltip as string || '';

    // ✅ ИСПРАВЛЕНИЕ: Удаляем старую секцию по маркерам
    const markerRegex = /<!-- BSL_STATS_START -->[\s\S]*?<!-- BSL_STATS_END -->/;
    currentTooltip = currentTooltip.replace(markerRegex, '');

    // ✅ ИСПРАВЛЕНИЕ: Добавляем placeholder с маркерами
    const placeholder = `\n${STATS_MARKER_START}\n\nTypeRepository: ⏳ Загрузка...${STATS_MARKER_END}`;
    statusBarItem.tooltip = currentTooltip + placeholder;
}
