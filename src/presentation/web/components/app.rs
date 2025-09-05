//! Главный компонент приложения с роутингом

#[cfg(feature = "web-ui")]
use leptos::*;
#[cfg(feature = "web-ui")]
use leptos_meta::*;
#[cfg(feature = "web-ui")]
use leptos_router::*;

#[cfg(feature = "web-ui")]
use crate::presentation::web::components::{Dashboard, TypeCards, TypeTable, TypeGraph};
#[cfg(feature = "web-ui")]
use crate::presentation::web::components::common::Header;

#[cfg(feature = "web-ui")]
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    
    view! {
        <Html lang="ru"/>
        <Title text="BSL Type System"/>
        <Meta charset="utf-8"/>
        <Meta name="viewport" content="width=device-width, initial-scale=1"/>
        
        <Router>
            <div class="app">
                <Header />
                
                <main class="main-content">
                    <Routes>
                        <Route path="/" view=Dashboard/>
                        <Route path="/cards" view=TypeCards/>
                        <Route path="/table" view=TypeTable/>
                        <Route path="/graph" view=TypeGraph/>
                    </Routes>
                </main>
                
                <style>
                    "* {
                        margin: 0;
                        padding: 0;
                        box-sizing: border-box;
                    }
                    
                    body {
                        font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
                        background: #f5f7fa;
                        color: #333;
                        line-height: 1.6;
                    }
                    
                    .app {
                        min-height: 100vh;
                        display: flex;
                        flex-direction: column;
                    }
                    
                    .main-content {
                        flex: 1;
                        padding: 20px;
                        max-width: 1400px;
                        margin: 0 auto;
                        width: 100%;
                    }"
                </style>
            </div>
        </Router>
    }
}
