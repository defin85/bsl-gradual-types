//! Type formatting utilities
//!
//! Functions for formatting ResolutionResult and other type constructs
//! into human-readable strings.

use bsl_shared::domain::types::ResolutionResult;

/// Formats ResolutionResult for display (human-readable format)
///
/// # Arguments
/// * `result` - The ResolutionResult to format
///
/// # Returns
/// A human-readable string representation of the type
pub fn format_resolution_result(result: &ResolutionResult) -> String {
    match result {
        // Use Display instead of Debug for readable format
        ResolutionResult::Concrete(concrete_type) => format!("{}", concrete_type),
        ResolutionResult::Union(types) => {
            let names: Vec<String> = types.iter().map(|wt| format!("{}", wt.type_)).collect();
            names.join(" | ")
        }
        ResolutionResult::Dynamic => "Произвольный".to_string(),
        ResolutionResult::Intersection(types) => {
            let names: Vec<String> = types.iter().map(|t| format!("{}", t)).collect();
            names.join(" & ")
        }
        ResolutionResult::Generic(generic) => {
            if generic.type_params.is_empty() {
                generic.base_type.clone()
            } else {
                let params: Vec<String> = generic
                    .type_params
                    .iter()
                    .map(|t| format!("{}", t))
                    .collect();
                format!("{}<{}>", generic.base_type, params.join(", "))
            }
        }
        ResolutionResult::Nullable(inner) => format!("{} | Неопределено", inner),
    }
}

#[cfg(test)]
#[path = "type_formatters/tests.rs"]
mod tests;
