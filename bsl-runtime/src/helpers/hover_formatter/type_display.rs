//! Отображение типов для hover
//!
//! Содержит функции для преобразования TypeResolution в строковое представление.

use bsl_shared::domain::types::{ResolutionResult, TypeResolution};

/// Форматирует тип для отображения в hover
pub fn format_type_string(resolution: &TypeResolution) -> String {
    match &resolution.result {
        ResolutionResult::Concrete(concrete_type) => {
            format!("{}", concrete_type)
        }
        ResolutionResult::Generic(generic_type) => {
            // Формат: Массив<Строка>
            let params = generic_type
                .type_params
                .iter()
                .map(|p| format!("{}", p))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}<{}>", generic_type.base_type, params)
        }
        ResolutionResult::Union(union_types) => {
            // Формат: Строка | Число
            union_types
                .iter()
                .map(|weighted| format!("{}", weighted.type_))
                .collect::<Vec<_>>()
                .join(" | ")
        }
        ResolutionResult::Intersection(intersection_types) => {
            // Формат: ТипА & ТипВ
            intersection_types
                .iter()
                .map(|t| format!("{}", t))
                .collect::<Vec<_>>()
                .join(" & ")
        }
        ResolutionResult::Dynamic => "Неопределено".to_string(),
        ResolutionResult::Nullable(inner) => {
            format!("{} | Неопределено", inner)
        }
    }
}

/// Форматирует certainty для отображения
pub fn format_certainty(certainty: &bsl_shared::domain::types::Certainty) -> String {
    use bsl_shared::domain::types::Certainty;

    match certainty {
        Certainty::Known => "🟢 Known (100%)".to_string(),
        Certainty::Inferred => "🟡 Inferred (80%)".to_string(),
        Certainty::InferredWeak => "🟠 InferredWeak (50%)".to_string(),
        Certainty::Unknown => "⚪ Unknown (0%)".to_string(),
    }
}

/// Получает имя типа платформы для документации
pub fn get_platform_type_name(resolution: &TypeResolution) -> Option<String> {
    match &resolution.result {
        ResolutionResult::Concrete(concrete_type) => {
            use bsl_shared::domain::types::ConcreteType;
            match concrete_type {
                ConcreteType::Platform(platform) => Some(platform.name.clone()),
                ConcreteType::Configuration(config) => {
                    Some(config.kind.to_prefix().trim_end_matches('ы').to_string())
                }
                _ => None,
            }
        }
        ResolutionResult::Generic(generic) => Some(generic.base_type.clone()),
        _ => None,
    }
}
