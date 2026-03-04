//! Type extraction utilities for BSL source code and AST
//!
//! Provides functions to extract type information from variable declarations,
//! function declarations, and AST expressions.

use crate::parsing::bsl::ast::Expression;

/// Maps AST Expression to type name (Application Layer responsibility)
///
/// Converts syntactic construct (Expression) to domain concept (type name)
/// for further resolution via AnalysisEngine/TypeResolver.
///
/// # Arguments
/// * `expr` - The AST expression to analyze
///
/// # Returns
/// The inferred type name or None for complex expressions
pub fn expression_to_type_name(expr: &Expression) -> Option<String> {
    match expr {
        Expression::Number { .. } => Some("Число".to_string()),
        Expression::String { .. } => Some("Строка".to_string()),
        Expression::Boolean { .. } => Some("Булево".to_string()),
        Expression::Identifier { name, .. } => Some(name.clone()),
        Expression::New { type_name, .. } => Some(type_name.clone()),
        _ => None, // Complex expressions require extended analysis
    }
}

/// Extracts variable name from declaration line
///
/// Parses BSL variable declaration pattern: `Перем ИмяПеременной: Тип`
///
/// # Arguments
/// * `line` - The source line containing variable declaration
///
/// # Returns
/// The variable name or None if pattern doesn't match
pub fn extract_var_name(line: &str) -> Option<String> {
    // Pattern: Перем ИмяПеременной: Тип
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 {
        let var_name = parts[1].trim_end_matches(':');
        return Some(var_name.to_string());
    }
    None
}

/// Extracts type from variable declaration
///
/// Parses BSL variable declaration pattern: `Перем ИмяПеременной: Тип`
///
/// # Arguments
/// * `line` - The source line containing variable declaration
///
/// # Returns
/// The type name or None if no type hint found
pub fn extract_type_from_var_declaration(line: &str) -> Option<String> {
    // Pattern: Перем ИмяПеременной: Тип
    if let Some(colon_pos) = line.find(':') {
        let type_part = &line[colon_pos + 1..];
        let type_name = type_part.split(';').next()?.trim();
        return Some(type_name.to_string());
    }
    None
}

/// Extracts function name from declaration line
///
/// Parses BSL function/procedure declaration: `Функция ИмяФункции(...)` or `Процедура ИмяПроцедуры(...)`
///
/// # Arguments
/// * `line` - The source line containing function/procedure declaration
///
/// # Returns
/// The function/procedure name or None if pattern doesn't match
pub fn extract_function_name(line: &str) -> Option<String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 {
        let func_name = parts[1].trim_end_matches('(');
        return Some(func_name.to_string());
    }
    None
}

/// Extracts return type from function declaration
///
/// Looks for "Возврат" keyword in the line to extract return type hint
///
/// # Arguments
/// * `line` - The source line containing return statement
///
/// # Returns
/// The return type or None if no return type found
pub fn extract_return_type(line: &str) -> Option<String> {
    // Look for "Возврат" in the line
    if let Some(return_pos) = line.find("Возврат") {
        let return_part = &line[return_pos + "Возврат".len()..];
        let type_name = return_part.split(';').next()?.trim();
        if !type_name.is_empty() {
            return Some(type_name.to_string());
        }
    }
    None
}

#[cfg(test)]
#[path = "type_extractor/tests.rs"]
mod tests;
