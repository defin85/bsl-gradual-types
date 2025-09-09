//! Dashboard - исполнительная панель с метриками

#[cfg(feature = "web-ui")]
use leptos::*;

#[cfg(feature = "web-ui")]
use crate::presentation::web::components::api::{ApiClient, DashboardMetrics, CategoryType, ArchitectureHealth};

#[cfg(feature = "web-ui")]
use crate::presentation::web::components::common::MetricCard;

#[cfg(feature = "web-ui")]
#[component]
pub fn Dashboard() -> impl IntoView {
    let metrics = create_resource(
        || (),
        |_| async move { ApiClient::get_dashboard_metrics().await }
    );

    let metrics_percentages = create_memo(move |_| {
        metrics.get().and_then(|res| res.ok()).map(|m| {
            let total = m.known_types + m.inferred_types + m.unknown_types;
            if total == 0 {
                return (0, 0, 0);
            }
            let known_percent = (m.known_types * 100) / total;
            let inferred_percent = (m.inferred_types * 100) / total;
            let unknown_percent = 100 - known_percent - inferred_percent;
            (known_percent, inferred_percent, unknown_percent)
        })
    });

    view! {
        <div class="container">
            <div class="header">
                <h1>"🎯 BSL Gradual Type System"</h1>
                <p>"<strong>Executive Dashboard</strong> - Overview of Type Analysis"</p>
                <p>"Simplified Architecture | Real-time Type Intelligence"</p>
            </div>

            <Suspense
                fallback=move || view! { <div class="loading">"Загрузка..."</div> }
            >
                {move || {
                    metrics.get().map(|res| match res {
                        Ok(m) => {
                            let (known, inferred, unknown) = metrics_percentages.get().unwrap_or((0,0,0));
                            view! {
                                <div>
                                    <div class="dashboard-grid">
                                        <MetricCard title="Known Types" value=known icon="✅" color="green" />
                                        <MetricCard title="Inferred Types" value=inferred icon="🔍" color="yellow" />
                                        <MetricCard title="Unknown Types" value=unknown icon="❓" color="red" />
                                    </div>
                                    <TypeCategories metrics=m.clone() />
                                    <ArchitectureHealth health=m.health.clone() />
                                </div>
                            }.into_view()
                        },
                        Err(_) => view! { <div class="error">"Ошибка загрузки данных"</div> }.into_view()
                    })
                }}
            </Suspense>

            <style>
                {
                    "body {
                        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                    }
                    .container {
                        max-width: 1400px;
                        margin: 0 auto;
                        padding: 20px;
                    }
                    .header {
                        background: rgba(255, 255, 255, 0.95);
                        border-radius: 12px;
                        padding: 30px;
                        margin-bottom: 30px;
                        box-shadow: 0 8px 32px rgba(0, 0, 0, 0.1);
                        text-align: center;
                    }
                    .dashboard-grid {
                        display: grid;
                        grid-template-columns: 1fr 1fr 1fr;
                        gap: 20px;
                        margin-bottom: 30px;
                    }
                    .loading, .error {
                        text-align: center;
                        padding: 50px;
                        background: white;
                        border-radius: 12px;
                        color: black;
                    }"
                }
            </style>
        </div>
    }
}

#[cfg(feature = "web-ui")]
#[component]
fn TypeCategories(metrics: DashboardMetrics) -> impl IntoView {
    view! {
        <div class="type-overview">
            <h2>"🎭 Type Categories"</h2>
            <div class="categories-grid">
                <CategoryColumn title="🔧 Platform Types" types=metrics.platform_types />
                <CategoryColumn title="⚙️ Configuration Types" types=metrics.config_types />
                <CategoryColumn title="🎯 Union Types" types=metrics.union_types />
                <CategoryColumn title="🔄 Flow-Sensitive Analysis" types=metrics.flow_sensitive_types />
            </div>
            <style>
                {
                    ".type-overview {
                        background: rgba(255, 255, 255, 0.95);
                        border-radius: 12px;
                        padding: 30px;
                        box-shadow: 0 8px 32px rgba(0, 0, 0, 0.1);
                        margin-bottom: 30px;
                    }
                    .categories-grid {
                        display: grid;
                        grid-template-columns: 1fr 1fr;
                        gap: 30px;
                        margin-top: 20px;
                    }"
                }
            </style>
        </div>
    }
}

#[cfg(feature = "web-ui")]
#[component]
fn CategoryColumn(title: &'static str, types: Vec<CategoryType>) -> impl IntoView {
    view! {
        <div>
            <h3>{title}</h3>
            {types.into_iter().map(|t| view! {
                <p>
                    <span class="facet-indicator"></span>
                    <strong>{t.name}</strong>" - "{t.description}
                </p>
            }).collect_view()}
            <style>
            {
                ".facet-indicator {
                    display: inline-block;
                    width: 12px;
                    height: 12px;
                    border-radius: 50%;
                    margin-right: 8px;
                    background: #007bff;
                }"
            }
            </style>
        </div>
    }
}

#[cfg(feature = "web-ui")]
#[component]
fn ArchitectureHealth(health: ArchitectureHealth) -> impl IntoView {
    view! {
        <div class="health-check">
            <h3>"🏗️ Architecture Health"</h3>
            <p>
                {format!(
                    "Components: {}/{} active | Cache Hit Rate: {}% | Analysis Speed: {}ms avg",
                    health.components_active,
                    health.components_total,
                    health.cache_hit_rate,
                    health.analysis_speed_ms
                )}
            </p>
            <style>
                {
                    ".health-check {
                        margin-top: 30px;
                        padding: 20px;
                        background: #f8f9fa;
                        border-radius: 8px;
                    }"
                }
            </style>
        </div>
    }
}