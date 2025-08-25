//! Улучшенный LSP сервер с интеграцией продвинутых анализаторов типов
//!
//! Этот модуль предоставляет расширенную функциональность LSP сервера:
//! - Инкрементальный парсинг с tree-sitter
//! - Интеграция flow-sensitive анализа
//! - Union типы в hover и completion
//! - Межпроцедурный анализ для лучшего автодополнения

use crate::core::platform_resolver::{
    CompletionItem as BslCompletion, CompletionKind, PlatformTypeResolver,
};
use crate::core::type_checker::{TypeChecker, TypeContext, TypeDiagnostic};
use crate::domain::types::{ConcreteType, ResolutionResult, TypeResolution};
use crate::parsing::bsl::ast::Program;
use crate::parsing::bsl::common::{Parser, ParserFactory, TextChange};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::lsp_types::*;

/// Состояние документа с кешированными результатами анализа
#[derive(Debug, Clone)]
pub struct DocumentState {
    /// Текущий текст документа
    pub content: String,
    /// Кешированная AST
    pub ast: Option<Program>,
    /// Версия документа
    pub version: i32,
    /// Результаты последнего анализа типов
    pub type_context: Option<TypeContext>,
    /// Кешированные диагностики
    pub diagnostics: Vec<TypeDiagnostic>,
    /// Время последнего анализа
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

    /// Проверить нужен ли повторный анализ
    pub fn needs_reanalysis(&self) -> bool {
        self.ast.is_none()
            || self.type_context.is_none()
            || self.last_analysis.elapsed() > std::time::Duration::from_secs(5)
    }
}

/// Менеджер инкрементального парсинга
pub struct IncrementalParsingManager {
    /// Парсер с поддержкой инкрементального парсинга
    parser: Box<dyn Parser>,
    /// Кеш AST для документов
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

    /// Инкрементально парсить документ
    pub fn parse_incremental(
        &mut self,
        uri: &str,
        new_content: &str,
        changes: &[TextDocumentContentChangeEvent],
    ) -> anyhow::Result<Program> {
        // Конвертируем LSP изменения в наш формат
        let text_changes: Vec<TextChange> = changes
            .iter()
            .filter_map(|change| {
                change.range.map(|range| TextChange {
                    start_byte: self.position_to_byte_offset(new_content, range.start),
                    old_end_byte: self.position_to_byte_offset(new_content, range.end),
                    new_end_byte: self.position_to_byte_offset(new_content, range.end),
                    start_position: crate::parser::common::Position {
                        row: range.start.line as usize,
                        column: range.start.character as usize,
                    },
                    old_end_position: crate::parser::common::Position {
                        row: range.end.line as usize,
                        column: range.end.character as usize,
                    },
                    new_end_position: crate::parser::common::Position {
                        row: range.end.line as usize,
                        column: range.end.character as usize,
                    },
                })
            })
            .collect();

        // Пытаемся инкрементальный парсинг
        let program = if !text_changes.is_empty() && self.ast_cache.contains_key(uri) {
            match self.parser.parse_incremental(new_content, &text_changes) {
                Ok(ast) => ast,
                Err(_) => {
                    // Fallback к полному парсингу
                    self.parser.parse(new_content)?
                }
            }
        } else {
            // Полный парсинг для новых файлов
            self.parser.parse(new_content)?
        };

        // Обновляем кеш
        self.ast_cache.insert(uri.to_string(), program.clone());

        Ok(program)
    }

    /// Конвертировать LSP Position в байтовый офсет
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

/// Улучшенный анализатор типов для LSP
pub struct EnhancedTypeAnalyzer {
    /// Менеджер инкрементального парсинга
    parsing_manager: IncrementalParsingManager,
    /// Кеш результатов анализа для быстрого доступа
    analysis_cache: HashMap<String, (TypeContext, Vec<TypeDiagnostic>)>,
}

impl Default for EnhancedTypeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl EnhancedTypeAnalyzer {
    pub fn new() -> Self {
        Self {
            parsing_manager: IncrementalParsingManager::new(),
            analysis_cache: HashMap::new(),
        }
    }

    /// Проанализировать документ с использованием всех продвинутых анализаторов
    pub fn analyze_document(
        &mut self,
        uri: &str,
        content: &str,
        changes: &[TextDocumentContentChangeEvent],
    ) -> anyhow::Result<(TypeContext, Vec<TypeDiagnostic>)> {
        // Инкрементально парсим документ
        let program = self
            .parsing_manager
            .parse_incremental(uri, content, changes)?;

        // Создаем type checker с улучшенными анализаторами
        let file_name = Self::uri_to_filename(uri);
        let type_checker = TypeChecker::new(file_name);

        // Проводим полный анализ с flow-sensitive, union types и межпроцедурным анализом
        let (context, diagnostics) = type_checker.check(&program);

        // Кешируем результат
        self.analysis_cache
            .insert(uri.to_string(), (context.clone(), diagnostics.clone()));

        Ok((context, diagnostics))
    }

    /// Получить тип переменной в конкретной позиции
    pub fn get_type_at_position(&self, uri: &str, _position: Position) -> Option<TypeResolution> {
        let (context, _) = self.analysis_cache.get(uri)?;

        // TODO: Более сложная логика для определения переменной по позиции
        // Пока возвращаем первую найденную переменную для демонстрации
        context.variables.values().next().cloned()
    }

    /// Получить автодополнения с учетом типов из продвинутого анализа
    pub fn get_enhanced_completions(
        &self,
        uri: &str,
        _position: Position,
        prefix: &str,
        platform_resolver: &PlatformTypeResolver,
    ) -> Vec<BslCompletion> {
        let mut completions = Vec::new();

        // Добавляем стандартные completion из platform resolver
        completions.extend(platform_resolver.get_completions(prefix));

        // Добавляем completion на основе локального контекста типов
        if let Some((context, _)) = self.analysis_cache.get(uri) {
            // Переменные из flow-sensitive анализа
            for (var_name, var_type) in &context.variables {
                if var_name.starts_with(prefix) {
                    completions.push(BslCompletion {
                        label: var_name.clone(),
                        kind: CompletionKind::Variable,
                        detail: Some(Self::format_type_short(var_type)),
                        documentation: Some(Self::format_type_info(var_type)),
                    });
                }
            }

            // Функции из межпроцедурного анализа
            for (func_name, signature) in &context.functions {
                if func_name.starts_with(prefix) {
                    let doc = format!(
                        "Функция: {} -> {}\nПараметры: {}",
                        func_name,
                        Self::format_type_short(&signature.return_type),
                        signature
                            .params
                            .iter()
                            .map(|(name, type_)| format!(
                                "{}: {}",
                                name,
                                Self::format_type_short(type_)
                            ))
                            .collect::<Vec<_>>()
                            .join(", ")
                    );

                    completions.push(BslCompletion {
                        label: func_name.clone(),
                        kind: CompletionKind::Function,
                        detail: Some(Self::format_type_short(&signature.return_type)),
                        documentation: Some(doc),
                    });
                }
            }
        }

        completions
    }

    /// Получить диагностики из кеша
    pub fn get_cached_diagnostics(&self, uri: &str) -> Vec<TypeDiagnostic> {
        self.analysis_cache
            .get(uri)
            .map(|(_, diagnostics)| diagnostics.clone())
            .unwrap_or_default()
    }

    /// Форматировать информацию о типе для hover
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
            ResolutionResult::Dynamic => {
                format!(
                    "**Динамический тип**\n**Уверенность:** {}\n**Источник:** {:?}",
                    Self::format_certainty(&type_resolution.certainty),
                    type_resolution.source
                )
            }
            ResolutionResult::Conditional(_) => {
                format!(
                    "**Условный тип**\n**Уверенность:** {}\n**Источник:** {:?}",
                    Self::format_certainty(&type_resolution.certainty),
                    type_resolution.source
                )
            }
            ResolutionResult::Contextual(_) => {
                format!(
                    "**Контекстный тип**\n**Уверенность:** {}\n**Источник:** {:?}",
                    Self::format_certainty(&type_resolution.certainty),
                    type_resolution.source
                )
            }
        }
    }

    /// Короткое форматирование типа для completion
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
            ResolutionResult::Dynamic => "Dynamic".to_string(),
            ResolutionResult::Conditional(_) => "Conditional".to_string(),
            ResolutionResult::Contextual(_) => "Contextual".to_string(),
        }
    }

    /// Форматировать конкретный тип
    fn format_concrete_type(concrete_type: &ConcreteType) -> String {
        match concrete_type {
            ConcreteType::Platform(platform) => platform.name.clone(),
            ConcreteType::Configuration(config) => format!("{:?}.{}", config.kind, config.name),
            ConcreteType::Primitive(primitive) => format!("{:?}", primitive),
            ConcreteType::Special(special) => format!("{:?}", special),
            ConcreteType::GlobalFunction(func) => format!("Функция {}", func.name),
        }
    }

    /// Форматировать уровень уверенности
    fn format_certainty(certainty: &crate::core::types::Certainty) -> String {
        match certainty {
            crate::core::types::Certainty::Known => "100%".to_string(),
            crate::core::types::Certainty::Inferred(conf) => format!("{}%", (conf * 100.0) as u32),
            crate::core::types::Certainty::Unknown => "неизвестно".to_string(),
        }
    }

    /// Конвертировать URI в имя файла
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

    /// Инвалидировать кеш для URI
    pub fn invalidate_cache(&mut self, uri: &str) {
        self.analysis_cache.remove(uri);
    }

    /// Получить статистику кеша
    pub fn get_cache_stats(&self) -> CacheStats {
        CacheStats {
            cached_documents: self.analysis_cache.len(),
            total_variables: self
                .analysis_cache
                .values()
                .map(|(ctx, _)| ctx.variables.len())
                .sum(),
            total_functions: self
                .analysis_cache
                .values()
                .map(|(ctx, _)| ctx.functions.len())
                .sum(),
        }
    }
}

/// Статистика кеша анализатора
#[derive(Debug)]
pub struct CacheStats {
    pub cached_documents: usize,
    pub total_variables: usize,
    pub total_functions: usize,
}

/// Конвертер диагностик в LSP формат
pub struct DiagnosticsConverter;

impl DiagnosticsConverter {
    /// Конвертировать наши диагностики в LSP формат
    pub fn convert_diagnostics(diagnostics: &[TypeDiagnostic]) -> Vec<Diagnostic> {
        diagnostics
            .iter()
            .map(|diag| {
                Diagnostic {
                    range: Range {
                        start: Position {
                            line: (diag.line.saturating_sub(1)) as u32,
                            character: diag.column as u32,
                        },
                        end: Position {
                            line: (diag.line.saturating_sub(1)) as u32,
                            character: (diag.column + 10) as u32, // Примерная длина
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
                }
            })
            .collect()
    }

    /// Конвертировать уровень серьезности
    fn convert_severity(
        severity: &crate::core::type_checker::DiagnosticSeverity,
    ) -> DiagnosticSeverity {
        match severity {
            crate::core::type_checker::DiagnosticSeverity::Error => DiagnosticSeverity::ERROR,
            crate::core::type_checker::DiagnosticSeverity::Warning => DiagnosticSeverity::WARNING,
            crate::core::type_checker::DiagnosticSeverity::Info => DiagnosticSeverity::INFORMATION,
            crate::core::type_checker::DiagnosticSeverity::Hint => DiagnosticSeverity::HINT,
        }
    }
}

/// Менеджер состояния документов с оптимизацией
pub struct DocumentManager {
    /// Состояния документов
    documents: Arc<RwLock<HashMap<String, DocumentState>>>,
    /// Анализатор типов
    analyzer: Arc<RwLock<EnhancedTypeAnalyzer>>,
}

impl Default for DocumentManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentManager {
    pub fn new() -> Self {
        Self {
            documents: Arc::new(RwLock::new(HashMap::new())),
            analyzer: Arc::new(RwLock::new(EnhancedTypeAnalyzer::new())),
        }
    }

    /// Обновить документ с инкрементальным анализом
    pub async fn update_document(
        &self,
        uri: String,
        content: String,
        version: i32,
        changes: Vec<TextDocumentContentChangeEvent>,
    ) -> anyhow::Result<Vec<Diagnostic>> {
        // Обновляем состояние документа
        {
            let mut docs = self.documents.write().await;
            docs.insert(uri.clone(), DocumentState::new(content.clone(), version));
        }

        // Проводим анализ
        let (context, diagnostics) = {
            let mut analyzer = self.analyzer.write().await;
            analyzer.analyze_document(&uri, &content, &changes)?
        };

        // Обновляем кешированные результаты
        {
            let mut docs = self.documents.write().await;
            if let Some(doc_state) = docs.get_mut(&uri) {
                doc_state.type_context = Some(context);
                doc_state.diagnostics = diagnostics.clone();
                doc_state.last_analysis = std::time::Instant::now();
            }
        }

        // Конвертируем в LSP диагностики
        Ok(DiagnosticsConverter::convert_diagnostics(&diagnostics))
    }

    /// Получить тип в позиции
    pub async fn get_type_at_position(
        &self,
        uri: &str,
        position: Position,
    ) -> Option<TypeResolution> {
        let analyzer = self.analyzer.read().await;
        analyzer.get_type_at_position(uri, position)
    }

    /// Получить hover информацию с типами
    pub async fn get_enhanced_hover(&self, uri: &str, position: Position) -> Option<String> {
        self.get_type_at_position(uri, position)
            .await
            .map(|type_resolution| EnhancedTypeAnalyzer::format_type_info(&type_resolution))
    }

    /// Получить улучшенные completion
    pub async fn get_completions(
        &self,
        uri: &str,
        position: Position,
        prefix: &str,
        platform_resolver: &PlatformTypeResolver,
    ) -> Vec<BslCompletion> {
        let analyzer = self.analyzer.read().await;
        analyzer.get_enhanced_completions(uri, position, prefix, platform_resolver)
    }

    /// Получить статистику для мониторинга
    pub async fn get_stats(&self) -> DocumentManagerStats {
        let docs = self.documents.read().await;
        let analyzer = self.analyzer.read().await;

        DocumentManagerStats {
            total_documents: docs.len(),
            cache_stats: analyzer.get_cache_stats(),
            avg_analysis_time_ms: docs
                .values()
                .map(|doc| doc.last_analysis.elapsed().as_millis() as f64)
                .sum::<f64>()
                / docs.len() as f64,
        }
    }
}

/// Статистика менеджера документов
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

        // Проверяем что AST кешируется
        assert!(manager.ast_cache.contains_key("test.bsl"));
    }

    #[test]
    fn test_type_formatting() {
        let string_type =
            crate::core::standard_types::primitive_type(crate::core::types::PrimitiveType::String);

        let formatted = EnhancedTypeAnalyzer::format_type_short(&string_type);
        assert_eq!(formatted, "String");
    }

    #[tokio::test]
    async fn test_document_manager() {
        let manager = DocumentManager::new();

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
