"use strict";
// vscode-extension/src/lsp/statsProvider.ts
Object.defineProperty(exports, "__esModule", { value: true });
exports.initializeStatsProvider = void 0;
const customRequests_1 = require("./customRequests");
const node_1 = require("vscode-languageclient/node");
const client_1 = require("./client");
const logger_1 = require("./logger");
let statusBarItem;
let updateInterval;
const UPDATE_INTERVAL_MS = 5000; // 5 секунд
// ✅ ИСПРАВЛЕНИЕ: Уникальные маркеры для секции статистики
const STATS_MARKER_START = '<!-- BSL_STATS_START -->';
const STATS_MARKER_END = '<!-- BSL_STATS_END -->';
/**
 * Инициализирует отслеживание статистики TypeRepository
 */
function initializeStatsProvider(context, statusBar) {
    statusBarItem = statusBar;
    // Начальное обновление
    updateTypeStats();
    // Периодическое обновление каждые 5 секунд
    updateInterval = setInterval(() => {
        const client = (0, client_1.getLanguageClient)();
        if (client && client.state === node_1.State.Running) {
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
exports.initializeStatsProvider = initializeStatsProvider;
/**
 * Запрашивает статистику TypeRepository и обновляет tooltip
 */
async function updateTypeStats() {
    if (!statusBarItem) {
        return;
    }
    try {
        const stats = await (0, customRequests_1.getTypeRepositoryStats)();
        if (stats) {
            updateTooltipWithStats(stats);
        }
        else {
            updateTooltipWithoutStats();
        }
    }
    catch (error) {
        logger_1.logger.error('[Stats Provider] Failed to update type stats', error);
        updateTooltipWithoutStats();
    }
}
/**
 * Обновляет tooltip с актуальной статистикой TypeRepository
 */
function updateTooltipWithStats(stats) {
    if (!statusBarItem) {
        return;
    }
    let currentTooltip = statusBarItem.tooltip || '';
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
function formatStatsSection(stats) {
    let result = '\n\nTypeRepository: ';
    if (stats.totalTypes === 0) {
        result += '⚠️ Типы не загружены';
    }
    else {
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
function formatUpdateTime(isoTimestamp) {
    try {
        const date = new Date(isoTimestamp);
        const now = new Date();
        const diffMs = now.getTime() - date.getTime();
        const diffMinutes = Math.floor(diffMs / 60000);
        if (diffMinutes < 1) {
            return 'только что';
        }
        else if (diffMinutes < 60) {
            return `${diffMinutes} мин назад`;
        }
        else {
            const hours = Math.floor(diffMinutes / 60);
            return `${hours} ч назад`;
        }
    }
    catch (error) {
        return 'неизвестно';
    }
}
/**
 * Обновляет tooltip без статистики (graceful degradation)
 */
function updateTooltipWithoutStats() {
    if (!statusBarItem) {
        return;
    }
    let currentTooltip = statusBarItem.tooltip || '';
    // ✅ ИСПРАВЛЕНИЕ: Удаляем старую секцию по маркерам
    const markerRegex = /<!-- BSL_STATS_START -->[\s\S]*?<!-- BSL_STATS_END -->/;
    currentTooltip = currentTooltip.replace(markerRegex, '');
    // ✅ ИСПРАВЛЕНИЕ: Добавляем placeholder с маркерами
    const placeholder = `\n${STATS_MARKER_START}\n\nTypeRepository: ⏳ Загрузка...${STATS_MARKER_END}`;
    statusBarItem.tooltip = currentTooltip + placeholder;
}
//# sourceMappingURL=statsProvider.js.map