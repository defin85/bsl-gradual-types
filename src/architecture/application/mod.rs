//! Application Layer - специализированные сервисы идеальной архитектуры
//!
//! Слой приложений обеспечивает специфичную логику для разных потребителей:
//! - LspTypeService: оптимизирован для LSP (скорость <10ms)
//! - WebTypeService: оптимизирован для веб-интерфейса (богатые данные)
//! - AnalysisTypeService: оптимизирован для анализа проектов

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use super::domain::{
    CompletionItem, CompletionKind, RawTypeDataForResult, TypeCheckerService, TypeContext,
    TypeResolutionService, TypeSearchResult, TypeSourceStub,
};
use crate::domain::types::{FacetKind, TypeResolution};

// === LSP TYPE SERVICE ===

/// Сервис типов для LSP (оптимизирован для скорости)
pub struct LspTypeService {
    /// Центральный сервис разрешения
    resolution_service: Arc<TypeResolutionService>,

    /// LSP-специфичный кеш (быстрые операции)
    lsp_cache: Arc<RwLock<LspCache>>,

    /// Монитор производительности
    performance_monitor: Arc<RwLock<PerformanceMonitor>>,
}

/// LSP кеш для быстрых операций
#[derive(Debug, Default)]
pub struct LspCache {
    /// Кеш hover информации
    hover_cache: HashMap<String, HoverInfo>,

    /// Кеш автодополнений
    completion_cache: HashMap<String, Vec<LspCompletion>>,

    /// Кеш разрешений типов в позициях
    position_cache: HashMap<PositionKey, TypeResolution>,
}

/// Ключ для кеша позиций
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct PositionKey {
    pub file_path: String,
    pub line: u32,
    pub column: u32,
}

/// Информация для hover в LSP
#[derive(Debug, Clone)]
pub struct HoverInfo {
    pub content: String,
    pub type_info: String,
    pub documentation: Option<String>,
    pub examples: Vec<String>,
}

/// LSP автодополнение (оптимизированное)
#[derive(Debug, Clone)]
pub struct LspCompletion {
    pub label: String,
    pub kind: LspCompletionKind,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: String,
    pub filter_text: Option<String>,
    pub sort_text: Option<String>,
}

/// Типы автодополнения для LSP
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum LspCompletionKind {
    Text = 1,
    Method = 2,
    Function = 3,
    Constructor = 4,
    Field = 5,
    Variable = 6,
    Class = 7,
    Interface = 8,
    Module = 9,
    Property = 10,
    Unit = 11,
    Value = 12,
    Enum = 13,
    Keyword = 14,
    Snippet = 15,
    Color = 16,
    File = 17,
    Reference = 18,
}

/// Монитор производительности LSP операций
#[derive(Debug, Default, Clone)]
pub struct PerformanceMonitor {
    pub total_requests: u64,
    pub average_response_time_ms: f64,
    pub slow_requests: u64, // >100ms
    pub cache_hit_rate: f64,
    pub last_request_time: Option<std::time::Instant>,
}

impl LspTypeService {
    /// Создать новый LSP сервис
    pub fn new(resolution_service: Arc<TypeResolutionService>) -> Self {
        Self {
            resolution_service,
            lsp_cache: Arc::new(RwLock::new(LspCache::default())),
            performance_monitor: Arc::new(RwLock::new(PerformanceMonitor::default())),
        }
    }

    /// Разрешить тип в позиции (основной LSP API)
    pub async fn resolve_at_position(
        &self,
        file_path: &str,
        line: u32,
        column: u32,
        expression: &str,
    ) -> TypeResolution {
        let start_time = std::time::Instant::now();

        // Проверяем LSP кеш
        let position_key = PositionKey {
            file_path: file_path.to_string(),
            line,
            column,
        };

        {
            let cache = self.lsp_cache.read().await;
            if let Some(cached_resolution) = cache.position_cache.get(&position_key) {
                self.record_cache_hit().await;
                return cached_resolution.clone();
            }
        }

        // Создаём контекст для разрешения
        let context = TypeContext {
            file_path: Some(file_path.to_string()),
            line: Some(line),
            column: Some(column),
            local_variables: HashMap::new(), // TODO: извлечь из файла
            current_function: None,          // TODO: определить из позиции
            current_facet: None,
        };

        // Разрешаем через центральный сервис
        let resolution = self
            .resolution_service
            .resolve_expression(expression, &context)
            .await
            .unwrap_or_else(|_| TypeResolution::unknown());

        // Кешируем результат
        {
            let mut cache = self.lsp_cache.write().await;
            cache
                .position_cache
                .insert(position_key, resolution.clone());
        }

        self.record_performance(start_time.elapsed()).await;
        resolution
    }

    /// Получить автодополнение (быстрое для LSP)
    pub async fn get_completions_fast(
        &self,
        prefix: &str,
        file_path: &str,
        line: u32,
        column: u32,
    ) -> Vec<LspCompletion> {
        let start_time = std::time::Instant::now();

        // Проверяем кеш автодополнений
        let cache_key = format!("{}:{}:{}:{}", file_path, line, column, prefix);
        {
            let cache = self.lsp_cache.read().await;
            if let Some(cached_completions) = cache.completion_cache.get(&cache_key) {
                self.record_cache_hit().await;
                return cached_completions.clone();
            }
        }

        // Создаём контекст
        let context = TypeContext {
            file_path: Some(file_path.to_string()),
            line: Some(line),
            column: Some(column),
            local_variables: HashMap::new(),
            current_function: None,
            current_facet: None,
        };

        // Получаем автодополнение через центральный сервис
        let completions = self
            .resolution_service
            .get_completions(prefix, &context)
            .await
            .unwrap_or_default();

        // Конвертируем в LSP формат
        let lsp_completions: Vec<LspCompletion> = completions
            .into_iter()
            .map(|comp| self.convert_to_lsp_completion(comp))
            .collect();

        // Кешируем результат
        {
            let mut cache = self.lsp_cache.write().await;
            cache
                .completion_cache
                .insert(cache_key, lsp_completions.clone());
        }

        self.record_performance(start_time.elapsed()).await;
        lsp_completions
    }

    /// Получить hover информацию
    pub async fn get_hover_info(
        &self,
        expression: &str,
        file_path: &str,
        line: u32,
        column: u32,
    ) -> Option<HoverInfo> {
        // Проверяем кеш hover
        let cache_key = format!("hover:{}:{}:{}:{}", file_path, line, column, expression);
        {
            let cache = self.lsp_cache.read().await;
            if let Some(cached_hover) = cache.hover_cache.get(&cache_key) {
                return Some(cached_hover.clone());
            }
        }

        // Разрешаем тип
        let resolution = self
            .resolve_at_position(file_path, line, column, expression)
            .await;

        // Создаём hover информацию
        let hover_info = self.create_hover_info(&resolution, expression);

        // Кешируем
        {
            let mut cache = self.lsp_cache.write().await;
            cache.hover_cache.insert(cache_key, hover_info.clone());
        }

        Some(hover_info)
    }

    /// Проверить совместимость типов для присваивания
    pub async fn check_assignment_compatibility(
        &self,
        from_expr: &str,
        to_expr: &str,
        context: &TypeContext,
    ) -> bool {
        let from_type = self
            .resolution_service
            .resolve_expression(from_expr, context)
            .await
            .unwrap_or_else(|_| TypeResolution::unknown());
        let to_type = self
            .resolution_service
            .resolve_expression(to_expr, context)
            .await
            .unwrap_or_else(|_| TypeResolution::unknown());
        let checker = TypeCheckerService::new();
        checker.is_assignment_compatible(&from_type, &to_type)
    }

    /// Получить метрики производительности
    pub async fn get_performance_metrics(&self) -> PerformanceMonitor {
        (*self.performance_monitor.read().await).clone()
    }

    // === ПРИВАТНЫЕ МЕТОДЫ ===

    fn convert_to_lsp_completion(&self, completion: CompletionItem) -> LspCompletion {
        let lsp_kind = match completion.kind {
            CompletionKind::Variable => LspCompletionKind::Variable,
            CompletionKind::Function => LspCompletionKind::Function,
            CompletionKind::Method => LspCompletionKind::Method,
            CompletionKind::Property => LspCompletionKind::Property,
            CompletionKind::Type => LspCompletionKind::Class,
            CompletionKind::Keyword => LspCompletionKind::Keyword,
            CompletionKind::Snippet => LspCompletionKind::Snippet,
            // Новые виды — мэппинг в ближайшие LSP аналоги
            CompletionKind::Global => LspCompletionKind::Module,
            CompletionKind::Catalog => LspCompletionKind::Class,
            CompletionKind::Document => LspCompletionKind::Class,
            CompletionKind::Enum => LspCompletionKind::Enum,
            CompletionKind::GlobalFunction => LspCompletionKind::Function,
        };

        LspCompletion {
            label: completion.label.clone(),
            kind: lsp_kind.clone(),
            detail: completion.detail,
            documentation: completion.documentation,
            insert_text: completion.insert_text,
            filter_text: Some(completion.label.clone()),
            sort_text: Some(format!("{:02}_{}", lsp_kind as u8, completion.label)),
        }
    }

    fn create_hover_info(&self, resolution: &TypeResolution, expression: &str) -> HoverInfo {
        let type_info = format!("{:?}", resolution.result);
        let content = format!(
            "**{}**\n\nТип: {}\nУверенность: {:?}",
            expression, type_info, resolution.certainty
        );

        HoverInfo {
            content,
            type_info,
            documentation: None,  // TODO: получить из репозитория
            examples: Vec::new(), // TODO: получить примеры использования
        }
    }

    async fn record_cache_hit(&self) {
        // TODO: обновить статистику кеша
    }

    async fn record_performance(&self, duration: std::time::Duration) {
        let mut monitor = self.performance_monitor.write().await;
        let time_ms = duration.as_millis() as f64;

        monitor.total_requests += 1;

        // Обновляем среднее время ответа
        if monitor.total_requests == 1 {
            monitor.average_response_time_ms = time_ms;
        } else {
            monitor.average_response_time_ms =
                (monitor.average_response_time_ms * (monitor.total_requests - 1) as f64 + time_ms)
                    / monitor.total_requests as f64;
        }

        // Считаем медленные запросы
        if time_ms > 100.0 {
            monitor.slow_requests += 1;
        }

        monitor.last_request_time = Some(std::time::Instant::now());
    }
}

// === WEB TYPE SERVICE ===

/// Сервис типов для веб-интерфейса (оптимизирован для богатых данных)
pub struct WebTypeService {
    /// Центральный сервис разрешения
    resolution_service: Arc<TypeResolutionService>,

    /// Построитель документации
    documentation_builder: Arc<DocumentationBuilder>,

    /// Поисковая система для веб
    search_engine: Arc<WebSearchEngine>,

    /// Монитор производительности веб-операций
    performance_monitor: Arc<RwLock<PerformanceMonitor>>,
}

/// Построитель документации для веб-интерфейса
pub struct DocumentationBuilder {
    template_cache: Arc<RwLock<HashMap<String, String>>>,
}

/// Поисковая система для веб-интерфейса
pub struct WebSearchEngine {
    search_cache: Arc<RwLock<HashMap<String, Vec<WebSearchResult>>>>,
}

/// Результат поиска для веб-интерфейса
#[derive(Debug, Clone)]
pub struct WebSearchResult {
    pub type_name: String,
    pub category: String,
    pub description: String,
    pub relevance_score: f32,
    pub match_highlights: Vec<String>,
    pub url: String, // Ссылка на детальную страницу
}

/// Иерархия типов для веб-интерфейса
#[derive(Debug, Clone)]
pub struct WebTypeHierarchy {
    pub categories: Vec<WebCategory>,
    pub total_types: usize,
    pub statistics: WebHierarchyStats,
}

/// Категория в веб-иерархии
#[derive(Debug, Clone)]
pub struct WebCategory {
    pub id: String,
    pub name: String,
    pub description: String,
    pub types: Vec<WebTypeInfo>,
    pub subcategories: Vec<WebCategory>,
    pub ui_metadata: WebUiMetadata,
}

/// Информация о типе для веб-интерфейса
#[derive(Debug, Clone)]
pub struct WebTypeInfo {
    pub name: String,
    pub description: String,
    pub methods_count: usize,
    pub properties_count: usize,
    pub examples: Vec<String>,
    pub url: String,
    pub tags: Vec<String>,
}

/// UI метаданные для веб
#[derive(Debug, Clone)]
pub struct WebUiMetadata {
    pub icon: String,
    pub color: String,
    pub css_classes: Vec<String>,
}

/// Статистика иерархии для веб
#[derive(Debug, Clone, Default)]
pub struct WebHierarchyStats {
    pub total_categories: usize,
    pub total_types: usize,
    pub platform_types: usize,
    pub configuration_types: usize,
}

impl WebTypeService {
    /// Создать новый веб-сервис
    pub fn new(resolution_service: Arc<TypeResolutionService>) -> Self {
        Self {
            resolution_service,
            documentation_builder: Arc::new(DocumentationBuilder::new()),
            search_engine: Arc::new(WebSearchEngine::new()),
            performance_monitor: Arc::new(RwLock::new(PerformanceMonitor::default())),
        }
    }

    /// Получить все типы с документацией для веб-интерфейса
    pub async fn get_all_types_with_documentation(&self) -> Result<Vec<WebTypeInfo>> {
        let start_time = std::time::Instant::now();
        info!("🌐 Получение всех типов для веб-интерфейса...");

        // Получаем все типы через поиск с пустым запросом
        let type_search_results = self.resolution_service.search_types("").await?;
        let all_types: Vec<RawTypeDataForResult> = type_search_results
            .into_iter()
            .map(|result| result.raw_data)
            .collect();

        // Конвертируем в веб-формат
        let mut web_types = Vec::new();
        for raw_type in all_types {
            let web_type = WebTypeInfo {
                name: raw_type.russian_name.clone(),
                description: raw_type.documentation.clone(),
                methods_count: raw_type.methods.len(),
                properties_count: raw_type.properties.len(),
                examples: raw_type.examples.clone(),
                url: format!("/types/{}", urlencoding::encode(&raw_type.russian_name)),
                tags: raw_type.category_path.clone(),
            };
            web_types.push(web_type);
        }

        info!("✅ Подготовлено {} типов для веб", web_types.len());
        self.record_performance(start_time.elapsed()).await;
        Ok(web_types)
    }

    /// Построить иерархию типов для веб-интерфейса
    pub async fn build_type_hierarchy(&self) -> Result<WebTypeHierarchy> {
        let start_time = std::time::Instant::now();
        info!("🌳 Построение иерархии типов для веб...");

        // Получаем типы через публичный API
        let type_search_results = self.resolution_service.search_types("").await?;
        let all_types: Vec<RawTypeDataForResult> = type_search_results
            .into_iter()
            .map(|result| result.raw_data)
            .collect();

        // Группируем типы по категориям
        let mut categories_map: HashMap<String, Vec<RawTypeDataForResult>> = HashMap::new();

        for raw_type in all_types {
            for category in &raw_type.category_path {
                categories_map
                    .entry(category.clone())
                    .or_insert_with(Vec::new)
                    .push(raw_type.clone());
            }
        }

        // Создаём веб-категории
        let mut web_categories = Vec::new();
        for (category_name, types) in categories_map {
            let web_types = types
                .into_iter()
                .map(|raw_type| WebTypeInfo {
                    name: raw_type.russian_name.clone(),
                    description: raw_type.documentation.clone(),
                    methods_count: raw_type.methods.len(),
                    properties_count: raw_type.properties.len(),
                    examples: raw_type.examples.clone(),
                    url: format!("/types/{}", urlencoding::encode(&raw_type.russian_name)),
                    tags: raw_type.category_path.clone(),
                })
                .collect::<Vec<_>>();

            web_categories.push(WebCategory {
                id: category_name.clone(),
                name: category_name.clone(),
                description: format!("Категория типов: {}", category_name),
                types: web_types,
                subcategories: Vec::new(), // TODO: реализовать подкатегории
                ui_metadata: WebUiMetadata {
                    icon: "folder".to_string(),
                    color: "#569cd6".to_string(),
                    css_classes: vec!["category".to_string()],
                },
            });
        }

        // Собираем статистику
        let total_types = web_categories.iter().map(|cat| cat.types.len()).sum();
        // TODO: Получить статистику через публичный API
        let stats = self.resolution_service.get_stats().await;

        let hierarchy = WebTypeHierarchy {
            categories: web_categories,
            total_types,
            statistics: WebHierarchyStats {
                total_categories: stats.total_resolutions as usize, // Используем доступные поля
                total_types: stats.successful_resolutions as usize,
                platform_types: stats.cache_hits as usize, // Временные заглушки
                configuration_types: stats.cache_misses as usize,
            },
        };

        info!(
            "✅ Иерархия построена: {} категорий, {} типов",
            hierarchy.categories.len(),
            hierarchy.total_types
        );
        self.record_performance(start_time.elapsed()).await;

        Ok(hierarchy)
    }

    /// Получить метрики производительности веб-операций
    pub async fn get_performance_metrics(&self) -> PerformanceMonitor {
        (*self.performance_monitor.read().await).clone()
    }

    async fn record_performance(&self, duration: std::time::Duration) {
        let mut monitor = self.performance_monitor.write().await;
        let time_ms = duration.as_millis() as f64;
        monitor.total_requests += 1;
        if monitor.total_requests == 1 {
            monitor.average_response_time_ms = time_ms;
        } else {
            monitor.average_response_time_ms =
                (monitor.average_response_time_ms * (monitor.total_requests - 1) as f64 + time_ms)
                    / monitor.total_requests as f64;
        }
        if time_ms > 100.0 {
            monitor.slow_requests += 1;
        }
        monitor.last_request_time = Some(std::time::Instant::now());
    }

    /// Расширенный поиск для веб-интерфейса
    pub async fn advanced_search(
        &self,
        query: &str,
        filters: SearchFilters,
    ) -> Result<Vec<WebSearchResult>> {
        println!("🔍 Расширенный поиск в веб: '{}'", query);

        // Поиск через центральный сервис
        let search_results = self.resolution_service.search_types(query).await?;

        // Фильтрация результатов (по source/category/methods/properties/facets)
        let filtered_results = self.apply_search_filters(search_results, &filters).await?;

        // Конвертация в веб-формат
        let web_results = filtered_results
            .into_iter()
            .map(|result| WebSearchResult {
                type_name: result.raw_data.russian_name.clone(),
                category: result
                    .raw_data
                    .category_path
                    .first()
                    .unwrap_or(&"Неопределено".to_string())
                    .clone(),
                description: result.raw_data.documentation.clone(),
                relevance_score: result.relevance_score as f32,
                match_highlights: result.match_highlights.clone(),
                url: format!(
                    "/types/{}",
                    urlencoding::encode(&result.raw_data.russian_name)
                ),
            })
            .collect();

        Ok(web_results)
    }

    /// Получить детальную информацию о типе
    pub async fn get_type_details(&self, type_name: &str) -> Result<WebTypeDetails> {
        println!("📄 Получение деталей для типа: {}", type_name);

        // Ищем тип по точному имени
        let search_results = self.resolution_service.search_types(type_name).await?;

        // Ищем точное совпадение
        if let Some(found_type) = search_results
            .into_iter()
            .find(|r| r.raw_data.russian_name == type_name)
        {
            let raw_data = found_type.raw_data;

            let methods = raw_data
                .methods
                .iter()
                .map(|m| WebMethodInfo {
                    name: m.name.clone(),
                    description: m.documentation.clone(),
                    parameters: m
                        .parameters
                        .iter()
                        .map(|p| WebParameterInfo {
                            name: p.name.clone(),
                            type_name: p.type_name.clone(),
                            description: p.description.clone(),
                            is_optional: false, // Заглушка
                        })
                        .collect(),
                    return_type: m.return_type.clone(),
                    examples: m.examples.clone(),
                })
                .collect();

            let properties = raw_data
                .properties
                .iter()
                .map(|p| WebPropertyInfo {
                    name: p.name.clone(),
                    description: p.description.clone(),
                    type_name: p.type_name.clone(),
                    is_readonly: false, // Заглушка
                })
                .collect();

            let details = WebTypeDetails {
                basic_info: WebTypeInfo {
                    name: raw_data.russian_name.clone(),
                    description: raw_data.documentation.clone(),
                    methods_count: raw_data.methods.len(),
                    properties_count: raw_data.properties.len(),
                    examples: raw_data.examples.clone(),
                    url: format!("/types/{}", urlencoding::encode(&raw_data.russian_name)),
                    tags: raw_data.category_path.clone(),
                },
                methods,
                properties,
                related_types: Vec::new(), // TODO: найти связанные типы
            };
            Ok(details)
        } else {
            Err(anyhow::anyhow!("Тип '{}' не найден", type_name))
        }
    }

    async fn apply_search_filters(
        &self,
        results: Vec<TypeSearchResult>,
        filters: &SearchFilters,
    ) -> Result<Vec<TypeSearchResult>> {
        let mut out = Vec::with_capacity(results.len());
        'outer: for r in results.into_iter() {
            let raw = &r.raw_data;

            // Источник
            if let Some(src) = &filters.source {
                let matches = match (&raw.source, src) {
                    (TypeSourceStub::Platform { .. }, TypeSourceStub::Platform { .. }) => true,
                    (
                        TypeSourceStub::Configuration { .. },
                        TypeSourceStub::Configuration { .. },
                    ) => true,
                    (TypeSourceStub::UserDefined { .. }, TypeSourceStub::UserDefined { .. }) => {
                        true
                    }
                    _ => false,
                };
                if !matches {
                    continue 'outer;
                }
            }

            // Категория
            if let Some(cat) = &filters.category {
                if !raw.category_path.iter().any(|c| c.contains(cat)) {
                    continue 'outer;
                }
            }

            // Наличие методов/свойств
            if let Some(has_methods) = filters.has_methods {
                if has_methods && raw.methods.is_empty() {
                    continue 'outer;
                }
                if !has_methods && !raw.methods.is_empty() {
                    continue 'outer;
                }
            }
            if let Some(has_properties) = filters.has_properties {
                if has_properties && raw.properties.is_empty() {
                    continue 'outer;
                }
                if !has_properties && !raw.properties.is_empty() {
                    continue 'outer;
                }
            }

            // Фасеты (если задан список, все перечисленные должны быть доступны)
            if !filters.facets.is_empty() {
                let available: std::collections::HashSet<_> =
                    raw.available_facets.iter().map(|f| f.kind).collect();
                if !filters.facets.iter().all(|k| available.contains(k)) {
                    continue 'outer;
                }
            }

            out.push(r);
        }

        Ok(out)
    }
}

/// Фильтры для поиска в веб-интерфейсе
#[derive(Debug, Clone, Default)]
pub struct SearchFilters {
    pub source: Option<TypeSourceStub>,
    pub category: Option<String>,
    pub has_methods: Option<bool>,
    pub has_properties: Option<bool>,
    pub facets: Vec<FacetKind>,
}

/// Детальная информация о типе для веб
#[derive(Debug, Clone)]
pub struct WebTypeDetails {
    pub basic_info: WebTypeInfo,
    pub methods: Vec<WebMethodInfo>,
    pub properties: Vec<WebPropertyInfo>,
    pub related_types: Vec<String>,
}

/// Информация о методе для веб
#[derive(Debug, Clone)]
pub struct WebMethodInfo {
    pub name: String,
    pub description: String,
    pub parameters: Vec<WebParameterInfo>,
    pub return_type: Option<String>,
    pub examples: Vec<String>,
}

/// Информация о параметре для веб
#[derive(Debug, Clone)]
pub struct WebParameterInfo {
    pub name: String,
    pub type_name: String,
    pub description: String,
    pub is_optional: bool,
}

/// Информация о свойстве для веб
#[derive(Debug, Clone)]
pub struct WebPropertyInfo {
    pub name: String,
    pub description: String,
    pub type_name: String,
    pub is_readonly: bool,
}

impl DocumentationBuilder {
    pub fn new() -> Self {
        Self {
            template_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl WebSearchEngine {
    pub fn new() -> Self {
        Self {
            search_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

// === ANALYSIS TYPE SERVICE ===

/// Сервис типов для анализа проектов (оптимизирован для аналитики)
pub struct AnalysisTypeService {
    /// Центральный сервис разрешения
    resolution_service: Arc<TypeResolutionService>,

    /// Анализатор проектов
    project_analyzer: Arc<ProjectAnalyzer>,

    /// Калькулятор покрытия типизации
    coverage_calculator: Arc<CoverageCalculator>,
}

/// Анализатор BSL проектов
pub struct ProjectAnalyzer {
    analysis_cache: Arc<RwLock<HashMap<String, ProjectAnalysisResult>>>,
}

/// Калькулятор покрытия типизации
pub struct CoverageCalculator {
    coverage_cache: Arc<RwLock<HashMap<String, CoverageReport>>>,
}

/// Результат анализа проекта
#[derive(Debug, Clone)]
pub struct ProjectAnalysisResult {
    pub project_path: String,
    pub total_files: usize,
    pub analyzed_files: usize,
    pub total_functions: usize,
    pub total_variables: usize,
    pub type_errors: Vec<TypeDiagnostic>,
    pub coverage_report: CoverageReport,
    pub analysis_time: std::time::Duration,
}

/// Отчёт о покрытии типизации
#[derive(Debug, Clone)]
pub struct CoverageReport {
    pub total_expressions: usize,
    pub typed_expressions: usize,
    pub coverage_percentage: f32,
    pub by_file: HashMap<String, FileCoverage>,
}

/// Покрытие типизации файла
#[derive(Debug, Clone)]
pub struct FileCoverage {
    pub file_path: String,
    pub total_expressions: usize,
    pub typed_expressions: usize,
    pub coverage_percentage: f32,
}

/// Диагностика типов
#[derive(Debug, Clone)]
pub struct TypeDiagnostic {
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub suggested_fix: Option<String>,
}

/// Уровень серьёзности диагностики
#[derive(Debug, Clone, PartialEq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

impl AnalysisTypeService {
    /// Создать новый сервис анализа
    pub fn new(resolution_service: Arc<TypeResolutionService>) -> Self {
        Self {
            resolution_service,
            project_analyzer: Arc::new(ProjectAnalyzer::new()),
            coverage_calculator: Arc::new(CoverageCalculator::new()),
        }
    }

    /// Проанализировать проект BSL
    pub async fn analyze_project(&self, project_path: &Path) -> Result<ProjectAnalysisResult> {
        println!("🔍 Анализ проекта: {}", project_path.display());
        let start_time = std::time::Instant::now();

        // Поиск всех BSL файлов
        let bsl_files = self.find_bsl_files(project_path).await?;
        println!("📁 Найдено {} BSL файлов", bsl_files.len());

        let mut total_functions = 0;
        let mut total_variables = 0;
        let mut type_errors = Vec::new();

        // Анализируем каждый файл
        for file_path in &bsl_files {
            match self.analyze_file(file_path).await {
                Ok(file_analysis) => {
                    total_functions += file_analysis.functions_count;
                    total_variables += file_analysis.variables_count;
                    type_errors.extend(file_analysis.diagnostics);
                }
                Err(e) => {
                    println!("⚠️ Ошибка анализа {}: {}", file_path.display(), e);
                }
            }
        }

        // Рассчитываем покрытие типизации
        let coverage_report = self
            .coverage_calculator
            .calculate_coverage(&bsl_files)
            .await?;

        let analysis_time = start_time.elapsed();

        Ok(ProjectAnalysisResult {
            project_path: project_path.to_string_lossy().to_string(),
            total_files: bsl_files.len(),
            analyzed_files: bsl_files.len(), // TODO: учесть файлы с ошибками
            total_functions,
            total_variables,
            type_errors,
            coverage_report,
            analysis_time,
        })
    }

    /// Вычислить покрытие типизации
    pub async fn calculate_type_coverage(
        &self,
        files: &[std::path::PathBuf],
    ) -> Result<CoverageReport> {
        self.coverage_calculator.calculate_coverage(files).await
    }

    /// Найти ошибки типов в файлах
    pub async fn find_type_errors(
        &self,
        files: &[std::path::PathBuf],
    ) -> Result<Vec<TypeDiagnostic>> {
        let mut all_errors = Vec::new();

        for file_path in files {
            if let Ok(file_analysis) = self.analyze_file(file_path).await {
                all_errors.extend(file_analysis.diagnostics);
            }
        }

        Ok(all_errors)
    }

    // === ПРИВАТНЫЕ МЕТОДЫ ===

    async fn find_bsl_files(&self, project_path: &Path) -> Result<Vec<std::path::PathBuf>> {
        use walkdir::WalkDir;

        let mut bsl_files = Vec::new();

        for entry in WalkDir::new(project_path).follow_links(true) {
            let entry = entry?;
            if let Some(extension) = entry.path().extension() {
                if extension == "bsl" {
                    bsl_files.push(entry.path().to_path_buf());
                }
            }
        }

        Ok(bsl_files)
    }

    async fn analyze_file(&self, file_path: &Path) -> Result<FileAnalysisResult> {
        // TODO: Реализовать анализ отдельного файла
        Ok(FileAnalysisResult {
            file_path: file_path.to_path_buf(),
            functions_count: 0,
            variables_count: 0,
            diagnostics: Vec::new(),
        })
    }
}

/// Результат анализа отдельного файла
#[derive(Debug, Clone)]
pub struct FileAnalysisResult {
    pub file_path: std::path::PathBuf,
    pub functions_count: usize,
    pub variables_count: usize,
    pub diagnostics: Vec<TypeDiagnostic>,
}

impl ProjectAnalyzer {
    pub fn new() -> Self {
        Self {
            analysis_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl CoverageCalculator {
    pub fn new() -> Self {
        Self {
            coverage_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Вычислить покрытие типизации для файлов
    pub async fn calculate_coverage(&self, files: &[std::path::PathBuf]) -> Result<CoverageReport> {
        // TODO: Реализовать расчёт покрытия
        Ok(CoverageReport {
            total_expressions: files.len() * 10, // Заглушка
            typed_expressions: files.len() * 7,  // Заглушка
            coverage_percentage: 70.0,           // Заглушка
            by_file: HashMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unified::data::InMemoryTypeRepository;

    #[tokio::test]
    async fn test_lsp_type_service() {
        // Создаём репозиторий с тестовыми данными
        let repo = Arc::new(InMemoryTypeRepository::new());
        let resolution_service = Arc::new(TypeResolutionService::new(repo));

        // Создаём LSP сервис
        let lsp_service = LspTypeService::new(resolution_service);

        // Тестируем разрешение в позиции
        let resolution = lsp_service
            .resolve_at_position("test.bsl", 10, 5, "Массив")
            .await;
        assert_ne!(resolution.certainty, crate::core::types::Certainty::Known); // Без данных будет Unknown

        // Тестируем автодополнение
        let completions = lsp_service
            .get_completions_fast("Стр", "test.bsl", 10, 5)
            .await;
        // В тестовом окружении может быть пустой

        println!("✅ LspTypeService работает");
    }

    #[tokio::test]
    async fn test_web_type_service() {
        let repo = Arc::new(InMemoryTypeRepository::new());
        let resolution_service = Arc::new(TypeResolutionService::new(repo));

        let web_service = WebTypeService::new(resolution_service);

        // Тестируем получение всех типов
        let web_types = web_service
            .get_all_types_with_documentation()
            .await
            .unwrap();
        // В тестовом окружении будет пустой

        // Тестируем построение иерархии
        let hierarchy = web_service.build_type_hierarchy().await.unwrap();

        println!("✅ WebTypeService работает");
    }
}
