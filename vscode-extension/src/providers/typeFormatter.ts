import { MethodInfo, PropertyInfo } from './typeModels';

/**
 * Форматирование отображения типов для UI
 */

/**
 * Форматирует tooltip для метода
 */
export function formatMethodTooltip(name: string, method: MethodInfo): string {
    let tooltip = `Метод: ${name}`;
    if (method.parameters) {
        tooltip += '\nПараметры: ' + JSON.stringify(method.parameters);
    }
    if (method.returns) {
        tooltip += '\nВозвращает: ' + method.returns;
    }
    if (method.documentation) {
        tooltip += '\n\n' + method.documentation;
    }
    return tooltip;
}

/**
 * Форматирует tooltip для свойства
 */
export function formatPropertyTooltip(name: string, prop: PropertyInfo): string {
    let tooltip = `Свойство: ${name}`;
    if (prop.type) {
        tooltip += '\nТип: ' + prop.type;
    }
    if (prop.readonly) {
        tooltip += '\n(Только чтение)';
    }
    if (prop.documentation) {
        tooltip += '\n\n' + prop.documentation;
    }
    return tooltip;
}

/**
 * Возвращает иконку для категории типов
 */
export function getCategoryIcon(categoryName: string): string {
    const iconMap: Record<string, string> = {
        'Примитивные типы': '🔤',
        'Коллекции': '📚',
        'Работа с данными': '🗃️',
        'Работа с XML': '📄',
        'Работа с JSON': '📋',
        'Файловая система': '📁',
        'Интерфейс': '🖼️',
        'Менеджеры': '👥',
        'Глобальные функции': '🔧',
        'Справочники': '📖',
        'Документы': '📃',
        'Регистры сведений': '📊',
        'Регистры накопления': '📈',
        'Регистры бухгалтерии': '💰',
        'Регистры расчета': '🧮',
        'Общие модули': '📦',
        'Отчеты': '📊',
        'Обработки': '⚙️'
    };

    return iconMap[categoryName] || '📂';
}

/**
 * Маппинг entity_kind на русское название категории
 */
export function getConfigCategoryName(entityKind: string): string {
    const categoryMap: Record<string, string> = {
        'Catalog': 'Справочники',
        'Document': 'Документы',
        'InformationRegister': 'Регистры сведений',
        'AccumulationRegister': 'Регистры накопления',
        'AccountingRegister': 'Регистры бухгалтерии',
        'CalculationRegister': 'Регистры расчета',
        'ChartOfCharacteristicTypes': 'Планы видов характеристик',
        'ChartOfAccounts': 'Планы счетов',
        'ChartOfCalculationTypes': 'Планы видов расчета',
        'BusinessProcess': 'Бизнес-процессы',
        'Task': 'Задачи',
        'ExchangePlan': 'Планы обмена',
        'CommonModule': 'Общие модули',
        'Report': 'Отчеты',
        'DataProcessor': 'Обработки'
    };

    return categoryMap[entityKind] || 'Другие объекты';
}

/**
 * Категории для платформенных типов с паттернами для матчинга
 */
export const PLATFORM_CATEGORIES: Record<string, string[]> = {
    'Примитивные типы': ['Число', 'Строка', 'Булево', 'Дата', 'Неопределено', 'Null', 'Тип'],
    'Коллекции': ['Массив', 'Структура', 'Соответствие', 'СписокЗначений', 'ТаблицаЗначений', 'ДеревоЗначений'],
    'Работа с данными': ['Запрос', 'ПостроительЗапроса', 'СхемаЗапроса', 'РезультатЗапроса', 'ВыборкаИзРезультатаЗапроса'],
    'Работа с XML': ['ЧтениеXML', 'ЗаписьXML', 'ФабрикаXDTO', 'СериализаторXDTO'],
    'Работа с JSON': ['ЧтениеJSON', 'ЗаписьJSON'],
    'Файловая система': ['Файл', 'ДиалогВыбораФайла', 'ЧтениеТекста', 'ЗаписьТекста'],
    'Интерфейс': ['Форма', 'ТабличныйДокумент', 'Диаграмма', 'ПолеHTMLДокумента'],
    'Менеджеры': ['Справочники', 'Документы', 'РегистрыСведений', 'РегистрыНакопления', 'ПланыВидовХарактеристик'],
    'Глобальные функции': ['Сообщить', 'СокрЛП', 'НачалоГода', 'СтрНайти', 'Формат', 'XMLСтрока']
};
