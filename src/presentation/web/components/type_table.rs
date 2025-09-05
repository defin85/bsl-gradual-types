//! Type Table - табличное представление типов

#[cfg(feature = "web-ui")]
use leptos::*;

#[cfg(feature = "web-ui")]
use crate::{
    domain::types::{Certainty, FacetKind, ResolutionResult, TypeResolution},
    presentation::web::components::api::{ApiClient, ApiError},
};

#[cfg(feature = "web-ui")]
#[component]
pub fn TypeTable() -> impl IntoView {
    let types_resource = create_resource(
        || (),
        |_| async move { ApiClient::get_types().await }
    );

    // Сигналы для фильтров
    let (name_filter, set_name_filter) = create_signal(String::new());
    let (category_filter, set_category_filter) = create_signal("All".to_string());

    let filtered_types = create_memo(move |_| {
        let name = name_filter.get().to_lowercase();
        let category = category_filter.get();
        
        types_resource.get().and_then(|res| res.ok()).map(move |types| {
            types.into_iter()
                .filter(move |t| {
                    let name_match = name.is_empty() || t.result.to_string().to_lowercase().contains(&name);
                    let category_match = category == "All" || t.result.category_str() == category;
                    name_match && category_match
                })
                .collect::<Vec<_>>()
        }).unwrap_or_default()
    });

    view! {
        <div class="type-table-view">
            <div class="header">
                <h1>"📊 BSL Type Analysis - Analytical Table"</h1>
                <p>"Comprehensive type analysis with facets, certainty levels, and flow-sensitive data"</p>
            </div>

            <div class="filters-toolbar">
                <div class="filter-group">
                    <label>"Поиск по имени:"</label>
                    <input 
                        type="text"
                        on:input=move |ev| set_name_filter.set(event_target_value(&ev))
                        prop:value=name_filter
                    />
                </div>
                <div class="filter-group">
                    <label>"Категория:"</label>
                    <select on:change=move |ev| set_category_filter.set(event_target_value(&ev))>
                        <option value="All">"Все"</option>
                        <option value="Platform">"Platform"</option>
                        <option value="Configuration">"Configuration"</option>
                        <option value="Union">"Union"</option>
                        <option value="Dynamic">"Dynamic"</option>
                    </select>
                </div>
            </div>

            <Suspense fallback=move || view!{<p>"Загрузка..."</p>}>
                {move || view! {
                    <div class="table-container">
                        <div class="table-wrapper">
                            <table>
                                <thead>
                                    <tr>
                                        <th>"Тип"</th>
                                        <th>"Категория"</th>
                                        <th>"Определённость"</th>
                                        <th>"Фасеты"</th>
                                        <th>"Источник"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    <For
                                        each=move || filtered_types.get()
                                        key=|t| t.result.to_string()
                                        children=move |t| view! { <TableRow type_info=t /> }
                                    />
                                </tbody>
                            </table>
                        </div>
                    </div>
                }}
            </Suspense>
            
            <style>{ TABLE_STYLES }</style>
        </div>
    }
}

#[cfg(feature = "web-ui")]
#[component]
fn TableRow(type_info: TypeResolution) -> impl IntoView {
    let name = type_info.result.to_string();
    let category = type_info.result.category_str().to_string();
    let source = format!("{:?}", type_info.source);
    let certainty = type_info.certainty;
    let facets = type_info.available_facets;

    view! {
        <tr>
            <td><div class="type-name">{name}</div></td>
            <td><span class=format!("type-category category-{}", category.to_lowercase())>{category}</span></td>
            <td>
                <div class="certainty-bar">
                    <div 
                        class=format!("certainty-fill certainty-{}", 
                            match certainty {
                                Certainty::Known => "known",
                                Certainty::Inferred(_) => "inferred",
                                Certainty::Unknown => "unknown",
                            })
                        style=format!("width: {}%", match certainty {
                            Certainty::Known => 100,
                            Certainty::Inferred(val) => (val * 100.0) as u32,
                            Certainty::Unknown => 0,
                        })>
                    </div>
                    <div class="certainty-text">{format!("{}%", match certainty {
                        Certainty::Known => 100,
                        Certainty::Inferred(val) => (val * 100.0) as u32,
                        Certainty::Unknown => 0,
                    })}</div>
                </div>
            </td>
            <td class="facets-cell">
                <For
                    each=move || facets.clone()
                    key=|f| format!("{:?}", f)
                    children=move |f| {
                        let facet_name = format!("{:?}", f).to_lowercase();
                        view! { <span class=format!("facet-tag facet-{}", facet_name)>{facet_name}</span> }
                    }
                />
            </td>
            <td>{source}</td>
        </tr>
    }
}

#[cfg(feature = "web-ui")]
pub trait CategoryExt {
    fn category_str(&self) -> &str;
}

#[cfg(feature = "web-ui")]
impl CategoryExt for ResolutionResult {
    fn category_str(&self) -> &str {
        match self {
            ResolutionResult::Concrete(c) => match c {
                crate::domain::types::ConcreteType::Platform(_) => "Platform",
                crate::domain::types::ConcreteType::Configuration(_) => "Configuration",
                _ => "Primitive",
            },
            ResolutionResult::Union(_) => "Union",
            ResolutionResult::Dynamic => "Dynamic",
            _ => "Other",
        }
    }
}

const TABLE_STYLES: &str = "
.type-table-view { max-width: 1800px; margin: 0 auto; }
.header, .filters-toolbar { background: white; padding: 25px; border-radius: 10px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); margin-bottom: 25px; }
.filters-toolbar { display: flex; gap: 15px; align-items: center; flex-wrap: wrap; }
.filter-group { display: flex; flex-direction: column; gap: 5px; }
.filter-group label { font-size: 0.9em; font-weight: 600; color: #6c757d; }
.filter-group select, .filter-group input { padding: 8px 12px; border: 1px solid #dee2e6; border-radius: 6px; }
.table-container { background: white; border-radius: 10px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); overflow: hidden; }
.table-wrapper { overflow-x: auto; }
table { width: 100%; border-collapse: collapse; min-width: 1200px; }
th { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 15px 12px; text-align: left; font-weight: 600; }
td { padding: 12px; border-bottom: 1px solid #f1f3f4; vertical-align: top; }
tr:hover { background: #f8f9fa; }
.type-name { font-weight: bold; color: #2c3e50; font-size: 1.1em; }
.type-category { display: inline-block; padding: 4px 10px; border-radius: 12px; font-size: 0.8em; font-weight: 500; color: white; }
.category-platform { background: #007bff; }
.category-configuration { background: #28a745; }
.category-union { background: #ffc107; color: #333; }
.category-dynamic { background: #dc3545; }
.category-other, .category-primitive { background: #6c757d; }
.certainty-bar { width: 100px; height: 20px; background: #e9ecef; border-radius: 10px; overflow: hidden; position: relative; }
.certainty-fill { height: 100%; transition: width 0.5s ease; border-radius: 10px; }
.certainty-known { background: linear-gradient(90deg, #28a745, #20c997); }
.certainty-inferred { background: linear-gradient(90deg, #ffc107, #ffb347); }
.certainty-unknown { background: linear-gradient(90deg, #dc3545, #e74c3c); }
.certainty-text { position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%); font-size: 0.8em; font-weight: bold; color: white; text-shadow: 1px 1px 2px rgba(0,0,0,0.5); }
.facets-cell { max-width: 200px; }
.facet-tag { display: inline-block; padding: 2px 6px; margin: 2px; border-radius: 8px; font-size: 0.7em; font-weight: 500; color: white; text-transform: capitalize; }
.facet-manager { background: #007bff; }
.facet-object { background: #28a745; }
.facet-reference { background: #ffc107; color: #333; }
.facet-collection { background: #17a2b8; }
.facet-metadata { background: #6f42c1; }
";