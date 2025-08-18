"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.extractTypeName = exports.parseMethodCall = void 0;
/**
 * Парсит вызов метода из выделенного текста
 * @param selectedText Выделенный текст
 * @returns Информация о вызове метода или null
 */
function parseMethodCall(selectedText) {
    // Паттерн для поиска вызова метода: Объект.Метод(
    // Используем [\wа-яА-ЯёЁ] для поддержки кириллицы
    const methodPattern = /([\wа-яА-ЯёЁ]+(?:\.[\wа-яА-ЯёЁ]+)*?)\.([\wа-яА-ЯёЁ]+)\s*\(/;
    const match = selectedText.match(methodPattern);
    if (match && match[1] && match[2]) {
        return {
            objectName: match[1],
            methodName: match[2],
            fullCall: match[0]
        };
    }
    return null;
}
exports.parseMethodCall = parseMethodCall;
/**
 * Извлекает имя типа из текста
 * @param text Текст для анализа
 * @returns Имя типа или пустую строку
 */
function extractTypeName(text) {
    // Пытаемся найти объявление переменной
    // Используем [\wа-яА-ЯёЁ] для поддержки кириллицы
    const varPattern = /(?:Перем|Var)\s+([\wа-яА-ЯёЁ]+)/i;
    const varMatch = text.match(varPattern);
    if (varMatch && varMatch[1]) {
        return varMatch[1];
    }
    // Пытаемся найти присваивание
    const assignPattern = /([\wа-яА-ЯёЁ]+)\s*=/;
    const assignMatch = text.match(assignPattern);
    if (assignMatch && assignMatch[1]) {
        return assignMatch[1];
    }
    // Если ничего не найдено, возвращаем первое слово
    const words = text.trim().split(/\s+/);
    return words[0] || '';
}
exports.extractTypeName = extractTypeName;
//# sourceMappingURL=parser.js.map