//! Type Cards - карточное представление типов

#[cfg(feature = "web-ui")]
use leptos::*;

#[cfg(feature = "web-ui")]
use crate::{
    domain::types::{ConcreteType, ResolutionResult, TypeResolution, Certainty, FacetKind},
    presentation::web::components::api::ApiClient,
    presentation::web::components::common::SearchBar,
};

// Helper to get a display name for a ResolutionResult
#[cfg(feature = "web-ui")]
impl ToString for ResolutionResult {
    fn to_string(&self) -> String {
        match self {
            ResolutionResult::Concrete(c) => match c {
                ConcreteType::Platform(p) => p.name.clone(),
                ConcreteType::Configuration(c) => c.name.clone(),
                ConcreteType::Primitive(p) => p.to_string(),
                ConcreteType::Special(s) => format!("{:?}", s),
                ConcreteType::GlobalFunction(f) => f.name.clone(),
            },
            ResolutionResult::Union(types) => {
                let names: Vec<String> = types.iter().map(|t| t.type_.to_string()).collect();
                format!("Union<{}>", names.join(", "))
            }
            ResolutionResult::Conditional(_) => "Conditional".to_string(),
            ResolutionResult::Contextual(_) => "Contextual".to_string(),
            ResolutionResult::Dynamic => "Dynamic".to_string(),
        }
    }
}

#[cfg(feature = "web-ui")]
impl ConcreteType {
    fn to_string(&self) -> String {
        match self {
            ConcreteType::Platform(p) => p.name.clone(),
            ConcreteType::Configuration(c) => c.name.clone(),
            ConcreteType::Primitive(p) => p.to_string(),
            ConcreteType::Special(s) => format!("{:?}", s),
            ConcreteType::GlobalFunction(f) => f.name.clone(),
        }
    }
}


#[cfg(feature = "web-ui")]
#[component]
pub fn TypeCards() -> impl IntoView {
    let types_resource = create_resource(
        || (),
        |_| async move { ApiClient::get_types().await }
    );
    
    let (search_query, set_search_query) = create_signal(String::new());

    let filtered_types = create_memo(move |_| {
        let query = search_query.get().to_lowercase();
        types_resource.get().and_then(|res| res.ok()).map(|types| {
            if query.is_empty() {
                types
            } else {
                types.into_iter()
                    .filter(|t| t.result.to_string().to_lowercase().contains(&query))
                    .collect()
            }
        }).unwrap_or_default()
    });

    view! {
        <div class="type-cards-view">
            <div class="cards-header">
                <h1>"🃏 BSL Type Explorer"</h1>
                <p>"Card-Based Type Visualization with Facets & Gradual Typing"</p>
            </div>

            <SearchBar 
                value=search_query
                on_input=move |query| set_search_query.set(query)
                placeholder="Поиск типов... (например: Массив, Справочники, Строка)"
            />

            <Suspense fallback=move || view!{<div class="loading">"Загрузка типов..."</div>}>
                {move || view! {
                    <div class="cards-grid">
                        <For
                            each=move || filtered_types.get()
                            key=|type_info| type_info.result.to_string()
                            children=move |type_info| {
                                view! { <TypeCard type_info=type_info /> }
                            }
                        />
                    </div>
                }}
            </Suspense>

            <style>
            {
                ".type-cards-view { max-width: 1600px; margin: 0 auto; }
                .cards-header { text-align: center; margin-bottom: 30px; }
                .cards-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(400px, 1fr)); gap: 25px; }
                .loading { text-align: center; font-size: 1.5em; padding: 40px; }"
            }
            </style>
        </div>
    }
}

#[cfg(feature = "web-ui")]
#[component]
fn TypeCard(type_info: TypeResolution) -> impl IntoView {
    let certainty_value = match type_info.certainty {
        Certainty::Known => 100,
        Certainty::Inferred(val) => (val * 100.0) as u32,
        Certainty::Unknown => 0,
    };

    let card_class = match certainty_value {
        100 => "type-card card-known",
        50..=99 => "type-card card-inferred",
        _ => "type-card card-unknown",
    };

    let badge_class = match certainty_value {
        100 => "certainty-badge badge-known",
        50..=99 => "certainty-badge badge-inferred",
        _ => "certainty-badge badge-unknown",
    };

    view! {
        <div class=card_class>
            <div class="type-header">
                <div class="type-name">{type_info.result.to_string()}</div>
                <div class=badge_class>{format!("{}%", certainty_value)}</div>
            </div>

            <div class="type-details">
                <div class="detail-row">
                    <span class="detail-label">"Источник:"</span>
                    <span class="detail-value">{format!("{:?}", type_info.source)}</span>
                </div>
            </div>

            <div class="facets-section">
                <strong>"Доступные фасеты:"</strong><br/>
                <For
                    each=move || type_info.available_facets.clone()
                    key=|facet| format!("{:?}", facet)
                    children=move |facet| {
                        let facet_name = format!("{:?}", facet).to_lowercase();
                        let facet_class = format!("facet-tag facet-{}", facet_name);
                        view! { <span class=facet_class>{facet_name}</span> }
                    }
                />
            </div>

            <style>
            {
                ".type-card { background: white; border-radius: 16px; padding: 25px; box-shadow: 0 4px 20px rgba(0,0,0,0.08); border-left: 6px solid; transition: all 0.3s ease; }
                .type-card:hover { transform: translateY(-5px); box-shadow: 0 8px 30px rgba(0,0,0,0.15); }
                .card-known { border-left-color: #28a745; }
                .card-inferred { border-left-color: #ffc107; }
                .card-unknown { border-left-color: #dc3545; }
                .type-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 15px; }
                .type-name { font-size: 1.4em; font-weight: bold; color: #2c3e50; }
                .certainty-badge { padding: 6px 12px; border-radius: 20px; font-size: 0.8em; font-weight: bold; color: white; }
                .badge-known { background: #28a745; }
                .badge-inferred { background: #ffc107; }
                .badge-unknown { background: #dc3545; }
                .type-details { margin-bottom: 20px; }
                .detail-row { display: flex; justify-content: space-between; margin-bottom: 8px; padding-bottom: 8px; border-bottom: 1px solid #f1f3f4; }
                .detail-label { font-weight: 600; color: #6c757d; }
                .detail-value { color: #2c3e50; }
                .facets-section { margin-bottom: 20px; }
                .facet-tag { display: inline-block; padding: 4px 10px; margin: 3px; border-radius: 12px; font-size: 0.8em; font-weight: 500; color: white; background: #6c757d; text-transform: capitalize; }
                .facet-manager { background: #007bff; }
                .facet-object { background: #28a745; }
                .facet-reference { background: #ffc107; color: #333; }
                .facet-collection { background: #17a2b8; }
                .facet-metadata { background: #6f42c1; }"
            }
            </style>
        </div>
    }
}