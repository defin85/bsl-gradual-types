"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.PLATFORM_CATEGORIES = exports.getConfigCategoryName = exports.getCategoryIcon = exports.formatPropertyTooltip = exports.formatMethodTooltip = void 0;
/**
 * Форматирование отображения типов для UI
 */
/**
 * Форматирует tooltip для метода
 */
function formatMethodTooltip(name, method) {
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
exports.formatMethodTooltip = formatMethodTooltip;
/**
 * Форматирует tooltip для свойства
 */
function formatPropertyTooltip(name, prop) {
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
exports.formatPropertyTooltip = formatPropertyTooltip;
/**
 * Возвращает иконку для категории типов
 */
function getCategoryIcon(categoryName) {
    const iconMap = {
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
exports.getCategoryIcon = getCategoryIcon;
/**
 * Маппинг entity_kind на русское название категории
 */
function getConfigCategoryName(entityKind) {
    const categoryMap = {
        'Catalog': 'Справочники',
        'Document': 'Документы',
        'InformationRegister': 'Регистры сведений',
        'AccumulationRegister': 'Регистры накопления',
        'AccountingRegister': 'Регистры бухгалтерии',
        'CalculationRegister': 'Регистры расчета',
        'Register': 'Регистры',
        'ChartOfCharacteristicTypes': 'Планы видов характеристик',
        'ChartOfAccounts': 'Планы счетов',
        'ChartOfCalculationTypes': 'Планы видов расчета',
        'BusinessProcess': 'Бизнес-процессы',
        'Task': 'Задачи',
        'ExchangePlan': 'Планы обмена',
        'CommonModule': 'Общие модули',
        'Report': 'Отчеты',
        'DataProcessor': 'Обработки',
        'Enum': 'Перечисления',
        'Constant': 'Константы',
        'Role': 'Роли',
        'Subsystem': 'Подсистемы',
        'Language': 'Языки'
    };
    return categoryMap[entityKind] || 'Другие объекты';
}
exports.getConfigCategoryName = getConfigCategoryName;
/**
 * Категории для платформенных типов с паттернами для матчинга
 */
exports.PLATFORM_CATEGORIES = {
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
//# sourceMappingURL=typeFormatter.js.map