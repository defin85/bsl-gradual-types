//! SignatureIndex - основная структура индекса сигнатур
//!
//! Содержит:
//! - SignatureIndex struct
//! - Методы add/find для методов и конструкторов
//! - Валидация сигнатур
//! - Интеграция с MetadataPatternRegistry

use super::super::metadata_patterns::{ExtractedPattern, MetadataPatternRegistry};
use super::super::types::{FacetKind, MetadataKind, ParameterInfo};
use super::facet_helpers;
use super::method::MethodSignature;
use super::types::{ConstructorSignature, SignatureMismatch, SignatureSource, SignatureValidationResult};
use std::collections::HashMap;

// Re-export ContextRequirements для обратной совместимости
pub use super::super::runtime_context::ContextRequirements;

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

    // ==================== Методы добавления ====================

    /// Добавить платформенный метод с поддержкой merge
    ///
    /// Если метод с таким именем уже существует, обновляет его поля если:
    /// - return_type у существующего None, а у нового Some
    /// - return_facet у существующего None, а у нового Some
    /// - context_requirements у существующего Universal, а у нового более специфичные
    ///
    /// Это позволяет "обогащать" методы из syntax_helper (без return types)
    /// данными из platform_types.rs (с return types).
    pub fn add_platform_method(&mut self, type_name: String, method: MethodSignature) {
        let methods = self.platform_methods.entry(type_name.clone()).or_default();

        // Ищем существующий метод с таким же именем (регистронезависимо)
        let method_name_lower = method.name.to_lowercase();
        if let Some(existing) = methods.iter_mut().find(|m| m.name.to_lowercase() == method_name_lower) {
            // Обновляем return_type если у существующего None/пустой
            if existing.return_type.as_ref().is_none_or(|s| s.is_empty()) {
                if let Some(ref new_return_type) = method.return_type {
                    tracing::debug!(
                        "Merge {}.{}: return_type updated to '{}'",
                        type_name, method.name, new_return_type
                    );
                    existing.return_type = method.return_type;
                }
            } else if method.return_type.is_some() && existing.return_type != method.return_type {
                // Конфликт: оба return_type непустые, но разные
                tracing::warn!(
                    "Merge conflict {}.{}: return_type '{}' vs '{}' - keeping first",
                    type_name, method.name,
                    existing.return_type.as_deref().unwrap_or("None"),
                    method.return_type.as_deref().unwrap_or("None")
                );
            }

            // Обновляем return_facet если у существующего None
            if existing.return_facet.is_none() && method.return_facet.is_some() {
                tracing::debug!(
                    "Merge {}.{}: return_facet updated to {:?}",
                    type_name, method.name, method.return_facet
                );
                existing.return_facet = method.return_facet;
            }

            // Обновляем context_requirements если у существующего Universal
            if existing.context_requirements == ContextRequirements::Universal
               && method.context_requirements != ContextRequirements::Universal {
                tracing::debug!(
                    "Merge {}.{}: context_requirements updated to {:?}",
                    type_name, method.name, method.context_requirements
                );
                existing.context_requirements = method.context_requirements;
            }

            // Обновляем params если у существующего пусто
            if existing.params.is_empty() && !method.params.is_empty() {
                tracing::debug!(
                    "Merge {}.{}: params updated ({} params)",
                    type_name, method.name, method.params.len()
                );
                existing.params = method.params;
            }
        } else {
            // Метод не найден - добавляем новый
            tracing::trace!(
                "Add new method {}.{} (return_type: {:?})",
                type_name, method.name, method.return_type
            );
            methods.push(method);
        }
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

    // ==================== Методы поиска ====================

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
    /// find_method("Массив", "Добавить") // -> найдёт "Массив.Добавить"
    ///
    /// // Фасетный поиск
    /// find_method("СправочникМенеджер.Контрагенты", "СоздатьЭлемент")
    /// // -> не найдёт по точному ключу
    /// // -> извлечёт базовый тип "СправочникМенеджер"
    /// // -> найдёт "СправочникМенеджер.СоздатьЭлемент"
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

    /// Получить все методы для указанного типа (Milestone 3.15: Pre-warm Cache)
    ///
    /// Возвращает все платформенные и конфигурационные методы для типа.
    ///
    /// # Example
    /// ```ignore
    /// let methods = signature_index.get_type_methods("Массив");
    /// for method in methods {
    ///     println!("{}", method.name);
    /// }
    /// ```
    pub fn get_type_methods(&self, type_name: &str) -> Vec<&MethodSignature> {
        let mut result = Vec::new();

        // Платформенные методы
        if let Some(methods) = self.platform_methods.get(type_name) {
            result.extend(methods.iter());
        }

        // Конфигурационные методы
        if let Some(methods) = self.config_methods.get(type_name) {
            result.extend(methods.iter());
        }

        result
    }

    /// Найти глобальную функцию
    pub fn find_global_function(&self, name: &str) -> Option<&MethodSignature> {
        let name_lower = name.to_lowercase();
        self.global_functions
            .iter()
            .find(|(k, _)| k.to_lowercase() == name_lower)
            .map(|(_, v)| v)
    }

    // ==================== Facet Helper Methods (делегирование) ====================

    /// Извлечь базовый фасетный тип из полного имени типа
    ///
    /// # Примеры
    /// - "СправочникМенеджер.Контрагенты" -> Some("СправочникМенеджер")
    /// - "ДокументОбъект.ЗаказКлиента" -> Some("ДокументОбъект")
    /// - "Массив" -> None (не фасетный тип)
    pub fn extract_base_facet_type(type_name: &str) -> Option<&str> {
        facet_helpers::extract_base_facet_type(type_name)
    }

    /// Получить FacetKind из фасетного префикса по суффиксу
    ///
    /// # Примеры
    /// - "СправочникМенеджер" -> Some(FacetKind::Manager)
    /// - "ДокументОбъект" -> Some(FacetKind::Object)
    pub fn get_facet_kind_from_prefix(prefix: &str) -> Option<FacetKind> {
        facet_helpers::get_facet_kind_from_prefix(prefix)
    }

    /// Получить MetadataKind из фасетного префикса по его началу
    ///
    /// # Примеры
    /// - "СправочникМенеджер" -> Some(MetadataKind::Catalog)
    /// - "ДокументОбъект" -> Some(MetadataKind::Document)
    pub fn get_metadata_kind_from_prefix(prefix: &str) -> Option<MetadataKind> {
        facet_helpers::get_metadata_kind_from_prefix(prefix)
    }

    /// Подставить реальное имя объекта в return type вместо placeholder
    ///
    /// # Примеры
    /// - ("СправочникОбъект", "Контрагенты") -> "СправочникОбъект.Контрагенты"
    pub fn substitute_type_name(return_type: &str, actual_name: &str) -> String {
        facet_helpers::substitute_type_name(return_type, actual_name)
    }

    /// Извлечь имя объекта метаданных из фасетного типа
    ///
    /// # Примеры
    /// - "СправочникМенеджер.Контрагенты" -> Some("Контрагенты")
    pub fn extract_metadata_name(type_name: &str) -> Option<&str> {
        facet_helpers::extract_metadata_name(type_name)
    }

    // ==================== MetadataPatternRegistry Integration ====================

    /// Обновить паттерны MetadataKind из распарсенного Syntax Helper
    ///
    /// Вызывается при загрузке типов платформы для извлечения паттернов
    /// из имён типов с placeholder (например, "СправочникМенеджер.<Имя справочника>")
    pub fn update_metadata_patterns(&mut self, patterns: Vec<ExtractedPattern>) {
        let count = patterns.len();
        self.metadata_patterns.update_from_patterns(patterns);
        tracing::debug!("SignatureIndex: обновлено {} паттернов MetadataKind", count);
    }

    /// Определить MetadataKind из префикса (instance метод - использует реестр паттернов)
    ///
    /// Сначала ищет в извлечённых паттернах из Syntax Helper,
    /// затем использует hardcoded fallback.
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

    // ==================== Инициализация встроенных конструкторов ====================

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

    // ==================== Валидация сигнатур ====================

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
