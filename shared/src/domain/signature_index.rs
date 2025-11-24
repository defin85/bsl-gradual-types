//! Signature Index - индекс сигнатур функций и методов
//!
//! Milestone 2.20: Function Signature Validation System
//! Milestone 3.11: Method Signature Enhancement with Facets and Context

use super::types::{FacetKind, ParameterInfo};
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
    pub fn find_method(&self, type_name: &str, method_name: &str) -> Option<&MethodSignature> {
        let method_name_lower = method_name.to_lowercase();

        // Поиск в платформенных
        if let Some(methods) = self.platform_methods.get(type_name) {
            if let Some(m) = methods
                .iter()
                .find(|m| m.name.to_lowercase() == method_name_lower)
            {
                return Some(m);
            }
        }

        // Поиск в конфигурационных
        if let Some(methods) = self.config_methods.get(type_name) {
            if let Some(m) = methods
                .iter()
                .find(|m| m.name.to_lowercase() == method_name_lower)
            {
                return Some(m);
            }
        }

        None
    }

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
}
