//! Current Context handler for LSP
//!
//! MILESTONE 2.20.3: Handles bsl.getCurrentContext command.

use bsl_shared::api::semantic_dtos::{
    SemanticNodeDto, SemanticTreeDto, SourceLocationDto, SourceRangeDto,
};

/// Response for getCurrentContext command
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentContextResponse {
    pub function_name: Option<String>,
    pub function_kind: String, // "function", "procedure", "none"

    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_type: Option<String>,
}

impl CurrentContextResponse {
    pub fn empty() -> Self {
        Self {
            function_name: None,
            function_kind: "none".to_string(),
            params: None,
            return_type: None,
        }
    }
}

/// Find function/procedure containing the specified position (MILESTONE 2.20.3)
pub fn find_containing_function_in_dto(
    tree_dto: &SemanticTreeDto,
    line: u32,
    character: u32,
) -> Option<(String, String, Vec<String>, Option<String>)> {
    for node in &tree_dto.root_nodes {
        if let Some(result) = find_in_node(node, line, character) {
            return Some(result);
        }
    }
    None
}

/// Recursive search in SemanticNodeDto
fn find_in_node(
    node: &SemanticNodeDto,
    line: u32,
    character: u32,
) -> Option<(String, String, Vec<String>, Option<String>)> {
    // Check if position is inside node's range (if exists)
    if let Some(ref range) = node.range {
        if !range_contains(range, line, character) {
            return None;
        }
    } else if !location_matches(&node.location, line, character) {
        return None;
    }

    // Check node type
    match node.kind.as_str() {
        "FunctionDeclaration" => {
            let name = node
                .name
                .clone()
                .unwrap_or_else(|| "<anonymous>".to_string());
            let params = extract_params_from_node(node);
            let return_type = extract_return_type_from_node(node);

            return Some((name, "function".to_string(), params, return_type));
        }
        "ProcedureDeclaration" => {
            let name = node
                .name
                .clone()
                .unwrap_or_else(|| "<anonymous>".to_string());
            let params = extract_params_from_node(node);

            return Some((name, "procedure".to_string(), params, None));
        }
        _ => {
            // Recursively search in children
            for child in &node.children {
                if let Some(result) = find_in_node(child, line, character) {
                    return Some(result);
                }
            }
        }
    }

    None
}

/// Check if range contains position
fn range_contains(range: &SourceRangeDto, line: u32, character: u32) -> bool {
    let start = &range.start;
    let end = &range.end;

    line >= start.line
        && line <= end.line
        && (line > start.line || character >= start.column)
        && (line < end.line || character <= end.column)
}

/// Check if location matches position
fn location_matches(location: &SourceLocationDto, line: u32, character: u32) -> bool {
    location.line == line && location.column == character
}

/// Extract parameters from node metadata (if exists)
fn extract_params_from_node(_node: &SemanticNodeDto) -> Vec<String> {
    // Stub for now - can be extended later through metadata
    vec![]
}

/// Extract return type from node metadata (if exists)
fn extract_return_type_from_node(_node: &SemanticNodeDto) -> Option<String> {
    // Stub for now - can be extended later through metadata
    None
}
