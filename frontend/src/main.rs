use leptos::prelude::*;

mod api;
mod components;
mod pages;
mod utils;

use pages::App;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App/> })
}
