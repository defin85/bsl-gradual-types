//! Cards page for card-based type visualization

use crate::api::{fetch_types, types::*};
use crate::components::{SearchBar, TypeCardsGrid};
use leptos::prelude::*;
use leptos::task::spawn_local;

/// Страница карточного представления типов
#[component]
#[allow(non_snake_case)]
pub fn CardsPage() -> impl IntoView {
    let filters = RwSignal::new(TypeFilters::default());
    let types = RwSignal::new(Vec::<TypeInfo>::new());
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    // Загружаем типы при изменении фильтров
    let load_types = move || {
        loading.set(true);
        error.set(None);
        
        spawn_local(async move {
            match fetch_types(filters.get()).await {
                Ok(result) => {
                    types.set(result.types);
                    loading.set(false);
                },
                Err(err) => {
                    error.set(Some(err));
                    loading.set(false);
                }
            }
        });
    };

    // Загружаем типы при монтировании компонента
    Effect::new(move |_| {
        load_types();
    });

    let handle_filters_change = move |new_filters: TypeFilters| {
        filters.set(new_filters);
        load_types();
    };

    let handle_card_click = move |type_info: TypeInfo| {
        web_sys::console::log_1(&format!("Clicked on type: {}", type_info.display_name).into());
        // Здесь можно добавить логику для открытия детального просмотра
    };

    view! {
        <main class="main-content">
            <div class="card-view-header">
                <h1>"🃏 BSL Type Explorer"</h1>
                <p>"Card-Based Type Visualization with Facets & Gradual Typing"</p>
            </div>

            <SearchBar 
                filters=filters
                on_filters_change=Callback::new(handle_filters_change)
                placeholder="Поиск типов... (например: Массив, Справочники, Строка)".to_string()
            />

            {move || {
                if loading.get() {
                    view! {
                        <div class="loading">
                            <p>"🔄 Загрузка типов..."</p>
                        </div>
                    }.into_any()
                } else if let Some(err) = error.get() {
                    view! {
                        <div class="error">
                            <p>"❌ Ошибка загрузки: " {err}</p>
                            <button on:click=move |_| load_types()>"Повторить"</button>
                        </div>
                    }.into_any()
                } else {
                    let types_signal = Signal::derive(move || types.get());
                    view! {
                        <div>
                            <div class="results-summary">
                                <p>{move || format!("Найдено типов: {}", types.get().len())}</p>
                            </div>
                            
                            <TypeCardsGrid 
                                types=types_signal
                                on_card_click=Callback::new(handle_card_click)
                            />
                        </div>
                    }.into_any()
                }
            }}
        </main>
    }
}