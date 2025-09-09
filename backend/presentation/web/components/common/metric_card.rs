//! Карточка с метрикой

#[cfg(feature = "web-ui")]
use leptos::*;

#[cfg(feature = "web-ui")]
#[component]
pub fn MetricCard(
    title: &'static str,
    value: u32, // Оставим u32 для гибкости, но будем отображать как процент
    icon: &'static str,
    color: &'static str,
) -> impl IntoView {
    let bar_class = format!("certainty-fill certainty-{}", color);
    let value_class = format!("metric-number certainty-{}", color);

    view! {
        <div class="metric-card">
            <div class="metric-icon">{icon}</div>
            <div class=value_class>{format!("{}%", value)}</div>
            <div class="metric-label">{title}</div>
            <div class="certainty-bar">
                <div class=bar_class style=format!("width: {}%", value)></div>
            </div>
            
            <style>
                {
                    ".metric-card {
                        background: rgba(255, 255, 255, 0.95);
                        border-radius: 12px;
                        padding: 25px;
                        text-align: center;
                        box-shadow: 0 8px 32px rgba(0, 0, 0, 0.1);
                        transition: transform 0.3s ease;
                        display: flex;
                        flex-direction: column;
                    }
                    
                    .metric-card:hover {
                        transform: translateY(-5px);
                    }
    
                    .metric-icon {
                        font-size: 1.5em;
                        margin-bottom: 10px;
                    }
                    
                    .metric-number {
                        font-size: 3em;
                        font-weight: bold;
                        margin-bottom: 10px;
                    }
                    
                    .metric-label {
                        font-size: 1.1em;
                        color: #666;
                        margin-bottom: 15px;
                    }
    
                    .certainty-bar {
                        width: 100%;
                        height: 8px;
                        background: #e9ecef;
                        border-radius: 4px;
                        overflow: hidden;
                        margin-top: auto; /* Прижимаем к низу */
                    }
    
                    .certainty-fill {
                        height: 100%;
                        transition: width 0.6s ease;
                    }
    
                    /* Цвета для текста и полосы */
                    .certainty-green { color: #28a745; }
                    .certainty-fill.certainty-green { background: linear-gradient(90deg, #28a745, #20c997); }
    
                    .certainty-yellow { color: #ffc107; }
                    .certainty-fill.certainty-yellow { background: linear-gradient(90deg, #ffc107, #fd7e14); }
    
                    .certainty-red { color: #dc3545; }
                    .certainty-fill.certainty-red { background: linear-gradient(90deg, #dc3545, #e83e8c); }
    
                    .certainty-blue { color: #007bff; }
                    .certainty-fill.certainty-blue { background: linear-gradient(90deg, #007bff, #0056b3); }
    
                    .certainty-purple { color: #6f42c1; }
                    .certainty-fill.certainty-purple { background: linear-gradient(90deg, #6f42c1, #483d8b); }"
                }
            </style>
        </div>
    }
}