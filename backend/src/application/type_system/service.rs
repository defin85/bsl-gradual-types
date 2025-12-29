//! TypeSystemService - Main unified service for type system operations
//!
//! Phase 4: API Unification - replaces LspTypeService + WebTypeService + AnalysisService
//! with a single unified API for all presentation layers.

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tracing::info;

use bsl_shared::api::dtos::AnalysisResultDto;
use bsl_shared::api::ValidationErrorDto;
use bsl_shared::domain::type_definition_location::TypeDefinitionLocation;
use bsl_shared::domain::types::{ParseError, TypeDiagnostic, TypeResolution};
use bsl_shared::domain::{CompletionItem, TypeMetadataLookup};
use bsl_shared::engine::AnalysisEngine;
use bsl_shared::formatting::DetailLevel;
use url::Url;

use crate::application::TypeInferenceService;
use crate::helpers::hover_formatter::{HoverFormatConfig, HoverFormatter, HoverOutputFormat};
use crate::system::{
    AnalysisCache, CacheAnalysisResult, IntellisenseIndexStore, IrCache, IrCacheStats,
    ParserCoordinator,
};

use super::loaders::configuration_loader;
use super::services::{
    completion_service, file_analysis_service, hover_service, validation_service, web_api_service,
};

/// Unified Type System Service for Application Layer
///
/// Phase 4: Replaces LspTypeService + WebTypeService + AnalysisService
/// with single unified API for all presentation layers
pub struct TypeSystemService {
    // Application Layer: AnalysisEngine - pure analysis orchestration
    analysis_engine: Arc<AnalysisEngine>,

    // Application Layer: Type Inference Service for high-level operations
    inference_service: Arc<TypeInferenceService>,

    // Domain Layer: TypeMetadataLookup - bridge between TypeResolution and RawTypeData
    metadata_lookup: TypeMetadataLookup,

    // Helper Layer: HoverFormatter - unified formatting of hover responses
    hover_formatter: HoverFormatter,

    // System Layer components
    cache: Arc<AnalysisCache>,
    ir_cache: Arc<IrCache>, // Milestone 2.13: IR caching for LSP hover
    parser: Arc<ParserCoordinator>,
    intellisense_index: Arc<IntellisenseIndexStore>,

    /// Mapping file URI -> content hash for smart cache invalidation.
    ///
    /// Allows determining if file changed (new hash) or not (old hash).
    /// When file changes, old hash is removed from ir_cache.
    ///
    /// # Milestone 2.13: IR Caching
    uri_to_hash: Arc<tokio::sync::RwLock<HashMap<String, u64>>>,

    /// Hover counter for periodic stats output (every 100).
    ///
    /// Used to monitor cache hit rate in real usage.
    ///
    /// # Milestone 2.13: Performance Metrics
    hover_count: Arc<std::sync::atomic::AtomicU64>,
}

impl TypeSystemService {
    /// Constructor according to architectural diagram
    pub fn new(
        analysis_engine: Arc<AnalysisEngine>,
        cache: Arc<AnalysisCache>,
        parser: Arc<ParserCoordinator>,
        ir_cache: Arc<IrCache>, // Milestone 2.13: IR caching
        intellisense_index: Arc<IntellisenseIndexStore>,
    ) -> Self {
        // Create TypeInferenceService based on AnalysisEngine
        let resolver = analysis_engine.get_resolver();
        let repository = analysis_engine.get_repository();
        let inference_service = Arc::new(TypeInferenceService::new(
            resolver.clone(),
            repository.clone(),
        ));

        // Create TypeMetadataLookup for getting methods/properties from RawTypeData
        let metadata_lookup = TypeMetadataLookup::new(repository);

        // Create HoverFormatter with default configuration
        let hover_config = HoverFormatConfig {
            max_methods: 10,
            max_properties: 5,
            output_format: HoverOutputFormat::Markdown,
            ..Default::default()
        };
        let hover_formatter = HoverFormatter::new(hover_config, metadata_lookup.clone());

        Self {
            analysis_engine,
            inference_service,
            metadata_lookup,
            hover_formatter,
            cache,
            ir_cache,
            parser,
            intellisense_index,
            // MILESTONE 2.13: Initialize new fields
            uri_to_hash: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            hover_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    pub fn initialize(&self) -> Result<()> {
        info!("TypeSystemService initialized (Phase 4: Unified API)");
        Ok(())
    }

    // ============================================================================
    // UNIFIED API FOR ALL CLIENTS
    // ============================================================================

    /// Get all platform global types (for Web API)
    pub fn get_all_platform_globals(&self) -> HashMap<String, TypeResolution> {
        // Delegate to TypeInferenceService (Application Layer)
        self.inference_service.get_all_platform_globals()
    }

    /// Phase 5: Get all types with DTO transformation (Web API)
    pub fn get_all_types_as_dto(
        &self,
        limit: usize,
        offset: usize,
        category_filter: Option<String>,
        certainty_filter: Option<String>,
        flow_sensitive_only: bool,
    ) -> AnalysisResultDto {
        web_api_service::get_all_types_as_dto(
            &self.inference_service,
            &self.metadata_lookup,
            &self.cache,
            limit,
            offset,
            category_filter,
            certainty_filter,
            flow_sensitive_only,
        )
    }

    /// Phase 5: Get metrics summary for Web API
    pub fn get_metrics_summary(&self) -> serde_json::Value {
        web_api_service::get_metrics_summary(&self.inference_service)
    }

    // ============================================================================
    // FILE ANALYSIS OPERATIONS
    // ============================================================================

    /// CLI operations - file analysis
    pub async fn analyze_file(&self, path: &str) -> Result<CacheAnalysisResult> {
        file_analysis_service::analyze_file(&self.parser, path).await
    }

    /// LSP operations - incremental parsing for textDocument/didChange
    pub async fn parse_incremental(
        &self,
        file_path: std::path::PathBuf,
        new_content: String,
        edits: Vec<crate::system::parser_coordinator::TextEdit>,
    ) -> Result<()> {
        file_analysis_service::parse_incremental(&self.parser, file_path, new_content, edits).await
    }

    /// Analyze file content without reading from disk (Phase 4: improved implementation)
    pub async fn analyze_file_content(
        &self,
        file_path: &str,
        content: &str,
    ) -> Result<CacheAnalysisResult> {
        file_analysis_service::analyze_file_content(
            &self.parser,
            &self.cache,
            &self.inference_service,
            file_path,
            content,
        )
        .await
    }

    /// MILESTONE 2.12: Get semantic tree for file
    ///
    /// # Arguments
    /// * `file_content` - BSL code content
    /// * `file_path` - File path for identification
    /// * `compact` - If true, returns compact version without symbol_table and call_graph
    /// * `include_call_graph` - Include call graph in response (default: true)
    /// * `include_flow_sensitive` - Include flow-sensitive info in response (default: true)
    pub async fn get_semantic_tree(
        &self,
        file_content: &str,
        file_path: &str,
        compact: bool,
        include_call_graph: bool,
        include_flow_sensitive: bool,
    ) -> Result<bsl_shared::api::semantic_dtos::SemanticTreeDto> {
        file_analysis_service::get_semantic_tree(
            &self.parser,
            file_content,
            file_path,
            compact,
            include_call_graph,
            include_flow_sensitive,
        )
        .await
    }

    /// MILESTONE E2: Parse file content to SemanticProgram for visualization
    pub async fn parse_semantic_program(
        &self,
        content: &str,
    ) -> Result<bsl_shared::ir::SemanticProgram> {
        file_analysis_service::parse_semantic_program(&self.parser, content).await
    }

    // ============================================================================
    // HOVER OPERATIONS
    // ============================================================================

    /// LSP operations - get symbol information at position (hover)
    ///
    /// MILESTONE 3.6 Phase 1: Added optional hover_config parameter
    /// for configurable detail levels
    pub async fn get_hover_info(
        &self,
        file_content: &str,
        line: u32,
        column: u32,
        hover_config: Option<HoverFormatConfig>,
    ) -> Result<Option<String>> {
        hover_service::get_hover_info(
            &self.parser,
            &self.analysis_engine,
            &self.ir_cache,
            &self.metadata_lookup,
            &self.hover_formatter,
            &self.hover_count,
            file_content,
            line,
            column,
            hover_config,
        )
        .await
    }

    /// Hover с учётом пути к файлу (важно для модулей форм).
    pub async fn get_hover_info_for_file(
        &self,
        file_content: &str,
        file_path: &str,
        line: u32,
        column: u32,
        hover_config: Option<HoverFormatConfig>,
    ) -> Result<Option<String>> {
        hover_service::get_hover_info_with_file_path(
            &self.parser,
            &self.analysis_engine,
            &self.ir_cache,
            &self.metadata_lookup,
            &self.hover_formatter,
            &self.hover_count,
            file_content,
            file_path,
            line,
            column,
            hover_config,
        )
        .await
    }

    /// Get TypeResolution for symbol at specified position (Milestone 3.14)
    ///
    /// Used for Go To Definition
    pub async fn get_type_at_position(
        &self,
        file_content: &str,
        line: u32,
        column: u32,
    ) -> Result<Option<TypeResolution>> {
        hover_service::get_type_at_position(
            &self.parser,
            &self.analysis_engine,
            &self.ir_cache,
            file_content,
            line,
            column,
        )
        .await
    }

    /// Go To Definition для метода/функции в позиции курсора (C7)
    pub async fn get_method_definition_at_position(
        &self,
        file_content: &str,
        line: u32,
        column: u32,
    ) -> Result<Option<TypeDefinitionLocation>> {
        hover_service::get_method_definition_at_position(
            &self.parser,
            &self.analysis_engine,
            &self.ir_cache,
            file_content,
            None,
            line,
            column,
        )
        .await
    }

    /// Go To Definition для метода/функции с учётом пути к файлу (для локальных/приватных методов)
    pub async fn get_method_definition_at_position_for_file(
        &self,
        file_content: &str,
        file_path: &str,
        line: u32,
        column: u32,
    ) -> Result<Option<TypeDefinitionLocation>> {
        hover_service::get_method_definition_at_position(
            &self.parser,
            &self.analysis_engine,
            &self.ir_cache,
            file_content,
            Some(file_path),
            line,
            column,
        )
        .await
    }

    /// Invalidate cache for changed file (MILESTONE 2.13)
    pub async fn invalidate_file_cache(
        &self,
        file_uri: &str,
        new_content: &str,
        config_root: Option<&Path>,
    ) {
        hover_service::invalidate_file_cache(&self.uri_to_hash, file_uri, new_content).await;
        let module_key = module_key_from_uri(file_uri, config_root);
        self.intellisense_index
            .invalidate_file(file_uri, module_key.as_deref());
    }

    // ============================================================================
    // COMPLETION OPERATIONS
    // ============================================================================

    /// LSP operations - get completion at position
    pub async fn get_completion(
        &self,
        file_content: &str,
        line: u32,
        column: u32,
        file_uri: Option<&str>,
    ) -> Result<completion_service::CompletionResult> {
        completion_service::get_completion(
            file_content,
            line,
            column,
            file_uri,
            &self.intellisense_index,
            &self.metadata_lookup,
        )
        .await
    }

    pub fn resolve_type_completion(
        &self,
        type_name: &str,
    ) -> Option<(Option<String>, Option<String>)> {
        completion_service::resolve_type_details(type_name, &self.metadata_lookup)
    }

    pub fn resolve_method_completion(
        &self,
        owner_type: &str,
        method_name: &str,
    ) -> Option<(Option<String>, Option<String>)> {
        completion_service::resolve_method_details(owner_type, method_name, &self.metadata_lookup)
    }

    // ============================================================================
    // VALIDATION OPERATIONS
    // ============================================================================

    /// Validate 1C code using TypeValidator
    pub async fn validate_code_fragment(&self, code: &str) -> Result<Vec<ValidationErrorDto>> {
        validation_service::validate_code_fragment(
            &self.parser,
            &self.analysis_engine,
            &self.metadata_lookup,
            code,
        )
        .await
    }

    /// Parse and validate BSL code (MILESTONE 2.19)
    pub fn parse_and_validate(&self, source: &str) -> Result<Vec<ParseError>> {
        validation_service::parse_and_validate(&self.parser, source)
    }

    pub fn parse_and_validate_for_file(
        &self,
        source: &str,
        file_path: &str,
    ) -> Result<Vec<ParseError>> {
        validation_service::parse_and_validate_for_file(&self.parser, source, file_path)
    }

    /// Validate code semantics via IR traversal (Milestone 3.7)
    pub async fn validate_semantics(
        &self,
        code: &str,
        detail_level: Option<DetailLevel>,
    ) -> Result<Vec<TypeDiagnostic>> {
        validation_service::validate_semantics(
            &self.parser,
            &self.analysis_engine,
            &self.metadata_lookup,
            code,
            detail_level,
        )
        .await
    }

    /// Validate code semantics with file path context (нужно для модулей форм).
    pub async fn validate_semantics_for_file(
        &self,
        code: &str,
        file_path: &str,
        detail_level: Option<DetailLevel>,
    ) -> Result<Vec<TypeDiagnostic>> {
        validation_service::validate_semantics_with_file_path(
            &self.parser,
            &self.analysis_engine,
            &self.metadata_lookup,
            code,
            file_path,
            detail_level,
        )
        .await
    }

    /// Debug version of validate_semantics with extended diagnostics
    pub async fn validate_semantics_debug(
        &self,
        code: &str,
    ) -> Result<(Vec<TypeDiagnostic>, serde_json::Value)> {
        validation_service::validate_semantics_debug(
            &self.parser,
            &self.analysis_engine,
            &self.metadata_lookup,
            code,
        )
        .await
    }

    // ============================================================================
    // WEB API OPERATIONS
    // ============================================================================

    /// Web operations - search types
    pub async fn search_types(&self, query: &str) -> Result<Vec<String>> {
        web_api_service::search_types(&self.inference_service, query).await
    }

    /// Phase 5: Search types with DTO transformation (Web API)
    pub async fn search_types_as_dto(&self, query: &str) -> Result<AnalysisResultDto> {
        web_api_service::search_types_as_dto(
            &self.inference_service,
            &self.metadata_lookup,
            &self.cache,
            query,
        )
        .await
    }

    /// Web operations - get type details
    pub async fn get_type_details(&self, type_name: &str) -> Result<Option<TypeResolution>> {
        web_api_service::get_type_details(&self.inference_service, type_name).await
    }

    /// Web operations - get completions for expression
    pub async fn get_type_completions(&self, expression: &str) -> Result<Vec<CompletionItem>> {
        web_api_service::get_type_completions(&self.inference_service, expression).await
    }

    // ============================================================================
    // CONFIGURATION LOADING OPERATIONS
    // ============================================================================

    /// MILESTONE 2.17: Load types from 1C configuration
    pub fn load_configuration_types(&self, config_path: &std::path::Path) -> Result<usize> {
        configuration_loader::load_configuration_types(&self.analysis_engine, config_path)
    }

    /// Get module paths for configuration type (Milestone 3.14)
    pub fn get_module_paths_for_type(
        &self,
        type_name: &str,
    ) -> Option<bsl_shared::domain::type_definition_location::ModulePaths> {
        configuration_loader::get_module_paths_for_type(&self.analysis_engine, type_name)
    }

    /// Pre-warm signature cache (Milestone 3.15)
    pub fn prewarm_signature_cache(&self) {
        configuration_loader::prewarm_signature_cache(&self.analysis_engine)
    }

    // ============================================================================
    // CACHE STATISTICS
    // ============================================================================

    /// MILESTONE 2.13: Get IR cache statistics
    pub async fn get_ir_cache_stats(&self) -> IrCacheStats {
        self.ir_cache.get_stats().await
    }
}

fn module_key_from_uri(file_uri: &str, config_root: Option<&Path>) -> Option<String> {
    let url = Url::parse(file_uri).ok()?;
    let path = url.to_file_path().ok()?;
    if let Some(root) = config_root {
        let key_path = path.strip_prefix(root).unwrap_or(&path);
        return Some(key_path.to_string_lossy().to_string());
    }
    Some(path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_key_from_uri_file_scheme() {
        let uri = "file:///tmp/example/Module.bsl";
        let key = module_key_from_uri(uri, None).expect("module key");
        assert!(key.ends_with("Module.bsl"));
    }

    #[test]
    fn module_key_from_uri_relative_to_root() {
        let uri = "file:///tmp/example/Module.bsl";
        let root = Path::new("/tmp");
        let key = module_key_from_uri(uri, Some(root)).expect("module key");
        assert_eq!(key, "example/Module.bsl");
    }
}

// Re-export CompletionContext for backward compatibility
pub use completion_service::CompletionContext;
