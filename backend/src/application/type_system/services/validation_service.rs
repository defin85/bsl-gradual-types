//! Validation Service - code validation operations
//!
//! Functions for validating BSL code syntax and semantics.

use anyhow::{anyhow, Result};
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
    let diagnostics =
        validate_semantics(parser, analysis_engine, metadata_lookup, code, None).await?;

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

/// Validate code semantics via IR traversal with a known file path.
///
/// Важно для модулей форм: `AstToIrConverter` засевает `Объект/Элементы/ЭтаФорма`
/// только если `file_path` распознаётся как `FormModule` по `CodeLocation`.
pub async fn validate_semantics_with_file_path(
    parser: &ParserCoordinator,
    analysis_engine: &AnalysisEngine,
    metadata_lookup: &TypeMetadataLookup,
    code: &str,
    file_path: &str,
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
        file_path.to_string(),
        repository,
        signature_index.clone(),
        Some(resolver_arc.clone()),
    )?;
    tracing::info!(
        "validate_semantics: IR created with {} nodes",
        ir.nodes.len()
    );

    // 7. Create TypeValidator
    let validator = TypeValidator::new(metadata_lookup);

    // 8. Get resolver reference for visitor
    let resolver = resolver_arc.as_ref();

    // 9. Create SemanticValidationVisitor with configurable detail_level (Milestone 3.6 Phase 3)
    let mut visitor = if let Some(level) = detail_level {
        SemanticValidationVisitor::with_detail_level(
            &validator,
            &ir,
            resolver,
            &signature_index,
            level,
        )
    } else {
        SemanticValidationVisitor::new(&validator, &ir, resolver, &signature_index)
    };

    // 10. Traverse IR
    walk_program(&ir, &mut visitor);

    // 11. Return errors
    Ok(visitor.into_errors())
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
pub fn parse_and_validate(parser: &ParserCoordinator, source: &str) -> Result<Vec<ParseError>> {
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
    validate_semantics_with_file_path(
        parser,
        analysis_engine,
        metadata_lookup,
        code,
        "<semantic_validation>",
        detail_level,
    )
    .await
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

    let steps = debug_info["steps"]
        .as_array_mut()
        .ok_or_else(|| anyhow!("debug_info.steps is not array"))?;
    steps.push(serde_json::json!({
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

    let steps = debug_info["steps"]
        .as_array_mut()
        .ok_or_else(|| anyhow!("debug_info.steps is not array"))?;
    steps.push(serde_json::json!({
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

    let steps = debug_info["steps"]
        .as_array_mut()
        .ok_or_else(|| anyhow!("debug_info.steps is not array"))?;
    steps.push(serde_json::json!({
        "step": "semantic_validation",
        "success": true,
        "errors_found": errors.len()
    }));

    Ok((errors, debug_info))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::loaders::config_metadata_parser::ConfigurationDiscovery;
    use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
    use bsl_shared::domain::resolver::TypeResolver;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn test_config_path() -> PathBuf {
        let backend_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = backend_root.parent().expect("Failed to get workspace root");
        workspace_root
            .join("examples")
            .join("conf")
            .join("conf_test")
    }

    #[test]
    fn semantic_validation_seeds_form_module_context_only_with_real_file_path() {
        let discovery = ConfigurationDiscovery::new(test_config_path(), false);
        let configs = discovery
            .discover_all_configurations()
            .expect("Failed to discover configurations");
        let first = &configs[0];

        let metadata = discovery
            .discover_metadata_in_configuration(first, None::<fn(_)>)
            .expect("Failed to discover metadata");

        let doc = metadata
            .iter()
            .find(|m| m.name == "ЗаказНаряды" && m.object_type_raw == "Document")
            .expect("Should find Document.ЗаказНаряды");

        let raw_types = doc.to_raw_type_data_with_forms(None);

        let repo = Arc::new(InMemoryTypeRepository::new());
        repo.load_types(raw_types).expect("Failed to load types");

        let resolver = Arc::new(TypeResolver::new(repo.clone() as Arc<dyn TypeRepository>));
        let analysis_engine = AnalysisEngine::new(resolver, repo.clone() as Arc<dyn TypeRepository>);
        let metadata_lookup = TypeMetadataLookup::new(repo.clone() as Arc<dyn TypeRepository>);
        let parser = ParserCoordinator::with_fallback();

        let code = "Процедура Тест()\n    b = Объект.Работы;\nКонецПроцедуры";

        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

        // Без корректного file_path контекст формы не засеется, и будет UnknownTypeAccess для Объект.Работы
        let errors_without_path = rt
            .block_on(validate_semantics(
                &parser,
                &analysis_engine,
                &metadata_lookup,
                code,
                None,
            ))
            .expect("validate_semantics");
        assert!(
            errors_without_path
                .iter()
                .any(|e| e.message.contains("Невозможно определить член 'Работы'") && e.message.contains("'Объект'")),
            "Expected UnknownTypeAccess for Объект.Работы when file_path is not a FormModule"
        );

        // С корректным file_path модуль распознаётся как FormModule, и 'Объект' получает тип данных формы
        let form_module_path = "Documents/ЗаказНаряды/Forms/ФормаДокумента/Ext/Form/Module.bsl";
        let errors_with_path = rt
            .block_on(validate_semantics_with_file_path(
                &parser,
                &analysis_engine,
                &metadata_lookup,
                code,
                form_module_path,
                None,
            ))
            .expect("validate_semantics_with_file_path");
        assert!(
            !errors_with_path
                .iter()
                .any(|e| e.message.contains("Невозможно определить член 'Работы'") && e.message.contains("'Объект'")),
            "Did not expect UnknownTypeAccess for Объект.Работы when file_path is a FormModule"
        );
    }
}
