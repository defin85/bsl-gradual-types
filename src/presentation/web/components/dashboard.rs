//! Dashboard - исполнительная панель с метриками

#[cfg(feature = "web-ui")]
use leptos::*;

#[cfg(feature = "web-ui")]
use crate::presentation::web::components::api::{ApiClient, DashboardMetrics};
#[cfg(feature = "web-ui")]
use crate::presentation::web::components::common::MetricCard;

#[cfg(feature = "web-ui")]
#[component]
pub fn Dashboard() -> impl IntoView {
    let (metrics, set_metrics) = create_signal(None::<DashboardMetrics>);
    let (loading, set_loading) = create_signal(true);
    
    // Загружаем данные при монтировании
    create_effect(move |_| {
        spawn_local(async move {
            match ApiClient::get_dashboard_metrics().await {
                Ok(data) => {
                    set_metrics.set(Some(data));
                    set_loading.set(false);
                },
                Err(e) => {
                    log::error!("Failed to load metrics: {}", e);
                    set_loading.set(false);
                }
            }
        });
    });
    
    view! {
        <div class="dashboard">
            <div class="dashboard-header">
                <h1>"🎯 BSL Type System Dashboard"</h1>
                <p>"Gradual Typing with Facets & Flow-Sensitive Analysis"</p>
            </div>
            
            {move || {
                if loading.get() {
                    view! {
                        <div class="loading">
                            <div class="spinner"></div>
                            <p>"Загрузка метрик..."</p>
                        </div>
                    }.into_view()
                } else if let Some(m) = metrics.get() {
                    view! {
                        <div class="metrics-grid">
                            <MetricCard
                                title="Total Types"
                                value=m.total_types
                                icon="📊"
                                color="blue"
                            />
                            <MetricCard
                                title="Known Types"
                                value=m.known_types
                                icon="✅"
                                color="green"
                            />
                            <MetricCard
                                title="Inferred Types"
                                value=m.inferred_types
                                icon="🔍"
                                color="yellow"
                            />
                            <MetricCard
                                title="Unknown Types"
                                value=m.unknown_types
                                icon="❓"
                                color="red"
                            />
                            <MetricCard
                                title="Flow-Sensitive"
                                value=m.flow_sensitive_types
                                icon="🔄"
                                color="purple"
                            />
                        </div>
                        
                        <TypeOverview />
                        <RecentActivity />
                    }.into_view()
                } else {
                    view! {
                        <div class="error">
                            <p>"Ошибка загрузки данных"</p>
                        </div>
                    }.into_view()
                }
            }}
            
            <style>
                ".dashboard {
                    max-width: 1400px;
                    margin: 0 auto;
                }
                
                .dashboard-header {
                    text-align: center;
                    margin-bottom: 40px;
                    padding: 30px;
                    background: white;
                    border-radius: 12px;
                    box-shadow: 0 4px 20px rgba(0,0,0,0.1);
                }
                
                .dashboard-header h1 {
                    font-size: 2.5em;
                    margin-bottom: 10px;
                    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                    -webkit-background-clip: text;
                    -webkit-text-fill-color: transparent;
                }
                
                .dashboard-header p {
                    color: #6c757d;
                    font-size: 1.2em;
                }
                
                .metrics-grid {
                    display: grid;
                    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
                    gap: 20px;
                    margin-bottom: 40px;
                }
                
                .loading {
                    text-align: center;
                    padding: 60px;
                }
                
                .spinner {
                    width: 40px;
                    height: 40px;
                    border: 4px solid #f3f3f3;
                    border-top: 4px solid #007bff;
                    border-radius: 50%;
                    animation: spin 1s linear infinite;
                    margin: 0 auto 20px;
                }
                
                @keyframes spin {
                    0% { transform: rotate(0deg); }
                    100% { transform: rotate(360deg); }
                }
                
                .error {
                    text-align: center;
                    padding: 40px;
                    background: #f8d7da;
                    border-radius: 8px;
                    color: #721c24;
                }"
            </style>
        </div>
    }
}

#[cfg(feature = "web-ui")]
#[component]
fn TypeOverview() -> impl IntoView {
    view! {
        <div class="type-overview">
            <h2>"🎭 Type Categories"</h2>
            <div class="overview-grid">
                <div class="overview-section">
                    <h3>"🔧 Platform Types"</h3>
                    <p>"Массив, Строка, Число, Булево"</p>
                </div>
                <div class="overview-section">
                    <h3>"⚙️ Configuration Types"</h3>
                    <p>"Справочники, Документы, Регистры"</p>
                </div>
                <div class="overview-section">
                    <h3>"🎯 Union Types"</h3>
                    <p>"Градуальная типизация с весами"</p>
                </div>
                <div class="overview-section">
                    <h3>"🔄 Flow-Sensitive"</h3>
                    <p>"Типы, изменяющиеся по потоку"</p>
                </div>
            </div>
            
            <style>
                ".type-overview {
                    background: white;
                    padding: 30px;
                    border-radius: 12px;
                    box-shadow: 0 4px 20px rgba(0,0,0,0.1);
                    margin-bottom: 30px;
                }
                
                .type-overview h2 {
                    margin-bottom: 20px;
                    color: #2c3e50;
                }
                
                .overview-grid {
                    display: grid;
                    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
                    gap: 20px;
                }
                
                .overview-section {
                    padding: 20px;
                    background: #f8f9fa;
                    border-radius: 8px;
                    border-left: 4px solid #007bff;
                }
                
                .overview-section h3 {
                    margin-bottom: 10px;
                    color: #495057;
                }
                
                .overview-section p {
                    color: #6c757d;
                }"
            </style>
        </div>
    }
}

#[cfg(feature = "web-ui")]
#[component]
fn RecentActivity() -> impl IntoView {
    view! {
        <div class="recent-activity">
            <h2>"📈 Architecture Health"</h2>
            <div class="health-stats">
                <div class="health-item">
                    <span class="health-label">"Components:"</span>
                    <span class="health-value">"6/8 active"</span>
                </div>
                <div class="health-item">
                    <span class="health-label">"Cache Hit Rate:"</span>
                    <span class="health-value">"94%"</span>
                </div>
                <div class="health-item">
                    <span class="health-label">"Analysis Speed:"</span>
                    <span class="health-value">"125ms avg"</span>
                </div>
            </div>
            
            <style>
                ".recent-activity {
                    background: white;
                    padding: 30px;
                    border-radius: 12px;
                    box-shadow: 0 4px 20px rgba(0,0,0,0.1);
                }
                
                .recent-activity h2 {
                    margin-bottom: 20px;
                    color: #2c3e50;
                }
                
                .health-stats {
                    display: flex;
                    gap: 30px;
                    flex-wrap: wrap;
                }
                
                .health-item {
                    display: flex;
                    flex-direction: column;
                    gap: 5px;
                }
                
                .health-label {
                    color: #6c757d;
                    font-size: 0.9em;
                }
                
                .health-value {
                    font-weight: bold;
                    font-size: 1.2em;
                    color: #28a745;
                }"
            </style>
        </div>
    }
}
