//! Улучшенный LSP сервер с интеграцией продвинутых анализаторов типов
//!
//! Этот модуль предоставляет расширенную функциональность LSP сервера:
//! - Инкрементальный парсинг с tree-sitter
//! - Интеграция flow-sensitive анализа
//! - Union типы в hover и completion
//! - Межпроцедурный анализ для лучшего автодополнения

use crate::parsing::bsl::ast::Program;
use crate::parsing::bsl::common::{Parser, ParserFactory, TextChange};
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::types::{ConcreteType, ResolutionResult, TypeResolution};
use bsl_shared::domain::types::{DiagnosticSeverity, TypeContext, TypeDiagnostic};
use bsl_shared::domain::{CompletionItem as BslCompletion, CompletionKind};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::lsp_types::*;

/// Расширенное автодополнение с дополнительной информацией от flow-sensitive анализа
#[derive(Debug, Clone)]
pub struct EnhancedCompletion {
    pub completion: BslCompletion,
    pub enhanced_info: Option<String>,
    pub flow_sensitive_type: Option<TypeResolution>,
}

/// Контекст для автодополнения
#[derive(Debug, Clone)]
pub struct CompletionContext {
    pub uri: String,
    pub prefix: String,
}

/// Состояние документа с кешированными результатами анализа
#[derive(Debug, Clone)]
pub struct DocumentState {
    pub content: String,
    pub ast: Option<Program>,
    pub version: i32,
    pub type_context: Option<TypeContext>,
    pub diagnostics: Vec<TypeDiagnostic>,
    pub last_analysis: std::time::Instant,
}

impl DocumentState {
    pub fn new(content: String, version: i32) -> Self {
        Self {
            content,
            ast: None,
            version,
            type_context: None,
            diagnostics: vec![],
            last_analysis: std::time::Instant::now(),
        }
    }

    pub fn needs_reanalysis(&self) -> bool {
        self.ast.is_none()
            || self.type_context.is_none()
            || self.last_analysis.elapsed() > std::time::Duration::from_secs(5)
    }
}

/// Менеджер инкрементального парсинга
pub struct IncrementalParsingManager {
    parser: Box<dyn Parser>,
    ast_cache: HashMap<String, Program>,
}

impl Default for IncrementalParsingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl IncrementalParsingManager {
    pub fn new() -> Self {
        Self {
            parser: ParserFactory::create(),
            ast_cache: HashMap::new(),
        }
    }

    pub fn parse_incremental(
        &mut self,
        uri: &str,
        new_content: &str,
        changes: &[TextDocumentContentChangeEvent],
    ) -> anyhow::Result<Program> {
        let text_changes: Vec<TextChange> = changes
            .iter()
            .filter_map(|change| {
                change.range.map(|range| TextChange {
                    start_byte: self.position_to_byte_offset(new_content, range.start),
                    old_end_byte: self.position_to_byte_offset(new_content, range.end),
                    new_end_byte: self.position_to_byte_offset(new_content, range.end),
                    start_position: crate::parsing::bsl::common::Position {
                        line: range.start.line,
                        column: range.start.character,
                    },
                    old_end_position: crate::parsing::bsl::common::Position {
                        line: range.end.line,
                        column: range.end.character,
                    },
                    new_end_position: crate::parsing::bsl::common::Position {
                        line: range.end.line,
                        column: range.end.character,
                    },
                })
            })
            .collect();

        let program = if !text_changes.is_empty() && self.ast_cache.contains_key(uri) {
            match self.parser.parse_incremental(new_content, &text_changes) {
                Ok(ast) => ast,
                Err(_) => self
                    .parser
                    .parse(new_content)
                    .map_err(|e| anyhow::anyhow!(e))?,
            }
        } else {
            self.parser
                .parse(new_content)
                .map_err(|e| anyhow::anyhow!(e))?
        };

        self.ast_cache.insert(uri.to_string(), program.clone());

        Ok(program)
    }

    fn position_to_byte_offset(&self, content: &str, position: Position) -> usize {
        let mut byte_offset = 0;
        let mut current_line = 0u32;
        let mut current_char = 0u32;

        for ch in content.chars() {
            if current_line > position.line {
                break;
            }

            if current_line == position.line && current_char >= position.character {
                break;
            }

            if ch == '\n' {
                current_line += 1;
                current_char = 0;
            } else {
                current_char += ch.len_utf16() as u32;
            }

            byte_offset += ch.len_utf8();
        }

        byte_offset
    }
}

#[allow(dead_code)]
pub struct LspEnhancedService {
    parser: std::sync::Arc<crate::system::ParserCoordinator>,
    type_resolution_service: std::sync::Arc<TypeResolver>,
}

pub struct EnhancedTypeAnalyzer {
    parsing_manager: IncrementalParsingManager,
    analysis_cache: HashMap<String, (TypeContext, Vec<TypeDiagnostic>)>,
    type_resolution_service: std::sync::Arc<TypeResolver>,
}

impl EnhancedTypeAnalyzer {
    pub fn new(type_resolution_service: std::sync::Arc<TypeResolver>) -> Self {
        Self {
            parsing_manager: IncrementalParsingManager::new(),
            analysis_cache: HashMap::new(),
            type_resolution_service,
        }
    }

    pub fn analyze_document(
        &mut self,
        uri: &str,
        content: &str,
        changes: &[TextDocumentContentChangeEvent],
    ) -> anyhow::Result<(TypeContext, Vec<TypeDiagnostic>)> {
        let _program = self
            .parsing_manager
            .parse_incremental(uri, content, changes)?;

        let _file_name = Self::uri_to_filename(uri);
        let context = TypeContext::new();
        // let _type_checker = TypeChecker::new(context.clone()); // TypeChecker does not exist yet

        // TODO: Implement real analysis
        let diagnostics = Vec::new();

        self.analysis_cache
            .insert(uri.to_string(), (context.clone(), diagnostics.clone()));

        Ok((context, diagnostics))
    }

    pub fn get_type_at_position(&self, uri: &str, _position: Position) -> Option<TypeResolution> {
        let (context, _) = self.analysis_cache.get(uri)?;
        context.symbol_table.values().next().cloned()
    }

    pub fn get_enhanced_completions(
        &mut self,
        _source: &str,
        _position: Position,
        context: CompletionContext,
    ) -> Vec<EnhancedCompletion> {
        let mut completions = Vec::new();
        let base_completions = self
            .type_resolution_service
            .get_completions(&context.prefix);
        for completion in base_completions {
            completions.push(EnhancedCompletion {
                completion,
                enhanced_info: None,
                flow_sensitive_type: None,
            });
        }

        if let Some((analysis_context, _)) = self.analysis_cache.get(&context.uri) {
            for (var_name, var_type) in &analysis_context.symbol_table {
                if var_name.starts_with(&context.prefix) {
                    let completion = BslCompletion::with_details(
                        var_name.clone(),
                        CompletionKind::Variable,
                        Some(Self::format_type_short(var_type)),
                        Some(Self::format_type_info(var_type)),
                    );

                    completions.push(EnhancedCompletion {
                        completion,
                        enhanced_info: Some(Self::format_type_info(var_type)),
                        flow_sensitive_type: Some(var_type.clone()),
                    });
                }
            }
        }
        completions
    }

    pub fn get_cached_diagnostics(&self, uri: &str) -> Vec<TypeDiagnostic> {
        self.analysis_cache
            .get(uri)
            .map(|(_, diagnostics)| diagnostics.clone())
            .unwrap_or_default()
    }

    fn format_type_info(type_resolution: &TypeResolution) -> String {
        match &type_resolution.result {
            ResolutionResult::Concrete(concrete_type) => {
                format!(
                    "**Тип:** `{}`\n**Уверенность:** {}\n**Источник:** {:?}",
                    Self::format_concrete_type(concrete_type),
                    Self::format_certainty(&type_resolution.certainty),
                    type_resolution.source
                )
            }
            ResolutionResult::Union(union_types) => {
                let types: Vec<String> = union_types
                    .iter()
                    .map(|wt| {
                        format!(
                            "{} ({}%)",
                            Self::format_concrete_type(&wt.type_),
                            (wt.weight * 100.0) as u32
                        )
                    })
                    .collect();

                format!(
                    "**Union тип:** `{}`\n**Варианты:** {}\n**Уверенность:** {}",
                    types.join(" | "),
                    union_types.len(),
                    Self::format_certainty(&type_resolution.certainty)
                )
            }
            _ => format!(
                "**Тип:** {:?}\n**Уверенность:** {}",
                type_resolution.result,
                Self::format_certainty(&type_resolution.certainty)
            ),
        }
    }

    fn format_type_short(type_resolution: &TypeResolution) -> String {
        match &type_resolution.result {
            ResolutionResult::Concrete(concrete_type) => Self::format_concrete_type(concrete_type),
            ResolutionResult::Union(union_types) => {
                if union_types.len() <= 2 {
                    union_types
                        .iter()
                        .map(|wt| Self::format_concrete_type(&wt.type_))
                        .collect::<Vec<_>>()
                        .join(" | ")
                } else {
                    format!("Union<{} типов>", union_types.len())
                }
            }
            _ => format!("{:?}", type_resolution.result),
        }
    }

    fn format_concrete_type(concrete_type: &ConcreteType) -> String {
        match concrete_type {
            ConcreteType::Platform(platform) => platform.name.clone(),
            ConcreteType::Configuration(config) => format!("{:?}.{}", config.kind, config.name),
            ConcreteType::Primitive(primitive) => format!("{:?}", primitive),
            ConcreteType::Special(special) => format!("{:?}", special),
            ConcreteType::GlobalFunction(func) => format!("Функция {}", func.name),
        }
    }

    fn format_certainty(certainty: &bsl_shared::domain::types::Certainty) -> String {
        match certainty {
            bsl_shared::domain::types::Certainty::Known => "100%".to_string(),
            bsl_shared::domain::types::Certainty::Inferred(conf) => {
                format!("{}%", (conf * 100.0) as u32)
            }
            bsl_shared::domain::types::Certainty::Unknown => "неизвестно".to_string(),
        }
    }

    fn uri_to_filename(uri: &str) -> String {
        if let Ok(url) = url::Url::parse(uri) {
            if let Ok(path) = url.to_file_path() {
                return path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("unknown.bsl")
                    .to_string();
            }
        }
        "unknown.bsl".to_string()
    }

    pub fn invalidate_cache(&mut self, uri: &str) {
        self.analysis_cache.remove(uri);
    }

    pub fn get_cache_stats(&self) -> CacheStats {
        CacheStats {
            cached_documents: self.analysis_cache.len(),
            total_variables: self
                .analysis_cache
                .values()
                .map(|(ctx, _)| ctx.symbol_table.len())
                .sum(),
            total_functions: self.analysis_cache.values().map(|(_ctx, _)| 0).sum(),
        }
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub cached_documents: usize,
    pub total_variables: usize,
    pub total_functions: usize,
}

pub struct DiagnosticsConverter;

impl DiagnosticsConverter {
    pub fn convert_diagnostics(diagnostics: &[TypeDiagnostic]) -> Vec<Diagnostic> {
        diagnostics
            .iter()
            .map(|diag| Diagnostic {
                range: Range {
                    start: Position {
                        line: diag.line.saturating_sub(1),
                        character: diag.column,
                    },
                    end: Position {
                        line: diag.line.saturating_sub(1),
                        character: diag.column + 1,
                    },
                },
                severity: Some(Self::convert_severity(&diag.severity)),
                code: None,
                code_description: None,
                source: Some("bsl-gradual-types".to_string()),
                message: diag.message.clone(),
                related_information: None,
                tags: None,
                data: None,
            })
            .collect()
    }

    fn convert_severity(severity: &DiagnosticSeverity) -> tower_lsp::lsp_types::DiagnosticSeverity {
        match severity {
            DiagnosticSeverity::Error => tower_lsp::lsp_types::DiagnosticSeverity::ERROR,
            DiagnosticSeverity::Warning => tower_lsp::lsp_types::DiagnosticSeverity::WARNING,
            DiagnosticSeverity::Info => tower_lsp::lsp_types::DiagnosticSeverity::INFORMATION,
            DiagnosticSeverity::Hint => tower_lsp::lsp_types::DiagnosticSeverity::HINT,
        }
    }
}

pub struct DocumentManager {
    documents: Arc<RwLock<HashMap<String, DocumentState>>>,
    analyzer: Arc<RwLock<EnhancedTypeAnalyzer>>,
}

impl DocumentManager {
    pub fn new(type_resolution_service: std::sync::Arc<TypeResolver>) -> Self {
        Self {
            documents: Arc::new(RwLock::new(HashMap::new())),
            analyzer: Arc::new(RwLock::new(EnhancedTypeAnalyzer::new(
                type_resolution_service,
            ))),
        }
    }

    pub async fn update_document(
        &self,
        uri: String,
        content: String,
        version: i32,
        changes: Vec<TextDocumentContentChangeEvent>,
    ) -> anyhow::Result<Vec<Diagnostic>> {
        {
            let mut docs = self.documents.write().await;
            docs.insert(uri.clone(), DocumentState::new(content.clone(), version));
        }

        let (context, diagnostics) = {
            let mut analyzer = self.analyzer.write().await;
            analyzer.analyze_document(&uri, &content, &changes)?
        };

        {
            let mut docs = self.documents.write().await;
            if let Some(doc_state) = docs.get_mut(&uri) {
                doc_state.type_context = Some(context);
                doc_state.diagnostics = diagnostics.clone();
                doc_state.last_analysis = std::time::Instant::now();
            }
        }

        Ok(DiagnosticsConverter::convert_diagnostics(&diagnostics))
    }

    pub async fn get_type_at_position(
        &self,
        uri: &str,
        position: Position,
    ) -> Option<TypeResolution> {
        let analyzer = self.analyzer.read().await;
        analyzer.get_type_at_position(uri, position)
    }

    pub async fn get_enhanced_hover(&self, uri: &str, position: Position) -> Option<String> {
        self.get_type_at_position(uri, position)
            .await
            .map(|type_resolution| EnhancedTypeAnalyzer::format_type_info(&type_resolution))
    }

    pub async fn get_completions(
        &self,
        uri: &str,
        position: Position,
        prefix: &str,
    ) -> Vec<EnhancedCompletion> {
        let mut analyzer = self.analyzer.write().await;
        let context = CompletionContext {
            uri: uri.to_string(),
            prefix: prefix.to_string(),
        };
        analyzer.get_enhanced_completions("", position, context)
    }

    pub async fn get_stats(&self) -> DocumentManagerStats {
        let docs = self.documents.read().await;
        let analyzer = self.analyzer.read().await;

        DocumentManagerStats {
            total_documents: docs.len(),
            cache_stats: analyzer.get_cache_stats(),
            avg_analysis_time_ms: if docs.is_empty() {
                0.0
            } else {
                docs.values()
                    .map(|doc| doc.last_analysis.elapsed().as_millis() as f64)
                    .sum::<f64>()
                    / docs.len() as f64
            },
        }
    }
}

#[derive(Debug)]
pub struct DocumentManagerStats {
    pub total_documents: usize,
    pub cache_stats: CacheStats,
    pub avg_analysis_time_ms: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_state_creation() {
        let state = DocumentState::new("test content".to_string(), 1);
        assert_eq!(state.content, "test content");
        assert_eq!(state.version, 1);
        assert!(state.needs_reanalysis());
    }

    #[test]
    fn test_incremental_parsing_manager() {
        let mut manager = IncrementalParsingManager::new();
        let simple_code = r#"Процедура Тест() КонецПроцедуры"#;
        let result = manager.parse_incremental("test.bsl", simple_code, &[]);
        assert!(result.is_ok());
        assert!(manager.ast_cache.contains_key("test.bsl"));
    }

    #[tokio::test]
    async fn test_document_manager() {
        let repository =
            std::sync::Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
        let type_resolution_service = std::sync::Arc::new(TypeResolver::new(repository));
        let manager = DocumentManager::new(type_resolution_service);

        let result = manager
            .update_document(
                "test://test.bsl".to_string(),
                "Процедура Тест() КонецПроцедуры".to_string(),
                1,
                vec![],
            )
            .await;

        assert!(result.is_ok());
        let stats = manager.get_stats().await;
        assert_eq!(stats.total_documents, 1);
    }
}
