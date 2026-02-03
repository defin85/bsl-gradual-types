//! SignatureIndex - основная структура индекса сигнатур
//!
//! Содержит:
//! - SignatureIndex struct
//! - Методы add/find для методов и конструкторов
//! - Валидация сигнатур
//! - Интеграция с MetadataPatternRegistry

use super::facet_helpers;
use super::method::MethodSignature;
use super::types::{ConstructorSignature, SignatureMismatch, SignatureValidationResult};
use bsl_types::metadata_patterns::{ExtractedPattern, MetadataPatternRegistry};
use bsl_types::types::{FacetKind, MetadataKind};
use bsl_types::{ContextRequirements, TypeId};
use std::collections::{HashMap, HashSet};

/// Индекс сигнатур функций и методов
#[derive(Debug, Clone)]
pub struct SignatureIndex {
    /// Платформенные методы: тип -> список методов
    platform_methods: HashMap<TypeId, Vec<MethodSignature>>,
    /// Конфигурационные методы: тип -> список методов
    config_methods: HashMap<TypeId, Vec<MethodSignature>>,
    /// Глобальные функции: имя -> сигнатура
    global_functions: HashMap<TypeId, MethodSignature>,
    /// Конструкторы: тип -> сигнатура конструктора
    /// Например: "Массив" -> ConstructorSignature { params: [размер?], ... }
    constructors: HashMap<TypeId, ConstructorSignature>,
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

    /// Добавить платформенный метод с поддержкой merge и overload'ов
    ///
    /// Если overload с такой же сигнатурой уже существует, обновляет его поля если:
    /// - return_type у существующего None, а у нового Some
    /// - return_facet у существующего None, а у нового Some
    /// - context_requirements у существующего Universal, а у нового более специфичные
    ///
    /// Это позволяет "обогащать" методы из syntax_helper данными из других источников,
    /// не теряя overload'ы (несколько вариантов синтаксиса одного метода).
    pub fn add_platform_method(&mut self, type_id: TypeId, method: MethodSignature) {
        if tracing::enabled!(tracing::Level::DEBUG) {
            tracing::debug!(
                type_id = %type_id,
                method_name = %method.name,
                "SignatureIndex: adding platform method"
            );
        }

        let methods = self.platform_methods.entry(type_id.clone()).or_default();

        // Ищем существующий overload с такой же сигнатурой (регистронезависимо)
        if let Some(existing) = methods.iter_mut().find(|m| Self::same_overload(m, &method)) {
            // Обновляем return_type если у существующего None/пустой
            if existing.return_type.as_ref().is_none_or(|s| s.is_empty()) {
                if let Some(ref new_return_type) = method.return_type {
                    tracing::debug!(
                        type_id = %type_id,
                        method_name = %method.name,
                        return_type = %new_return_type,
                        "SignatureIndex: merged return_type"
                    );
                    existing.return_type = method.return_type;
                }
            } else if method.return_type.is_some() && existing.return_type != method.return_type {
                // Конфликт: оба return_type непустые, но разные
                tracing::warn!(
                    type_id = %type_id,
                    method_name = %method.name,
                    existing_return_type = ?existing.return_type,
                    new_return_type = ?method.return_type,
                    "SignatureIndex: merge conflict on return_type, keeping first"
                );
            }

            // Обновляем return_facet если у существующего None
            if existing.return_facet.is_none() && method.return_facet.is_some() {
                tracing::debug!(
                    type_id = %type_id,
                    method_name = %method.name,
                    return_facet = ?method.return_facet,
                    "SignatureIndex: merged return_facet"
                );
                existing.return_facet = method.return_facet;
            }

            // Обновляем context_requirements если у существующего Universal
            if existing.context_requirements == ContextRequirements::Universal
                && method.context_requirements != ContextRequirements::Universal
            {
                tracing::debug!(
                    type_id = %type_id,
                    method_name = %method.name,
                    context_requirements = ?method.context_requirements,
                    "SignatureIndex: merged context_requirements"
                );
                existing.context_requirements = method.context_requirements;
            }

            // Обновляем params:
            // - если у существующего пусто -> берём целиком
            // - иначе "обогащаем" поэлементно (тип/имя/default), не меняя shape overload'а
            if existing.params.is_empty() && !method.params.is_empty() {
                tracing::debug!(
                    type_id = %type_id,
                    method_name = %method.name,
                    param_count = method.params.len(),
                    "SignatureIndex: merged params (filled empty)"
                );
                existing.params = method.params;
            } else if !existing.params.is_empty() && existing.params.len() == method.params.len() {
                for (existing_param, new_param) in
                    existing.params.iter_mut().zip(method.params.iter())
                {
                    if existing_param.name.is_empty() && !new_param.name.is_empty() {
                        existing_param.name = new_param.name.clone();
                    }

                    let existing_type = existing_param.type_name.as_deref().unwrap_or("").trim();
                    let new_type = new_param.type_name.as_deref().unwrap_or("").trim();
                    let existing_is_unknown = existing_type.is_empty()
                        || existing_type.eq_ignore_ascii_case("произвольный");
                    let new_is_unknown =
                        new_type.is_empty() || new_type.eq_ignore_ascii_case("произвольный");
                    if existing_is_unknown && !new_is_unknown {
                        existing_param.type_name = new_param.type_name.clone();
                    }

                    if existing_param.default_value.is_none() && new_param.default_value.is_some() {
                        existing_param.default_value = new_param.default_value.clone();
                    }

                    if existing_param.description.is_none() && new_param.description.is_some() {
                        existing_param.description = new_param.description.clone();
                    }
                }
            }
        } else {
            // Метод не найден - добавляем новый
            if tracing::enabled!(tracing::Level::TRACE) {
                tracing::trace!(
                    type_id = %type_id,
                    method_name = %method.name,
                    return_type = ?method.return_type,
                    "SignatureIndex: added new platform method"
                );
            }
            methods.push(method);
        }
    }

    fn same_overload(a: &MethodSignature, b: &MethodSignature) -> bool {
        if a.name.to_lowercase() != b.name.to_lowercase() {
            return false;
        }
        if a.params.len() != b.params.len() {
            return false;
        }

        // Overload matching policy:
        // - required/optional shape must match
        // - если тип параметра известен у ОБОИХ и различается -> это разные overload'ы
        // - если тип неизвестен хотя бы у одного -> считаем совместимыми (можно merge'ить)
        a.params.iter().zip(b.params.iter()).all(|(ap, bp)| {
            if ap.is_optional != bp.is_optional {
                return false;
            }

            match (&ap.type_name, &bp.type_name) {
                (Some(a_ty), Some(b_ty)) => a_ty.to_lowercase() == b_ty.to_lowercase(),
                _ => true,
            }
        })
    }

    /// Добавить конфигурационный метод
    pub fn add_config_method(&mut self, type_id: TypeId, method: MethodSignature) {
        self.config_methods.entry(type_id).or_default().push(method);
    }

    /// Добавить глобальную функцию
    pub fn add_global_function(&mut self, name: TypeId, method: MethodSignature) {
        self.global_functions.insert(name, method);
    }

    /// Удалить конфигурационные методы по имени (регистронезависимо)
    pub fn remove_config_methods(&mut self, type_name: &str, method_names: &[String]) -> usize {
        let type_id = TypeId::new(type_name);
        let Some(methods) = self.config_methods.get_mut(&type_id) else {
            return 0;
        };

        if method_names.is_empty() {
            return 0;
        }

        let target: HashSet<String> = method_names.iter().map(|n| n.to_lowercase()).collect();
        let before = methods.len();
        methods.retain(|m| !target.contains(&m.name.to_lowercase()));
        let removed = before.saturating_sub(methods.len());

        if methods.is_empty() {
            self.config_methods.remove(&type_id);
        }

        removed
    }

    /// Удалить глобальные функции по имени (регистронезависимо)
    pub fn remove_global_functions(&mut self, function_names: &[String]) -> usize {
        if function_names.is_empty() {
            return 0;
        }

        let mut removed = 0;
        for name in function_names {
            if self.global_functions.remove(&TypeId::new(name)).is_some() {
                removed += 1;
            }
        }

        removed
    }

    /// Добавить конструктор
    pub fn add_constructor(&mut self, type_id: TypeId, constructor: ConstructorSignature) {
        self.constructors.insert(type_id, constructor);
    }

    // ==================== Методы поиска ====================

    /// Найти конструктор по имени типа (регистронезависимо)
    pub fn find_constructor(&self, type_name: &str) -> Option<&ConstructorSignature> {
        let type_id = TypeId::new(type_name);
        self.constructors.get(&type_id)
    }

    /// Получить все конструкторы
    pub fn get_all_constructors(&self) -> &HashMap<TypeId, ConstructorSignature> {
        &self.constructors
    }

    /// Получить все глобальные функции
    pub fn get_global_functions(&self) -> &HashMap<TypeId, MethodSignature> {
        &self.global_functions
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
    /// ```rust,no_run
    /// # use bsl_repository::signature_index::SignatureIndex;
    /// let index = SignatureIndex::new();
    ///
    /// // Точный поиск
    /// let exact = index.find_method("Массив", "Добавить");
    /// // -> найдёт "Массив.Добавить" (если индекс заполнен)
    ///
    /// // Фасетный поиск
    /// let facet = index.find_method("СправочникМенеджер.Контрагенты", "СоздатьЭлемент");
    /// // -> не найдёт по точному ключу
    /// // -> извлечёт базовый тип "СправочникМенеджер"
    /// // -> найдёт "СправочникМенеджер.СоздатьЭлемент"
    /// # let _ = (exact, facet);
    /// ```
    pub fn find_method(&self, type_name: &str, method_name: &str) -> Option<&MethodSignature> {
        if tracing::enabled!(tracing::Level::DEBUG) {
            tracing::debug!(
                type_name = %type_name,
                method_name = %method_name,
                "SignatureIndex: searching for method"
            );
        }

        let type_id = TypeId::new(type_name);
        self.find_method_by_type_id(&type_id, method_name)
    }

    /// Найти все overload'ы метода по имени типа и имени метода.
    pub fn find_methods(&self, type_name: &str, method_name: &str) -> Vec<&MethodSignature> {
        let type_id = TypeId::new(type_name);
        self.find_methods_by_type_id(&type_id, method_name)
    }

    /// Внутренний поиск метода по TypeId с поддержкой fallback на базовый тип
    fn find_method_by_type_id(
        &self,
        type_id: &TypeId,
        method_name: &str,
    ) -> Option<&MethodSignature> {
        let method_name_lower = method_name.to_lowercase();

        // Поиск в platform_methods
        if let Some(methods) = self.platform_methods.get(type_id) {
            if let Some(m) = methods
                .iter()
                .find(|m| m.name.to_lowercase() == method_name_lower)
            {
                if tracing::enabled!(tracing::Level::DEBUG) {
                    tracing::debug!(
                        type_id = %type_id,
                        method_name = %method_name,
                        "SignatureIndex: method found in platform_methods"
                    );
                }
                return Some(m);
            }
        }

        // Поиск в config_methods
        if let Some(methods) = self.config_methods.get(type_id) {
            if let Some(m) = methods
                .iter()
                .find(|m| m.name.to_lowercase() == method_name_lower)
            {
                if tracing::enabled!(tracing::Level::DEBUG) {
                    tracing::debug!(
                        type_id = %type_id,
                        method_name = %method_name,
                        "SignatureIndex: method found in config_methods"
                    );
                }
                return Some(m);
            }
        }

        // Fallback для фасетных типов через base_type()
        if let Some(base_id) = type_id.base_type() {
            if tracing::enabled!(tracing::Level::DEBUG) {
                tracing::debug!(
                    original_type = %type_id,
                    base_type = %base_id,
                    method_name = %method_name,
                    "SignatureIndex: falling back to base facet type"
                );
            }
            return self.find_method_by_type_id(&base_id, method_name);
        }

        if tracing::enabled!(tracing::Level::DEBUG) {
            tracing::debug!(
                type_id = %type_id,
                method_name = %method_name,
                "SignatureIndex: method not found"
            );
        }

        None
    }

    fn find_methods_by_type_id<'a>(
        &'a self,
        type_id: &TypeId,
        method_name: &str,
    ) -> Vec<&'a MethodSignature> {
        let method_name_lower = method_name.to_lowercase();

        // 1) точный тип
        let mut out = Vec::new();
        if let Some(methods) = self.platform_methods.get(type_id) {
            out.extend(
                methods
                    .iter()
                    .filter(|m| m.name.to_lowercase() == method_name_lower),
            );
        }
        if let Some(methods) = self.config_methods.get(type_id) {
            out.extend(
                methods
                    .iter()
                    .filter(|m| m.name.to_lowercase() == method_name_lower),
            );
        }
        if !out.is_empty() {
            return out;
        }

        // 2) fallback на базовый фасетный тип
        if let Some(base) = facet_helpers::extract_base_facet_type(type_id.display()) {
            let base_id = TypeId::new(base);
            if let Some(methods) = self.platform_methods.get(&base_id) {
                out.extend(
                    methods
                        .iter()
                        .filter(|m| m.name.to_lowercase() == method_name_lower),
                );
            }
            if let Some(methods) = self.config_methods.get(&base_id) {
                out.extend(
                    methods
                        .iter()
                        .filter(|m| m.name.to_lowercase() == method_name_lower),
                );
            }
        }

        out
    }

    /// Получить все методы для указанного типа (Milestone 3.15: Pre-warm Cache)
    ///
    /// Возвращает все платформенные и конфигурационные методы для типа.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use bsl_repository::signature_index::SignatureIndex;
    /// let signature_index = SignatureIndex::new();
    /// let methods = signature_index.get_type_methods("Массив");
    /// for method in methods {
    ///     println!("{}", method.name);
    /// }
    /// ```
    pub fn get_type_methods(&self, type_name: &str) -> Vec<&MethodSignature> {
        let type_id = TypeId::new(type_name);
        let mut result = Vec::new();

        // Платформенные методы
        if let Some(methods) = self.platform_methods.get(&type_id) {
            result.extend(methods.iter());
        }

        // Конфигурационные методы
        if let Some(methods) = self.config_methods.get(&type_id) {
            result.extend(methods.iter());
        }

        result
    }

    /// Найти глобальную функцию
    pub fn find_global_function(&self, name: &str) -> Option<&MethodSignature> {
        let id = TypeId::new(name);
        self.global_functions.get(&id)
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
    /// Ищет в извлечённых паттернах из Syntax Helper.
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

    /// Валидировать сигнатуру вызова по набору overload'ов.
    ///
    /// Считается валидным, если совпал хотя бы один overload.
    pub fn validate_overloaded_signature(
        &self,
        expected: &[&MethodSignature],
        actual: &MethodSignature,
    ) -> SignatureValidationResult {
        if expected.is_empty() {
            return SignatureValidationResult::Valid;
        }

        let mut best_mismatches: Option<Vec<SignatureMismatch>> = None;

        for sig in expected {
            match self.validate_signature(Some(sig), actual) {
                SignatureValidationResult::Valid => return SignatureValidationResult::Valid,
                SignatureValidationResult::Invalid(m) => {
                    if best_mismatches
                        .as_ref()
                        .is_none_or(|best| m.len() < best.len())
                    {
                        best_mismatches = Some(m);
                    }
                }
            }
        }

        SignatureValidationResult::Invalid(best_mismatches.unwrap_or_default())
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
    use crate::signature_index::SignatureSource;

    #[test]
    fn test_type_id_normalization_fallback() {
        let mut index = SignatureIndex::new();

        // Добавим метод для типа "Табличная часть" (с пробелом)
        let method = MethodSignature::new(
            "Выгрузить".to_string(),
            Some("Табличная часть".to_string()),
            vec![],
            Some("Массив".to_string()),
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::Universal,
        );

        index.add_platform_method(TypeId::new("Табличная часть"), method);

        // Поиск по CamelCase варианту должен работать через TypeId нормализацию
        let result = index.find_method("ТабличнаяЧасть", "Выгрузить");
        assert!(
            result.is_some(),
            "Метод должен быть найден через TypeId нормализацию CamelCase -> с пробелами"
        );

        let found_method = result.unwrap();
        assert_eq!(found_method.name, "Выгрузить");
        assert_eq!(found_method.return_type.as_deref(), Some("Массив"));
    }

    #[test]
    fn test_type_id_normalization() {
        // Проверяем что TypeId правильно нормализует имена
        // TypeId("ТабличнаяЧасть") == TypeId("Табличная часть")
        let id1 = TypeId::new("ТабличнаяЧасть");
        let id2 = TypeId::new("Табличная часть");
        assert_eq!(
            id1, id2,
            "TypeId должен нормализовать CamelCase и варианты с пробелами"
        );

        // Проверяем lowercase нормализацию
        let id3 = TypeId::new("МАССИВ");
        let id4 = TypeId::new("массив");
        let id5 = TypeId::new("Массив");
        assert_eq!(id3, id4);
        assert_eq!(id4, id5);
    }
}
