//! Type Cards - карточное представление типов

#[cfg(feature = "web-ui")]
use leptos::*;

#[cfg(feature = "web-ui")]
#[component]
pub fn TypeCards() -> impl IntoView {
    view! {
        <div class="type-cards-view">
            <div class="cards-header">
                <h1>"🃏 BSL Type Explorer"</h1>
                <p>"Card-Based Type Visualization with Facets & Gradual Typing"</p>
            </div>
            
            <div class="cards-placeholder">
                <p>"Type Cards component - в разработке"</p>
                <p>"Здесь будет карточное представление типов на основе HTML прототипа"</p>
            </div>
            
            <style>
                ".type-cards-view {
                    max-width: 1600px;
                    margin: 0 auto;
                }
                
                .cards-header {
                    text-align: center;
                    margin-bottom: 30px;
                    padding: 30px;
                    background: white;
                    border-radius: 12px;
                    box-shadow: 0 4px 20px rgba(0,0,0,0.1);
                }
                
                .cards-placeholder {
                    background: white;
                    padding: 60px;
                    border-radius: 12px;
                    text-align: center;
                    box-shadow: 0 4px 20px rgba(0,0,0,0.1);
                }
                
                .cards-placeholder p {
                    margin-bottom: 10px;
                    color: #6c757d;
                }"
            </style>
        </div>
    }
}
