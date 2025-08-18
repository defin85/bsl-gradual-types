/**
 * Информация о вызове метода
 */
export interface MethodCallInfo {
    objectName: string;
    methodName: string;
    fullCall: string;
}
/**
 * Парсит вызов метода из выделенного текста
 * @param selectedText Выделенный текст
 * @returns Информация о вызове метода или null
 */
export declare function parseMethodCall(selectedText: string): MethodCallInfo | null;
/**
 * Извлекает имя типа из текста
 * @param text Текст для анализа
 * @returns Имя типа или пустую строку
 */
export declare function extractTypeName(text: string): string;
//# sourceMappingURL=parser.d.ts.map