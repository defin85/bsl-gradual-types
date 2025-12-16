//! File Analysis Service - file content analysis operations
//!
//! Functions for analyzing BSL files and extracting type information.

use anyhow::Result;
use std::collections::HashMap;
use tracing::info;

use bsl_shared::utils::hash::hash_content;

use super::super::extractors::type_extractor::{
    extract_function_name, extract_return_type, extract_type_from_var_declaration, extract_var_name,
};
use crate::application::TypeInferenceService;
use crate::system::{AnalysisCache, CacheAnalysisResult, ParserCoordinator};

/// CLI operations - file analysis
///
/// # Arguments
/// * `parser` - ParserCoordinator for parsing
/// * `path` - File path to analyze
///
/// # Returns
/// CacheAnalysisResult with type resolutions
pub async fn analyze_file(parser: &ParserCoordinator, path: &str) -> Result<CacheAnalysisResult> {
    info!("Analyzing file: {}", path);

    let file_content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", path, e))?;

    let _parse_result = parser
        .parse(&file_content)
        .map_err(|e| anyhow::anyhow!("Parse error for file {}: {}", path, e))?;

    let analysis_result = CacheAnalysisResult {
        file_path: path.to_string(),
        type_resolutions: HashMap::new(),
        analysis_duration_ms: 0,
        cached_at: std::time::Instant::now(),
    };

    info!("File analysis completed: {}", path);
    Ok(analysis_result)
}

/// Analyze file content without reading from disk (Phase 4: improved implementation)
///
/// # Arguments
/// * `parser` - ParserCoordinator for parsing
/// * `cache` - Analysis cache
/// * `inference_service` - TypeInferenceService for type resolution
/// * `file_path` - Virtual file path
/// * `content` - File content to analyze
///
/// # Returns
/// CacheAnalysisResult with type resolutions
pub async fn analyze_file_content(
    parser: &ParserCoordinator,
    cache: &AnalysisCache,
    inference_service: &TypeInferenceService,
    file_path: &str,
    content: &str,
) -> Result<CacheAnalysisResult> {
    let start_time = std::time::Instant::now();
    info!("Analyzing file content: {}", file_path);

    // 1. Check cache (Application Layer logic)
    let cache_key = format!("{}:{}", file_path, hash_content(content));
    if let Some(cached_result) = cache.get_analysis(&cache_key) {
        info!("Cache hit for file: {}", file_path);
        return Ok(cached_result);
    }

    // 2. Parse file
    let parse_result = parser
        .parse(content)
        .map_err(|e| anyhow::anyhow!("Parse error for content {}: {}", file_path, e))?;

    info!(
        "Parse successful, found {} statements",
        parse_result.program.statements.len()
    );

    // 3. Extract variables and types from AST
    let mut type_resolutions = HashMap::new();

    // Simple heuristic: extract variables with types
    for line in content.lines() {
        // Pattern: Перем ИмяПеременной: ТипДанных
        if line.trim().starts_with("Перем ") {
            if let Some(type_hint) = extract_type_from_var_declaration(line) {
                let var_name = extract_var_name(line).unwrap_or("unknown".to_string());

                // Use TypeInferenceService to resolve type
                let resolution = inference_service.resolve_expression_async(&type_hint).await;
                type_resolutions.insert(var_name, resolution);
            }
        }

        // Pattern: Функция ИмяФункции() Возврат Тип;
        if line.trim().starts_with("Функция ") || line.trim().starts_with("Процедура ")
        {
            if let Some(return_type) = extract_return_type(line) {
                let func_name = extract_function_name(line).unwrap_or("unknown".to_string());

                let resolution = inference_service
                    .resolve_expression_async(&return_type)
                    .await;
                type_resolutions.insert(format!("return_{}", func_name), resolution);
            }
        }
    }

    let analysis_duration_ms = start_time.elapsed().as_millis();

    let analysis_result = CacheAnalysisResult {
        file_path: file_path.to_string(),
        type_resolutions,
        analysis_duration_ms: analysis_duration_ms as u64,
        cached_at: std::time::Instant::now(),
    };

    // 4. Save to cache
    cache.store_analysis(cache_key, analysis_result.clone());

    info!(
        "Analysis of {} completed in {}ms",
        file_path, analysis_duration_ms
    );
    Ok(analysis_result)
}

/// Get semantic tree for file (MILESTONE 2.12)
///
/// Parses file, converts AST to IR, and returns SemanticTreeDto
///
/// # Arguments
/// * `parser` - ParserCoordinator for parsing
/// * `file_content` - File content
/// * `file_path` - File path for identification
/// * `compact` - If true, returns compact version without symbol_table and call_graph
/// * `include_call_graph` - Include call graph in response (ignored if compact=true)
/// * `include_flow_sensitive` - Include flow-sensitive info in response (ignored if compact=true)
///
/// # Returns
/// SemanticTreeDto with semantic information
pub async fn get_semantic_tree(
    parser: &ParserCoordinator,
    file_content: &str,
    file_path: &str,
    compact: bool,
    include_call_graph: bool,
    include_flow_sensitive: bool,
) -> Result<bsl_shared::api::semantic_dtos::SemanticTreeDto> {
    info!(
        "Generating semantic tree for: {} (compact: {}, call_graph: {}, flow_sensitive: {})",
        file_path, compact, include_call_graph, include_flow_sensitive
    );

    // 1. Parse file -> AST -> IR (using ready method from ParserCoordinator)
    let semantic_program = parser
        .parse_to_ir(file_content, file_path)
        .map_err(|e| anyhow::anyhow!("Failed to parse file: {}", e))?;

    // 2. Convert SemanticProgram -> SemanticTreeDto
    let dto = if compact {
        semantic_program.to_compact_dto()
    } else {
        semantic_program.to_dto(include_call_graph, include_flow_sensitive)
    };

    info!(
        "Semantic tree generated: {} root nodes, {} symbols, {} metrics: {:?}",
        dto.root_nodes.len(),
        dto.symbol_table.len(),
        dto.metrics.node_count,
        dto.metrics
    );

    Ok(dto)
}

/// Parse file content to SemanticProgram for visualization (MILESTONE E2)
///
/// # Arguments
/// * `parser` - ParserCoordinator for parsing
/// * `content` - File content as string
///
/// # Returns
/// Result<SemanticProgram> - Parsed semantic representation
pub async fn parse_semantic_program(
    parser: &ParserCoordinator,
    content: &str,
) -> Result<bsl_shared::ir::SemanticProgram> {
    let program = parser
        .parse_to_ir(content, "visualization.bsl")
        .map_err(|e| anyhow::anyhow!("Failed to parse semantic program: {}", e))?;
    Ok(program)
}

/// Parse incremental file changes (for LSP textDocument/didChange)
///
/// # Arguments
/// * `parser` - ParserCoordinator for parsing
/// * `file_path` - Path to the file
/// * `new_content` - New file content
/// * `edits` - List of text edits
///
/// # Returns
/// Result indicating success
pub async fn parse_incremental(
    parser: &ParserCoordinator,
    file_path: std::path::PathBuf,
    new_content: String,
    edits: Vec<crate::system::parser_coordinator::TextEdit>,
) -> Result<()> {
    info!("Incremental parsing file: {:?}", file_path);

    let _result = parser
        .parse_incremental(file_path, new_content, edits)
        .map_err(|e| anyhow::anyhow!("Incremental parsing error: {}", e))?;

    info!("Incremental parsing completed successfully");
    Ok(())
}
