//! Dashboard page

use leptos::prelude::*;
use crate::components::MetricCard;

#[component]
pub fn Dashboard() -> impl IntoView {
    view! {
        <main class="main-content">
            <div class="dashboard-header">
                <h1>"Type System Dashboard"</h1>
                <p>"Анализ и визуализация типов BSL"</p>
            </div>
            
            <div class="dashboard-grid">
                <MetricCard 
                    value=Signal::derive(|| "150".to_string())
                    title=Signal::derive(|| "Known Types".to_string())
                    color=Signal::derive(|| "#28a745".to_string())
                />
                <MetricCard 
                    value=Signal::derive(|| "45".to_string())
                    title=Signal::derive(|| "Inferred Types".to_string())
                    color=Signal::derive(|| "#ffc107".to_string())
                />
                <MetricCard 
                    value=Signal::derive(|| "12".to_string())
                    title=Signal::derive(|| "Unknown Types".to_string())
                    color=Signal::derive(|| "#dc3545".to_string())
                />
                <MetricCard 
                    value=Signal::derive(|| "85%".to_string())
                    title=Signal::derive(|| "Coverage".to_string())
                    color=Signal::derive(|| "#17a2b8".to_string())
                />
            </div>
            
            <div style="background: white; padding: 2rem; border-radius: 8px; box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);">
                <h2 style="margin-bottom: 1rem;">"🚀 Система запущена и работает!"</h2>
                <p>"Модульная архитектура готова к масштабированию."</p>
                <button
                    style="margin-top: 1rem; padding: 0.5rem 1rem; background: #007bff; color: white; border: none; border-radius: 4px; cursor: pointer;"
                    on:click=move |_| {
                        web_sys::console::log_1(&"✅ Тестовая кнопка нажата!".into());
                        let window = web_sys::window().unwrap();
                        window.alert_with_message("🎉 Модульная архитектура работает!").unwrap();
                    }>
                    "Тест взаимодействия"
                </button>
            </div>
        </main>
    }
}
