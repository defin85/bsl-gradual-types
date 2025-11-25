//! Signature Index - индекс сигнатур функций и методов
//!
//! Milestone 2.20: Function Signature Validation System
//! Milestone 3.11: Method Signature Enhancement with Facets and Context
//! Milestone 3.13: MetadataPatternRegistry Integration

use super::metadata_patterns::{ExtractedPattern, MetadataPatternRegistry};
use super::types::{FacetKind, MetadataKind, ParameterInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// MILESTONE 3.11 Phase 3: Re-export ContextRequirements для обратной совместимости
pub use super::runtime_context::ContextRequirements;

/// Индекс сигнатур функций и методов
#[derive(Debug, Clone)]
pub struct SignatureIndex {
    /// Платформенные методы: тип -> список методов
    platform_methods: HashMap<String, Vec<MethodSignature>>,
    /// Конфигурационные методы: тип -> список методов
    config_methods: HashMap<String, Vec<MethodSignature>>,
    /// Глобальные функции: имя -> сигнатура
    global_functions: HashMap<String, MethodSignature>,
    /// Конструкторы: тип -> сигнатура конструктора
    /// Например: "Массив" -> ConstructorSignature { params: [размер?], ... }
    constructors: HashMap<String, ConstructorSignature>,
    /// Реестр паттернов MetadataKind (Milestone 3.13)
    /// Используется для определения MetadataKind из фасетных префиксов
    metadata_patterns: MetadataPatternRegistry,
}

/// Сигнатура метода
///
/// Расширенная информация о методе/функции включая:
/// - Базовые параметры (имя, тип владельца, параметры)
/// - Facet информацию для методов конфигурационных объектов
/// - Требования к контексту выполнения
///
/// # Примеры
/// ```
/// use bsl_shared::domain::signature_index::{MethodSignature, SignatureSource, ContextRequirements};
/// use bsl_shared::domain::types::{ParameterInfo, FacetKind};
///
/// // Метод Справочник.СоздатьЭлемент() → Object, ServerOnly
/// let signature = MethodSignature {
///     name: "СоздатьЭлемент".to_string(),
///     owner_type: Some("СправочникМенеджер.Номенклатура".to_string()),
///     params: vec![],
///     return_type: Some("СправочникОбъект.Номенклатура".to_string()),
///     source: SignatureSource::Platform,
///     return_facet: Some(FacetKind::Object),
///     context_requirements: ContextRequirements::ServerOnly,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodSignature {
    pub name: String,
    pub owner_type: Option<String>, // None для глобальных функций
    pub params: Vec<ParameterInfo>,
    pub return_type: Option<String>,
    pub source: SignatureSource,

    /// Facet возвращаемого типа (для методов конфигурационных объектов)
    ///
    /// # Примеры
    /// - `СоздатьЭлемент()` → Object
    /// - `НайтиПоКоду()` → Reference
    /// - `Выбрать()` → Selection
    #[serde(default)]
    pub return_facet: Option<FacetKind>,

    /// Требования к контексту выполнения
    ///
    /// Определяет где может быть вызван метод (сервер/клиент/везде)
    #[serde(default)]
    pub context_requirements: ContextRequirements,
}

/// Сигнатура конструктора
#[derive(Debug, Clone, PartialEq)]
pub struct ConstructorSignature {
    /// Имя типа ("Массив", "ТаблицаЗначений")
    pub type_name: String,

    /// Параметры конструктора
    pub params: Vec<ParameterInfo>,

    /// Результирующий facet (Object, Reference, Manager)
    /// None означает что конструктор возвращает сам тип
    pub facet: Option<String>,

    /// Источник сигнатуры
    pub source: SignatureSource,

    /// Является ли тип коллекцией (для generic inference)
    pub is_collection: bool,

    /// Количество generic параметров для коллекций
    /// Массив → 1, Соответствие → 2, СписокЗначений → 1
    pub generic_params_count: usize,
}

/// Источник сигнатуры
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatureSource {
    Platform,
    Configuration,
    UserCode, // Код пользователя (для валидации)
}

/// Результат валидации сигнатуры
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureValidationResult {
    /// Сигнатура валидна
    Valid,
    /// Сигнатура невалидна (список несоответствий)
    Invalid(Vec<SignatureMismatch>),
}

/// Несоответствие в сигнатуре
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureMismatch {
    /// Неправильное количество параметров
    ParameterCount { expected: usize, actual: usize },
    /// Неправильный тип параметра
    ParameterType {
        index: usize,
        param_name: String,
        expected: String,
        actual: String,
    },
    /// Неправильный тип возврата
    ReturnType {
        expected: Option<String>,
        actual: Option<String>,
    },
}

impl SignatureIndex {
    pub fn new() -> Self {
        Self {
            platform_methods: HashMap::new(),
            config_methods: HashMap::new(),
            global_functions: HashMap::new(),
            constructors: HashMap::new(),
            metadata_patterns: MetadataPatternRegistry::new(),
        }
    }

    /// Добавить платформенный метод
    pub fn add_platform_method(&mut self, type_name: String, method: MethodSignature) {
        self.platform_methods
            .entry(type_name)
            .or_default()
            .push(method);
    }

    /// Добавить конфигурационный метод
    pub fn add_config_method(&mut self, type_name: String, method: MethodSignature) {
        self.config_methods
            .entry(type_name)
            .or_default()
            .push(method);
    }

    /// Добавить глобальную функцию
    pub fn add_global_function(&mut self, name: String, method: MethodSignature) {
        self.global_functions.insert(name, method);
    }

    /// Добавить конструктор
    pub fn add_constructor(&mut self, type_name: String, constructor: ConstructorSignature) {
        self.constructors.insert(type_name, constructor);
    }

    /// Найти конструктор по имени типа (регистронезависимо)
    pub fn find_constructor(&self, type_name: &str) -> Option<&ConstructorSignature> {
        let type_name_lower = type_name.to_lowercase();

        self.constructors
            .iter()
            .find(|(k, _)| k.to_lowercase() == type_name_lower)
            .map(|(_, v)| v)
    }

    /// Получить все конструкторы
    pub fn get_all_constructors(&self) -> &HashMap<String, ConstructorSignature> {
        &self.constructors
    }

    /// Проверить является ли тип коллекцией
    pub fn is_collection_type(&self, type_name: &str) -> bool {
        self.find_constructor(type_name)
            .map(|c| c.is_collection)
            .unwrap_or(false)
    }

    /// Получить количество generic параметров для типа
    pub fn get_generic_params_count(&self, type_name: &str) -> Option<usize> {
        self.find_constructor(type_name)
            .map(|c| c.generic_params_count)
    }

    /// Найти метод по имени типа и имени метода
    ///
    /// Поддерживает фасетные типы (Milestone 3.11 Phase 2):
    /// - Сначала ищет по точному имени типа
    /// - Если не найдено, извлекает базовый фасетный тип и ищет по нему
    ///
    /// # Примеры
    /// ```ignore
    /// // Точный поиск
    /// find_method("Массив", "Добавить") // → найдёт "Массив.Добавить"
    ///
    /// // Фасетный поиск
    /// find_method("СправочникМенеджер.Контрагенты", "СоздатьЭлемент")
    /// // → не найдёт по точному ключу
    /// // → извлечёт базовый тип "СправочникМенеджер"
    /// // → найдёт "СправочникМенеджер.СоздатьЭлемент"
    /// ```
    pub fn find_method(&self, type_name: &str, method_name: &str) -> Option<&MethodSignature> {
        let method_name_lower = method_name.to_lowercase();

        // 1. Сначала ищем по точному имени типа
        if let Some(method) = self.find_method_in_maps(type_name, &method_name_lower) {
            return Some(method);
        }

        // 2. Если не найдено и это фасетный тип, ищем по базовому типу
        if let Some(base_type) = Self::extract_base_facet_type(type_name) {
            if let Some(method) = self.find_method_in_maps(base_type, &method_name_lower) {
                return Some(method);
            }
        }

        None
    }

    /// Внутренний поиск метода в HashMap'ах (без fallback)
    fn find_method_in_maps(&self, type_name: &str, method_name_lower: &str) -> Option<&MethodSignature> {
        // Поиск в платформенных
        if let Some(methods) = self.platform_methods.get(type_name) {
            if let Some(m) = methods
                .iter()
                .find(|m| m.name.to_lowercase() == *method_name_lower)
            {
                return Some(m);
            }
        }

        // Поиск в конфигурационных
        if let Some(methods) = self.config_methods.get(type_name) {
            if let Some(m) = methods
                .iter()
                .find(|m| m.name.to_lowercase() == *method_name_lower)
            {
                return Some(m);
            }
        }

        None
    }

    /// Извлечь базовый фасетный тип из полного имени типа
    ///
    /// # Примеры
    /// - "СправочникМенеджер.Контрагенты" → Some("СправочникМенеджер")
    /// - "ДокументОбъект.ЗаказКлиента" → Some("ДокументОбъект")
    /// - "Массив" → None (не фасетный тип)
    /// - "СправочникМенеджер" → None (уже базовый тип)
    pub fn extract_base_facet_type(type_name: &str) -> Option<&str> {
        // Проверяем наличие точки (признак конкретизированного типа)
        let dot_pos = type_name.find('.')?;

        let prefix = &type_name[..dot_pos];

        // Проверяем что префикс - известный фасетный тип
        if Self::is_known_facet_prefix(prefix) {
            Some(prefix)
        } else {
            None
        }
    }

    /// Проверить является ли строка известным фасетным префиксом
    fn is_known_facet_prefix(prefix: &str) -> bool {
        matches!(
            prefix,
            // Справочники
            "СправочникМенеджер" | "СправочникОбъект" | "СправочникСсылка" |
            "СправочникВыборка" | "СправочникСписок" |
            // Документы
            "ДокументМенеджер" | "ДокументОбъект" | "ДокументСсылка" |
            "ДокументВыборка" | "ДокументСписок" |
            // Планы видов характеристик
            "ПланВидовХарактеристикМенеджер" | "ПланВидовХарактеристикОбъект" |
            "ПланВидовХарактеристикСсылка" | "ПланВидовХарактеристикВыборка" |
            // Планы счетов
            "ПланСчетовМенеджер" | "ПланСчетовОбъект" | "ПланСчетовСсылка" |
            "ПланСчетовВыборка" |
            // Планы видов расчета
            "ПланВидовРасчетаМенеджер" | "ПланВидовРасчетаОбъект" |
            "ПланВидовРасчетаСсылка" | "ПланВидовРасчетаВыборка" |
            // Планы обмена
            "ПланОбменаМенеджер" | "ПланОбменаОбъект" | "ПланОбменаСсылка" |
            "ПланОбменаВыборка" |
            // Бизнес-процессы
            "БизнесПроцессМенеджер" | "БизнесПроцессОбъект" | "БизнесПроцессСсылка" |
            "БизнесПроцессВыборка" | "БизнесПроцессТочкаМаршрутаБизнесПроцесса" |
            // Задачи
            "ЗадачаМенеджер" | "ЗадачаОбъект" | "ЗадачаСсылка" | "ЗадачаВыборка" |
            // Перечисления
            "ПеречислениеМенеджер" | "ПеречислениеСсылка" |
            // Регистры сведений
            "РегистрСведенийМенеджер" | "РегистрСведенийНаборЗаписей" |
            "РегистрСведенийВыборка" | "РегистрСведенийЗапись" |
            "РегистрСведенийМенеджерЗаписи" |
            // Регистры накопления
            "РегистрНакопленияМенеджер" | "РегистрНакопленияНаборЗаписей" |
            "РегистрНакопленияВыборка" | "РегистрНакопленияЗапись" |
            // Регистры бухгалтерии
            "РегистрБухгалтерииМенеджер" | "РегистрБухгалтерииНаборЗаписей" |
            "РегистрБухгалтерииВыборка" | "РегистрБухгалтерииЗапись" |
            // Регистры расчета
            "РегистрРасчетаМенеджер" | "РегистрРасчетаНаборЗаписей" |
            "РегистрРасчетаВыборка" | "РегистрРасчетаЗапись"
        )
    }

    /// Получить FacetKind из фасетного префикса по суффиксу
    ///
    /// # Примеры
    /// - "СправочникМенеджер" -> Some(FacetKind::Manager)
    /// - "ДокументОбъект" -> Some(FacetKind::Object)
    /// - "ПеречислениеСсылка" -> Some(FacetKind::Reference)
    /// - "Массив" -> None (не фасетный тип)
    pub fn get_facet_kind_from_prefix(prefix: &str) -> Option<FacetKind> {
        // Порядок проверки важен: сначала длинные суффиксы!
        if prefix.ends_with("НаборЗаписей") {
            return Some(FacetKind::Collection); // Набор записей регистра - коллекция
        }
        if prefix.ends_with("МенеджерЗаписи") {
            return Some(FacetKind::Manager); // РегистрСведенийМенеджерЗаписи
        }
        if prefix.ends_with("Менеджер") {
            return Some(FacetKind::Manager);
        }
        if prefix.ends_with("Объект") {
            return Some(FacetKind::Object);
        }
        if prefix.ends_with("Ссылка") {
            return Some(FacetKind::Reference);
        }
        if prefix.ends_with("Выборка") {
            return Some(FacetKind::Selection);
        }
        if prefix.ends_with("Список") {
            return Some(FacetKind::List);
        }
        if prefix.ends_with("Запись") {
            return Some(FacetKind::Object); // Запись регистра - объект
        }
        None
    }

    /// Получить MetadataKind из фасетного префикса по его началу
    ///
    /// # Примеры
    /// - "СправочникМенеджер" -> Some(MetadataKind::Catalog)
    /// - "ДокументОбъект" -> Some(MetadataKind::Document)
    /// - "РегистрСведенийНаборЗаписей" -> Some(MetadataKind::InformationRegister)
    ///
    /// Делегирует вызов в `MetadataPatternRegistry::hardcoded_metadata_kind()`.
    pub fn get_metadata_kind_from_prefix(prefix: &str) -> Option<MetadataKind> {
        MetadataPatternRegistry::hardcoded_metadata_kind(prefix)
    }

    /// Подставить реальное имя объекта в return type вместо placeholder
    ///
    /// # Примеры
    /// - ("СправочникОбъект", "Контрагенты") → "СправочникОбъект.Контрагенты"
    /// - ("СправочникОбъект.<Имя справочника>", "Контрагенты") → "СправочникОбъект.Контрагенты"
    /// - ("СправочникСсылка", "Номенклатура") → "СправочникСсылка.Номенклатура"
    /// - ("Неопределено", "Контрагенты") → "Неопределено" (не фасетный тип)
    pub fn substitute_type_name(return_type: &str, actual_name: &str) -> String {
        // 1. Если return_type содержит placeholder с точкой (FacetPrefix.<placeholder>)
        //    Примеры: "СправочникОбъект.<Имя справочника>", "ДокументОбъект.&lt;Имя документа&gt;"
        if let Some(dot_pos) = return_type.find('.') {
            let prefix = &return_type[..dot_pos];
            if Self::is_known_facet_prefix(prefix) {
                // Заменяем placeholder на actual_name
                return format!("{}.{}", prefix, actual_name);
            }
        }

        // 2. Если return_type - базовый фасетный тип (без точки), добавляем имя
        if Self::is_known_facet_prefix(return_type) {
            format!("{}.{}", return_type, actual_name)
        } else {
            // Не фасетный тип - возвращаем как есть
            return_type.to_string()
        }
    }

    /// Извлечь имя объекта метаданных из фасетного типа
    ///
    /// # Примеры
    /// - "СправочникМенеджер.Контрагенты" → Some("Контрагенты")
    /// - "ДокументОбъект.ЗаказКлиента" → Some("ЗаказКлиента")
    /// - "Массив" → None
    pub fn extract_metadata_name(type_name: &str) -> Option<&str> {
        let dot_pos = type_name.find('.')?;
        let prefix = &type_name[..dot_pos];

        if Self::is_known_facet_prefix(prefix) {
            Some(&type_name[dot_pos + 1..])
        } else {
            None
        }
    }

    // ==================== MILESTONE 3.13: MetadataPatternRegistry Integration ====================

    /// Обновить паттерны MetadataKind из распарсенного Syntax Helper
    ///
    /// Вызывается при загрузке типов платформы для извлечения паттернов
    /// из имён типов с placeholder (например, "СправочникМенеджер.<Имя справочника>")
    ///
    /// # Примеры
    /// ```ignore
    /// let mut index = SignatureIndex::new();
    /// let patterns = vec![
    ///     ExtractedPattern {
    ///         prefix: "Справочник".to_string(),
    ///         kind: MetadataKind::Catalog,
    ///         placeholder_suffix: Some("справочника".to_string()),
    ///     }
    /// ];
    /// index.update_metadata_patterns(patterns);
    /// ```
    pub fn update_metadata_patterns(&mut self, patterns: Vec<ExtractedPattern>) {
        let count = patterns.len();
        self.metadata_patterns.update_from_patterns(patterns);
        tracing::debug!("SignatureIndex: обновлено {} паттернов MetadataKind", count);
    }

    /// Определить MetadataKind из префикса (instance метод - использует реестр паттернов)
    ///
    /// Сначала ищет в извлечённых паттернах из Syntax Helper,
    /// затем использует hardcoded fallback.
    ///
    /// # Примеры
    /// ```ignore
    /// let index = SignatureIndex::new();
    /// assert_eq!(index.resolve_metadata_kind("СправочникМенеджер"), Some(MetadataKind::Catalog));
    /// assert_eq!(index.resolve_metadata_kind("ПланОбменаОбъект"), Some(MetadataKind::ExchangePlan));
    /// ```
    pub fn resolve_metadata_kind(&self, prefix: &str) -> Option<MetadataKind> {
        self.metadata_patterns.get_metadata_kind(prefix)
    }

    /// Проверить есть ли извлечённые паттерны MetadataKind
    pub fn has_extracted_metadata_patterns(&self) -> bool {
        self.metadata_patterns.has_extracted_patterns()
    }

    /// Получить количество извлечённых паттернов MetadataKind
    pub fn extracted_metadata_patterns_count(&self) -> usize {
        self.metadata_patterns.extracted_count()
    }

    /// Получить ссылку на реестр паттернов MetadataKind
    pub fn metadata_patterns(&self) -> &MetadataPatternRegistry {
        &self.metadata_patterns
    }

    // ==================== END MILESTONE 3.13 ====================

    /// Найти глобальную функцию
    pub fn find_global_function(&self, name: &str) -> Option<&MethodSignature> {
        let name_lower = name.to_lowercase();
        self.global_functions
            .iter()
            .find(|(k, _)| k.to_lowercase() == name_lower)
            .map(|(_, v)| v)
    }

    /// Инициализировать встроенные конструкторы коллекций
    pub fn initialize_builtin_constructors(&mut self) {
        // Массив - коллекция с 1 generic параметром
        self.add_constructor(
            "Массив".to_string(),
            ConstructorSignature {
                type_name: "Массив".to_string(),
                params: vec![
                    // Необязательный параметр размера
                    ParameterInfo {
                        name: "Размер".to_string(),
                        type_name: Some("Число".to_string()),
                        is_optional: true,
                        default_value: None,
                        description: Some("Начальный размер массива".to_string()),
                    },
                ],
                facet: None,
                source: SignatureSource::Platform,
                is_collection: true,
                generic_params_count: 1,
            },
        );

        // Соответствие (Map) - коллекция с 2 generic параметрами
        self.add_constructor(
            "Соответствие".to_string(),
            ConstructorSignature {
                type_name: "Соответствие".to_string(),
                params: vec![],
                facet: None,
                source: SignatureSource::Platform,
                is_collection: true,
                generic_params_count: 2, // Key, Value
            },
        );

        // ТаблицаЗначений
        self.add_constructor(
            "ТаблицаЗначений".to_string(),
            ConstructorSignature {
                type_name: "ТаблицаЗначений".to_string(),
                params: vec![],
                facet: None,
                source: SignatureSource::Platform,
                is_collection: false,
                generic_params_count: 0,
            },
        );

        // СписокЗначений - коллекция с 1 generic параметром
        self.add_constructor(
            "СписокЗначений".to_string(),
            ConstructorSignature {
                type_name: "СписокЗначений".to_string(),
                params: vec![],
                facet: None,
                source: SignatureSource::Platform,
                is_collection: true,
                generic_params_count: 1,
            },
        );

        // ФиксированныйМассив - коллекция с 1 generic параметром
        self.add_constructor(
            "ФиксированныйМассив".to_string(),
            ConstructorSignature {
                type_name: "ФиксированныйМассив".to_string(),
                params: vec![ParameterInfo {
                    name: "Массив".to_string(),
                    type_name: Some("Массив".to_string()),
                    is_optional: false,
                    default_value: None,
                    description: Some(
                        "Исходный массив для преобразования в фиксированный".to_string(),
                    ),
                }],
                facet: None,
                source: SignatureSource::Platform,
                is_collection: true,
                generic_params_count: 1,
            },
        );
    }

    /// Валидировать сигнатуру метода
    ///
    /// # Arguments
    /// * `expected` - Ожидаемая сигнатура из индекса (Option, так как метод может не существовать)
    /// * `actual` - Фактическая сигнатура из кода пользователя
    ///
    /// # Returns
    /// ValidationResult с деталями валидации
    pub fn validate_signature(
        &self,
        expected: Option<&MethodSignature>,
        actual: &MethodSignature,
    ) -> SignatureValidationResult {
        // Если метод не найден в индексе, считаем это OK (может быть пользовательский метод)
        let expected_sig = match expected {
            Some(sig) => sig,
            None => return SignatureValidationResult::Valid,
        };

        let mut mismatches = Vec::new();

        // Проверка количества параметров
        if expected_sig.params.len() != actual.params.len() {
            mismatches.push(SignatureMismatch::ParameterCount {
                expected: expected_sig.params.len(),
                actual: actual.params.len(),
            });
        }

        // Проверка типов параметров
        let param_count = expected_sig.params.len().min(actual.params.len());
        for i in 0..param_count {
            let expected_param = &expected_sig.params[i];
            let actual_param = &actual.params[i];

            if expected_param.type_name != actual_param.type_name {
                mismatches.push(SignatureMismatch::ParameterType {
                    index: i,
                    param_name: expected_param.name.clone(),
                    expected: expected_param
                        .type_name
                        .clone()
                        .unwrap_or_else(|| "Any".to_string()),
                    actual: actual_param
                        .type_name
                        .clone()
                        .unwrap_or_else(|| "Any".to_string()),
                });
            }
        }

        // Проверка типа возврата
        if expected_sig.return_type != actual.return_type {
            mismatches.push(SignatureMismatch::ReturnType {
                expected: expected_sig.return_type.clone(),
                actual: actual.return_type.clone(),
            });
        }

        if mismatches.is_empty() {
            SignatureValidationResult::Valid
        } else {
            SignatureValidationResult::Invalid(mismatches)
        }
    }
}

impl Default for SignatureIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_index_basic() {
        let mut index = SignatureIndex::new();

        let sig = MethodSignature {
            name: "Добавить".to_string(),
            owner_type: Some("Массив".to_string()),
            params: vec![],
            return_type: None,
            source: SignatureSource::Platform,
            return_facet: None,
            context_requirements: ContextRequirements::default(),
        };

        index.add_platform_method("Массив".to_string(), sig);

        let found = index.find_method("Массив", "Добавить");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Добавить");
    }

    #[test]
    fn test_signature_index_case_insensitive() {
        let mut index = SignatureIndex::new();

        let sig = MethodSignature {
            name: "Добавить".to_string(),
            owner_type: Some("Массив".to_string()),
            params: vec![],
            return_type: None,
            source: SignatureSource::Platform,
            return_facet: None,
            context_requirements: ContextRequirements::default(),
        };

        index.add_platform_method("Массив".to_string(), sig);

        // Разный регистр должен работать
        let found = index.find_method("Массив", "добавить");
        assert!(found.is_some());

        let found2 = index.find_method("Массив", "ДОБАВИТЬ");
        assert!(found2.is_some());
    }

    #[test]
    fn test_signature_index_not_found() {
        let index = SignatureIndex::new();

        let found = index.find_method("Массив", "НесуществующийМетод");
        assert!(found.is_none());
    }

    #[test]
    fn test_add_and_find_constructor() {
        let mut index = SignatureIndex::new();

        let constructor = ConstructorSignature {
            type_name: "Массив".to_string(),
            params: vec![],
            facet: None,
            source: SignatureSource::Platform,
            is_collection: true,
            generic_params_count: 1,
        };

        index.add_constructor("Массив".to_string(), constructor);

        let found = index.find_constructor("Массив");
        assert!(found.is_some());
        assert_eq!(found.unwrap().type_name, "Массив");
    }

    #[test]
    fn test_find_constructor_case_insensitive() {
        let mut index = SignatureIndex::new();
        index.initialize_builtin_constructors();

        // Поиск в разных регистрах
        assert!(index.find_constructor("Массив").is_some());
        assert!(index.find_constructor("массив").is_some());
        assert!(index.find_constructor("МАССИВ").is_some());
    }

    #[test]
    fn test_is_collection_type() {
        let mut index = SignatureIndex::new();
        index.initialize_builtin_constructors();

        assert!(index.is_collection_type("Массив"));
        assert!(index.is_collection_type("Соответствие"));
        assert!(!index.is_collection_type("ТаблицаЗначений"));
    }

    #[test]
    fn test_get_generic_params_count() {
        let mut index = SignatureIndex::new();
        index.initialize_builtin_constructors();

        assert_eq!(index.get_generic_params_count("Массив"), Some(1));
        assert_eq!(index.get_generic_params_count("Соответствие"), Some(2));
        assert_eq!(index.get_generic_params_count("ТаблицаЗначений"), Some(0));
    }

    #[test]
    fn test_builtin_constructors() {
        let mut index = SignatureIndex::new();
        index.initialize_builtin_constructors();

        // Проверяем что все встроенные конструкторы добавлены
        assert!(index.find_constructor("Массив").is_some());
        assert!(index.find_constructor("Соответствие").is_some());
        assert!(index.find_constructor("ТаблицаЗначений").is_some());
        assert!(index.find_constructor("СписокЗначений").is_some());
        assert!(index.find_constructor("ФиксированныйМассив").is_some());
    }

    // ================= Milestone 3.11 Phase 2: Faceted Type Support =================

    #[test]
    fn test_extract_base_facet_type_catalog() {
        // Справочники
        assert_eq!(
            SignatureIndex::extract_base_facet_type("СправочникМенеджер.Контрагенты"),
            Some("СправочникМенеджер")
        );
        assert_eq!(
            SignatureIndex::extract_base_facet_type("СправочникОбъект.Номенклатура"),
            Some("СправочникОбъект")
        );
        assert_eq!(
            SignatureIndex::extract_base_facet_type("СправочникСсылка.Валюты"),
            Some("СправочникСсылка")
        );
        assert_eq!(
            SignatureIndex::extract_base_facet_type("СправочникВыборка.Контрагенты"),
            Some("СправочникВыборка")
        );
    }

    #[test]
    fn test_extract_base_facet_type_document() {
        // Документы
        assert_eq!(
            SignatureIndex::extract_base_facet_type("ДокументМенеджер.ЗаказКлиента"),
            Some("ДокументМенеджер")
        );
        assert_eq!(
            SignatureIndex::extract_base_facet_type("ДокументОбъект.РасходнаяНакладная"),
            Some("ДокументОбъект")
        );
    }

    #[test]
    fn test_extract_base_facet_type_registers() {
        // Регистры сведений
        assert_eq!(
            SignatureIndex::extract_base_facet_type("РегистрСведенийМенеджер.КурсыВалют"),
            Some("РегистрСведенийМенеджер")
        );
        // Регистры накопления
        assert_eq!(
            SignatureIndex::extract_base_facet_type("РегистрНакопленияМенеджер.ОстаткиТоваров"),
            Some("РегистрНакопленияМенеджер")
        );
    }

    #[test]
    fn test_extract_base_facet_type_non_faceted() {
        // Не-фасетные типы должны возвращать None
        assert_eq!(SignatureIndex::extract_base_facet_type("Массив"), None);
        assert_eq!(SignatureIndex::extract_base_facet_type("Строка"), None);
        assert_eq!(
            SignatureIndex::extract_base_facet_type("ТаблицаЗначений"),
            None
        );
        // Базовые фасетные типы без имени тоже None
        assert_eq!(
            SignatureIndex::extract_base_facet_type("СправочникМенеджер"),
            None
        );
    }

    #[test]
    fn test_substitute_type_name() {
        // Базовые фасетные типы
        assert_eq!(
            SignatureIndex::substitute_type_name("СправочникОбъект", "Контрагенты"),
            "СправочникОбъект.Контрагенты"
        );
        assert_eq!(
            SignatureIndex::substitute_type_name("СправочникСсылка", "Номенклатура"),
            "СправочникСсылка.Номенклатура"
        );
        assert_eq!(
            SignatureIndex::substitute_type_name("ДокументОбъект", "ЗаказКлиента"),
            "ДокументОбъект.ЗаказКлиента"
        );

        // Не-фасетные типы остаются как есть
        assert_eq!(
            SignatureIndex::substitute_type_name("Неопределено", "Контрагенты"),
            "Неопределено"
        );
        assert_eq!(
            SignatureIndex::substitute_type_name("Строка", "Контрагенты"),
            "Строка"
        );
    }

    #[test]
    fn test_extract_metadata_name() {
        assert_eq!(
            SignatureIndex::extract_metadata_name("СправочникМенеджер.Контрагенты"),
            Some("Контрагенты")
        );
        assert_eq!(
            SignatureIndex::extract_metadata_name("ДокументОбъект.ЗаказКлиента"),
            Some("ЗаказКлиента")
        );
        // Не-фасетные типы
        assert_eq!(SignatureIndex::extract_metadata_name("Массив"), None);
        assert_eq!(
            SignatureIndex::extract_metadata_name("Файл.Существует"),
            None
        ); // Файл - не фасетный тип
    }

    #[test]
    fn test_find_method_faceted_fallback() {
        let mut index = SignatureIndex::new();

        // Добавляем метод под базовым типом (как в platform_types.rs)
        let sig = MethodSignature {
            name: "СоздатьЭлемент".to_string(),
            owner_type: Some("СправочникМенеджер".to_string()),
            params: vec![],
            return_type: Some("СправочникОбъект".to_string()),
            source: SignatureSource::Platform,
            return_facet: None,
            context_requirements: ContextRequirements::default(),
        };

        index.add_platform_method("СправочникМенеджер".to_string(), sig);

        // Поиск по точному имени (базовый тип) - должен найти
        let found_exact = index.find_method("СправочникМенеджер", "СоздатьЭлемент");
        assert!(found_exact.is_some());

        // Поиск по конкретизированному типу (fallback к базовому) - тоже должен найти
        let found_fallback =
            index.find_method("СправочникМенеджер.Контрагенты", "СоздатьЭлемент");
        assert!(found_fallback.is_some());
        assert_eq!(found_fallback.unwrap().name, "СоздатьЭлемент");
        assert_eq!(
            found_fallback.unwrap().return_type,
            Some("СправочникОбъект".to_string())
        );
    }

    #[test]
    fn test_find_method_document_faceted() {
        let mut index = SignatureIndex::new();

        let sig = MethodSignature {
            name: "Провести".to_string(),
            owner_type: Some("ДокументОбъект".to_string()),
            params: vec![],
            return_type: Some("Неопределено".to_string()),
            source: SignatureSource::Platform,
            return_facet: None,
            context_requirements: ContextRequirements::default(),
        };

        index.add_platform_method("ДокументОбъект".to_string(), sig);

        // Поиск через конкретизированный тип
        let found = index.find_method("ДокументОбъект.ЗаказКлиента", "Провести");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Провести");
    }

    #[test]
    fn test_find_method_non_faceted_still_works() {
        let mut index = SignatureIndex::new();

        let sig = MethodSignature {
            name: "Добавить".to_string(),
            owner_type: Some("Массив".to_string()),
            params: vec![],
            return_type: None,
            source: SignatureSource::Platform,
            return_facet: None,
            context_requirements: ContextRequirements::default(),
        };

        index.add_platform_method("Массив".to_string(), sig);

        // Обычный поиск по не-фасетному типу должен работать как раньше
        let found = index.find_method("Массив", "Добавить");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Добавить");
    }

    // ================= MILESTONE 3.11: Pattern-Matching Functions =================

    #[test]
    fn test_get_facet_kind_from_prefix_manager() {
        // Все менеджеры
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("СправочникМенеджер"),
            Some(FacetKind::Manager)
        );
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("ДокументМенеджер"),
            Some(FacetKind::Manager)
        );
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("РегистрСведенийМенеджер"),
            Some(FacetKind::Manager)
        );
        // МенеджерЗаписи тоже Manager
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("РегистрСведенийМенеджерЗаписи"),
            Some(FacetKind::Manager)
        );
    }

    #[test]
    fn test_get_facet_kind_from_prefix_object() {
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("СправочникОбъект"),
            Some(FacetKind::Object)
        );
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("ДокументОбъект"),
            Some(FacetKind::Object)
        );
        // Запись регистра - тоже Object
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("РегистрСведенийЗапись"),
            Some(FacetKind::Object)
        );
    }

    #[test]
    fn test_get_facet_kind_from_prefix_reference() {
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("СправочникСсылка"),
            Some(FacetKind::Reference)
        );
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("ДокументСсылка"),
            Some(FacetKind::Reference)
        );
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("ПеречислениеСсылка"),
            Some(FacetKind::Reference)
        );
    }

    #[test]
    fn test_get_facet_kind_from_prefix_selection() {
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("СправочникВыборка"),
            Some(FacetKind::Selection)
        );
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("ДокументВыборка"),
            Some(FacetKind::Selection)
        );
    }

    #[test]
    fn test_get_facet_kind_from_prefix_list() {
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("СправочникСписок"),
            Some(FacetKind::List)
        );
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("ДокументСписок"),
            Some(FacetKind::List)
        );
    }

    #[test]
    fn test_get_facet_kind_from_prefix_collection() {
        // Наборы записей - Collection
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("РегистрСведенийНаборЗаписей"),
            Some(FacetKind::Collection)
        );
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("РегистрНакопленияНаборЗаписей"),
            Some(FacetKind::Collection)
        );
    }

    #[test]
    fn test_get_facet_kind_from_prefix_non_faceted() {
        // Не-фасетные типы -> None
        assert_eq!(SignatureIndex::get_facet_kind_from_prefix("Массив"), None);
        assert_eq!(SignatureIndex::get_facet_kind_from_prefix("Строка"), None);
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("ТаблицаЗначений"),
            None
        );
    }

    #[test]
    fn test_get_metadata_kind_from_prefix_catalog() {
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("СправочникМенеджер"),
            Some(MetadataKind::Catalog)
        );
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("СправочникОбъект"),
            Some(MetadataKind::Catalog)
        );
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("СправочникСсылка"),
            Some(MetadataKind::Catalog)
        );
    }

    #[test]
    fn test_get_metadata_kind_from_prefix_document() {
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("ДокументМенеджер"),
            Some(MetadataKind::Document)
        );
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("ДокументОбъект"),
            Some(MetadataKind::Document)
        );
    }

    #[test]
    fn test_get_metadata_kind_from_prefix_registers() {
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("РегистрСведенийМенеджер"),
            Some(MetadataKind::InformationRegister)
        );
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("РегистрНакопленияНаборЗаписей"),
            Some(MetadataKind::AccumulationRegister)
        );
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("РегистрБухгалтерииВыборка"),
            Some(MetadataKind::AccountingRegister)
        );
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("РегистрРасчетаЗапись"),
            Some(MetadataKind::CalculationRegister)
        );
    }

    #[test]
    fn test_get_metadata_kind_from_prefix_plans() {
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("ПланСчетовМенеджер"),
            Some(MetadataKind::ChartOfAccounts)
        );
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("ПланВидовХарактеристикОбъект"),
            Some(MetadataKind::ChartOfCharacteristicTypes)
        );
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("ПланВидовРасчетаСсылка"),
            Some(MetadataKind::ChartOfCalculationTypes)
        );
    }

    #[test]
    fn test_get_metadata_kind_from_prefix_other() {
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("БизнесПроцессМенеджер"),
            Some(MetadataKind::BusinessProcess)
        );
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("ЗадачаОбъект"),
            Some(MetadataKind::Task)
        );
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("ПеречислениеМенеджер"),
            Some(MetadataKind::Enum)
        );
    }

    #[test]
    fn test_get_metadata_kind_from_prefix_non_faceted() {
        // Не-фасетные типы -> None
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("Массив"),
            None
        );
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("ТаблицаЗначений"),
            None
        );
    }

    // ================= MILESTONE 3.13: MetadataPatternRegistry Tests =================

    #[test]
    fn test_get_metadata_kind_from_prefix_exchange_plan() {
        // Планы обмена
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("ПланОбменаМенеджер"),
            Some(MetadataKind::ExchangePlan)
        );
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("ПланОбменаОбъект"),
            Some(MetadataKind::ExchangePlan)
        );
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("ПланОбменаСсылка"),
            Some(MetadataKind::ExchangePlan)
        );
    }

    #[test]
    fn test_resolve_metadata_kind_instance_method() {
        let index = SignatureIndex::new();

        // Instance method должен работать как статический (fallback)
        assert_eq!(
            index.resolve_metadata_kind("СправочникМенеджер"),
            Some(MetadataKind::Catalog)
        );
        assert_eq!(
            index.resolve_metadata_kind("ДокументОбъект"),
            Some(MetadataKind::Document)
        );
        assert_eq!(
            index.resolve_metadata_kind("ПланОбменаСсылка"),
            Some(MetadataKind::ExchangePlan)
        );
    }

    #[test]
    fn test_update_metadata_patterns() {
        use super::super::metadata_patterns::ExtractedPattern;

        let mut index = SignatureIndex::new();

        // Изначально нет извлечённых паттернов
        assert!(!index.has_extracted_metadata_patterns());
        assert_eq!(index.extracted_metadata_patterns_count(), 0);

        // Добавляем паттерны
        index.update_metadata_patterns(vec![
            ExtractedPattern {
                prefix: "Справочник".to_string(),
                kind: MetadataKind::Catalog,
                placeholder_suffix: Some("справочника".to_string()),
            },
            ExtractedPattern {
                prefix: "Документ".to_string(),
                kind: MetadataKind::Document,
                placeholder_suffix: Some("документа".to_string()),
            },
        ]);

        // Теперь есть извлечённые паттерны
        assert!(index.has_extracted_metadata_patterns());
        assert_eq!(index.extracted_metadata_patterns_count(), 2);

        // Резолвинг использует извлечённые паттерны
        assert_eq!(
            index.resolve_metadata_kind("СправочникМенеджер"),
            Some(MetadataKind::Catalog)
        );
    }

    #[test]
    fn test_metadata_patterns_accessor() {
        let index = SignatureIndex::new();

        // Можно получить ссылку на реестр
        let patterns = index.metadata_patterns();
        assert!(!patterns.has_extracted_patterns());
    }

    #[test]
    fn test_extract_base_facet_type_exchange_plan() {
        // Планы обмена должны распознаваться
        assert_eq!(
            SignatureIndex::extract_base_facet_type("ПланОбменаМенеджер.РаспределённаяБаза"),
            Some("ПланОбменаМенеджер")
        );
        assert_eq!(
            SignatureIndex::extract_base_facet_type("ПланОбменаОбъект.РаспределённаяБаза"),
            Some("ПланОбменаОбъект")
        );
    }
}
