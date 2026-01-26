import { MethodInfo, PropertyInfo } from './typeModels';
/**
 * Форматирование отображения типов для UI
 */
/**
 * Форматирует tooltip для метода
 */
export declare function formatMethodTooltip(name: string, method: MethodInfo): string;
/**
 * Форматирует tooltip для свойства
 */
export declare function formatPropertyTooltip(name: string, prop: PropertyInfo): string;
/**
 * Возвращает иконку для категории типов
 */
export declare function getCategoryIcon(categoryName: string): string;
/**
 * Маппинг entity_kind на русское название категории
 */
export declare function getConfigCategoryName(entityKind: string): string;
/**
 * Категории для платформенных типов с паттернами для матчинга
 */
export declare const PLATFORM_CATEGORIES: Record<string, string[]>;
//# sourceMappingURL=typeFormatter.d.ts.map