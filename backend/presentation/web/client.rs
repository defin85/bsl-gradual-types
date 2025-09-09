//! WASM entrypoint for CSR build (Trunk)

#[cfg(all(feature = "web-ui", target_arch = "wasm32"))]
use wasm_bindgen::prelude::*;

#[cfg(all(feature = "web-ui", target_arch = "wasm32"))]
#[wasm_bindgen(start)]
pub fn main_js() {
    use leptos::*;
    use crate::presentation::web::components::app::App;

    // Better panic messages in browser console
    #[cfg(feature = "web-ui")]
    console_error_panic_hook::set_once();

    mount_to_body(|| view! { <App/> });
}

