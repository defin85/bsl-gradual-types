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
use crate::ir::Span;

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
    pub fn to_diagnostic(&self, span: Span) -> TypeDiagnostic {
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
            line: span.start_line,
            column: span.start_column,
            end_line: span.end_line,
            end_column: span.end_column,
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

}
