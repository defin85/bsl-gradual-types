//! Web-based Type Browser и Documentation Server
//!
//! HTTP сервер для браузерного интерфейса просмотра типов BSL

use anyhow::Result;
use clap::Parser;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use bsl_gradual_types::core::type_checker::{TypeChecker, TypeContext};
use bsl_gradual_types::core::types::{TypeResolution, ResolutionResult, ConcreteType};
use bsl_gradual_types::core::platform_resolver::PlatformTypeResolver;
use bsl_gradual_types::parser::common::ParserFactory;
use bsl_gradual_types::documentation::{
    DocumentationSearchEngine, PlatformDocumentationProvider, ConfigurationDocumentationProvider,
    AdvancedSearchQuery
};
use bsl_gradual_types::documentation::core::{DocumentationProvider, ProviderConfig};

#[derive(Parser)]
#[command(name = "bsl-web-server")]
#[command(about = "Web-based type browser для BSL Gradual Type System")]
struct Cli {
    /// Порт для HTTP сервера
    #[arg(short, long, default_value = "8080")]
    port: u16,
    
    /// Путь к проекту 1С для анализа
    #[arg(short = 'j', long)]
    project: Option<PathBuf>,
    
    /// Включить hot reload для разработки
    #[arg(long)]
    hot_reload: bool,
    
    /// Путь к статическим файлам
    #[arg(long, default_value = "web")]
    static_dir: PathBuf,
}

/// Состояние web сервера
#[derive(Clone)]
struct AppState {
    /// Контекст типов
    type_context: Arc<RwLock<Option<TypeContext>>>,
    /// Platform resolver
    platform_resolver: Arc<RwLock<PlatformTypeResolver>>,
    /// Кеш для быстрого поиска (TODO: реализовать)
    #[allow(dead_code)]
    search_cache: Arc<RwLock<HashMap<String, Vec<SearchResult>>>>,
    /// Статус загрузки платформенных типов
    loading_status: Arc<RwLock<LoadingStatus>>,
    /// Поисковая система документации
    search_engine: Arc<DocumentationSearchEngine>,
    /// Платформенный провайдер документации
    platform_provider: Arc<PlatformDocumentationProvider>,
}

/// Статус загрузки документации
#[derive(Debug, Clone, Serialize)]
struct LoadingStatus {
    /// Загружаются ли данные сейчас
    pub is_loading: bool,
    /// Текущий прогресс (0-100)
    pub progress: u8,
    /// Обработано файлов
    pub processed_files: usize,
    /// Всего файлов
    pub total_files: usize,
    /// Текущая операция
    pub current_operation: String,
    /// Ошибки парсинга
    pub errors: usize,
}

/// Результат поиска типов
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SearchResult {
    /// Имя типа
    name: String,
    /// Категория типа
    category: String,
    /// Описание
    description: Option<String>,
    /// Количество методов
    methods_count: usize,
    /// Количество свойств
    properties_count: usize,
    /// Тип результата
    result_type: String,
}

/// API response для типов
#[derive(Serialize)]
struct TypesResponse {
    types: Vec<SearchResult>,
    total: usize,
    page: usize,
    per_page: usize,
}

/// Детальная информация о типе
#[derive(Serialize)]
struct TypeDetails {
    name: String,
    category: String,
    description: Option<String>,
    methods: Vec<MethodInfo>,
    properties: Vec<PropertyInfo>,
    related_types: Vec<String>,
    usage_examples: Vec<String>,
}

#[derive(Serialize)]
struct MethodInfo {
    name: String,
    parameters: Vec<String>,
    return_type: Option<String>,
    description: Option<String>,
}

#[derive(Serialize)]
struct PropertyInfo {
    name: String,
    type_name: String,
    readonly: bool,
    description: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Настраиваем логирование
    tracing_subscriber::fmt()
        .with_env_filter("bsl_web_server=info,warp=info")
        .init();
    
    let cli = Cli::parse();
    
    println!("🌐 Starting BSL Type Browser Web Server on port {}", cli.port);
    
    // Инициализируем поисковую систему и провайдеры
    println!("🔧 Инициализация поисковой системы...");
    let search_engine = Arc::new(DocumentationSearchEngine::new());
    let platform_provider = Arc::new(PlatformDocumentationProvider::new());
    
    // Инициализируем платформенный провайдер
    let config = ProviderConfig::default();
    if let Err(e) = platform_provider.initialize(&config).await {
        println!("⚠️ Предупреждение при инициализации провайдера: {}", e);
        println!("   Система будет работать без справки синтакс-помощника");
    }
    
    // Строим индексы для поиска
    let config_provider = ConfigurationDocumentationProvider::new();
    if let Err(e) = search_engine.build_indexes(&*platform_provider, &config_provider).await {
        println!("⚠️ Предупреждение при построении индексов: {}", e);
    } else {
        println!("✅ Индексы поиска построены");
    }
    
    // Инициализируем состояние приложения
    let app_state = AppState {
        type_context: Arc::new(RwLock::new(None)),
        platform_resolver: Arc::new(RwLock::new(PlatformTypeResolver::new())),
        search_cache: Arc::new(RwLock::new(HashMap::new())),
        loading_status: Arc::new(RwLock::new(LoadingStatus {
            is_loading: false,
            progress: 100,
            processed_files: 0,
            total_files: 0,
            current_operation: "Поисковая система готова".to_string(),
            errors: 0,
        })),
        search_engine,
        platform_provider,
    };
    
    // Если указан проект, анализируем его
    if let Some(project_path) = cli.project {
        println!("📁 Analyzing project: {}", project_path.display());
        let context = analyze_project(&project_path).await?;
        *app_state.type_context.write().await = Some(context);
        println!("✅ Project analysis completed");
    }
    
    // Запускаем web сервер
    start_web_server(cli.port, app_state, cli.static_dir).await?;
    
    Ok(())
}

/// Анализ проекта для получения типов
async fn analyze_project(project_path: &PathBuf) -> Result<TypeContext> {
    use bsl_gradual_types::core::parallel_analysis::{ParallelAnalyzer, ParallelAnalysisConfig};
    
    let config = ParallelAnalysisConfig {
        show_progress: false, // Отключаем для web сервера
        use_cache: true,
        ..Default::default()
    };
    
    let analyzer = ParallelAnalyzer::new(config)?;
    let results = analyzer.analyze_project(project_path)?;
    
    // Объединяем все контексты типов
    let mut combined_context = bsl_gradual_types::core::type_checker::TypeContext {
        variables: HashMap::new(),
        functions: HashMap::new(),
        current_scope: bsl_gradual_types::core::dependency_graph::Scope::Global,
        scope_stack: vec![],
    };
    
    for file_result in results.file_results {
        if file_result.success {
            // Объединяем функции
            for (name, signature) in file_result.type_context.functions {
                combined_context.functions.insert(name, signature);
            }
            
            // Объединяем переменные (глобальные)
            for (name, type_res) in file_result.type_context.variables {
                combined_context.variables.insert(name, type_res);
            }
        }
    }
    
    Ok(combined_context)
}

/// Запуск web сервера
async fn start_web_server(port: u16, app_state: AppState, static_dir: PathBuf) -> Result<()> {
    use warp::Filter;
    
    // CORS для разработки
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type"])
        .allow_methods(vec!["GET", "POST", "OPTIONS"]);
    
    // API routes
    let api = warp::path("api").and(
        // GET /api/types?search=&page=&per_page=
        warp::path("types")
            .and(warp::get())
            .and(warp::query::<SearchQuery>())
            .and(with_state(app_state.clone()))
            .and_then(handle_search_types)
        .or(
            // GET /api/types/{name}
            warp::path("types")
                .and(warp::path::param::<String>())
                .and(warp::get())
                .and(with_state(app_state.clone()))
                .and_then(handle_get_type_details)
        )
        .or(
            // GET /api/stats
            warp::path("stats")
                .and(warp::get())
                .and(with_state(app_state.clone()))
                .and_then(handle_get_stats)
        )
        .or(
            // GET /api/loading-status
            warp::path("loading-status")
                .and(warp::get())
                .and(with_state(app_state.clone()))
                .and_then(handle_get_loading_status)
        )
        .or(
            // POST /api/analyze
            warp::path("analyze")
                .and(warp::post())
                .and(warp::body::json())
                .and(with_state(app_state.clone()))
                .and_then(handle_analyze_code)
        )
        .or(
            // POST /api/v1/search - новый расширенный поиск
            warp::path("v1")
                .and(warp::path("search"))
                .and(warp::post())
                .and(warp::body::json())
                .and(with_state(app_state.clone()))
                .and_then(handle_advanced_search)
        )
        .or(
            // GET /api/v1/suggestions?q=query - автодополнение
            warp::path("v1")
                .and(warp::path("suggestions"))
                .and(warp::get())
                .and(warp::query::<SuggestionsQuery>())
                .and(with_state(app_state.clone()))
                .and_then(handle_get_suggestions)
        )
        .or(
            // GET /api/v1/search-stats - статистика поиска
            warp::path("v1")
                .and(warp::path("search-stats"))
                .and(warp::get())
                .and(with_state(app_state.clone()))
                .and_then(handle_get_search_stats)
        )
        .or(
            // GET /api/v1/categories - список категорий
            warp::path("v1")
                .and(warp::path("categories"))
                .and(warp::get())
                .and(with_state(app_state.clone()))
                .and_then(handle_get_categories)
        )
    ).with(cors);
    
    // Статические файлы
    let static_files = warp::fs::dir(static_dir);
    
    // Главная страница
    let index = warp::path::end()
        .and(warp::get())
        .and_then(handle_index);
    
    let routes = api.or(static_files).or(index);
    
    println!("🚀 Web server running on http://localhost:{}", port);
    println!("📖 Open http://localhost:{} to browse BSL types", port);
    
    warp::serve(routes)
        .run(([127, 0, 0, 1], port))
        .await;
    
    Ok(())
}

/// Helper для передачи состояния в handlers
fn with_state(
    state: AppState,
) -> impl Filter<Extract = (AppState,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || state.clone())
}

/// Query параметры для поиска
#[derive(Deserialize)]
struct SearchQuery {
    search: Option<String>,
    page: Option<usize>,
    per_page: Option<usize>,
}

/// Query параметры для автодополнения
#[derive(Deserialize)]
struct SuggestionsQuery {
    q: String,
    limit: Option<usize>,
}

/// Ответ API с ошибкой
#[derive(Serialize)]
struct ApiError {
    error: String,
    code: u16,
}

/// Ответ для автодополнения
#[derive(Serialize)]
struct SuggestionsResponse {
    suggestions: Vec<String>,
    query: String,
    count: usize,
}

/// Ответ для категорий
#[derive(Serialize)]
struct CategoriesResponse {
    categories: Vec<CategoryInfo>,
    total_count: usize,
}

/// Информация о категории
#[derive(Serialize)]
struct CategoryInfo {
    name: String,
    path: String,
    types_count: usize,
    subcategories: usize,
}

/// Обработчик поиска типов
async fn handle_search_types(
    query: SearchQuery,
    state: AppState,
) -> Result<impl warp::Reply, warp::Rejection> {
    let search_term = query.search.unwrap_or_default().to_lowercase();
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(20).min(100); // Максимум 100
    
    let results = search_types(&state, &search_term).await;
    
    // Пагинация
    let total = results.len();
    let start = (page - 1) * per_page;
    let end = (start + per_page).min(total);
    let page_results = results[start..end].to_vec();
    
    let response = TypesResponse {
        types: page_results,
        total,
        page,
        per_page,
    };
    
    Ok(warp::reply::json(&response))
}

/// Поиск типов
async fn search_types(state: &AppState, search_term: &str) -> Vec<SearchResult> {
    let mut results = Vec::new();
    
    // Поиск в platform resolver
    let platform_resolver = state.platform_resolver.read().await;
    let completions = platform_resolver.get_completions(search_term);
    
    println!("🔍 Search for '{}': found {} platform completions", search_term, completions.len());
    
    for completion in completions {
        results.push(SearchResult {
            name: completion.label.clone(),
            category: format!("{:?}", completion.kind),
            description: completion.documentation,
            methods_count: 0, // TODO: Получить реальное количество
            properties_count: 0,
            result_type: "Platform".to_string(),
        });
    }
    
    // Поиск в type context
    if let Some(context) = state.type_context.read().await.as_ref() {
        // Функции
        for (name, signature) in &context.functions {
            if name.to_lowercase().contains(search_term) {
                results.push(SearchResult {
                    name: name.clone(),
                    category: "Function".to_string(),
                    description: Some(format!("Параметры: {}", signature.params.len())),
                    methods_count: 0,
                    properties_count: 0,
                    result_type: format_type_result(&signature.return_type),
                });
            }
        }
        
        // Переменные
        for (name, type_res) in &context.variables {
            if name.to_lowercase().contains(search_term) {
                results.push(SearchResult {
                    name: name.clone(),
                    category: "Variable".to_string(),
                    description: Some(format!("Уверенность: {:?}", type_res.certainty)),
                    methods_count: 0,
                    properties_count: 0,
                    result_type: format_type_result(type_res),
                });
            }
        }
    }
    
    // Сортируем по релевантности
    results.sort_by(|a, b| {
        let a_exact = a.name.to_lowercase() == search_term;
        let b_exact = b.name.to_lowercase() == search_term;
        
        match (a_exact, b_exact) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        }
    });
    
    results
}

/// Форматирование типа для отображения
fn format_type_result(type_res: &TypeResolution) -> String {
    match &type_res.result {
        ResolutionResult::Concrete(ConcreteType::Primitive(primitive)) => {
            format!("{:?}", primitive)
        }
        ResolutionResult::Concrete(ConcreteType::Platform(platform)) => {
            platform.name.clone()
        }
        ResolutionResult::Union(union_types) => {
            let names: Vec<String> = union_types.iter()
                .map(|wt| format!("{:?}", wt.type_))
                .collect();
            format!("Union({})", names.join(" | "))
        }
        _ => "Dynamic".to_string(),
    }
}

/// Обработчик получения деталей типа
async fn handle_get_type_details(
    type_name: String,
    state: AppState,
) -> Result<impl warp::Reply, warp::Rejection> {
    let details = get_type_details(&state, &type_name).await;
    Ok(warp::reply::json(&details))
}

/// Получение деталей типа
async fn get_type_details(_state: &AppState, type_name: &str) -> TypeDetails {
    // TODO: Реализовать получение детальной информации о типе
    TypeDetails {
        name: type_name.to_string(),
        category: "Unknown".to_string(),
        description: Some("Type details from BSL Gradual Type System".to_string()),
        methods: vec![],
        properties: vec![],
        related_types: vec![],
        usage_examples: vec![
            format!("// Пример использования {}", type_name),
            format!("переменная = Новый {};", type_name),
        ],
    }
}

/// Обработчик статистики
async fn handle_get_stats(
    state: AppState,
) -> Result<impl warp::Reply, warp::Rejection> {
    let stats = get_system_stats(&state).await;
    Ok(warp::reply::json(&stats))
}

/// Обработчик статуса загрузки
async fn handle_get_loading_status(
    state: AppState,
) -> Result<impl warp::Reply, warp::Rejection> {
    let status = state.loading_status.read().await.clone();
    Ok(warp::reply::json(&status))
}

/// Получение статистики системы
async fn get_system_stats(state: &AppState) -> SystemStats {
    let context = state.type_context.read().await;
    let platform_resolver = state.platform_resolver.read().await;
    
    // Получаем количество платформенных типов
    let platform_types_count = platform_resolver.get_completions("").len();
    println!("📊 Platform types count: {}", platform_types_count);
    
    if let Some(ctx) = context.as_ref() {
        SystemStats {
            total_functions: ctx.functions.len(),
            total_variables: ctx.variables.len(),
            platform_types: platform_types_count,
            analysis_cache_size: 0, // TODO: Интеграция с cache
            memory_usage_mb: estimate_memory_usage(ctx),
        }
    } else {
        SystemStats {
            total_functions: 0,
            total_variables: 0,
            platform_types: platform_types_count,
            analysis_cache_size: 0,
            memory_usage_mb: 0.0,
        }
    }
}

#[derive(Serialize, Default)]
struct SystemStats {
    total_functions: usize,
    total_variables: usize,
    platform_types: usize,
    analysis_cache_size: usize,
    memory_usage_mb: f64,
}

/// Приблизительная оценка использования памяти
fn estimate_memory_usage(context: &TypeContext) -> f64 {
    use std::mem;
    
    let base_size = mem::size_of::<TypeContext>();
    let vars_size = context.variables.len() * mem::size_of::<(String, TypeResolution)>();
    let funcs_size = context.functions.len() * mem::size_of::<(String, bsl_gradual_types::core::type_checker::FunctionSignature)>();
    
    (base_size + vars_size + funcs_size) as f64 / (1024.0 * 1024.0)
}

/// Request для анализа кода
#[derive(Deserialize)]
struct AnalyzeRequest {
    code: String,
    filename: Option<String>,
}

/// Response анализа кода
#[derive(Serialize)]
struct AnalyzeResponse {
    success: bool,
    functions: usize,
    variables: usize,
    diagnostics: Vec<DiagnosticInfo>,
    analysis_time_ms: u64,
}

#[derive(Serialize)]
struct DiagnosticInfo {
    line: usize,
    column: usize,
    severity: String,
    message: String,
}

/// Обработчик анализа кода
async fn handle_analyze_code(
    request: AnalyzeRequest,
    _state: AppState,
) -> Result<impl warp::Reply, warp::Rejection> {
    let result = analyze_code_snippet(&request.code, &request.filename).await;
    Ok(warp::reply::json(&result))
}

/// Анализ фрагмента кода
async fn analyze_code_snippet(code: &str, filename: &Option<String>) -> AnalyzeResponse {
    let start_time = std::time::Instant::now();
    
    let file_name = filename.as_deref().unwrap_or("snippet.bsl").to_string();
    
    // Парсим и анализируем код
    let mut parser = ParserFactory::create();
    
    match parser.parse(code) {
        Ok(program) => {
            let type_checker = TypeChecker::new(file_name);
            let (context, diagnostics) = type_checker.check(&program);
            
            AnalyzeResponse {
                success: true,
                functions: context.functions.len(),
                variables: context.variables.len(),
                diagnostics: diagnostics.iter().map(|d| DiagnosticInfo {
                    line: d.line,
                    column: d.column,
                    severity: format!("{:?}", d.severity),
                    message: d.message.clone(),
                }).collect(),
                analysis_time_ms: start_time.elapsed().as_millis() as u64,
            }
        }
        Err(e) => {
            AnalyzeResponse {
                success: false,
                functions: 0,
                variables: 0,
                diagnostics: vec![DiagnosticInfo {
                    line: 1,
                    column: 1,
                    severity: "Error".to_string(),
                    message: format!("Parse error: {}", e),
                }],
                analysis_time_ms: start_time.elapsed().as_millis() as u64,
            }
        }
    }
}

// === НОВЫЕ API ENDPOINTS ДЛЯ ПОИСКОВОЙ СИСТЕМЫ ===

/// Обработчик расширенного поиска
async fn handle_advanced_search(
    query: AdvancedSearchQuery,
    state: AppState,
) -> Result<impl warp::Reply, warp::Rejection> {
    println!("🔍 API поиск: '{}'", query.query);
    
    match state.search_engine.search(query).await {
        Ok(results) => {
            println!("✅ Найдено {} результатов", results.total_count);
            Ok(warp::reply::json(&results))
        }
        Err(e) => {
            println!("❌ Ошибка поиска: {}", e);
            let error = ApiError {
                error: e.to_string(),
                code: 500,
            };
            Ok(warp::reply::json(&error))
        }
    }
}

/// Обработчик автодополнения
async fn handle_get_suggestions(
    query: SuggestionsQuery,
    state: AppState,
) -> Result<impl warp::Reply, warp::Rejection> {
    let limit = query.limit.unwrap_or(10);
    
    match state.search_engine.get_suggestions(&query.q).await {
        Ok(suggestions) => {
            let limited_suggestions: Vec<String> = suggestions.into_iter().take(limit).collect();
            let response = SuggestionsResponse {
                query: query.q.clone(),
                count: limited_suggestions.len(),
                suggestions: limited_suggestions,
            };
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            let error = ApiError {
                error: e.to_string(),
                code: 500,
            };
            Ok(warp::reply::json(&error))
        }
    }
}

/// Обработчик статистики поиска
async fn handle_get_search_stats(
    state: AppState,
) -> Result<impl warp::Reply, warp::Rejection> {
    match state.search_engine.get_statistics().await {
        Ok(stats) => Ok(warp::reply::json(&stats)),
        Err(e) => {
            let error = ApiError {
                error: e.to_string(),
                code: 500,
            };
            Ok(warp::reply::json(&error))
        }
    }
}

/// Обработчик списка категорий
async fn handle_get_categories(
    state: AppState,
) -> Result<impl warp::Reply, warp::Rejection> {
    // Пока простая реализация - возвращаем фиксированный список
    let categories = vec![
        CategoryInfo {
            name: "Универсальные коллекции".to_string(),
            path: "Global context/Universal collections".to_string(),
            types_count: 15,
            subcategories: 0,
        },
        CategoryInfo {
            name: "Справочники".to_string(),
            path: "Catalogs".to_string(),
            types_count: 8,
            subcategories: 2,
        },
        CategoryInfo {
            name: "Документы".to_string(),
            path: "Documents".to_string(),
            types_count: 6,
            subcategories: 1,
        },
        CategoryInfo {
            name: "Перечисления".to_string(),
            path: "Enums".to_string(),
            types_count: 4,
            subcategories: 0,
        },
    ];
    
    let response = CategoriesResponse {
        total_count: categories.len(),
        categories,
    };
    
    Ok(warp::reply::json(&response))
}

/// Обработчик главной страницы
async fn handle_index() -> Result<impl warp::Reply, warp::Rejection> {
    let html = generate_index_html();
    Ok(warp::reply::html(html))
}

/// Генерация HTML главной страницы
fn generate_index_html() -> String {
    r#"
<!DOCTYPE html>
<html lang="ru">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>BSL Type Browser</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { 
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; 
            background: #1e1e1e; 
            color: #d4d4d4; 
            line-height: 1.6;
        }
        .container { max-width: 1200px; margin: 0 auto; padding: 20px; }
        .header { text-align: center; margin-bottom: 40px; }
        .header h1 { color: #569cd6; font-size: 2.5em; margin-bottom: 10px; }
        .header p { color: #9cdcfe; font-size: 1.2em; }
        
        .search-section { margin-bottom: 40px; }
        .search-box { 
            width: 100%; 
            padding: 15px; 
            font-size: 16px; 
            border: 2px solid #3c3c3c; 
            border-radius: 5px; 
            background: #2d2d30; 
            color: #d4d4d4;
        }
        .search-box:focus { outline: none; border-color: #569cd6; }
        
        .results { margin-top: 20px; }
        .type-card { 
            background: #2d2d30; 
            border: 1px solid #3c3c3c; 
            border-radius: 5px; 
            padding: 20px; 
            margin-bottom: 15px;
            transition: background 0.2s;
        }
        .type-card:hover { background: #3c3c3c; }
        .type-name { color: #4ec9b0; font-size: 1.3em; font-weight: bold; }
        .type-category { color: #9cdcfe; font-size: 0.9em; }
        .type-description { color: #d4d4d4; margin-top: 10px; }
        
        .stats-grid { 
            display: grid; 
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); 
            gap: 20px; 
            margin-bottom: 40px; 
        }
        .stat-card { 
            background: #2d2d30; 
            padding: 20px; 
            border-radius: 5px; 
            text-align: center; 
        }
        .stat-value { color: #4fc1ff; font-size: 2em; font-weight: bold; }
        .stat-label { color: #9cdcfe; }
        
        .code-analysis { margin-top: 40px; }
        .code-input { 
            width: 100%; 
            height: 200px; 
            font-family: 'Consolas', 'Monaco', monospace; 
            background: #1e1e1e; 
            border: 2px solid #3c3c3c; 
            color: #d4d4d4; 
            padding: 15px; 
            border-radius: 5px;
        }
        .analyze-btn { 
            background: #0e639c; 
            color: white; 
            border: none; 
            padding: 10px 20px; 
            border-radius: 5px; 
            cursor: pointer; 
            margin-top: 10px;
        }
        .analyze-btn:hover { background: #1177bb; }
        
        .loading { color: #ffcc00; }
        .error { color: #f44747; }
        .success { color: #4fc1ff; }
        
        .progress-section { 
            margin-bottom: 30px; 
            background: #2d2d30; 
            padding: 20px; 
            border-radius: 5px; 
            border: 1px solid #3c3c3c; 
        }
        .progress-bar { 
            width: 100%; 
            height: 20px; 
            background: #1e1e1e; 
            border-radius: 10px; 
            overflow: hidden; 
            margin-top: 10px; 
        }
        .progress-fill { 
            height: 100%; 
            background: linear-gradient(90deg, #0e639c, #4fc1ff); 
            transition: width 0.3s ease; 
            width: 0%; 
        }
        .progress-text { 
            color: #9cdcfe; 
            margin-bottom: 5px; 
        }
        .progress-details { 
            color: #d4d4d4; 
            font-size: 0.9em; 
            margin-top: 10px; 
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🚀 BSL Type Browser</h1>
            <p>Production-ready система типов для 1С:Предприятие</p>
        </div>
        
        <div class="progress-section" id="progress-section" style="display: none;">
            <div class="progress-text" id="progress-text">📊 Загрузка документации 1С...</div>
            <div class="progress-bar">
                <div class="progress-fill" id="progress-fill"></div>
            </div>
            <div class="progress-details" id="progress-details">Подготовка...</div>
        </div>
        
        <div class="stats-grid" id="stats">
            <div class="stat-card">
                <div class="stat-value" id="functions-count">-</div>
                <div class="stat-label">Функций</div>
            </div>
            <div class="stat-card">
                <div class="stat-value" id="variables-count">-</div>
                <div class="stat-label">Переменных</div>
            </div>
            <div class="stat-card">
                <div class="stat-value" id="platform-types">-</div>
                <div class="stat-label">Платформенных типов</div>
            </div>
            <div class="stat-card">
                <div class="stat-value" id="memory-usage">-</div>
                <div class="stat-label">Память (MB)</div>
            </div>
        </div>
        
        <div class="search-section">
            <input type="text" class="search-box" id="search-input" 
                   placeholder="Поиск типов BSL... (например: Массив, Структура, ТаблицаЗначений)">
        </div>
        
        <div class="results" id="results">
            <p style="text-align: center; color: #9cdcfe;">
                💡 Введите название типа для поиска или загрузите проект для анализа
            </p>
        </div>
        
        <div class="code-analysis">
            <h2>🔍 Анализ кода в реальном времени</h2>
            <textarea class="code-input" id="code-input" 
                placeholder="Введите BSL код для анализа...&#10;&#10;Пример:&#10;Функция ТестоваяФункция(параметр)&#10;    Возврат Строка(параметр);&#10;КонецФункции"></textarea>
            <button class="analyze-btn" onclick="analyzeCode()">Анализировать код</button>
            <div id="analysis-results"></div>
        </div>
    </div>
    
    <script>
        // Загрузка статистики при старте
        loadStats();
        checkLoadingStatus();
        
        // Поиск типов
        let searchTimeout;
        document.getElementById('search-input').addEventListener('input', (e) => {
            clearTimeout(searchTimeout);
            searchTimeout = setTimeout(() => {
                searchTypes(e.target.value);
            }, 300);
        });
        
        async function loadStats() {
            try {
                const response = await fetch('/api/stats');
                const stats = await response.json();
                
                document.getElementById('functions-count').textContent = stats.total_functions || 0;
                document.getElementById('variables-count').textContent = stats.total_variables || 0;
                document.getElementById('platform-types').textContent = stats.platform_types || 0;
                document.getElementById('memory-usage').textContent = (stats.memory_usage_mb || 0).toFixed(1);
            } catch (error) {
                console.error('Error loading stats:', error);
            }
        }
        
        async function searchTypes(query) {
            if (!query.trim()) {
                document.getElementById('results').innerHTML = 
                    '<p style="text-align: center; color: #9cdcfe;">💡 Введите название типа для поиска</p>';
                return;
            }
            
            document.getElementById('results').innerHTML = '<p class="loading">🔍 Поиск...</p>';
            
            try {
                const response = await fetch(`/api/types?search=${encodeURIComponent(query)}&per_page=10`);
                const data = await response.json();
                
                if (data.types.length === 0) {
                    document.getElementById('results').innerHTML = 
                        '<p style="text-align: center; color: #ffcc00;">❓ Типы не найдены</p>';
                    return;
                }
                
                const html = data.types.map(type => `
                    <div class="type-card">
                        <div class="type-name">${type.name}</div>
                        <div class="type-category">${type.category} • ${type.result_type}</div>
                        ${type.description ? `<div class="type-description">${type.description}</div>` : ''}
                    </div>
                `).join('');
                
                document.getElementById('results').innerHTML = html;
                
            } catch (error) {
                document.getElementById('results').innerHTML = 
                    '<p class="error">❌ Ошибка поиска: ' + error.message + '</p>';
            }
        }
        
        let progressInterval;
        
        async function checkLoadingStatus() {
            try {
                const response = await fetch('/api/loading-status');
                const status = await response.json();
                
                const progressSection = document.getElementById('progress-section');
                const progressFill = document.getElementById('progress-fill');
                const progressText = document.getElementById('progress-text');
                const progressDetails = document.getElementById('progress-details');
                
                if (status.is_loading) {
                    // Показываем прогресс-бар
                    progressSection.style.display = 'block';
                    progressFill.style.width = status.progress + '%';
                    progressText.textContent = '📊 ' + status.current_operation;
                    
                    let details = `Обработано: ${status.processed_files}`;
                    if (status.total_files > 0) {
                        details += ` из ${status.total_files}`;
                    }
                    if (status.errors > 0) {
                        details += ` (ошибок: ${status.errors})`;
                    }
                    progressDetails.textContent = details;
                    
                    // Продолжаем проверять статус
                    if (!progressInterval) {
                        progressInterval = setInterval(checkLoadingStatus, 1000);
                    }
                } else {
                    // Скрываем прогресс-бар
                    progressSection.style.display = 'none';
                    
                    // Останавливаем проверку
                    if (progressInterval) {
                        clearInterval(progressInterval);
                        progressInterval = null;
                    }
                    
                    // Обновляем статистику
                    loadStats();
                }
            } catch (error) {
                console.error('Error checking loading status:', error);
                // Скрываем прогресс-бар при ошибке
                document.getElementById('progress-section').style.display = 'none';
                if (progressInterval) {
                    clearInterval(progressInterval);
                    progressInterval = null;
                }
            }
        }
        
        async function analyzeCode() {
            const code = document.getElementById('code-input').value;
            const resultsDiv = document.getElementById('analysis-results');
            
            if (!code.trim()) {
                resultsDiv.innerHTML = '<p class="error">❓ Введите код для анализа</p>';
                return;
            }
            
            resultsDiv.innerHTML = '<p class="loading">🔍 Анализ...</p>';
            
            try {
                const response = await fetch('/api/analyze', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ code, filename: 'snippet.bsl' })
                });
                
                const result = await response.json();
                
                let html = `<h3>📊 Результаты анализа (${result.analysis_time_ms}ms)</h3>`;
                
                if (result.success) {
                    html += `
                        <p class="success">✅ Функций: ${result.functions}, Переменных: ${result.variables}</p>
                    `;
                } else {
                    html += '<p class="error">❌ Ошибка парсинга</p>';
                }
                
                if (result.diagnostics.length > 0) {
                    html += '<h4>🚨 Диагностики:</h4>';
                    result.diagnostics.forEach(diag => {
                        const severity = diag.severity === 'Error' ? 'error' : 'success';
                        html += `<p class="${severity}">[${diag.line}:${diag.column}] ${diag.message}</p>`;
                    });
                }
                
                resultsDiv.innerHTML = html;
                
            } catch (error) {
                resultsDiv.innerHTML = '<p class="error">❌ Ошибка анализа: ' + error.message + '</p>';
            }
        }
    </script>
</body>
</html>
    "#.to_string()
}

// Добавляем warp dependency
use warp::Filter;