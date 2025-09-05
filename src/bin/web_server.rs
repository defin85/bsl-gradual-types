//! Простой веб-сервер для BSL Type System

#[cfg(feature = "web-ui")]
use axum::{
    routing::get,
    Router,
    Json,
};
#[cfg(feature = "web-ui")]
use serde_json::{json, Value};

#[cfg(feature = "web-ui")]
#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(index))
        .route("/api/metrics", get(get_metrics))
        .route("/api/types", get(get_types))
        .route("/api/search", get(search_types));

    let addr = "127.0.0.1:8080";
    println!("🚀 BSL Type System Web UI listening on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .await
        .unwrap();
}

#[cfg(not(feature = "web-ui"))]
#[tokio::main]
async fn main() {
    println!("Web UI feature is not enabled. Use --features web-ui to enable.");
}

#[cfg(feature = "web-ui")]
async fn index() -> &'static str {
    r#"
<!DOCTYPE html>
<html>
<head>
    <title>BSL Type System</title>
    <style>
        body { font-family: Arial, sans-serif; padding: 40px; background: #f5f7fa; }
        .container { max-width: 800px; margin: 0 auto; background: white; padding: 40px; border-radius: 10px; }
        h1 { color: #333; margin-bottom: 20px; }
        p { color: #666; line-height: 1.6; }
        .btn { display: inline-block; background: #007bff; color: white; padding: 10px 20px; text-decoration: none; border-radius: 5px; margin-top: 20px; }
    </style>
</head>
<body>
    <div class="container">
        <h1>🚀 BSL Type System</h1>
        <p>Gradual Type System для 1C:Enterprise BSL</p>
        <p>Leptos компоненты готовы, но требуют дополнительной настройки сборки для WebAssembly.</p>
        <p>Пока что доступны API эндпоинты:</p>
        <ul>
            <li><a href="/api/metrics">/api/metrics</a> - метрики типов</li>
            <li><a href="/api/types">/api/types</a> - список типов</li>
            <li><a href="/api/search">/api/search</a> - поиск типов</li>
        </ul>
        <a href="/api/metrics" class="btn">Посмотреть метрики</a>
    </div>
</body>
</html>
    "#
}

#[cfg(feature = "web-ui")]
async fn get_metrics() -> Json<Value> {
    // Заглушка для метрик - в реальности здесь будет вызов SystemCoordinator
    Json(json!({
        "total_types": 87,
        "known_types": 76,
        "inferred_types": 8,
        "unknown_types": 3,
        "flow_sensitive_types": 23,
        "status": "ok"
    }))
}

#[cfg(feature = "web-ui")]
async fn get_types() -> Json<Value> {
    // Заглушка для типов - в реальности здесь будет получение данных из TypeSystemService
    Json(json!({
        "types": [
            {
                "name": "Массив",
                "category": "Platform",
                "certainty": 100,
                "facets": ["Object", "Collection"]
            },
            {
                "name": "Справочники.Номенклатура", 
                "category": "Configuration",
                "certainty": 100,
                "facets": ["Manager", "Reference", "Object", "Metadata"]
            }
        ],
        "count": 2
    }))
}

#[cfg(feature = "web-ui")]
async fn search_types() -> Json<Value> {
    // Заглушка для поиска
    Json(json!({
        "results": [],
        "query": "",
        "count": 0
    }))
}
