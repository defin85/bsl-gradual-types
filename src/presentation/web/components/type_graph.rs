//! Type Graph - сетевое представление типов

#[cfg(feature = "web-ui")]
use leptos::*;

#[cfg(feature = "web-ui")]
use crate::{
    domain::types::TypeResolution,
    presentation::web::components::api::{ApiClient, ApiError},
    presentation::web::components::type_table::CategoryExt, // Используем трейт для получения категории
};
use std::collections::HashMap;

#[cfg(feature = "web-ui")]
#[component]
pub fn TypeGraph() -> impl IntoView {
    let types_resource = create_resource(
        || (),
        |_| async move { ApiClient::get_types().await }
    );

    view! {
        <div class="container">
            <div class="header">
                <h1>"🕸️ BSL Type Network"</h1>
                <p>"Interactive graph visualization of type relationships, dependencies, and flow-sensitive analysis"</p>
            </div>
            <div class="main-content">
                <div class="graph-container">
                    <Suspense fallback=move || view!{<p>"Загрузка графа..."</p>}>
                        {move || {
                            types_resource.get().map(|res| match res {
                                Ok(types) => view! { <StaticGraphRenderer types=types/> }.into_view(),
                                Err(e) => view! { <p>{format!("Ошибка загрузки данных для графа: {:?}", e)}</p> }.into_view(),
                            })
                        }}
                    </Suspense>
                </div>
                <Sidebar />
            </div>
            <style>{ GRAPH_STYLES }</style>
        </div>
    }
}

#[cfg(feature = "web-ui")]
#[component]
fn StaticGraphRenderer(types: Vec<TypeResolution>) -> impl IntoView {
    let memo_types = types.clone();
    let nodes = create_memo(move |_| {
        let mut positions = HashMap::new();
        let count = memo_types.len();
        if count == 0 { return positions; }

        let radius = 250.0;
        let center_x = 400.0;
        let center_y = 300.0;

        for (i, t) in memo_types.iter().enumerate() {
            let angle = 2.0 * std::f64::consts::PI / count as f64 * i as f64;
            let x = center_x + radius * angle.cos();
            let y = center_y + radius * angle.sin();
            positions.insert(t.result.to_string(), (x, y));
        }
        positions
    });

    view! {
        <svg class="graph-svg" id="graphSvg" viewBox="0 0 800 600">
            <For
                each=move || types.clone()
                key=|t| t.result.to_string()
                children=move |t| {
                    let (x, y) = nodes.get().get(&t.result.to_string()).cloned().unwrap_or((0.0, 0.0));
                    let category = t.result.category_str();
                    let color = match category {
                        "Platform" => "#3b82f6",
                        "Configuration" => "#10b981",
                        "Union" => "#f59e0b",
                        "Dynamic" => "#ef4444",
                        _ => "#6b7280",
                    };

                    view! {
                        <g transform=format!("translate({},{})", x, y)>
                            <circle class="node-circle" r="30" fill=color />
                            <text class="node-label" y="5">{t.result.to_string()}</text>
                        </g>
                    }
                }
            />
        </svg>
    }
}

#[cfg(feature = "web-ui")]
#[component]
fn Sidebar() -> impl IntoView {
    view! {
        <div class="sidebar">
            <div class="node-info">
                <div class="node-name">"📋 Selected Type Details"</div>
                <p>"Кликните на узел для просмотра информации"</p>
            </div>
            <div class="legend">
                <div class="legend-title">"🎨 Type Categories"</div>
                <div class="legend-item"><div class="legend-color color-platform"></div><span>"Platform"</span></div>
                <div class="legend-item"><div class="legend-color color-config"></div><span>"Configuration"</span></div>
                <div class="legend-item"><div class="legend-color color-union"></div><span>"Union"</span></div>
                <div class="legend-item"><div class="legend-color color-dynamic"></div><span>"Dynamic"</span></div>
                <div class="legend-item"><div class="legend-color color-unknown"></div><span>"Unknown"</span></div>
            </div>
        </div>
    }
}

const GRAPH_STYLES: &str = "
body { background: #1a1a2e; color: white; overflow: hidden; }
.container { height: 100vh; display: flex; flex-direction: column; }
.header { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); padding: 20px; box-shadow: 0 2px 20px rgba(0,0,0,0.3); z-index: 1000; }
.main-content { flex: 1; display: flex; position: relative; }
.graph-container { flex: 1; position: relative; background: radial-gradient(circle at center, #16213e 0%, #0f172a 100%); }
.sidebar { width: 350px; background: rgba(30, 41, 59, 0.95); padding: 20px; overflow-y: auto; backdrop-filter: blur(10px); border-left: 1px solid rgba(255,255,255,0.1); }
.node-info, .legend { background: rgba(255,255,255,0.05); border-radius: 10px; padding: 20px; border: 1px solid rgba(255,255,255,0.1); margin-bottom: 20px; }
.node-name, .legend-title { font-weight: bold; margin-bottom: 15px; color: #fbbf24; }
.legend-item { display: flex; align-items: center; margin-bottom: 10px; }
.legend-color { width: 20px; height: 20px; border-radius: 50%; margin-right: 10px; }
.color-platform { background: #3b82f6; }
.color-config { background: #10b981; }
.color-union { background: #f59e0b; }
.color-dynamic { background: #ef4444; }
.color-unknown { background: #6b7280; }
.graph-svg { width: 100%; height: 100%; }
.node-circle { transition: all 0.3s ease; }
.node-label { font-size: 10px; fill: white; text-anchor: middle; pointer-events: none; font-weight: 500; }
";