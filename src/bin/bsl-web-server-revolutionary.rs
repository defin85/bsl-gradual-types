//! Революционный веб-сервер на идеальной архитектуре
//!
//! Полная переработка веб-сервера для использования CentralTypeSystem
//! вместо множественных отдельных компонентов

use anyhow::Result;
use clap::Parser;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::sync::Arc;
use warp::Filter;

use bsl_gradual_types::ideal::system::{CentralTypeSystem, CentralSystemConfig};
use bsl_gradual_types::ideal::presentation::{
    WebSearchRequest, WebHierarchyResponse, WebSearchResponse, WebTypeDetailsResponse
};

#[derive(Parser)]
#[command(name = "bsl-web-server-revolutionary")]
#[command(about = "Революционный веб-сервер BSL типов на идеальной архитектуре")]
struct Cli {
    /// Порт для HTTP сервера
    #[arg(short, long, default_value = "8080")]
    port: u16,
    
    /// Путь к HTML справке
    #[arg(long, default_value = "examples/syntax_helper/rebuilt.shcntx_ru")]
    html_path: String,
    
    /// Путь к XML конфигурации (опционально)
    #[arg(long)]
    config_path: Option<PathBuf>,
    
    /// Включить детальное логирование
    #[arg(long)]
    verbose: bool,
    
    /// Путь к статическим файлам
    #[arg(long, default_value = "web")]
    static_dir: PathBuf,
}

/// Революционное состояние приложения
#[derive(Clone)]
struct RevolutionaryAppState {
    /// Единая центральная система типов
    central_system: Arc<CentralTypeSystem>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Настраиваем логирование
    tracing_subscriber::fmt()
        .with_env_filter("bsl_web_server_revolutionary=info,warp=info")
        .init();
    
    let cli = Cli::parse();
    
    println!("🚀 Запуск РЕВОЛЮЦИОННОГО веб-сервера BSL типов на порту {}", cli.port);
    
    // === ЕДИНАЯ ИНИЦИАЛИЗАЦИЯ РЕВОЛЮЦИОННОЙ АРХИТЕКТУРЫ ===
    println!("🏗️ Создание CentralTypeSystem...");
    
    let central_config = CentralSystemConfig {
        html_path: cli.html_path.clone(),
        configuration_path: cli.config_path.map(|p| p.to_string_lossy().to_string()),
        verbose_logging: cli.verbose,
        ..Default::default()
    };
    
    let central_system = Arc::new(CentralTypeSystem::new(central_config));
    
    println!("⚡ Инициализация всех слоёв архитектуры...");
    if let Err(e) = central_system.initialize().await {
        println!("❌ Критическая ошибка инициализации: {}", e);
        return Err(e);
    }
    
    // Показываем метрики системы
    let metrics = central_system.get_system_metrics().await;
    println!("📊 Революционная система готова:");
    println!("   - Типов в системе: {}", metrics.total_types);
    println!("   - Платформенных: {}", metrics.platform_types);
    println!("   - Конфигурационных: {}", metrics.configuration_types);
    println!("   - Память: {:.2} MB", metrics.cache_memory_mb);
    
    // Проверяем здоровье системы
    let health = central_system.health_check().await;
    println!("🏥 Статус здоровья: {} (score: {:.2})", health.status, health.overall_score);
    
    // Создаём революционное состояние
    let app_state = RevolutionaryAppState {
        central_system: central_system.clone(),
    };
    
    // Запускаем революционный веб-сервер
    start_revolutionary_web_server(cli.port, app_state, cli.static_dir).await?;
    
    Ok(())
}

/// Запуск революционного веб-сервера
async fn start_revolutionary_web_server(port: u16, app_state: RevolutionaryAppState, static_dir: PathBuf) -> Result<()> {
    // CORS для разработки
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type"])
        .allow_methods(vec!["GET", "POST", "OPTIONS"]);
    
    // === РЕВОЛЮЦИОННЫЕ API ROUTES ===
    let api = warp::path("api").and(
        // GET /api/v2/hierarchy - новая иерархия через CentralTypeSystem
        warp::path("v2")
            .and(warp::path("hierarchy"))
            .and(warp::get())
            .and(with_revolutionary_state(app_state.clone()))
            .and_then(handle_hierarchy_revolutionary)
        .or(
            // POST /api/v2/search - новый поиск через CentralTypeSystem
            warp::path("v2")
                .and(warp::path("search"))
                .and(warp::post())
                .and(warp::body::json())
                .and(with_revolutionary_state(app_state.clone()))
                .and_then(handle_search_revolutionary)
        )
        .or(
            // GET /api/v2/types/{name} - детали типа через CentralTypeSystem
            warp::path("v2")
                .and(warp::path("types"))
                .and(warp::path::param::<String>())
                .and(warp::get())
                .and(with_revolutionary_state(app_state.clone()))
                .and_then(handle_type_details_revolutionary)
        )
        .or(
            // GET /api/v2/health - здоровье системы
            warp::path("v2")
                .and(warp::path("health"))
                .and(warp::get())
                .and(with_revolutionary_state(app_state.clone()))
                .and_then(handle_health_check_revolutionary)
        )
        .or(
            // GET /api/v2/metrics - метрики системы
            warp::path("v2")
                .and(warp::path("metrics"))
                .and(warp::get())
                .and(with_revolutionary_state(app_state.clone()))
                .and_then(handle_metrics_revolutionary)
        )
    ).with(cors);
    
    // Статические файлы
    let static_files = warp::fs::dir(static_dir);
    
    // Революционная главная страница
    let index = warp::path::end()
        .and(warp::get())
        .and(with_revolutionary_state(app_state.clone()))
        .and_then(handle_index_revolutionary);
    
    // Революционная страница иерархии
    let hierarchy_page = warp::path("hierarchy")
        .and(warp::get())
        .and(with_revolutionary_state(app_state.clone()))
        .and_then(handle_hierarchy_page_revolutionary);
    
    let routes = api.or(static_files).or(hierarchy_page).or(index);
    
    println!("🌐 Революционный веб-сервер запущен на http://localhost:{}", port);
    println!("📖 Откройте http://localhost:{}/hierarchy для просмотра иерархии типов", port);
    println!("🔍 API v2: http://localhost:{}/api/v2/hierarchy", port);
    
    warp::serve(routes)
        .run(([127, 0, 0, 1], port))
        .await;
    
    Ok(())
}

/// Helper для передачи революционного состояния
fn with_revolutionary_state(
    state: RevolutionaryAppState,
) -> impl Filter<Extract = (RevolutionaryAppState,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || state.clone())
}

// === РЕВОЛЮЦИОННЫЕ HANDLERS ===

/// Обработчик иерархии через CentralTypeSystem
async fn handle_hierarchy_revolutionary(state: RevolutionaryAppState) -> Result<impl warp::Reply, warp::Rejection> {
    println!("🌳 Революционный запрос иерархии через CentralTypeSystem");
    
    match state.central_system.web_interface().handle_hierarchy_request().await {
        Ok(response) => {
            println!("✅ Иерархия получена: {} категорий", response.categories.len());
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            println!("❌ Ошибка получения иерархии: {}", e);
            let error_response = ErrorResponse {
                error: e.to_string(),
                code: 500,
            };
            Ok(warp::reply::json(&error_response))
        }
    }
}

/// Обработчик поиска через CentralTypeSystem
async fn handle_search_revolutionary(
    request: WebSearchRequest,
    state: RevolutionaryAppState,
) -> Result<impl warp::Reply, warp::Rejection> {
    println!("🔍 Революционный поиск: '{}'", request.query);
    
    match state.central_system.web_interface().handle_search_request(request).await {
        Ok(response) => {
            println!("✅ Поиск выполнен: {} результатов", response.results.len());
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            println!("❌ Ошибка поиска: {}", e);
            let error_response = ErrorResponse {
                error: e.to_string(),
                code: 500,
            };
            Ok(warp::reply::json(&error_response))
        }
    }
}

/// Обработчик деталей типа через CentralTypeSystem
async fn handle_type_details_revolutionary(
    type_name: String,
    state: RevolutionaryAppState,
) -> Result<impl warp::Reply, warp::Rejection> {
    println!("📋 Революционный запрос деталей типа: '{}'", type_name);
    
    match state.central_system.web_interface().handle_type_details_request(&type_name).await {
        Ok(response) => {
            println!("✅ Детали типа получены: {} методов, {} свойств", 
                    response.methods.len(), response.properties.len());
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            println!("❌ Ошибка получения деталей: {}", e);
            let error_response = ErrorResponse {
                error: e.to_string(),
                code: 404,
            };
            Ok(warp::reply::json(&error_response))
        }
    }
}

/// Обработчик проверки здоровья системы
async fn handle_health_check_revolutionary(state: RevolutionaryAppState) -> Result<impl warp::Reply, warp::Rejection> {
    println!("🏥 Революционная проверка здоровья системы");
    
    let health = state.central_system.health_check().await;
    println!("✅ Здоровье проверено: {} компонентов", health.components.len());
    
    Ok(warp::reply::json(&health))
}

/// Обработчик метрик системы
async fn handle_metrics_revolutionary(state: RevolutionaryAppState) -> Result<impl warp::Reply, warp::Rejection> {
    println!("📊 Революционные метрики системы");
    
    let metrics = state.central_system.get_system_metrics().await;
    
    Ok(warp::reply::json(&metrics))
}

/// Революционная главная страница
async fn handle_index_revolutionary(state: RevolutionaryAppState) -> Result<impl warp::Reply, warp::Rejection> {
    println!("🏠 Революционная главная страница");
    
    let metrics = state.central_system.get_system_metrics().await;
    let health = state.central_system.health_check().await;
    
    let html = generate_revolutionary_index_html(&metrics, &health);
    Ok(warp::reply::html(html))
}

/// Революционная страница иерархии
async fn handle_hierarchy_page_revolutionary(state: RevolutionaryAppState) -> Result<impl warp::Reply, warp::Rejection> {
    println!("🌳 Революционная страница иерархии");
    
    match state.central_system.web_interface().handle_hierarchy_request().await {
        Ok(hierarchy_data) => {
            let html = generate_revolutionary_hierarchy_html(&hierarchy_data);
            Ok(warp::reply::html(html))
        }
        Err(e) => {
            let error_html = format!(
                "<html><body><h1>❌ Ошибка</h1><p>Не удалось загрузить иерархию: {}</p></body></html>", 
                e
            );
            Ok(warp::reply::html(error_html))
        }
    }
}

/// Ошибка API
#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    code: u16,
}

/// Генерация революционной главной страницы
fn generate_revolutionary_index_html(metrics: &bsl_gradual_types::ideal::system::SystemMetrics, health: &bsl_gradual_types::ideal::system::HealthStatus) -> String {
    format!(r#"
<!DOCTYPE html>
<html lang="ru">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>🚀 Революционный BSL Type Browser</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ 
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; 
            background: #1e1e1e; 
            color: #d4d4d4; 
            line-height: 1.6;
        }}
        .container {{ max-width: 1200px; margin: 0 auto; padding: 20px; }}
        .header {{ text-align: center; margin-bottom: 40px; }}
        .header h1 {{ color: #569cd6; font-size: 2.5em; margin-bottom: 10px; }}
        .header p {{ color: #9cdcfe; font-size: 1.2em; }}
        
        .revolution-banner {{
            background: linear-gradient(135deg, #ff6b35, #f7931e);
            color: white;
            padding: 20px;
            border-radius: 10px;
            margin-bottom: 30px;
            text-align: center;
        }}
        
        .metrics-grid {{ 
            display: grid; 
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); 
            gap: 20px; 
            margin-bottom: 40px; 
        }}
        .metric-card {{ 
            background: #2d2d30; 
            padding: 20px; 
            border-radius: 10px; 
            border-left: 4px solid #4ec9b0;
            text-align: center; 
        }}
        .metric-value {{ color: #4fc1ff; font-size: 2em; font-weight: bold; }}
        .metric-label {{ color: #9cdcfe; margin-top: 5px; }}
        
        .health-status {{
            background: {};
            color: white;
            padding: 15px;
            border-radius: 8px;
            margin-bottom: 20px;
            text-align: center;
            font-weight: bold;
        }}
        
        .navigation {{
            display: flex;
            gap: 20px;
            margin-top: 30px;
            justify-content: center;
        }}
        .nav-button {{
            background: #0e639c;
            color: white;
            padding: 15px 30px;
            border-radius: 8px;
            text-decoration: none;
            font-weight: bold;
            transition: background 0.3s;
        }}
        .nav-button:hover {{ background: #1177bb; }}
        
        .architecture-info {{
            background: #2d2d30;
            padding: 20px;
            border-radius: 10px;
            margin-top: 30px;
            border-left: 4px solid #569cd6;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🚀 Революционный BSL Type Browser</h1>
            <p>Идеальная слоистая архитектура для системы типов 1С:Предприятие</p>
        </div>
        
        <div class="revolution-banner">
            <h2>🏗️ РЕВОЛЮЦИОННАЯ АРХИТЕКТУРА АКТИВНА!</h2>
            <p>Data → Domain → Application → Presentation слои работают как единая система</p>
        </div>
        
        <div class="health-status">
            🏥 Статус системы: {} (оценка: {:.1}/10)
        </div>
        
        <div class="metrics-grid">
            <div class="metric-card">
                <div class="metric-value">{}</div>
                <div class="metric-label">Всего типов</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{}</div>
                <div class="metric-label">Платформенных</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{}</div>
                <div class="metric-label">Конфигурационных</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{:.1} MB</div>
                <div class="metric-label">Память</div>
            </div>
        </div>
        
        <div class="navigation">
            <a href="/hierarchy" class="nav-button">🌳 Иерархия типов</a>
            <a href="/api/v2/hierarchy" class="nav-button">📊 API v2</a>
            <a href="/api/v2/health" class="nav-button">🏥 Здоровье</a>
            <a href="/api/v2/metrics" class="nav-button">📈 Метрики</a>
        </div>
        
        <div class="architecture-info">
            <h3>🏗️ Архитектурные преимущества:</h3>
            <ul>
                <li>✅ <strong>Single Source of Truth</strong> - все типы в едином репозитории</li>
                <li>✅ <strong>Слоистая архитектура</strong> - четкое разделение ответственности</li>
                <li>✅ <strong>TreeSitter парсер</strong> - полноценный BSL парсинг</li>
                <li>✅ <strong>Dependency Inversion</strong> - зависимость от абстракций</li>
                <li>✅ <strong>Производительность</strong> - единая инициализация вместо множественной</li>
                <li>✅ <strong>Тестируемость</strong> - каждый слой тестируется независимо</li>
            </ul>
        </div>
    </div>
</body>
</html>
    "#,
        // Цвет статуса здоровья
        match health.status.as_str() {
            "healthy" => "#4CAF50",
            "degraded" => "#FF9800", 
            "unhealthy" => "#F44336",
            _ => "#9E9E9E"
        },
        // Статус и оценка
        health.status.to_uppercase(),
        health.overall_score * 10.0,
        // Метрики
        metrics.total_types,
        metrics.platform_types,
        metrics.configuration_types,
        metrics.cache_memory_mb
    )
}

/// Генерация революционной страницы иерархии
fn generate_revolutionary_hierarchy_html(hierarchy: &WebHierarchyResponse) -> String {
    let mut html = String::new();
    
    html.push_str(r#"
<!DOCTYPE html>
<html lang="ru">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>🌳 Революционная иерархия типов BSL</title>
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
        
        .revolution-badge {
            background: linear-gradient(135deg, #ff6b35, #f7931e);
            color: white;
            padding: 10px 20px;
            border-radius: 20px;
            display: inline-block;
            margin-bottom: 20px;
            font-weight: bold;
        }
        
        .stats { 
            background: #2d2d30; 
            padding: 20px; 
            border-radius: 10px; 
            margin-bottom: 30px;
            border-left: 4px solid #4ec9b0;
        }
        .stats h3 { color: #4ec9b0; margin-bottom: 15px; }
        .stats-grid { 
            display: grid; 
            grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); 
            gap: 15px; 
        }
        .stat-item { text-align: center; }
        .stat-value { color: #4fc1ff; font-size: 1.5em; font-weight: bold; }
        .stat-label { color: #9cdcfe; font-size: 0.9em; }
        
        .categories { margin-top: 30px; }
        .category-item { 
            background: #2d2d30; 
            border: 1px solid #3c3c3c; 
            border-radius: 10px; 
            margin-bottom: 20px; 
            padding: 20px;
            transition: all 0.3s ease;
        }
        .category-item:hover { 
            border-color: #569cd6; 
            transform: translateY(-2px);
            box-shadow: 0 4px 12px rgba(86, 156, 214, 0.3);
        }
        .category-title { color: #569cd6; font-size: 1.3em; margin-bottom: 10px; }
        .category-description { color: #d4d4d4; margin-bottom: 15px; }
        .category-stats { 
            display: flex; 
            gap: 20px; 
            color: #9cdcfe; 
            font-size: 0.9em; 
        }
        
        .back-link {
            display: inline-block;
            background: #0e639c;
            color: white;
            padding: 10px 20px;
            border-radius: 5px;
            text-decoration: none;
            margin-bottom: 20px;
        }
        .back-link:hover { background: #1177bb; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <div class="revolution-badge">🚀 РЕВОЛЮЦИОННАЯ АРХИТЕКТУРА</div>
            <h1>🌳 Иерархия типов BSL</h1>
            <p>Через CentralTypeSystem с идеальной слоистой архитектурой</p>
        </div>
        
        <a href="/" class="back-link">← Вернуться на главную</a>
        
        <div class="stats">
            <h3>📊 Статистика революционной системы</h3>
            <div class="stats-grid">
                <div class="stat-item">
                    <div class="stat-value">{}</div>
                    <div class="stat-label">Категорий</div>
                </div>
                <div class="stat-item">
                    <div class="stat-value">{}</div>
                    <div class="stat-label">Всего типов</div>
                </div>
                <div class="stat-item">
                    <div class="stat-value">{}</div>
                    <div class="stat-label">Платформенных</div>
                </div>
                <div class="stat-item">
                    <div class="stat-value">{}</div>
                    <div class="stat-label">Конфигурационных</div>
                </div>
            </div>
        
        <div class="categories">"#,
        categories_count,
        total_types,  
        platform_types,
        configuration_types
        </div>
        
        <div class="categories">
    "#);
    
    let categories_count = hierarchy.categories.len();
    let total_types = hierarchy.total_types;
    let platform_types = hierarchy.statistics.platform_types;
    let configuration_types = hierarchy.statistics.configuration_types;
    
    // Добавляем категории
    for (i, category) in hierarchy.categories.iter().enumerate() {
        html.push_str(&format!(
            r#"
            <div class="category-item">
                <div class="category-title">📂 {}. {}</div>
                <div class="category-description">{}</div>
                <div class="category-stats">
                    <span>📊 Типов: {}</span>
                    <span>📁 Подкатегорий: {}</span>
                    <span>🔗 <a href="{}" style="color: #4fc1ff;">Подробнее</a></span>
                </div>
            </div>
            "#,
            i + 1,
            category.name,
            category.description,
            category.types_count,
            category.subcategories_count,
            category.url
        ));
    }
    
    html.push_str("</div></div></body></html>");
    
    html
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_revolutionary_web_server_creation() {
        let config = CentralSystemConfig::default();
        let central_system = Arc::new(CentralTypeSystem::new(config));
        
        let app_state = RevolutionaryAppState {
            central_system: central_system.clone(),
        };
        
        // Тестируем создание состояния
        let _web_interface = app_state.central_system.web_interface();
    }
}