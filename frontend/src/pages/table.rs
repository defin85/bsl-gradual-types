//! Table page for tabular type analysis

use crate::api::*;
use crate::components::{SearchBar, TableView};
use leptos::prelude::*;
use leptos::task::spawn_local;

/// Страница табличного представления типов
#[component]
#[allow(non_snake_case)]
pub fn TablePage(
    /// Поисковый запрос для фильтрации
    #[prop(optional)] _search_query: Option<RwSignal<String>>,
) -> impl IntoView {
    let filters = RwSignal::new(TypeFilters::new());
    let types = RwSignal::new(Vec::<TypeInfo>::new());
    let search_result = RwSignal::new(None::<AnalysisResultDto>);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let selected_type = RwSignal::new(None::<TypeInfo>);

    // Загружаем типы при изменении фильтров
    let load_types = move || {
        loading.set(true);
        error.set(None);
        
        spawn_local(async move {
            match fetch_types(filters.get()).await {
                Ok(result) => {
                    types.set(result.types.clone());
                    search_result.set(Some(result));
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

    let handle_page_change = move |page: usize| {
        let mut current_filters = filters.get();
        current_filters.page = page;
        filters.set(current_filters);
        load_types();
    };

    let _handle_row_click = move |type_info: TypeInfo| {
        selected_type.set(Some(type_info.clone()));
        web_sys::console::log_1(&format!("Selected type: {}", type_info.name).into());
    };

    let _handle_action = move |(action, type_info): (String, TypeInfo)| {
        match action.as_str() {
            "view" => {
                web_sys::console::log_1(&format!("View type: {}", type_info.name).into());
                // Открыть детальный просмотр
            },
            "copy" => {
                web_sys::console::log_1(&format!("Copy type: {}", type_info.name).into());
                // Скопировать в буфер обмена
            },
            "link" => {
                web_sys::console::log_1(&format!("Link to type: {}", type_info.name).into());
                // Создать ссылку на тип
            },
            _ => {}
        }
    };

    // Вычисляем статистику
    let stats = Signal::derive(move || {
        let types_list = types.get();
        let total = types_list.len() as u32;
        let known = types_list.iter().filter(|t| t.certainty >= 90).count() as u32;
        let inferred = types_list.iter().filter(|t| t.certainty >= 50 && t.certainty < 90).count() as u32;
        let unknown = types_list.iter().filter(|t| t.certainty < 50).count() as u32;
        let flow_sensitive = types_list.iter().filter(|t| t.flow_sensitive).count() as u32;

        (total, known, inferred, unknown, flow_sensitive)
    });

    view! {
        <main class="main-content">
            <div class="table-header">
                <h1>"📊 BSL Type Analysis - Analytical Table"</h1>
                <p>"Comprehensive type analysis with facets, certainty levels, and flow-sensitive data"</p>

                <div class="summary-stats">
                    <div class="stat-card">
                        <div class="stat-number">{move || stats.get().0}</div>
                        <div class="stat-label">"Total Types"</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-number">{move || stats.get().1}</div>
                        <div class="stat-label">"Known Types"</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-number">{move || stats.get().2}</div>
                        <div class="stat-label">"Inferred Types"</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-number">{move || stats.get().3}</div>
                        <div class="stat-label">"Unknown Types"</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-number">{move || stats.get().4}</div>
                        <div class="stat-label">"Flow-Sensitive"</div>
                    </div>
                </div>
            </div>

            <SearchBar 
                filters=filters
                on_filters_change=Callback::new(handle_filters_change)
                placeholder="Поиск типов...".to_string()
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
                    let search_result_signal = Signal::derive(move || search_result.get());
                    view! {
                        <div>
                            <TableView
                                types=types_signal
                                search_result=search_result_signal
                                on_page_change=Callback::new(handle_page_change)
                            />
                            
                            {move || {
                                if let Some(selected) = selected_type.get() {
                                    view! {
                                        <div class="selected-type-info">
                                            <h3>"Выбранный тип: " {selected.name.clone()}</h3>
                                            <p>"Категория: " {selected.category.clone()}</p>
                                            <p>"Уверенность: " {selected.certainty_percentage()}</p>
                                            <p>"Описание: " {selected.description.clone()}</p>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {}.into_any()
                                }
                            }}
                        </div>
                    }.into_any()
                }
            }}
        </main>
    }
}