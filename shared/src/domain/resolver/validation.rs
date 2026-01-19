//! Function/Method Call Validation
//!
//! Milestone 2.20: Function Signature Validation
//! Milestone 3.13: Object-Based Type Comparison

use super::result_types::{ValidationResult, ValidationResultV2};
use super::type_resolver::TypeResolver;
use crate::domain::signature_index::SignatureIndex;
use crate::domain::types::TypeResolution;

impl TypeResolver {
    /// Валидирует вызов функции/метода
    ///
    /// # Параметры
    /// - `type_name` - имя типа для методов (None для глобальных функций)
    /// - `method_name` - имя метода или функции
    /// - `arg_types` - список типов аргументов в вызове
    /// - `signature_index` - индекс с сигнатурами методов и функций
    ///
    /// # Возвращает
    /// `ValidationResult` - результат валидации (Ok, MissingRequiredParam, TooManyArgs, TypeMismatch, NotFound)
    pub fn validate_call(
        &self,
        type_name: Option<&str>,
        method_name: &str,
        arg_types: &[String],
        signature_index: &SignatureIndex,
    ) -> ValidationResult {
        // 1. Найти сигнатуру
        let signature = if let Some(type_name) = type_name {
            signature_index.find_method(type_name, method_name)
        } else {
            signature_index.find_global_function(method_name)
        };

        let signature = match signature {
            Some(sig) => sig,
            None => return ValidationResult::NotFound,
        };

        // 2. Проверить количество аргументов
        let required_count = signature.params.iter().filter(|p| !p.is_optional).count();

        if arg_types.len() < required_count {
            // Найти первый отсутствующий обязательный параметр
            if let Some(missing_param) = signature
                .params
                .iter()
                .enumerate()
                .find(|(i, p)| !p.is_optional && *i >= arg_types.len())
            {
                return ValidationResult::MissingRequiredParam {
                    param_name: missing_param.1.name.clone(),
                    param_index: missing_param.0,
                };
            }

            return ValidationResult::NotFound;
        }

        if arg_types.len() > signature.params.len() {
            return ValidationResult::TooManyArgs {
                expected: signature.params.len(),
                actual: arg_types.len(),
            };
        }

        // 3. Проверить типы аргументов
        for (param, arg_type) in signature.params.iter().zip(arg_types.iter()) {
            if let Some(expected_type) = &param.type_name {
                // Проверяем совместимость типов
                if !Self::check_type_compatible(expected_type, arg_type) {
                    return ValidationResult::TypeMismatch {
                        param_name: param.name.clone(),
                        expected: expected_type.clone(),
                        actual: arg_type.clone(),
                    };
                }
            }
            // Если expected_type = None (Произвольный), то любой тип подходит
        }

        // 4. Вернуть тип возврата
        ValidationResult::Ok(signature.return_type.clone())
    }

    // ===== Assignment Compatibility =====

    /// Проверить совместимость присваивания типов (Domain логика)
    pub fn is_assignment_compatible(&self, from: &TypeResolution, to: &TypeResolution) -> bool {
        use crate::domain::types::{Certainty, ResolutionResult};

        // Если "to" - Unknown, то любое присваивание допустимо (градуальная типизация)
        if matches!(to.certainty, Certainty::Unknown) {
            return true;
        }

        // Если "from" - Unknown, допускаем присваивание с предупреждением
        if matches!(from.certainty, Certainty::Unknown) {
            return true;
        }

        // Точное совпадение типов
        match (&from.result, &to.result) {
            (ResolutionResult::Concrete(from_type), ResolutionResult::Concrete(to_type)) => {
                // Простое сравнение типов (можно расширить)
                format!("{:?}", from_type) == format!("{:?}", to_type)
            }
            // Milestone 2.3: Union type compatibility
            (_, ResolutionResult::Union(_)) => {
                // Присваивание в Union: проверяем совместимость с любым членом
                self.is_assignable_to_union(from, to)
            }
            (ResolutionResult::Union(union_types), ResolutionResult::Concrete(_)) => {
                // Присваивание из Union: все члены должны быть совместимы
                union_types.iter().all(|wt| {
                    let union_member = TypeResolution {
                        certainty: from.certainty,
                        result: ResolutionResult::Concrete(wt.type_.clone()),
                        source: from.source,
                        metadata: from.metadata.clone(),
                        active_facet: from.active_facet,
                        available_facets: from.available_facets.clone(),
                    };
                    self.is_assignment_compatible(&union_member, to)
                })
            }
            _ => false,
        }
    }

    // ===== Milestone 3.13: Object-Based Type Comparison =====

    /// Объектное сравнение типов (v2 версия)
    pub fn is_type_compatible_v2(
        &self,
        expected: &str,
        actual: &str,
    ) -> crate::domain::types::TypeCompatibility {
        let expected_resolution = self.resolve_expression_sync(expected);
        let actual_resolution = self.resolve_expression_sync(actual);

        actual_resolution.is_compatible_with(&expected_resolution)
    }

    /// Валидация вызова с объектным сравнением типов (v2 версия)
    pub fn validate_call_v2(
        &self,
        type_name: Option<&str>,
        method_name: &str,
        arg_types: &[String],
        signature_index: &SignatureIndex,
    ) -> ValidationResultV2 {
        // 1. Найти сигнатуру
        let signature = if let Some(type_name) = type_name {
            signature_index.find_method(type_name, method_name)
        } else {
            signature_index.find_global_function(method_name)
        };

        let signature = match signature {
            Some(sig) => sig,
            None => return ValidationResultV2::NotFound,
        };

        // 2. Проверка количества параметров
        let required_count = signature.params.iter().filter(|p| !p.is_optional).count();

        if arg_types.len() < required_count {
            return ValidationResultV2::MissingRequiredParam {
                param_name: signature.params[arg_types.len()].name.clone(),
                param_index: arg_types.len(),
            };
        }

        if arg_types.len() > signature.params.len() {
            return ValidationResultV2::TooManyArgs {
                expected: signature.params.len(),
                actual: arg_types.len(),
            };
        }

        // 3. Проверяем типы параметров с объектным сравнением
        for (i, (param, arg_type)) in signature.params.iter().zip(arg_types.iter()).enumerate() {
            if let Some(expected_type) = &param.type_name {
                let compat = self.is_type_compatible_v2(expected_type, arg_type);
                if !compat.is_compatible() {
                    return ValidationResultV2::TypeMismatch {
                        param_name: param.name.clone(),
                        param_index: i,
                        expected: expected_type.clone(),
                        actual: arg_type.clone(),
                        reason: compat.reason(),
                    };
                }
            }
        }

        // 4. Вернуть тип возврата с подстановкой имени объекта для фасетных типов
        let return_type = if let Some(ref rt) = signature.return_type {
            if let Some(type_name) = type_name {
                if let Some(metadata_name) = SignatureIndex::extract_metadata_name(type_name) {
                    Some(SignatureIndex::substitute_type_name(rt, metadata_name))
                } else {
                    Some(rt.clone())
                }
            } else {
                Some(rt.clone())
            }
        } else {
            None
        };
        ValidationResultV2::Ok(return_type)
    }
}
