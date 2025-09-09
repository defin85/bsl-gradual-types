//! Компонент строки поиска

#[cfg(feature = "web-ui")]
use leptos::*;

#[cfg(feature = "web-ui")]
#[component]
pub fn SearchBar(
    value: ReadSignal<String>,
    on_input: impl Fn(String) + 'static,
    placeholder: &'static str,
) -> impl IntoView {
    view! {
        <div class="search-container">
            <input
                type="text"
                class="search-input"
                placeholder=placeholder
                prop:value=move || value.get()
                on:input=move |ev| {
                    let value = event_target_value(&ev);
                    on_input(value);
                }
            />
            
            <style>
                ".search-container {
                    max-width: 600px;
                    margin: 0 auto 40px;
                    position: relative;
                }
                
                .search-input {
                    width: 100%;
                    padding: 15px 20px;
                    border: 2px solid #e9ecef;
                    border-radius: 25px;
                    font-size: 16px;
                    outline: none;
                    transition: border-color 0.3s ease;
                    background: white;
                }
                
                .search-input:focus {
                    border-color: #007bff;
                    box-shadow: 0 0 0 3px rgba(0,123,255,0.1);
                }"
            </style>
        </div>
    }
}
