//! Back to top button component

use leptos::prelude::*;
use leptos::leptos_dom::helpers::StoredValue;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

/// Back to top button (floating, shows after 300px scroll)
#[component]
#[allow(non_snake_case)]
pub fn BackToTop() -> impl IntoView {
    let visible = RwSignal::new(false);

    // ✅ Store closure для cleanup (паттерн из type_details_app.rs)
    let listener_ref = StoredValue::new(None::<Closure<dyn FnMut()>>);

    // Track scroll position
    Effect::new(move |_| {
        if let Some(window) = web_sys::window() {
            let handle_scroll = move || {
                if let Some(window) = web_sys::window() {
                    let scroll_y = window.scroll_y().unwrap_or(0.0);
                    visible.set(scroll_y > 300.0);
                }
            };

            // Create closure for event listener
            let closure = Closure::wrap(Box::new(handle_scroll) as Box<dyn FnMut()>);

            let _ = window.add_event_listener_with_callback(
                "scroll",
                closure.as_ref().unchecked_ref(),
            );

            // ✅ Store вместо forget - правильное управление памятью
            listener_ref.set_value(Some(closure));
        }
    });

    // ✅ Cleanup при unmount компонента
    on_cleanup(move || {
        if let Some(closure) = listener_ref.get_value() {
            // Remove event listener before dropping
            if let Some(window) = web_sys::window() {
                let _ = window.remove_event_listener_with_callback(
                    "scroll",
                    closure.as_ref().unchecked_ref(),
                );
            }
            drop(closure);
        }
    });

    let scroll_to_top = move |_| {
        if let Some(window) = web_sys::window() {
            // Простой scroll to top (smooth behavior через CSS)
            let _ = window.scroll_to_with_x_and_y(0.0, 0.0);
        }
    };

    view! {
        <button
            class=move || format!(
                "fixed bottom-8 right-8 p-4 rounded-full bg-bsl-teal-500 dark:bg-bsl-teal-400 text-white dark:text-bsl-slate-900 shadow-lg hover:bg-bsl-teal-600 dark:hover:bg-bsl-teal-300 hover:shadow-xl transition-all duration-300 z-40 {}",
                if visible.get() { "opacity-100 translate-y-0" } else { "opacity-0 translate-y-4 pointer-events-none" }
            )
            on:click=scroll_to_top
            aria-label="Вернуться наверх"
            title="Вернуться наверх"
        >
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 10l7-7m0 0l7 7m-7-7v18"></path>
            </svg>
        </button>
    }
}
