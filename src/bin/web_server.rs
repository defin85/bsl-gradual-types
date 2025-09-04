//! Web Server for BSL Type Browser - Simple Architecture
//!
//! Простой веб-интерфейс для просмотра типов согласно simplified_architecture.md

use anyhow::Result;
use bsl_gradual_types::system::SystemCoordinator;
use clap::Parser;
use std::sync::Arc;
use tracing::info;
use warp::Filter;

#[derive(Parser, Debug)]
#[command(name = "web-server")]
#[command(about = "BSL Type Browser - Simple Web Interface")]
struct Args {
    /// Port to run the web server on
    #[arg(short, long, default_value = "8080")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    let args = Args::parse();
    
    info!("🌐 Starting BSL Type Browser on port {}", args.port);
    
    // Создаём SystemCoordinator согласно simplified architecture
    let _coordinator = Arc::new(SystemCoordinator::new());
    
    // Простой HTML dashboard
    let index_route = warp::path::end()
        .map(|| {
            warp::reply::html(r#"
<!DOCTYPE html>
<html>
<head>
    <title>BSL Type Browser</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .header { color: #333; border-bottom: 1px solid #ddd; padding-bottom: 20px; }
        .status { background: #f0f8f0; padding: 15px; border-radius: 5px; margin: 20px 0; }
    </style>
</head>
<body>
    <div class="header">
        <h1>🎯 BSL Gradual Type System</h1>
        <p>Simple Web Interface - Simplified Architecture</p>
    </div>
    
    <div class="status">
        <h2>✅ System Status</h2>
        <p><strong>Architecture:</strong> Simplified (6-8 components)</p>
        <p><strong>Status:</strong> Running</p>
        <p><strong>Components:</strong> SystemCoordinator, AnalysisCache, ParserCoordinator, TypeSystemService</p>
    </div>
    
    <h2>🔍 Available APIs</h2>
    <ul>
        <li><a href="/health">Health Check</a></li>
        <li><a href="/api/types">Browse Types</a></li>
        <li><a href="/api/search?q=Массив">Search Types</a></li>
    </ul>
</body>
</html>
            "#)
        });
    
    // Health endpoint
    let health_route = warp::path("health")
        .map(|| {
            warp::reply::json(&serde_json::json!({
                "status": "healthy",
                "architecture": "simplified",
                "components": 6
            }))
        });
    
    // API routes
    let api_routes = warp::path("api")
        .and(
            warp::path("types")
                .map(|| warp::reply::json(&serde_json::json!({
                    "message": "BSL Type Browser API",
                    "architecture": "simplified"
                })))
        );
    
    let routes = index_route
        .or(health_route)
        .or(api_routes);
    
    info!("🚀 Web server running at http://localhost:{}", args.port);
    
    warp::serve(routes)
        .run(([127, 0, 0, 1], args.port))
        .await;
    
    Ok(())
}
