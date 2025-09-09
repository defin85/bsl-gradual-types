//! Main App component

use crate::components::Navigation;
use crate::pages::Dashboard;
use leptos::prelude::*;

#[component]
#[allow(non_snake_case)]
pub fn App() -> impl IntoView {
    view! {
        <div class="app">
            <Navigation />
            <Dashboard />
        </div>
    }
}
