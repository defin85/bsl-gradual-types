//! Type validator based on Balyuk & Popova (2021) research
//!
//! Implements validation rules for:
//! 1. Incorrect parameter passing to methods
//! 2. Access to non-existent properties of objects
//! 3. Treating simple types as collections

use crate::domain::metadata_lookup::TypeMetadataLookup;
use crate::domain::resolver::names_equal_ignore_case;
use crate::domain::types::{
    ConcreteType, MetadataKind, SpecialType, TypeResolution, UncertaintyReason,
};

use super::TypeErrorKind;

/// Валидатор типов на основе правил из статьи
pub struct TypeValidator<'a> {
    metadata_lookup: &'a TypeMetadataLookup,
}

impl<'a> TypeValidator<'a> {
    /// Создаёт новый валидатор с доступом к метаданным
    pub fn new(metadata_lookup: &'a TypeMetadataLookup) -> Self {
        Self { metadata_lookup }
    }

    /// Проверка существования метода у объекта (новый метод!)
    pub fn validate_method_exists(
        &self,
        object_resolution: &TypeResolution,
        method_name: &str,
    ) -> Option<TypeErrorKind> {
        let methods = self.metadata_lookup.get_methods(object_resolution);

        // Проверяем есть ли метод с таким именем (case-insensitive для кириллицы и латиницы)
        let method_exists = methods.iter().any(|m| {
            names_equal_ignore_case(&m.name, method_name)
                || names_equal_ignore_case(&m.english_name, method_name)
        });

        if !method_exists {
            // Получаем читаемое имя типа для сообщения об ошибке
            let type_name = Self::resolution_to_string(object_resolution);
            Some(TypeErrorKind::NonExistentMethod {
                object_type: type_name,
                method_name: method_name.to_string(),
                variable_name: None,  // MILESTONE 3.6 Phase 3: will be passed from visitor
            })
        } else {
            None
        }
    }

    /// MILESTONE 3.6 Phase 3: version with variable_name
    pub fn validate_method_exists_with_variable(
        &self,
        object_resolution: &TypeResolution,
        method_name: &str,
        variable_name: Option<String>,
    ) -> Option<TypeErrorKind> {
        let methods = self.metadata_lookup.get_methods(object_resolution);

        tracing::info!(
            "validate_method: method='{}', type='{}', active_facet={:?}, found {} methods",
            method_name,
            object_resolution.type_name(),
            object_resolution.active_facet,
            methods.len()
        );

        let method_exists = methods.iter().any(|m| {
            names_equal_ignore_case(&m.name, method_name)
                || names_equal_ignore_case(&m.english_name, method_name)
        });

        if !method_exists {
            let type_name = Self::resolution_to_string(object_resolution);
            tracing::warn!(
                "Method '{}' NOT found for type '{}' (active_facet={:?})",
                method_name,
                type_name,
                object_resolution.active_facet
            );
            Some(TypeErrorKind::NonExistentMethod {
                object_type: type_name,
                method_name: method_name.to_string(),
                variable_name,
            })
        } else {
            None
        }
    }

    /// Проверка существования свойства у объекта (обновлённый метод)
    pub fn validate_property_exists(
        &self,
        object_resolution: &TypeResolution,
        property_name: &str,
    ) -> Option<TypeErrorKind> {
        let properties = self.metadata_lookup.get_properties(object_resolution);

        // Проверяем есть ли свойство с таким именем (case-insensitive)
        let property_exists = properties
            .iter()
            .any(|p| names_equal_ignore_case(&p.name, property_name));

        if !property_exists {
            let type_name = Self::resolution_to_string(object_resolution);
            Some(TypeErrorKind::NonExistentProperty {
                object_type: type_name,
                property_name: property_name.to_string(),
                variable_name: None,  // MILESTONE 3.6 Phase 3: will be passed from visitor
            })
        } else {
            None
        }
    }

    /// MILESTONE 3.6 Phase 3: version with variable_name
    pub fn validate_property_exists_with_variable(
        &self,
        object_resolution: &TypeResolution,
        property_name: &str,
        variable_name: Option<String>,
    ) -> Option<TypeErrorKind> {
        let properties = self.metadata_lookup.get_properties(object_resolution);

        let property_exists = properties
            .iter()
            .any(|p| names_equal_ignore_case(&p.name, property_name));

        if !property_exists {
            let type_name = Self::resolution_to_string(object_resolution);
            Some(TypeErrorKind::NonExistentProperty {
                object_type: type_name,
                property_name: property_name.to_string(),
                variable_name,
            })
        } else {
            None
        }
    }

    /// MILESTONE 3.16: Валидация существования объекта метаданных
    ///
    /// # Параметры
    ///
    /// * `kind` - вид метаданных (Catalog, Document, etc.)
    /// * `name` - имя объекта (например, "Контрагенты")
    /// * `variable_name` - имя переменной (для диагностического сообщения)
    ///
    /// # Возвращает
    ///
    /// `Some(TypeErrorKind::UnknownMetadataObject)` если объект не найден,
    /// `None` если объект существует или конфигурация не загружена
    ///
    /// # Пример
    ///
    /// ```ignore
    /// // Валидация: Справочники.Контрогенты (опечатка)
    /// if let Some(error) = validator.validate_metadata_object_exists(
    ///     MetadataKind::Catalog,
    ///     "Контрогенты",
    ///     Some("спр".to_string()),
    /// ) {
    ///     // error = UnknownMetadataObject { kind: Catalog, name: "Контрогенты", suggestions: ["Контрагенты"], ... }
    /// }
    /// ```
    pub fn validate_metadata_object_exists(
        &self,
        kind: MetadataKind,
        name: &str,
        variable_name: Option<String>,
    ) -> Option<TypeErrorKind> {
        // Graceful degradation: если конфигурация не загружена, не показываем ошибку
        if !self.metadata_lookup.is_configuration_loaded() {
            return None;
        }

        // Проверяем существование объекта метаданных
        if self.metadata_lookup.exists_metadata_object(kind, name) {
            return None; // Объект найден
        }

        // Объект не найден - генерируем предложения
        let suggestions = self.metadata_lookup.suggest_similar_names(kind, name, 3);

        Some(TypeErrorKind::UnknownMetadataObject {
            kind,
            name: name.to_string(),
            suggestions,
            variable_name,
        })
    }

    /// MILESTONE 3.16: Validate type resolution and extract error if appropriate
    ///
    /// This method checks the `uncertainty_reason` field of a TypeResolution
    /// and generates appropriate validation errors.
    ///
    /// # Behavior
    ///
    /// - If `uncertainty_reason` is `MetadataObjectNotFound` and configuration is loaded,
    ///   returns `TypeErrorKind::UnknownMetadataObject` with suggestions
    /// - If `uncertainty_reason` is `ConfigurationNotLoaded`, returns `None` (graceful degradation)
    /// - Otherwise returns `None`
    ///
    /// # Example
    ///
    /// ```ignore
    /// let resolution = resolver.resolve_expression_sync("Справочники.Контрогенты");
    /// if let Some(error) = validator.validate_from_resolution(&resolution) {
    ///     // error = UnknownMetadataObject { kind: Catalog, name: "Контрогенты", ... }
    /// }
    /// ```
    pub fn validate_from_resolution(&self, resolution: &TypeResolution) -> Option<TypeErrorKind> {
        // Check if there's an uncertainty reason in the resolution metadata
        if let Some(reason) = &resolution.metadata.uncertainty_reason {
            match reason {
                UncertaintyReason::MetadataObjectNotFound { kind, name } => {
                    // Generate suggestions for similar names
                    let suggestions = self.metadata_lookup.suggest_similar_names(*kind, name, 3);

                    Some(TypeErrorKind::UnknownMetadataObject {
                        kind: *kind,
                        name: name.clone(),
                        suggestions,
                        variable_name: None, // Will be filled by the caller if needed
                    })
                }
                UncertaintyReason::ConfigurationNotLoaded => {
                    // Graceful degradation: don't report errors when config is not loaded
                    None
                }
                UncertaintyReason::Other(_) => {
                    // Other reasons don't generate validation errors
                    None
                }
                UncertaintyReason::UndeclaredVariable { name } => {
                    // Undeclared variable errors are handled separately in visitor
                    // to provide better context (method name, param index)
                    Some(TypeErrorKind::UndeclaredVariable {
                        variable_name: name.clone(),
                        method_name: None,
                        param_index: None,
                    })
                }
            }
        } else {
            None
        }
    }

    /// Проверка операций с коллекциями
    pub fn validate_collection_operation(
        type_resolution: &TypeResolution,
        operation: &str,
    ) -> Option<TypeErrorKind> {
        use crate::domain::types::ResolutionResult;

        match &type_resolution.result {
            ResolutionResult::Concrete(ConcreteType::Primitive(prim)) => {
                // Примитивные типы нельзя использовать как коллекции
                Some(TypeErrorKind::SimpleTypeAsCollection {
                    type_name: prim.display_name().to_string(),
                    operation: operation.to_string(),
                    variable_name: None,  // MILESTONE 3.6 Phase 3: will be passed from visitor
                })
            }
            ResolutionResult::Concrete(ConcreteType::Special(SpecialType::Undefined))
            | ResolutionResult::Concrete(ConcreteType::Special(SpecialType::Null)) => {
                Some(TypeErrorKind::SimpleTypeAsCollection {
                    type_name: "Неопределено".to_string(),
                    operation: operation.to_string(),
                    variable_name: None,  // MILESTONE 3.6 Phase 3: will be passed from visitor
                })
            }
            _ => None,
        }
    }

    // Helper methods

    fn resolution_to_string(resolution: &TypeResolution) -> String {
        use crate::domain::types::ResolutionResult;

        match &resolution.result {
            ResolutionResult::Concrete(concrete) => concrete.to_string(),
            ResolutionResult::Union(types) => {
                let type_names: Vec<_> = types.iter().map(|wt| wt.type_.to_string()).collect();
                format!(
                    "({}) | вероятность неопределённости",
                    type_names.join(" | ")
                )
            }
            ResolutionResult::Intersection(types) => {
                let type_names: Vec<_> = types.iter().map(|t| t.to_string()).collect();
                format!("({})", type_names.join(" & "))
            }
            ResolutionResult::Generic(gen) => {
                if gen.type_params.is_empty() {
                    gen.base_type.clone()
                } else {
                    let params: Vec<_> = gen.type_params.iter().map(|t| t.to_string()).collect();
                    format!("{}<{}>", gen.base_type, params.join(", "))
                }
            }
            ResolutionResult::Nullable(inner) => {
                format!("{} | Null", inner)
            }
            ResolutionResult::Dynamic => "Произвольный".to_string(),
        }
    }
}
