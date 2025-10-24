//! Type details modal component

use crate::api::{TypeInfo, MethodInfo};
use leptos::prelude::*;

#[component]
#[allow(non_snake_case)]
pub fn TypeDetailsModal(
    /// Type to display
    type_info: Signal<Option<TypeInfo>>,
    /// Callback to close modal
    on_close: Callback<()>,
) -> impl IntoView {
    view! {
        <Show when=move || type_info.get().is_some()>
            {move || {
                let info = type_info.get().unwrap();

                // Clone ALL fields upfront to avoid borrow issues
                let name = info.name.clone();
                let category = info.category.clone();
                let certainty = info.certainty;
                let source = info.source.clone();
                let flow_sensitive = info.flow_sensitive;
                let description = info.description.clone();
                let facets = info.facets.clone();
                let methods = info.methods.clone();
                let properties = info.properties.clone();
                let enum_values = info.enum_values.clone();
                let attributes_count = info.attributes_count;
                let tabular_sections = info.tabular_sections.clone();

                // Precompute flags for conditional rendering
                let description_empty = description.is_empty();
                let facets_empty = facets.is_empty();
                let methods_empty = methods.is_empty();
                let properties_empty = properties.is_empty();
                let enum_values_empty = enum_values.as_ref().map(|v| v.is_empty()).unwrap_or(true);
                let tabular_sections_empty = tabular_sections.is_empty();
                let methods_len = methods.len();
                let properties_len = properties.len();
                let enum_values_len = enum_values.as_ref().map(|v| v.len()).unwrap_or(0);
                let tabular_sections_len = tabular_sections.len();
                let has_metadata = !methods_empty || !properties_empty || attributes_count.unwrap_or(0) > 0;

                view! {
                    <div
                        class="modal-overlay"
                        on:click=move |_| on_close.run(())
                    >
                        <div
                            class="modal-content"
                            on:click=|e| e.stop_propagation()
                        >
                            // Header
                            <div class="modal-header">
                                <h2 class="modal-title">{name.clone()}</h2>
                                <button
                                    class="modal-close"
                                    on:click=move |_| on_close.run(())
                                    title="Закрыть"
                                >
                                    "×"
                                </button>
                            </div>

                            // Body
                            <div class="modal-body">
                                // Basic info section
                                <section class="detail-section">
                                    <h3 class="section-title">"📊 Общая информация"</h3>
                                    <div class="detail-grid">
                                        <div class="detail-item">
                                            <span class="detail-label">"Категория:"</span>
                                            <span class="detail-value">
                                                <span class=format!("category-badge category-{}", category.to_lowercase())>
                                                    {category.clone()}
                                                </span>
                                            </span>
                                        </div>
                                        <div class="detail-item">
                                            <span class="detail-label">"Определённость:"</span>
                                            <span class="detail-value">
                                                <div class="certainty-indicator">
                                                    <div class="certainty-bar">
                                                        <div
                                                            class="certainty-fill"
                                                            style=format!("width: {}%", certainty)
                                                        ></div>
                                                    </div>
                                                    <span class="certainty-text">{certainty}"%"</span>
                                                </div>
                                            </span>
                                        </div>
                                        <div class="detail-item">
                                            <span class="detail-label">"Источник:"</span>
                                            <span class="detail-value">{source.clone()}</span>
                                        </div>
                                        <div class="detail-item">
                                            <span class="detail-label">"Flow-sensitive:"</span>
                                            <span class="detail-value">
                                                {if flow_sensitive { "✅ Да" } else { "❌ Нет" }}
                                            </span>
                                        </div>
                                    </div>
                                </section>

                                // Description section
                                {if !description_empty {
                                    view! {
                                        <section class="detail-section">
                                            <h3 class="section-title">"📝 Описание"</h3>
                                            <p class="type-description">{description.clone()}</p>
                                        </section>
                                    }.into_any()
                                } else {
                                    let _: () = view! {};
                                    ().into_any()
                                }}

                                // Facets section
                                {if !facets_empty {
                                    view! {
                                        <section class="detail-section">
                                            <h3 class="section-title">"🎭 Фасеты"</h3>
                                            <div class="facets-grid">
                                                {facets.iter().map(|facet| {
                                                    view! {
                                                        <div class="facet-item">
                                                            <span class="facet-badge-large">{facet.clone()}</span>
                                                        </div>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </div>
                                        </section>
                                    }.into_any()
                                } else {
                                    let _: () = view! {};
                                    ().into_any()
                                }}

                                // Enum values section (for platform enumerations)
                                {if !enum_values_empty {
                                    view! {
                                        <section class="detail-section">
                                            <h3 class="section-title">"🔢 Значения перечисления (" {enum_values_len} ")"</h3>
                                            <div class="enum-values-grid">
                                                {enum_values.iter().map(|value| {
                                                    view! {
                                                        <div class="enum-value-item">
                                                            <span class="enum-value-badge">{value.clone()}</span>
                                                        </div>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </div>
                                        </section>
                                    }.into_any()
                                } else {
                                    let _: () = view! {};
                                    ().into_any()
                                }}

                                // Tabular sections (for documents, catalogs)
                                {if !tabular_sections_empty {
                                    view! {
                                        <section class="detail-section">
                                            <h3 class="section-title">"📋 Табличные части (" {tabular_sections_len} ")"</h3>
                                            {tabular_sections.iter().map(|ts| {
                                                let ts_name = ts.name.clone();
                                                let ts_attrs = ts.attributes.clone();
                                                let attrs_count = ts_attrs.len();

                                                view! {
                                                    <div class="tabular-section-detail">
                                                        <h4 class="subsection-title">
                                                            "📄 " {ts_name.clone()} " (" {attrs_count} " атрибутов)"
                                                        </h4>
                                                        <div class="tabular-attributes-list">
                                                            {ts_attrs.iter().map(|attr| {
                                                                let attr_name = attr.name.clone();
                                                                let attr_type = attr.attr_type.clone().unwrap_or_else(|| "?".to_string());

                                                                view! {
                                                                    <div class="tabular-attribute-item">
                                                                        <span class="attribute-name">{attr_name}</span>
                                                                        <span class="attribute-type">{attr_type}</span>
                                                                    </div>
                                                                }
                                                            }).collect::<Vec<_>>()}
                                                        </div>
                                                    </div>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </section>
                                    }.into_any()
                                } else {
                                    let _: () = view! {};
                                    ().into_any()
                                }}

                                // Methods and properties section
                                {if has_metadata {
                                    view! {
                                        <section class="detail-section">
                                            <h3 class="section-title">"🔧 Методы и свойства"</h3>

                                            // Methods subsection
                                            {if !methods_empty {
                                                view! {
                                                    <div class="methods-detail">
                                                        <h4 class="subsection-title">
                                                            "📋 Методы (" {methods_len} ")"
                                                        </h4>
                                                        <div class="methods-list">
                                                            {methods.iter().map(|method| {
                                                                let signature = format_method_signature(method);
                                                                let tooltip = method.english_name.clone().unwrap_or_default();
                                                                view! {
                                                                    <div
                                                                        class="method-item"
                                                                        title={tooltip}
                                                                    >
                                                                        <span class="method-name">{method.name.clone()}</span>
                                                                        <span class="method-signature">{signature}</span>
                                                                    </div>
                                                                }
                                                            }).collect::<Vec<_>>()}
                                                        </div>
                                                    </div>
                                                }.into_any()
                                            } else {
                                                let _: () = view! {};
                                                ().into_any()
                                            }}

                                            // Properties subsection
                                            {if !properties_empty {
                                                view! {
                                                    <div class="properties-detail">
                                                        <h4 class="subsection-title">
                                                            "📌 Свойства (" {properties_len} ")"
                                                        </h4>
                                                        <div class="properties-list">
                                                            {properties.iter().map(|property| {
                                                                view! {
                                                                    <div class="property-item">
                                                                        <span class="property-name">{property.clone()}</span>
                                                                    </div>
                                                                }
                                                            }).collect::<Vec<_>>()}
                                                        </div>
                                                    </div>
                                                }.into_any()
                                            } else {
                                                let _: () = view! {};
                                                ().into_any()
                                            }}

                                            // Attributes info
                                            {if let Some(attrs_count) = attributes_count {
                                                if attrs_count > 0 {
                                                    view! {
                                                        <div class="attributes-info">
                                                            <h4 class="subsection-title">
                                                                "📦 Атрибуты (" {attrs_count} ")"
                                                            </h4>
                                                            <p class="info-text">
                                                                "Доступно " {attrs_count} " атрибутов для этого типа"
                                                            </p>
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    let _: () = view! {};
                                                    ().into_any()
                                                }
                                            } else {
                                                let _: () = view! {};
                                                ().into_any()
                                            }}
                                        </section>
                                    }.into_any()
                                } else {
                                    // Fallback when no methods/properties
                                    view! {
                                        <section class="detail-section detail-section--muted">
                                            <h3 class="section-title">"🔧 Методы и свойства"</h3>
                                            <p class="placeholder-text">
                                                "Для этого типа нет доступной информации о методах и свойствах."
                                            </p>
                                        </section>
                                    }.into_any()
                                }}
                            </div>

                            // Footer
                            <div class="modal-footer">
                                <button
                                    class="btn btn--secondary"
                                    on:click=move |_| {
                                        web_sys::console::log_1(&format!("Copy: {}", name.clone()).into());
                                    }
                                >
                                    "📋 Скопировать имя"
                                </button>
                                <button
                                    class="btn btn--primary"
                                    on:click=move |_| on_close.run(())
                                >
                                    "Закрыть"
                                </button>
                            </div>
                        </div>
                    </div>
                }
            }}
        </Show>
    }
}

/// Helper функция для форматирования сигнатуры метода с поддержкой optional параметров
fn format_method_signature(method: &MethodInfo) -> String {
    let params_str = method
        .params
        .iter()
        .map(|p| {
            let param_name = &p.name;
            let param_type = &p.param_type;
            // Добавляем "?" для optional параметров
            let optional_marker = if p.is_optional { "?" } else { "" };
            format!("{}{}: {}", param_name, optional_marker, param_type)
        })
        .collect::<Vec<_>>()
        .join(", ");

    let return_str = method
        .return_type
        .as_ref()
        .map(|t| format!(" → {}", t))
        .unwrap_or_default();

    format!("({}){}", params_str, return_str)
}
