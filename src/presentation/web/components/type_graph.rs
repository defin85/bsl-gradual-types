//! Type Graph - сетевое представление типов

#[cfg(feature = "web-ui")]
use leptos::*;

#[cfg(feature = "web-ui")]
#[component]
pub fn TypeGraph() -> impl IntoView {
    view! {
        <div class="type-graph-view">
            <div class="graph-header">
                <h1>"🕸️ BSL Type Network"</h1>
                <p>"Interactive graph visualization of type relationships"</p>
            </div>
            
            <div class="graph-placeholder">
                <p>"Type Graph component - в разработке"</p>
                <p>"Здесь будет сетевое представление типов на основе HTML прототипа"</p>
            </div>
            
            <style>
                ".type-graph-view {
                    max-width: 1600px;
                    margin: 0 auto;
                }
                
                .graph-header {
                    text-align: center;
                    margin-bottom: 30px;
                    padding: 30px;
                    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                    color: white;
                    border-radius: 12px;
                    box-shadow: 0 4px 20px rgba(0,0,0,0.1);
                }
                
                .graph-placeholder {
                    background: #1a1a2e;
                    color: white;
                    padding: 60px;
                    border-radius: 12px;
                    text-align: center;
                    box-shadow: 0 4px 20px rgba(0,0,0,0.1);
                }
                
                .graph-placeholder p {
                    margin-bottom: 10px;
                    color: #c9d1d9;
                }"
            </style>
        </div>
    }
}
