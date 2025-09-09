//! Main App component

use crate::components::Navigation;
use crate::pages::Dashboard;
use leptos::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <div class="app">
            <Navigation />
            <Dashboard />
        </div>
    }
}
