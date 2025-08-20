//! Presentation Layer - интерфейсы-адаптеры идеальной архитектуры
//! 
//! Слой представления обеспечивает адаптацию между специализированными сервисами
//! и конкретными потребителями (LSP протокол, HTTP API, CLI вывод)

use anyhow::Result;
use std::sync::Arc;
use serde::{Serialize, Deserialize};

use super::application::{LspTypeService, WebTypeService, AnalysisTypeService};
use super::application::{LspCompletion, LspCompletionKind, HoverInfo, PerformanceMonitor};
use super::application::{WebTypeHierarchy, WebSearchResult, WebTypeDetails, SearchFilters};
use super::application::{ProjectAnalysisResult, CoverageReport, TypeDiagnostic, DiagnosticSeverity};

// === LSP INTERFACE ===

/// Интерфейс для LSP сервера
/// 
/// Адаптирует LspTypeService к LSP протоколу
pub struct LspInterface {
    lsp_service: Arc<LspTypeService>,
}

/// LSP запрос автодополнения
#[derive(Debug, Clone, Deserialize)]
pub struct LspCompletionRequest {
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub prefix: String,
    pub trigger_character: Option<String>,
}

/// LSP ответ автодополнения
#[derive(Debug, Clone, Serialize)]
pub struct LspCompletionResponse {
    pub items: Vec<LspCompletionItem>,
    pub is_incomplete: bool,
}

/// Элемент автодополнения для LSP
#[derive(Debug, Clone, Serialize)]
pub struct LspCompletionItem {
    pub label: String,
    pub kind: u8, // LSP CompletionItemKind
    pub detail: Option<String>,
    pub documentation: Option<String>,
    #[serde(rename = "insertText")]
    pub insert_text: String,
    #[serde(rename = "filterText")]
    pub filter_text: Option<String>,
    #[serde(rename = "sortText")]
    pub sort_text: Option<String>,
}

/// LSP запрос hover
#[derive(Debug, Clone, Deserialize)]
pub struct LspHoverRequest {
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub expression: String,
}

/// LSP ответ hover
#[derive(Debug, Clone, Serialize)]
pub struct LspHoverResponse {
    pub contents: Vec<String>,
    pub range: Option<LspRange>,
}

/// LSP диапазон в файле
#[derive(Debug, Clone, Serialize)]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

/// LSP позиция в файле
#[derive(Debug, Clone, Serialize)]
pub struct LspPosition {
    pub line: u32,
    pub character: u32,
}

impl LspInterface {
    /// Создать новый LSP интерфейс
    pub fn new(lsp_service: Arc<LspTypeService>) -> Self {
        Self { lsp_service }
    }
    
    /// Обработать запрос автодополнения
    pub async fn handle_completion_request(&self, request: LspCompletionRequest) -> Result<LspCompletionResponse> {
        println!("🔍 LSP автодополнение: '{}' в {}:{}:{}", 
                request.prefix, request.file_path, request.line, request.column);
        
        // Получаем автодополнение от LSP сервиса
        let lsp_completions = self.lsp_service.get_completions_fast(
            &request.prefix, 
            &request.file_path, 
            request.line, 
            request.column
        ).await;
        
        // Конвертируем в LSP протокол формат
        let lsp_items = lsp_completions.into_iter()
            .map(|comp| LspCompletionItem {
                label: comp.label.clone(),
                kind: comp.kind as u8,
                detail: comp.detail,
                documentation: comp.documentation,
                insert_text: comp.insert_text,
                filter_text: comp.filter_text,
                sort_text: comp.sort_text,
            })
            .collect();
        
        Ok(LspCompletionResponse {
            items: lsp_items,
            is_incomplete: false, // TODO: реализовать пагинацию
        })
    }
    
    /// Обработать запрос hover
    pub async fn handle_hover_request(&self, request: LspHoverRequest) -> Result<Option<LspHoverResponse>> {
        // Получаем hover информацию
        if let Some(hover_info) = self.lsp_service.get_hover_info(
            &request.expression,
            &request.file_path,
            request.line,
            request.column
        ).await {
            Ok(Some(LspHoverResponse {
                contents: vec![hover_info.content, hover_info.type_info],
                range: Some(LspRange {
                    start: LspPosition { line: request.line, character: request.column },
                    end: LspPosition { line: request.line, character: request.column + request.expression.len() as u32 },
                }),
            }))
        } else {
            Ok(None)
        }
    }
    
    /// Получить метрики производительности LSP
    pub async fn get_performance_metrics(&self) -> Result<LspPerformanceMetrics> {
        let metrics = self.lsp_service.get_performance_metrics().await;
        
        Ok(LspPerformanceMetrics {
            total_requests: metrics.total_requests,
            average_response_time_ms: metrics.average_response_time_ms,
            slow_requests: metrics.slow_requests,
            cache_hit_rate: metrics.cache_hit_rate,
        })
    }
}

/// Метрики производительности для LSP
#[derive(Debug, Clone, Serialize)]
pub struct LspPerformanceMetrics {
    pub total_requests: u64,
    pub average_response_time_ms: f64,
    pub slow_requests: u64,
    pub cache_hit_rate: f64,
}

// === WEB INTERFACE ===

/// Интерфейс для веб-сервера
/// 
/// Адаптирует WebTypeService к HTTP API
pub struct WebInterface {
    web_service: Arc<WebTypeService>,
}

/// HTTP запрос поиска
#[derive(Debug, Clone, Deserialize)]
pub struct WebSearchRequest {
    pub query: String,
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub filters: Option<WebSearchFilters>,
}

/// Фильтры поиска для веб
#[derive(Debug, Clone, Deserialize, Default)]
pub struct WebSearchFilters {
    pub source: Option<String>, // "platform" | "configuration" | "user"
    pub category: Option<String>,
    pub has_methods: Option<bool>,
    pub has_properties: Option<bool>,
}

/// HTTP ответ поиска
#[derive(Debug, Clone, Serialize)]
pub struct WebSearchResponse {
    pub results: Vec<WebSearchResultItem>,
    pub total_count: usize,
    pub page: usize,
    pub per_page: usize,
    pub total_pages: usize,
}

/// Элемент результата поиска для веб
#[derive(Debug, Clone, Serialize)]
pub struct WebSearchResultItem {
    pub name: String,
    pub category: String,
    pub description: String,
    pub relevance_score: f32,
    pub url: String,
    pub tags: Vec<String>,
}

/// HTTP ответ иерархии
#[derive(Debug, Clone, Serialize)]
pub struct WebHierarchyResponse {
    pub categories: Vec<WebCategoryItem>,
    pub total_types: usize,
    pub statistics: WebHierarchyStatsResponse,
}

/// Элемент категории для веб
#[derive(Debug, Clone, Serialize)]
pub struct WebCategoryItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub types_count: usize,
    pub subcategories_count: usize,
    pub url: String,
}

/// Статистика иерархии для веб API
#[derive(Debug, Clone, Serialize)]
pub struct WebHierarchyStatsResponse {
    pub total_categories: usize,
    pub total_types: usize,
    pub platform_types: usize,
    pub configuration_types: usize,
}

impl WebInterface {
    /// Создать новый веб-интерфейс
    pub fn new(web_service: Arc<WebTypeService>) -> Self {
        Self { web_service }
    }
    
    /// Обработать запрос иерархии типов
    pub async fn handle_hierarchy_request(&self) -> Result<WebHierarchyResponse> {
        println!("🌳 Веб-запрос иерархии типов");
        
        let hierarchy = self.web_service.build_type_hierarchy().await?;
        
        // Конвертируем в HTTP API формат
        let categories = hierarchy.categories.into_iter()
            .map(|cat| WebCategoryItem {
                id: cat.id.clone(),
                name: cat.name.clone(),
                description: cat.description,
                types_count: cat.types.len(),
                subcategories_count: cat.subcategories.len(),
                url: format!("/categories/{}", urlencoding::encode(&cat.id)),
            })
            .collect();
        
        Ok(WebHierarchyResponse {
            categories,
            total_types: hierarchy.total_types,
            statistics: WebHierarchyStatsResponse {
                total_categories: hierarchy.statistics.total_categories,
                total_types: hierarchy.statistics.total_types,
                platform_types: hierarchy.statistics.platform_types,
                configuration_types: hierarchy.statistics.configuration_types,
            },
        })
    }
    
    /// Обработать запрос поиска
    pub async fn handle_search_request(&self, request: WebSearchRequest) -> Result<WebSearchResponse> {
        println!("🔍 Веб-поиск: '{}'", request.query);
        
        // Конвертируем веб-фильтры в внутренний формат
        let search_filters = self.convert_web_filters(request.filters.unwrap_or_default());
        
        // Выполняем поиск
        let search_results = self.web_service.advanced_search(&request.query, search_filters).await?;
        
        // Пагинация
        let page = request.page.unwrap_or(1);
        let per_page = request.per_page.unwrap_or(20).min(100);
        let total_count = search_results.len();
        let total_pages = (total_count + per_page - 1) / per_page;
        
        let start = (page - 1) * per_page;
        let end = (start + per_page).min(total_count);
        let page_results = if start < total_count {
            search_results[start..end].to_vec()
        } else {
            Vec::new()
        };
        
        // Конвертируем в HTTP API формат
        let result_items = page_results.into_iter()
            .map(|result| WebSearchResultItem {
                name: result.type_name.clone(),
                category: result.category,
                description: result.description,
                relevance_score: result.relevance_score,
                url: result.url,
                tags: vec![], // TODO: получить теги
            })
            .collect();
        
        Ok(WebSearchResponse {
            results: result_items,
            total_count,
            page,
            per_page,
            total_pages,
        })
    }
    
    /// Обработать запрос деталей типа
    pub async fn handle_type_details_request(&self, type_name: &str) -> Result<WebTypeDetailsResponse> {
        println!("📋 Веб-запрос деталей типа: '{}'", type_name);
        
        let details = self.web_service.get_type_details(type_name).await?;
        
        Ok(WebTypeDetailsResponse {
            name: details.basic_info.name,
            description: details.basic_info.description,
            methods: details.methods.into_iter().map(|m| WebMethodResponse {
                name: m.name,
                description: m.description,
                parameters: m.parameters.into_iter().map(|p| WebParameterResponse {
                    name: p.name,
                    type_name: p.type_name,
                    is_optional: p.is_optional,
                    description: p.description,
                }).collect(),
                return_type: m.return_type,
                examples: m.examples,
            }).collect(),
            properties: details.properties.into_iter().map(|p| WebPropertyResponse {
                name: p.name,
                type_name: p.type_name,
                is_readonly: p.is_readonly,
                description: p.description,
            }).collect(),
            related_types: details.related_types,
        })
    }
    
    fn convert_web_filters(&self, web_filters: WebSearchFilters) -> SearchFilters {
        use super::data::TypeSource;
        
        let source = web_filters.source.and_then(|s| match s.as_str() {
            "platform" => Some(TypeSource::Platform { version: "8.3".to_string() }),
            "configuration" => Some(TypeSource::Configuration { config_version: "8.3".to_string() }),
            "user" => Some(TypeSource::UserDefined { file_path: "".to_string() }),
            _ => None,
        });
        
        SearchFilters {
            source,
            category: web_filters.category,
            has_methods: web_filters.has_methods,
            has_properties: web_filters.has_properties,
            facets: Vec::new(), // TODO: конвертировать фасеты
        }
    }
}

/// Ответ деталей типа для веб
#[derive(Debug, Clone, Serialize)]
pub struct WebTypeDetailsResponse {
    pub name: String,
    pub description: String,
    pub methods: Vec<WebMethodResponse>,
    pub properties: Vec<WebPropertyResponse>,
    pub related_types: Vec<String>,
}

/// Метод в ответе веб API
#[derive(Debug, Clone, Serialize)]
pub struct WebMethodResponse {
    pub name: String,
    pub description: String,
    pub parameters: Vec<WebParameterResponse>,
    pub return_type: Option<String>,
    pub examples: Vec<String>,
}

/// Параметр в ответе веб API
#[derive(Debug, Clone, Serialize)]
pub struct WebParameterResponse {
    pub name: String,
    pub type_name: String,
    pub is_optional: bool,
    pub description: String,
}

/// Свойство в ответе веб API
#[derive(Debug, Clone, Serialize)]
pub struct WebPropertyResponse {
    pub name: String,
    pub type_name: String,
    pub is_readonly: bool,
    pub description: String,
}

// === CLI INTERFACE ===

/// Интерфейс для CLI инструментов
/// 
/// Адаптирует AnalysisTypeService к CLI выводу
pub struct CliInterface {
    analysis_service: Arc<AnalysisTypeService>,
}

/// CLI запрос анализа проекта
#[derive(Debug, Clone)]
pub struct CliAnalysisRequest {
    pub project_path: std::path::PathBuf,
    pub output_format: CliOutputFormat,
    pub include_coverage: bool,
    pub include_errors: bool,
    pub verbose: bool,
}

/// Форматы вывода CLI
#[derive(Debug, Clone, PartialEq)]
pub enum CliOutputFormat {
    Text,
    Json,
    Csv,
    Html,
}

/// CLI ответ анализа
#[derive(Debug, Clone)]
pub struct CliAnalysisResponse {
    pub summary: CliAnalysisSummary,
    pub coverage: Option<CliCoverageReport>,
    pub errors: Vec<CliTypeError>,
    pub formatted_output: String,
}

/// Сводка анализа для CLI
#[derive(Debug, Clone, Serialize)]
pub struct CliAnalysisSummary {
    pub project_path: String,
    pub total_files: usize,
    pub analyzed_files: usize,
    pub total_functions: usize,
    pub total_variables: usize,
    pub error_count: usize,
    pub analysis_time_seconds: f64,
}

/// Отчёт покрытия для CLI
#[derive(Debug, Clone)]
pub struct CliCoverageReport {
    pub total_expressions: usize,
    pub typed_expressions: usize,
    pub coverage_percentage: f32,
    pub top_uncovered_files: Vec<String>,
}

/// Ошибка типа для CLI
#[derive(Debug, Clone)]
pub struct CliTypeError {
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub severity: String,
    pub message: String,
    pub suggested_fix: Option<String>,
}

impl CliInterface {
    /// Создать новый CLI интерфейс
    pub fn new(analysis_service: Arc<AnalysisTypeService>) -> Self {
        Self { analysis_service }
    }
    
    /// Обработать запрос анализа проекта
    pub async fn handle_analysis_request(&self, request: CliAnalysisRequest) -> Result<CliAnalysisResponse> {
        println!("🔍 CLI анализ проекта: {}", request.project_path.display());
        
        // Выполняем анализ проекта
        let analysis_result = self.analysis_service.analyze_project(&request.project_path).await?;
        
        // Создаём сводку
        let summary = CliAnalysisSummary {
            project_path: analysis_result.project_path.clone(),
            total_files: analysis_result.total_files,
            analyzed_files: analysis_result.analyzed_files,
            total_functions: analysis_result.total_functions,
            total_variables: analysis_result.total_variables,
            error_count: analysis_result.type_errors.len(),
            analysis_time_seconds: analysis_result.analysis_time.as_secs_f64(),
        };
        
        // Конвертируем покрытие если запрошено
        let coverage = if request.include_coverage {
            Some(CliCoverageReport {
                total_expressions: analysis_result.coverage_report.total_expressions,
                typed_expressions: analysis_result.coverage_report.typed_expressions,
                coverage_percentage: analysis_result.coverage_report.coverage_percentage,
                top_uncovered_files: Vec::new(), // TODO: найти файлы с низким покрытием
            })
        } else {
            None
        };
        
        // Конвертируем ошибки если запрошены
        let errors = if request.include_errors {
            analysis_result.type_errors.into_iter()
                .map(|err| CliTypeError {
                    file_path: err.file_path,
                    line: err.line,
                    column: err.column,
                    severity: format!("{:?}", err.severity),
                    message: err.message,
                    suggested_fix: err.suggested_fix,
                })
                .collect()
        } else {
            Vec::new()
        };
        
        // Форматируем вывод
        let formatted_output = self.format_analysis_output(&summary, &coverage, &errors, &request.output_format);
        
        Ok(CliAnalysisResponse {
            summary,
            coverage,
            errors,
            formatted_output,
        })
    }
    
    /// Экспортировать отчёты в файлы
    pub async fn export_reports(&self, analysis: &CliAnalysisResponse, output_dir: &std::path::Path) -> Result<Vec<String>> {
        let mut exported_files = Vec::new();
        
        // Экспорт JSON отчёта
        let json_path = output_dir.join("analysis_report.json");
        let json_content = serde_json::to_string_pretty(&analysis.summary)?;
        std::fs::write(&json_path, json_content)?;
        exported_files.push(json_path.to_string_lossy().to_string());
        
        // Экспорт HTML отчёта (если есть покрытие)
        if let Some(coverage) = &analysis.coverage {
            let html_path = output_dir.join("coverage_report.html");
            let html_content = self.generate_html_report(&analysis.summary, coverage, &analysis.errors);
            std::fs::write(&html_path, html_content)?;
            exported_files.push(html_path.to_string_lossy().to_string());
        }
        
        println!("✅ Отчёты экспортированы: {} файлов", exported_files.len());
        Ok(exported_files)
    }
    
    // === ПРИВАТНЫЕ МЕТОДЫ ===
    
    fn format_analysis_output(&self, summary: &CliAnalysisSummary, coverage: &Option<CliCoverageReport>, errors: &[CliTypeError], format: &CliOutputFormat) -> String {
        match format {
            CliOutputFormat::Text => self.format_text_output(summary, coverage, errors),
            CliOutputFormat::Json => serde_json::to_string_pretty(summary).unwrap_or_else(|_| "{}".to_string()),
            CliOutputFormat::Csv => self.format_csv_output(summary, errors),
            CliOutputFormat::Html => self.format_html_output(summary, coverage, errors),
        }
    }
    
    fn format_text_output(&self, summary: &CliAnalysisSummary, coverage: &Option<CliCoverageReport>, errors: &[CliTypeError]) -> String {
        let mut output = String::new();
        
        output.push_str("📊 Анализ проекта BSL\n");
        output.push_str("===================\n\n");
        output.push_str(&format!("📁 Проект: {}\n", summary.project_path));
        output.push_str(&format!("📄 Файлов: {} (проанализировано: {})\n", summary.total_files, summary.analyzed_files));
        output.push_str(&format!("🔧 Функций: {}\n", summary.total_functions));
        output.push_str(&format!("📦 Переменных: {}\n", summary.total_variables));
        output.push_str(&format!("⚠️ Ошибок: {}\n", summary.error_count));
        output.push_str(&format!("⏱️ Время анализа: {:.2}с\n\n", summary.analysis_time_seconds));
        
        if let Some(cov) = coverage {
            output.push_str("📈 Покрытие типизации:\n");
            output.push_str(&format!("   Выражений: {} / {} ({:.1}%)\n\n", 
                           cov.typed_expressions, cov.total_expressions, cov.coverage_percentage));
        }
        
        if !errors.is_empty() {
            output.push_str("🚨 Ошибки типов:\n");
            for (i, error) in errors.iter().take(5).enumerate() {
                output.push_str(&format!("   {}. {}:{}:{} [{}] {}\n", 
                               i + 1, error.file_path, error.line, error.column, error.severity, error.message));
            }
            if errors.len() > 5 {
                output.push_str(&format!("   ... и ещё {} ошибок\n", errors.len() - 5));
            }
        }
        
        output
    }
    
    fn format_csv_output(&self, _summary: &CliAnalysisSummary, _errors: &[CliTypeError]) -> String {
        // TODO: Реализовать CSV формат
        "file,line,column,severity,message\n".to_string()
    }
    
    fn format_html_output(&self, _summary: &CliAnalysisSummary, _coverage: &Option<CliCoverageReport>, _errors: &[CliTypeError]) -> String {
        // TODO: Реализовать HTML формат
        "<html><body><h1>Анализ проекта BSL</h1></body></html>".to_string()
    }
    
    fn generate_html_report(&self, _summary: &CliAnalysisSummary, _coverage: &CliCoverageReport, _errors: &[CliTypeError]) -> String {
        // TODO: Реализовать генерацию HTML отчёта
        "<html><body><h1>Отчёт покрытия типизации</h1></body></html>".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ideal::data::InMemoryTypeRepository;
    use crate::ideal::domain::TypeResolutionService;
    
    #[tokio::test]
    async fn test_lsp_interface() {
        let repo = Arc::new(InMemoryTypeRepository::new());
        let resolution_service = Arc::new(TypeResolutionService::new(repo));
        let lsp_service = Arc::new(LspTypeService::new(resolution_service));
        
        let lsp_interface = LspInterface::new(lsp_service);
        
        // Тестируем автодополнение
        let completion_request = LspCompletionRequest {
            file_path: "test.bsl".to_string(),
            line: 10,
            column: 5,
            prefix: "Стр".to_string(),
            trigger_character: None,
        };
        
        let response = lsp_interface.handle_completion_request(completion_request).await.unwrap();
        // В тестовом окружении будет пустой список
        
        println!("✅ LspInterface работает");
    }
    
    #[tokio::test]
    async fn test_web_interface() {
        let repo = Arc::new(InMemoryTypeRepository::new());
        let resolution_service = Arc::new(TypeResolutionService::new(repo));
        let web_service = Arc::new(WebTypeService::new(resolution_service));
        
        let web_interface = WebInterface::new(web_service);
        
        // Тестируем иерархию
        let hierarchy = web_interface.handle_hierarchy_request().await.unwrap();
        
        // Тестируем поиск
        let search_request = WebSearchRequest {
            query: "массив".to_string(),
            page: Some(1),
            per_page: Some(10),
            filters: None,
        };
        
        let search_response = web_interface.handle_search_request(search_request).await.unwrap();
        
        println!("✅ WebInterface работает");
    }
    
    #[tokio::test]
    async fn test_cli_interface() {
        let repo = Arc::new(InMemoryTypeRepository::new());
        let resolution_service = Arc::new(TypeResolutionService::new(repo));
        let analysis_service = Arc::new(AnalysisTypeService::new(resolution_service));
        
        let cli_interface = CliInterface::new(analysis_service);
        
        // Тестируем анализ проекта
        let analysis_request = CliAnalysisRequest {
            project_path: std::path::PathBuf::from("test_project"),
            output_format: CliOutputFormat::Text,
            include_coverage: true,
            include_errors: true,
            verbose: false,
        };
        
        let response = cli_interface.handle_analysis_request(analysis_request).await.unwrap();
        assert!(!response.formatted_output.is_empty());
        
        println!("✅ CliInterface работает");
    }
}