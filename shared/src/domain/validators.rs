//! Type validation rules based on Balyuk & Popova (2021) research
//!
//! This module implements static type-checking validation inspired by:
//! "Static type-checking for programs developed on the platform 1C:Enterprise"
//! https://ceur-ws.org/Vol-2984/paper13.pdf
//!
//! Three main categories of errors detected:
//! 1. Incorrect parameter passing to methods
//! 2. Access to non-existent properties of objects
//! 3. Treating simple types as collections

use crate::domain::metadata_lookup::TypeMetadataLookup;
use crate::domain::types::{
    ConcreteType, DiagnosticSeverity, SpecialType, TypeDiagnostic, TypeResolution,
};

/// Категории ошибок типизации из статьи Balyuk & Popova
#[derive(Debug, Clone, PartialEq)]
pub enum TypeErrorKind {
    /// Некорректная передача параметров методу
    IncorrectParameterType {
        method_name: String,
        param_index: usize,
        expected: String,
        actual: String,
    },
    /// Обращение к несуществующему свойству объекта
    NonExistentProperty {
        object_type: String,
        property_name: String,
    },
    /// Обращение к несуществующему методу объекта
    NonExistentMethod {
        object_type: String,
        method_name: String,
    },
    /// Обработка простого типа как коллекции
    SimpleTypeAsCollection {
        type_name: String,
        operation: String,
    },
}

impl TypeErrorKind {
    pub fn to_diagnostic(&self, line: u32, column: u32) -> TypeDiagnostic {
        let message = match self {
            TypeErrorKind::IncorrectParameterType {
                method_name,
                param_index,
                expected,
                actual,
            } => format!(
                "Некорректный параметр #{} для метода '{}': ожидается {}, получено {}",
                param_index + 1,
                method_name,
                expected,
                actual
            ),
            TypeErrorKind::NonExistentProperty {
                object_type,
                property_name,
            } => format!(
                "Свойство '{}' не существует для типа '{}'",
                property_name, object_type
            ),
            TypeErrorKind::NonExistentMethod {
                object_type,
                method_name,
            } => format!(
                "Метод '{}' не существует для типа '{}'",
                method_name, object_type
            ),
            TypeErrorKind::SimpleTypeAsCollection {
                type_name,
                operation,
            } => format!(
                "Тип '{}' является примитивным и не поддерживает операцию '{}'",
                type_name, operation
            ),
        };

        TypeDiagnostic {
            severity: DiagnosticSeverity::Error,
            message,
            line,
            column,
        }
    }
}

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
            Self::names_equal_ignore_case(&m.name, method_name)
                || Self::names_equal_ignore_case(&m.english_name, method_name)
        });

        if !method_exists {
            // Получаем читаемое имя типа для сообщения об ошибке
            let type_name = Self::resolution_to_string(object_resolution);
            Some(TypeErrorKind::NonExistentMethod {
                object_type: type_name,
                method_name: method_name.to_string(),
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
            .any(|p| Self::names_equal_ignore_case(&p.name, property_name));

        if !property_exists {
            let type_name = Self::resolution_to_string(object_resolution);
            Some(TypeErrorKind::NonExistentProperty {
                object_type: type_name,
                property_name: property_name.to_string(),
            })
        } else {
            None
        }
    }

    /// Case-insensitive сравнение строк (работает с кириллицей и латиницей)
    fn names_equal_ignore_case(a: &str, b: &str) -> bool {
        if a.len() != b.len() {
            return false;
        }
        a.chars()
            .zip(b.chars())
            .all(|(ca, cb)| ca.to_lowercase().eq(cb.to_lowercase()))
    }

    /// Проверка корректности передачи параметров (старый API для совместимости)
    pub fn validate_method_call(
        method_name: &str,
        expected_params: &[String],
        actual_params: &[TypeResolution],
    ) -> Vec<TypeErrorKind> {
        let mut errors = Vec::new();

        for (index, (expected, actual)) in
            expected_params.iter().zip(actual_params.iter()).enumerate()
        {
            if !Self::types_compatible(expected, actual) {
                errors.push(TypeErrorKind::IncorrectParameterType {
                    method_name: method_name.to_string(),
                    param_index: index,
                    expected: expected.clone(),
                    actual: Self::resolution_to_string(actual),
                });
            }
        }

        errors
    }

    /// Проверка существования свойства у объекта (старый API для совместимости)
    pub fn validate_property_access(
        object_type: &ConcreteType,
        property_name: &str,
        available_properties: &[String],
    ) -> Option<TypeErrorKind> {
        if !available_properties.contains(&property_name.to_string()) {
            Some(TypeErrorKind::NonExistentProperty {
                object_type: object_type.to_string(),
                property_name: property_name.to_string(),
            })
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
                })
            }
            ResolutionResult::Concrete(ConcreteType::Special(SpecialType::Undefined))
            | ResolutionResult::Concrete(ConcreteType::Special(SpecialType::Null)) => {
                Some(TypeErrorKind::SimpleTypeAsCollection {
                    type_name: "Неопределено".to_string(),
                    operation: operation.to_string(),
                })
            }
            _ => None,
        }
    }

    // Вспомогательные методы

    fn types_compatible(expected: &str, actual: &TypeResolution) -> bool {
        use crate::domain::types::{Certainty, ResolutionResult};

        // Если тип неизвестен, предполагаем совместимость (градуальная типизация)
        if matches!(actual.certainty, Certainty::Unknown) {
            return true;
        }

        match &actual.result {
            ResolutionResult::Dynamic => true, // Dynamic совместим со всем
            ResolutionResult::Concrete(concrete) => {
                let actual_str = concrete.to_string();
                expected == actual_str || Self::is_subtype(&actual_str, expected)
            }
            ResolutionResult::Union(types) => {
                // Хотя бы один из типов должен совпадать
                types.iter().any(|wt| {
                    let type_str = wt.type_.to_string();
                    expected == type_str || Self::is_subtype(&type_str, expected)
                })
            }
            ResolutionResult::Intersection(types) => {
                // Все типы должны совпадать для intersection
                types.iter().all(|t| {
                    let type_str = t.to_string();
                    expected == type_str || Self::is_subtype(&type_str, expected)
                })
            }
            ResolutionResult::Generic(gen) => {
                // Проверяем базовый тип
                expected == gen.base_type || Self::is_subtype(&gen.base_type, expected)
            }
            ResolutionResult::Nullable(inner) => {
                // Nullable тип совместим если внутренний тип совместим, или если ожидается Null
                if expected == "Null" {
                    return true;
                }
                let inner_str = inner.to_string();
                expected == inner_str || Self::is_subtype(&inner_str, expected)
            }
        }
    }

    fn is_subtype(_actual: &str, _expected: &str) -> bool {
        // TODO: Реализовать иерархию типов 1С
        // Например: СправочникСсылка.Контрагенты <: ЛюбаяСсылка
        false
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::{
        Certainty, PrimitiveType, ResolutionMetadata, ResolutionResult, ResolutionSource,
    };

    #[test]
    fn test_incorrect_parameter_type() {
        let expected = vec!["Строка".to_string(), "Строка".to_string()];
        let actual = vec![
            TypeResolution {
                certainty: Certainty::Known,
                result: ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::String)),
                source: ResolutionSource::Static,
                metadata: ResolutionMetadata::default(),
                active_facet: None,
                available_facets: vec![],
            },
            TypeResolution {
                certainty: Certainty::Known,
                result: ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Number)),
                source: ResolutionSource::Static,
                metadata: ResolutionMetadata::default(),
                active_facet: None,
                available_facets: vec![],
            },
        ];

        let errors = TypeValidator::validate_method_call("СтрЗаменить", &expected, &actual);

        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            TypeErrorKind::IncorrectParameterType { param_index: 1, .. }
        ));
    }

    #[test]
    fn test_simple_type_as_collection() {
        let resolution = TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Number)),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        let error = TypeValidator::validate_collection_operation(&resolution, "Добавить");

        assert!(error.is_some());
        assert!(matches!(
            error.unwrap(),
            TypeErrorKind::SimpleTypeAsCollection { .. }
        ));
    }

    #[test]
    fn test_gradual_typing_compatibility() {
        let expected = vec!["Строка".to_string()];
        let actual = vec![TypeResolution::unknown()];

        let errors = TypeValidator::validate_method_call("Метод", &expected, &actual);

        // Неизвестные типы не вызывают ошибок (градуальная типизация)
        assert_eq!(errors.len(), 0);
    }
}
