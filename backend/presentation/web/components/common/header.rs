//! Заголовок приложения с навигацией

#[cfg(feature = "web-ui")]
use leptos::*;
#[cfg(feature = "web-ui")]
use leptos_router::*;

#[cfg(feature = "web-ui")]
#[component]
pub fn Header() -> impl IntoView {
    view! {
        <header class="header">
            <div class="header-content">
                <h1 class="logo">"🚀 BSL Type System"</h1>
                <nav class="nav">
                    <A href="/" class="nav-link">"Dashboard"</A>
                    <A href="/cards" class="nav-link">"Cards"</A>
                    <A href="/table" class="nav-link">"Table"</A>
                    <A href="/graph" class="nav-link">"Graph"</A>
                </nav>
            </div>
            
            <style>
                ".header {
                    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                    color: white;
                    padding: 20px;
                    box-shadow: 0 2px 10px rgba(0,0,0,0.1);
                }
                
                .header-content {
                    max-width: 1400px;
                    margin: 0 auto;
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                }
                
                .logo {
                    font-size: 1.5em;
                    font-weight: bold;
                    margin: 0;
                }
                
                .nav {
                    display: flex;
                    gap: 20px;
                }
                
                .nav-link {
                    color: white;
                    text-decoration: none;
                    padding: 8px 16px;
                    border-radius: 6px;
                    transition: background-color 0.3s ease;
                }
                
                .nav-link:hover {
                    background-color: rgba(255,255,255,0.2);
                }
                
                .nav-link.active {
                    background-color: rgba(255,255,255,0.3);
                }"
            </style>
        </header>
    }
}
