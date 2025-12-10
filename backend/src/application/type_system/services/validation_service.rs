//! Validation Service - code validation operations
//!
//! Functions for validating BSL code syntax and semantics.

use anyhow::Result;
use tracing::info;

use bsl_shared::api::ValidationErrorDto;
use bsl_shared::domain::types::{DiagnosticSeverity, ParseError, TypeDiagnostic};
use bsl_shared::domain::validators::TypeValidator;
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::engine::AnalysisEngine;
use bsl_shared::formatting::DetailLevel;
use bsl_shared::ir::walk_program;

use crate::application::ast_to_ir::AstToIrConverter;
use crate::application::semantic_validation_visitor::SemanticValidationVisitor;
use crate::system::ParserCoordinator;

/// Validates BSL code fragment via unified semantic validation
///
/// # Arguments
/// * `parser` - ParserCoordinator for parsing
/// * `analysis_engine` - AnalysisEngine for type resolution
/// * `metadata_lookup` - Lookup for type metadata
/// * `code` - Code fragment to validate
///
/// # Returns
/// List of validation errors or empty vector if code is valid
pub async fn validate_code_fragment(
    parser: &ParserCoordinator,
    analysis_engine: &AnalysisEngine,
    metadata_lookup: &TypeMetadataLookup,
    code: &str,
) -> Result<Vec<ValidationErrorDto>> {
    use std::time::Instant;

    let start = Instant::now();

    // Use validate_semantics with default detail_level
    let diagnostics = validate_semantics(
        parser,
        analysis_engine,
        metadata_lookup,
        code,
        None,
    )
    .await?;

    // Convert TypeDiagnostic -> ValidationErrorDto
    let errors: Vec<ValidationErrorDto> = diagnostics
        .iter()
        .map(|d| ValidationErrorDto {
            message: d.message.clone(),
            severity: match d.severity {
                DiagnosticSeverity::Error => "error".to_string(),
                DiagnosticSeverity::Warning => "warning".to_string(),
                DiagnosticSeverity::Info | DiagnosticSeverity::Hint => "info".to_string(),
            },
            line: d.line,
            column: d.column,
            end_line: d.end_line,
            end_column: d.end_column,
            error_type: if d.message.contains("не существует") {
                "NonExistentMethod".to_string()
            } else if d.message.contains("параметр") {
                "ParameterError".to_string()
            } else {
                "SemanticError".to_string()
            },
        })
        .collect();

    info!(
        "Validation completed in {:?}: {} errors found",
        start.elapsed(),
        errors.len()
    );
    Ok(errors)
}

/// Parse and validate BSL code (Unified API for LSP/CLI)
///
/// # Arguments
/// * `parser` - ParserCoordinator for parsing
/// * `source` - BSL source code to parse
///
/// # Returns
/// - `Ok(Vec<ParseError>)` - list of syntax errors (may be empty)
/// - `Err(anyhow::Error)` - critical parser error
pub fn parse_and_validate(
    parser: &ParserCoordinator,
    source: &str,
) -> Result<Vec<ParseError>> {
    // Delegate to ParserCoordinator (System Layer)
    let parse_result = parser
        .parse(source)
        .map_err(|e| anyhow::anyhow!("Parser error: {}", e))?;

    // Return syntax errors (may be empty Vec)
    Ok(parse_result.syntax_errors)
}

/// Validate code semantics via IR traversal
///
/// # Arguments
/// * `parser` - ParserCoordinator for parsing
/// * `analysis_engine` - AnalysisEngine for type resolution
/// * `metadata_lookup` - Lookup for type metadata
/// * `code` - Code to validate
/// * `detail_level` - Optional detail level for diagnostics
///
/// # Returns
/// List of TypeDiagnostic for semantic errors
pub async fn validate_semantics(
    parser: &ParserCoordinator,
    analysis_engine: &AnalysisEngine,
    metadata_lookup: &TypeMetadataLookup,
    code: &str,
    detail_level: Option<DetailLevel>,
) -> Result<Vec<TypeDiagnostic>> {
    // 1. Parse
    let parse_result = parser
        .parse(code)
        .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

    // 2. If syntax errors exist -> semantic validation is meaningless
    if !parse_result.syntax_errors.is_empty() {
        return Ok(Vec::new());
    }

    // 3. Get repository and SignatureIndex BEFORE conversion
    let repository = analysis_engine.get_repository();

    // 4. Get SignatureIndex clone BEFORE repository is moved (Milestone 3.10)
    let signature_index = repository.get_signature_index_clone();

    // 5. Get resolver for Milestone 3.17 (TypeResolver DI)
    // IMPORTANT: resolver needed BEFORE conversion for correct active_facet
    let resolver_arc = analysis_engine.get_resolver();

    // 6. Convert AST -> IR with TypeResolver for active_facet resolution (Milestone 3.17)
    tracing::info!("validate_semantics: converting AST to IR with resolver");
    let ir = AstToIrConverter::convert_with_resolver(
        parse_result.program,
        code.to_string(),
        "<semantic_validation>".to_string(),
        repository,
        signature_index.clone(),
        Some(resolver_arc.clone()),
    )?;
    tracing::info!("validate_semantics: IR created with {} nodes", ir.nodes.len());

    // 7. Create TypeValidator
    let validator = TypeValidator::new(metadata_lookup);

    // 8. Get resolver reference for visitor
    let resolver = resolver_arc.as_ref();

    // 9. Create SemanticValidationVisitor with configurable detail_level (Milestone 3.6 Phase 3)
    let mut visitor = if let Some(level) = detail_level {
        SemanticValidationVisitor::with_detail_level(&validator, &ir, resolver, &signature_index, level)
    } else {
        SemanticValidationVisitor::new(&validator, &ir, resolver, &signature_index)
    };

    // 10. Traverse IR
    walk_program(&ir, &mut visitor);

    // 11. Return errors
    Ok(visitor.into_errors())
}

/// Debug version of validate_semantics with extended diagnostics
///
/// # Arguments
/// * `parser` - ParserCoordinator for parsing
/// * `analysis_engine` - AnalysisEngine for type resolution
/// * `metadata_lookup` - Lookup for type metadata
/// * `code` - Code to validate
///
/// # Returns
/// Tuple of (errors, debug_info)
pub async fn validate_semantics_debug(
    parser: &ParserCoordinator,
    analysis_engine: &AnalysisEngine,
    metadata_lookup: &TypeMetadataLookup,
    code: &str,
) -> Result<(Vec<TypeDiagnostic>, serde_json::Value)> {
    let mut debug_info = serde_json::json!({
        "steps": [],
        "resolver_available": false,
        "property_accesses": []
    });

    // 1. Parse
    let parse_result = parser
        .parse(code)
        .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

    debug_info["steps"].as_array_mut().unwrap().push(serde_json::json!({
        "step": "parse",
        "success": true,
        "syntax_errors": parse_result.syntax_errors.len()
    }));

    if !parse_result.syntax_errors.is_empty() {
        return Ok((Vec::new(), debug_info));
    }

    // 2. Get repository and SignatureIndex
    let repository = analysis_engine.get_repository();
    let signature_index = repository.get_signature_index_clone();
    let resolver_arc = analysis_engine.get_resolver();

    debug_info["resolver_available"] = serde_json::json!(true);

    // 3. Convert AST -> IR
    let ir = AstToIrConverter::convert_with_resolver(
        parse_result.program,
        code.to_string(),
        "<debug_validation>".to_string(),
        repository.clone(),
        signature_index.clone(),
        Some(resolver_arc.clone()),
    )?;

    debug_info["steps"].as_array_mut().unwrap().push(serde_json::json!({
        "step": "ast_to_ir",
        "success": true,
        "ir_nodes": ir.nodes.len()
    }));

    // 4. Add basic IR info
    debug_info["ir_info"] = serde_json::json!({
        "nodes_count": ir.nodes.len(),
        "has_cfg": ir.cfg.is_some()
    });

    // 5. Create TypeValidator and run validation
    let validator = TypeValidator::new(metadata_lookup);
    let resolver = resolver_arc.as_ref();
    let mut visitor = SemanticValidationVisitor::new(&validator, &ir, resolver, &signature_index);

    walk_program(&ir, &mut visitor);
    let errors = visitor.into_errors();

    debug_info["steps"].as_array_mut().unwrap().push(serde_json::json!({
        "step": "semantic_validation",
        "success": true,
        "errors_found": errors.len()
    }));

    Ok((errors, debug_info))
}
