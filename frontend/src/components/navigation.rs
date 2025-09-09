//! Navigation component

use leptos::prelude::*;

#[component]
pub fn Navigation() -> impl IntoView {
    view! {
        <nav class="main-nav">
            <div class="nav-brand">
                <h1>"BSL Gradual Type System"</h1>
            </div>
            <div class="nav-links">
                <a href="#" class="nav-link active">"Dashboard"</a>
                <a href="#" class="nav-link">"Types"</a>
                <a href="#" class="nav-link">"Graph"</a>
                <a href="#" class="nav-link">"Settings"</a>
            </div>
        </nav>
    }
}
