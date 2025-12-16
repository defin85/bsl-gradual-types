//! Hover formatting utilities
//!
//! Functions for formatting hover tooltips for variables, functions,
//! semantic nodes, and other BSL constructs.

use bsl_shared::domain::types::{
    Certainty, ConcreteType, GenericType, ResolutionResult, TypeResolution,
};
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::ir::{SemanticNode, SemanticNodeKind};

use super::type_formatters::format_resolution_result;

/// Formats information about a SemanticNode for hover (Milestone 2.11)
///
/// # Arguments
/// * `node` - The semantic node to format
/// * `_file_content` - File content (reserved for future use)
/// * `metadata_lookup` - Lookup for type metadata
///
/// # Returns
/// A formatted markdown string with node information
pub fn format_semantic_node_info(
    node: &SemanticNode,
    _file_content: &str,
    metadata_lookup: &TypeMetadataLookup,
) -> String {
    match &node.kind {
        SemanticNodeKind::VariableDeclaration {
            name,
            type_hint,
            initial_value_type,
            ..
        } => {
            // Phase 3: type_hint and initial_value_type are now Option<TypeResolution>
            let type_info = type_hint
                .as_ref()
                .or(initial_value_type.as_ref())
                .map(|resolution| format!("*Тип:* {}", resolution.type_name()))
                .unwrap_or_else(|| "*Тип:* Неопределено".to_string());

            format!(
                "**Переменная:** `{}`\n\n{}\n\n📍 Позиция: {}:{}-{}:{}",
                name,
                type_info,
                node.span.start_line,
                node.span.start_column,
                node.span.end_line,
                node.span.end_column
            )
        }
        SemanticNodeKind::Assignment {
            variable,
            value_type,
            ..
        } => format_assignment_hover(node, variable, value_type, metadata_lookup),
        SemanticNodeKind::FunctionDeclaration {
            name,
            params,
            return_type,
            body,
            ..
        } => {
            // Phase 3: type_hint is now Option<TypeResolution>
            let params_str = params
                .iter()
                .map(|p| {
                    format!(
                        "{}: {}",
                        p.name,
                        p.type_hint
                            .as_ref()
                            .map(|t| t.type_name())
                            .unwrap_or_else(|| "Неопределено".to_string())
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            // Phase 3: return_type is now Option<TypeResolution>
            let return_str = return_type
                .as_ref()
                .map(|t| format!("*Возвращает:* {}", t.type_name()))
                .unwrap_or_else(|| "*Возвращает:* Неопределено".to_string());

            let body_info = if body.is_empty() {
                "Тело пустое".to_string()
            } else {
                format!("Содержит {} узлов", body.len())
            };

            format!(
                "**Функция:** `{}({})`\n\n{}\n\n📦 {}\n\n📍 Позиция: {}:{}-{}:{}",
                name,
                params_str,
                return_str,
                body_info,
                node.span.start_line,
                node.span.start_column,
                node.span.end_line,
                node.span.end_column
            )
        }
        SemanticNodeKind::ProcedureDeclaration {
            name, params, body, ..
        } => {
            // Phase 3: type_hint is now Option<TypeResolution>
            let params_str = params
                .iter()
                .map(|p| {
                    format!(
                        "{}: {}",
                        p.name,
                        p.type_hint
                            .as_ref()
                            .map(|t| t.type_name())
                            .unwrap_or_else(|| "Неопределено".to_string())
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");

            let body_info = if body.is_empty() {
                "Тело пустое".to_string()
            } else {
                format!("Содержит {} узлов", body.len())
            };

            format!(
                "**Процедура:** `{}({})`\n\n📦 {}\n\n📍 Позиция: {}:{}-{}:{}",
                name,
                params_str,
                body_info,
                node.span.start_line,
                node.span.start_column,
                node.span.end_line,
                node.span.end_column
            )
        }
        SemanticNodeKind::IfStatement { condition_type, .. } => {
            // Phase 3: condition_type is now TypeResolution
            format!(
                "**Условие:** `Если ... Тогда`\n\n*Условие:* {}\n\n📍 Позиция: {}:{}-{}:{}",
                condition_type.type_name(),
                node.span.start_line,
                node.span.start_column,
                node.span.end_line,
                node.span.end_column
            )
        }
        SemanticNodeKind::WhileLoop { condition_type, .. } => {
            // Phase 3: condition_type is now TypeResolution
            format!(
                "**Цикл:** `Пока ... Цикл`\n\n*Условие:* {}\n\n📍 Позиция: {}:{}-{}:{}",
                condition_type.type_name(),
                node.span.start_line,
                node.span.start_column,
                node.span.end_line,
                node.span.end_column
            )
        }
        _ => {
            format!(
                "**Узел IR:** {:?}\n\n📍 Позиция: {}:{}-{}:{}",
                node.kind,
                node.span.start_line,
                node.span.start_column,
                node.span.end_line,
                node.span.end_column
            )
        }
    }
}

/// Formats hover for assignment node
fn format_assignment_hover(
    node: &SemanticNode,
    variable: &str,
    value_type: &TypeResolution,
    metadata_lookup: &TypeMetadataLookup,
) -> String {
    let resolution = value_type.clone();
    let raw_type = metadata_lookup.get_raw_type(&resolution);

    // Show certainty in user-friendly format
    let mut output = format!("**Присваивание:** `{} = ...`\n", variable);
    output.push_str(&format!("*Тип:* `{}`\n", value_type.type_name()));

    // Always show certainty
    let certainty_text = match resolution.certainty {
        Certainty::Known => "🟢 Known (100%)".to_string(),
        Certainty::Inferred => "🟡 Inferred (80%)".to_string(),
        Certainty::InferredWeak => "🟠 InferredWeak (50%)".to_string(),
        Certainty::Unknown => "⚪ Unknown (0%)".to_string(),
    };
    output.push_str(&format!("*Уверенность:* {}\n\n", certainty_text));

    // Check for RawTypeData availability
    if raw_type.is_none() {
        output.push_str("⚠️ **Детали типа недоступны**\n\n");
        output.push_str(&format!(
            "📍 Позиция: {}:{}-{}:{}\n",
            node.span.start_line, node.span.start_column, node.span.end_line, node.span.end_column
        ));
        return output;
    }

    // If Unknown - show additional hint
    if matches!(resolution.certainty, Certainty::Unknown) {
        output.push_str("⚠️ **Тип не распознан системой**\n\n");
        output.push_str(&format!(
            "📍 Позиция: {}:{}-{}:{}\n",
            node.span.start_line, node.span.start_column, node.span.end_line, node.span.end_column
        ));
        return output;
    }

    // RawTypeData found - show full information
    let methods = metadata_lookup.get_methods(&resolution);
    let properties = metadata_lookup.get_properties(&resolution);

    // Format description from RawTypeData
    let description = raw_type
        .as_ref()
        .map(|rt| rt.description.clone())
        .filter(|d| !d.is_empty())
        .unwrap_or_else(|| format!("Тип {}", value_type.type_name()));

    output.push_str(&format!("📝 {}\n\n", description));

    // Add methods (first 10)
    if !methods.is_empty() {
        output.push_str("📚 **Методы:**\n");
        for method in methods.iter().take(10) {
            let params_str = method
                .params
                .iter()
                .map(|p| format!("{}: {}", p.name, p.param_type))
                .collect::<Vec<_>>()
                .join(", ");

            if !method.return_type.is_empty() {
                output.push_str(&format!(
                    "- `{}({})` → `{}`\n",
                    method.name, params_str, method.return_type
                ));
            } else {
                output.push_str(&format!("- `{}({})`\n", method.name, params_str));
            }
        }
        if methods.len() > 10 {
            output.push_str(&format!("- ... и ещё {} методов\n", methods.len() - 10));
        }
        output.push('\n');
    }

    // Add properties (first 10)
    if !properties.is_empty() {
        output.push_str("📦 **Свойства:**\n");
        for prop in properties.iter().take(10) {
            output.push_str(&format!("- `{}`: `{}`\n", prop.name, prop.prop_type));
        }
        if properties.len() > 10 {
            output.push_str(&format!("- ... и ещё {} свойств\n", properties.len() - 10));
        }
        output.push('\n');
    }

    output.push_str(&format!(
        "📍 Позиция: {}:{}-{}:{}\n",
        node.span.start_line, node.span.start_column, node.span.end_line, node.span.end_column
    ));

    output
}

/// Formats variable hover information (using Inline Scope Analysis)
///
/// # Arguments
/// * `var_name` - Variable name from IR
/// * `resolution` - TypeResolution from SymbolTable
/// * `metadata_lookup` - Lookup for type metadata
///
/// # Returns
/// Formatted markdown with type information, methods, and properties
#[allow(dead_code)]
pub fn format_variable_hover(
    var_name: &str,
    resolution: &TypeResolution,
    metadata_lookup: &TypeMetadataLookup,
) -> String {
    // Check if type is known
    if matches!(resolution.certainty, Certainty::Unknown) {
        return format!(
            "**Переменная:** `{}`\n\n*Тип:* Неопределено\n\n*Подсказка:* Переменная объявлена, но тип не выведен из присваивания",
            var_name
        );
    }

    // Get type name from TypeResolution
    let type_name = resolution.type_name();

    // Special handling for Generic types
    if let ResolutionResult::Generic(ref generic_type) = resolution.result {
        return format_generic_hover(var_name, generic_type, resolution, metadata_lookup);
    }

    // Get RawTypeData to check type existence
    let raw_type = metadata_lookup.get_raw_type(resolution);

    // Build base information ALWAYS (regardless of raw_type)
    let mut output = format!("**Переменная:** `{}`\n", var_name);
    output.push_str(&format!("*Тип:* `{}`\n", type_name));

    // ALWAYS show certainty (KEY: BEFORE raw_type check)
    let certainty_text = match resolution.certainty {
        Certainty::Known => "🟢 Known (100%)".to_string(),
        Certainty::Inferred => "🟡 Inferred (80%)".to_string(),
        Certainty::InferredWeak => "🟠 InferredWeak (50%)".to_string(),
        Certainty::Unknown => "⚪ Unknown (0%)".to_string(),
    };
    output.push_str(&format!("*Уверенность:* {}\n\n", certainty_text));

    // NOW check RawTypeData for methods/properties display
    if raw_type.is_none() {
        output.push_str("⚠️ **Детали типа недоступны**\n\n");
        output.push_str("💡 *Возможные причины:*\n");
        output.push_str("- Тип не загружен из Syntax Helper\n");
        output.push_str("- Требуется парсинг документации платформы\n");
        return output;
    }

    // If Unknown - show additional hint
    if matches!(resolution.certainty, Certainty::Unknown) {
        output.push_str("⚠️ **Тип не распознан системой**\n\n");
        output.push_str("💡 *Возможные причины:*\n");
        output.push_str("- Опечатка в имени типа\n");
        output.push_str("- Тип из конфигурации 1С (требуется Configuration Loader)\n");
        return output;
    }

    // RawTypeData found - show full information
    let methods = metadata_lookup.get_methods(resolution);
    let properties = metadata_lookup.get_properties(resolution);

    // Format description from RawTypeData
    let description = raw_type
        .as_ref()
        .map(|rt| rt.description.clone())
        .filter(|d| !d.is_empty())
        .unwrap_or_else(|| format!("Тип {}", type_name));

    output.push_str(&format!("📝 {}\n\n", description));

    // Add methods (first 10 for brevity)
    if !methods.is_empty() {
        output.push_str("📚 **Методы:**\n");
        for method in methods.iter().take(10) {
            let params_str = method
                .params
                .iter()
                .map(|p| format!("{}: {}", p.name, p.param_type))
                .collect::<Vec<_>>()
                .join(", ");

            if !method.return_type.is_empty() {
                output.push_str(&format!(
                    "- `{}({})` → `{}`\n",
                    method.name, params_str, method.return_type
                ));
            } else {
                output.push_str(&format!("- `{}({})`\n", method.name, params_str));
            }
        }
        if methods.len() > 10 {
            output.push_str(&format!("- ... и ещё {} методов\n", methods.len() - 10));
        }
        output.push('\n');
    }

    // Add properties (first 10)
    if !properties.is_empty() {
        output.push_str("📦 **Свойства:**\n");
        for prop in properties.iter().take(10) {
            output.push_str(&format!("- `{}`: `{}`\n", prop.name, prop.prop_type));
        }
        if properties.len() > 10 {
            output.push_str(&format!("- ... и ещё {} свойств\n", properties.len() - 10));
        }
    }

    output
}

/// Formats hover for Generic type (e.g., ТабличнаяЧасть<СтрокаРаботы>)
///
/// Special formatting for tabular sections displaying:
/// - Base type and type parameter
/// - Collection methods (with substituted types)
/// - Row attributes of tabular section
pub fn format_generic_hover(
    var_name: &str,
    generic_type: &GenericType,
    resolution: &TypeResolution,
    metadata_lookup: &TypeMetadataLookup,
) -> String {
    tracing::debug!(
        "🎨 Formatting Generic hover: {} = {}<{}>",
        var_name,
        generic_type.base_type,
        generic_type.type_params.len()
    );

    let mut output = String::new();

    // Variable header
    output.push_str(&format!("**Переменная:** `{}`\n", var_name));

    // Format full Generic type name
    let full_type_name = if let Some(param_type) = generic_type.type_params.first() {
        let param_name = format_concrete_type_name(param_type);
        format!("{}<{}>", generic_type.base_type, param_name)
    } else {
        generic_type.base_type.clone()
    };

    output.push_str(&format!("*Тип:* `{}`\n", full_type_name));

    // Certainty
    let certainty_text = match resolution.certainty {
        Certainty::Known => "🟢 Known (100%)".to_string(),
        Certainty::Inferred => "🟡 Inferred (80%)".to_string(),
        Certainty::InferredWeak => "🟠 InferredWeak (50%)".to_string(),
        Certainty::Unknown => "⚪ Unknown (0%)".to_string(),
    };
    output.push_str(&format!("*Уверенность:* {}\n\n", certainty_text));

    // Additional info for tabular sections
    if generic_type.base_type == "ТабличнаяЧасть" {
        if let Some(ConcreteType::TabularRow(row_type)) = generic_type.type_params.first() {
            output.push_str(&format!(
                "📋 *Табличная часть:* `{}`\n",
                row_type.tabular_section_name
            ));
            output.push_str(&format!(
                "📄 *Родительский объект:* `{}`\n\n",
                row_type.parent_type
            ));

            // Collection methods
            let methods = metadata_lookup.get_methods(resolution);
            if !methods.is_empty() {
                output.push_str("📚 **Методы коллекции:**\n");
                for method in methods.iter().take(10) {
                    let params_str = method
                        .params
                        .iter()
                        .map(|p| format!("{}: {}", p.name, p.param_type))
                        .collect::<Vec<_>>()
                        .join(", ");

                    if !method.return_type.is_empty() {
                        output.push_str(&format!(
                            "- `{}({})` → `{}`\n",
                            method.name, params_str, method.return_type
                        ));
                    } else {
                        output.push_str(&format!("- `{}({})`\n", method.name, params_str));
                    }
                }
                if methods.len() > 10 {
                    output.push_str(&format!("- ... и ещё {} методов\n", methods.len() - 10));
                }
                output.push('\n');
            }

            // Row attributes of tabular section
            if !row_type.attributes.is_empty() {
                output.push_str("📦 **Атрибуты строки:**\n");
                for attr in row_type.attributes.iter().take(15) {
                    if !attr.attr_type.is_empty() {
                        output.push_str(&format!("- `{}`: `{}`\n", attr.name, attr.attr_type));
                    } else {
                        output.push_str(&format!("- `{}`\n", attr.name));
                    }
                }
                if row_type.attributes.len() > 15 {
                    output.push_str(&format!(
                        "- ... и ещё {} атрибутов\n",
                        row_type.attributes.len() - 15
                    ));
                }
            }
        }
    } else {
        // For other Generic types (not tabular sections) - basic format
        let methods = metadata_lookup.get_methods(resolution);
        if !methods.is_empty() {
            output.push_str("📚 **Методы:**\n");
            for method in methods.iter().take(10) {
                let params_str = method
                    .params
                    .iter()
                    .map(|p| format!("{}: {}", p.name, p.param_type))
                    .collect::<Vec<_>>()
                    .join(", ");

                if !method.return_type.is_empty() {
                    output.push_str(&format!(
                        "- `{}({})` → `{}`\n",
                        method.name, params_str, method.return_type
                    ));
                } else {
                    output.push_str(&format!("- `{}({})`\n", method.name, params_str));
                }
            }
            if methods.len() > 10 {
                output.push_str(&format!("- ... и ещё {} методов\n", methods.len() - 10));
            }
        }
    }

    output
}

/// Formats ConcreteType name for display (human-readable format)
pub fn format_concrete_type_name(concrete: &ConcreteType) -> String {
    match concrete {
        ConcreteType::Platform(pt) => pt.name.clone(),
        ConcreteType::Configuration(ct) => {
            // Use to_prefix() for correct formatting
            format!("{}.{}", ct.kind.to_prefix(), ct.name)
        }
        // Use display_name() instead of Debug for readable format
        ConcreteType::Primitive(prim) => prim.display_name().to_string(),
        ConcreteType::Special(spec) => spec.display_name().to_string(),
        ConcreteType::GlobalFunction(gf) => gf.name.clone(),
        ConcreteType::TabularRow(tr) => tr.get_full_name(),
    }
}

/// Formats TypeResolution for hover tooltip with full type description
#[allow(dead_code)] // Kept for public API, may be used by external consumers
pub fn format_type_for_hover(type_name: &str, resolution: &TypeResolution) -> String {
    let type_str = format_resolution_result(&resolution.result);
    format!(
        "**Тип:** `{}`\n\n*Категория:* {:?}\n*Certainty:* {:?}\n*Структура:* {}",
        type_name, resolution.source, resolution.certainty, type_str
    )
}
