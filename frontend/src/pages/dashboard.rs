//! Dashboard page

use crate::api::*;

use leptos::prelude::*;
use leptos::task::spawn_local;

/// Страница дашборда с метриками системы типизации
#[component]
#[allow(non_snake_case)]
pub fn Dashboard(
    /// Поисковый запрос для фильтрации
    #[prop(optional)] _search_query: Option<RwSignal<String>>,
) -> impl IntoView {
    let metrics = RwSignal::new(None::<MetricsDto>);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    // Загружаем метрики при монтировании компонента
    let load_metrics = move || {
        loading.set(true);
        error.set(None);
        
        spawn_local(async move {
            match fetch_metrics().await {
                Ok(result) => {
                    metrics.set(Some(result));
                    loading.set(false);
                },
                Err(err) => {
                    error.set(Some(err));
                    loading.set(false);
                }
            }
        });
    };

    Effect::new(move |_| {
        load_metrics();
    });

    view! {
        <main class="main-content">
            <div class="dashboard-header">
                <h1>"🎯 BSL Gradual Type System"</h1>
                <p><strong>"Executive Dashboard"</strong>" - Overview of Type Analysis"</p>
                <p>"Simplified Architecture | Real-time Type Intelligence"</p>
            </div>

            {move || {
                if loading.get() {
                    view! {
                        <div class="loading">
                            <p>"🔄 Загрузка метрик..."</p>
                        </div>
                    }.into_any()
                } else if let Some(err) = error.get() {
                    view! {
                        <div class="error">
                            <p>"❌ Ошибка загрузки: " {err}</p>
                            <button on:click=move |_| load_metrics()>"Повторить"</button>
                        </div>
                    }.into_any()
                } else if let Some(m) = metrics.get() {
                    let total = m.known_types() + m.inferred_types() + m.unknown_types();
                    let known_percentage = if total > 0 {
                        (m.known_types() as f32 / total as f32 * 100.0) as u32
                    } else { 0 };
                    let inferred_percentage = if total > 0 {
                        (m.inferred_types() as f32 / total as f32 * 100.0) as u32
                    } else { 0 };
                    let unknown_percentage = if total > 0 {
                        (m.unknown_types() as f32 / total as f32 * 100.0) as u32
                    } else { 0 };
                    
                    view! {
                        <div>
                            <div class="dashboard-grid">
                                <div class="metric-card">
                                    <div class="metric-number" style="color: #28a745;">{known_percentage}"%"</div>
                                    <div class="metric-label">"Known Types"</div>
                                    <div class="certainty-bar">
                                        <div class="certainty-fill" style=format!("width: {}%; background: linear-gradient(90deg, #28a745, #20c997);", known_percentage)></div>
                                    </div>
                                </div>
                                
                                <div class="metric-card">
                                    <div class="metric-number" style="color: #ffc107;">{inferred_percentage}"%"</div>
                                    <div class="metric-label">"Inferred Types"</div>
                                    <div class="certainty-bar">
                                        <div class="certainty-fill" style=format!("width: {}%; background: linear-gradient(90deg, #ffc107, #fd7e14);", inferred_percentage)></div>
                                    </div>
                                </div>
                                
                                <div class="metric-card">
                                    <div class="metric-number" style="color: #dc3545;">{unknown_percentage}"%"</div>
                                    <div class="metric-label">"Unknown Types"</div>
                                    <div class="certainty-bar">
                                        <div class="certainty-fill" style=format!("width: {}%; background: linear-gradient(90deg, #dc3545, #e83e8c);", unknown_percentage)></div>
                                    </div>
                                </div>
                            </div>

                            <div class="type-overview" style="background: rgba(255, 255, 255, 0.95); border-radius: 12px; padding: 30px; box-shadow: 0 8px 32px rgba(0, 0, 0, 0.1);">
                                <h2>"🎭 Type Categories"</h2>

                                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 30px; margin-top: 20px;">
                                    <div>
                                        <h3>"🔧 Platform Types"</h3>
                                        <p><span class="facet-indicator" style="background: #007bff;"></span>"Массив (Array) - 95% certainty"</p>
                                        <p><span class="facet-indicator" style="background: #28a745;"></span>"Соответствие (Map) - 90% certainty"</p>
                                        <p><span class="facet-indicator" style="background: #17a2b8;"></span>"СписокЗначений - 98% certainty"</p>

                                        <h3 style="margin-top: 20px;">"⚙️ Configuration Types"</h3>
                                        <p><span class="facet-indicator" style="background: #ffc107;"></span>"Справочники.Номенклатура - Known"</p>
                                        <p><span class="facet-indicator" style="background: #28a745;"></span>"Документы.ПоступлениеТоваров - Known"</p>
                                    </div>

                                    <div>
                                        <h3>"🎯 Union Types"</h3>
                                        <p><strong>"ТипЗначения:"</strong>" Строка (60%) | Число (40%)"</p>
                                        <p><strong>"РезультатОбработки:"</strong>" Булево (70%) | Неопределено (30%)"</p>

                                        <h3 style="margin-top: 20px;">"🔄 Flow-Sensitive Analysis"</h3>
                                        <p><strong>"Переменная1:"</strong>" Неопределено → Строка"</p>
                                        <p><strong>"Результат:"</strong>" Неопределено → Число → Строка"</p>
                                    </div>
                                </div>

                                <div style="margin-top: 30px; padding: 20px; background: #f8f9fa; border-radius: 8px;">
                                    <h3>"🏗️ Architecture Health"</h3>
                                    <p>
                                        <strong>"Components:"</strong>" 6/8 active | "
                                        <strong>"Cache Hit Rate:"</strong>" " {m.cache_hit_rate.clone()} " | "
                                        <strong>"Analysis Speed:"</strong>" " {format!("{:.0}ms avg", m.analysis_speed_ms())}
                                    </p>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div class="loading">
                            <p>"Инициализация..."</p>
                        </div>
                    }.into_any()
                }
            }}
        </main>
    }
}
