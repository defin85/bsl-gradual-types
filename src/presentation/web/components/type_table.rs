//! Type Table - табличное представление типов

#[cfg(feature = "web-ui")]
use leptos::*;

#[cfg(feature = "web-ui")]
#[component]
pub fn TypeTable() -> impl IntoView {
    view! {
        <div class="type-table-view">
            <div class="table-header">
                <h1>"📊 BSL Type Analysis"</h1>
                <p>"Analytical Table View"</p>
            </div>
            
            <div class="table-placeholder">
                <p>"Type Table component - в разработке"</p>
                <p>"Здесь будет табличное представление типов на основе HTML прототипа"</p>
            </div>
            
            <style>
                ".type-table-view {
                    max-width: 1800px;
                    margin: 0 auto;
                }
                
                .table-header {
                    text-align: center;
                    margin-bottom: 30px;
                    padding: 30px;
                    background: white;
                    border-radius: 12px;
                    box-shadow: 0 4px 20px rgba(0,0,0,0.1);
                }
                
                .table-placeholder {
                    background: white;
                    padding: 60px;
                    border-radius: 12px;
                    text-align: center;
                    box-shadow: 0 4px 20px rgba(0,0,0,0.1);
                }
                
                .table-placeholder p {
                    margin-bottom: 10px;
                    color: #6c757d;
                }"
            </style>
        </div>
    }
}
