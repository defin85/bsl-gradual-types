//! Карточка с метрикой

#[cfg(feature = "web-ui")]
use leptos::*;

#[cfg(feature = "web-ui")]
#[component]
pub fn MetricCard(
    title: &'static str,
    value: u32,
    icon: &'static str,
    color: &'static str,
) -> impl IntoView {
    let css_class = format!("metric-card metric-{}", color);
    
    view! {
        <div class=css_class>
            <div class="metric-icon">{icon}</div>
            <div class="metric-number">{value}</div>
            <div class="metric-label">{title}</div>
            
            <style>
                ".metric-card {
                    background: white;
                    border-radius: 12px;
                    padding: 25px;
                    text-align: center;
                    box-shadow: 0 4px 20px rgba(0,0,0,0.1);
                    transition: transform 0.3s ease;
                    border-left: 4px solid;
                }
                
                .metric-card:hover {
                    transform: translateY(-5px);
                }
                
                .metric-blue { border-left-color: #007bff; }
                .metric-green { border-left-color: #28a745; }
                .metric-yellow { border-left-color: #ffc107; }
                .metric-red { border-left-color: #dc3545; }
                .metric-purple { border-left-color: #6f42c1; }
                
                .metric-icon {
                    font-size: 2em;
                    margin-bottom: 10px;
                }
                
                .metric-number {
                    font-size: 2.5em;
                    font-weight: bold;
                    margin-bottom: 5px;
                    color: #2c3e50;
                }
                
                .metric-label {
                    color: #6c757d;
                    font-weight: 500;
                }"
            </style>
        </div>
    }
}
