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

use crate::domain::types::{
    ConcreteType, TypeResolution, TypeDiagnostic, DiagnosticSeverity,
    SpecialType,
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
                param_index + 1, method_name, expected, actual
            ),
            TypeErrorKind::NonExistentProperty {
                object_type,
                property_name,
            } => format!(
                "Свойство '{}' не существует для типа '{}'",
                property_name, object_type
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
pub struct TypeValidator;

impl TypeValidator {
    /// Проверка корректности передачи параметров
    pub fn validate_method_call(
        method_name: &str,
        expected_params: &[String],
        actual_params: &[TypeResolution],
    ) -> Vec<TypeErrorKind> {
        let mut errors = Vec::new();

        for (index, (expected, actual)) in expected_params.iter().zip(actual_params.iter()).enumerate() {
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

    /// Проверка существования свойства у объекта
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
            ResolutionResult::Concrete(ConcreteType::Special(SpecialType::Undefined)) |
            ResolutionResult::Concrete(ConcreteType::Special(SpecialType::Null)) => {
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
        use crate::domain::types::{ResolutionResult, Certainty};

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
                let type_names: Vec<_> = types.iter()
                    .map(|wt| wt.type_.to_string())
                    .collect();
                format!("({}) | вероятность неопределённости", type_names.join(" | "))
            }
            ResolutionResult::Dynamic => "Произвольный".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::{
        Certainty, ResolutionResult, ResolutionSource, ResolutionMetadata,
        PrimitiveType,
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
