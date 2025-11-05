//! Type details modal component

use crate::api::{TypeInfo, MethodInfo};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

// ============================================================================
// Helper Functions for Tailwind CSS Classes
// ============================================================================

/// Helper function for detail section classes with conditional border
fn detail_section_classes(is_last: bool) -> &'static str {
    if is_last {
        "mb-6 pb-6 border-0"
    } else {
        "mb-6 pb-6 border-b border-gray-100 dark:border-gray-800"
    }
}

/// Helper function for category badge classes based on category type
fn category_badge_classes(category: &str) -> String {
    let base = "inline-block px-3 py-1 rounded-full text-sm font-medium";
    let color = match category.to_lowercase().as_str() {
        "primitive" => "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200",
        "platform" => "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200",
        "user" => "bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200",
        "union" => "bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200",
        _ => "bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-200",
    };
    format!("{} {}", base, color)
}

/// Helper function for certainty bar color based on certainty percentage
fn certainty_bar_color(certainty: u8) -> &'static str {
    match certainty {
        0..=33 => "bg-gradient-to-r from-red-500 to-orange-500",
        34..=66 => "bg-gradient-to-r from-orange-500 to-yellow-500",
        67..=89 => "bg-gradient-to-r from-yellow-500 to-green-500",
        _ => "bg-gradient-to-r from-blue-500 to-green-500",
    }
}

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
                        class="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50 animate-in fade-in duration-200"
                        on:click=move |_| on_close.run(())
                        on:keydown=move |ev| {
                            if ev.key() == "Escape" {
                                on_close.run(());
                            }
                        }
                        role="dialog"
                        aria-modal="true"
                        aria-labelledby="modal-title"
                        tabindex="-1"
                    >
                        <div
                            class="bg-white dark:bg-gray-900 rounded-2xl max-w-4xl w-[90%] max-h-[90vh] flex flex-col shadow-2xl animate-in slide-in-from-bottom-4 duration-300"
                            on:click=|e| e.stop_propagation()
                        >
                            // Header
                            <div class="flex justify-between items-center px-6 py-4 border-b border-gray-200 dark:border-gray-700">
                                <h2
                                    id="modal-title"
                                    class="text-2xl font-bold text-gray-900 dark:text-white"
                                >
                                    {name.clone()}
                                </h2>
                                <button
                                    class="text-3xl text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 transition-colors duration-200 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-900 rounded-lg px-2"
                                    on:click=move |_| on_close.run(())
                                    aria-label="Закрыть"
                                    title="Закрыть (ESC)"
                                >
                                    "×"
                                </button>
                            </div>

                            // Body
                            <div class="flex-1 overflow-y-auto px-6 py-4">
                                // Basic info section
                                <section class={detail_section_classes(false)}>
                                    <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4 flex items-center">
                                        "📊 Общая информация"
                                    </h3>
                                    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                                        <div class="flex flex-col gap-1">
                                            <span class="text-sm text-gray-600 dark:text-gray-400 font-medium">
                                                "Категория:"
                                            </span>
                                            <span class="text-base text-gray-900 dark:text-white">
                                                <span class={category_badge_classes(&category)}>
                                                    {category.clone()}
                                                </span>
                                            </span>
                                        </div>
                                        <div class="flex flex-col gap-1">
                                            <span class="text-sm text-gray-600 dark:text-gray-400 font-medium">
                                                "Определённость:"
                                            </span>
                                            <span class="text-base text-gray-900 dark:text-white">
                                                <div class="flex items-center gap-3">
                                                    <div class="flex-1 h-2 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
                                                        <div
                                                            class={format!("{} h-full transition-all duration-500", certainty_bar_color(certainty))}
                                                            style=format!("width: {}%", certainty)
                                                        ></div>
                                                    </div>
                                                    <span class="text-sm font-semibold text-gray-700 dark:text-gray-300">
                                                        {certainty}"%"
                                                    </span>
                                                </div>
                                            </span>
                                        </div>
                                        <div class="flex flex-col gap-1">
                                            <span class="text-sm text-gray-600 dark:text-gray-400 font-medium">
                                                "Источник:"
                                            </span>
                                            <span class="text-base text-gray-900 dark:text-white">
                                                {source.clone()}
                                            </span>
                                        </div>
                                        <div class="flex flex-col gap-1">
                                            <span class="text-sm text-gray-600 dark:text-gray-400 font-medium">
                                                "Flow-sensitive:"
                                            </span>
                                            <span class="text-base text-gray-900 dark:text-white">
                                                {if flow_sensitive { "✅ Да" } else { "❌ Нет" }}
                                            </span>
                                        </div>
                                    </div>
                                </section>

                                // Description section
                                {if !description_empty {
                                    view! {
                                        <section class={detail_section_classes(false)}>
                                            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4 flex items-center">
                                                "📝 Описание"
                                            </h3>
                                            <p class="text-base text-gray-700 dark:text-gray-300 leading-relaxed">
                                                {description.clone()}
                                            </p>
                                        </section>
                                    }.into_any()
                                } else {
                                    let _: () = view! {};
                                    ().into_any()
                                }}

                                // Facets section
                                {if !facets_empty {
                                    view! {
                                        <section class={detail_section_classes(false)}>
                                            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4 flex items-center">
                                                "🎭 Фасеты"
                                            </h3>
                                            <div class="flex flex-wrap gap-2">
                                                {facets.iter().map(|facet| {
                                                    view! {
                                                        <span class="px-4 py-2 bg-blue-600 text-white rounded-lg font-medium hover:bg-blue-700 transition-colors duration-200">
                                                            {facet.clone()}
                                                        </span>
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
                                        <section class={detail_section_classes(false)}>
                                            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4 flex items-center">
                                                "🔢 Значения перечисления (" {enum_values_len} ")"
                                            </h3>
                                            <div class="flex flex-wrap gap-2">
                                                {enum_values.iter().map(|value| {
                                                    view! {
                                                        <span class="px-3 py-1.5 bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200 rounded-lg text-sm font-medium hover:bg-purple-200 dark:hover:bg-purple-800 transition-colors duration-200">
                                                            {value.clone()}
                                                        </span>
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
                                        <section class={detail_section_classes(false)}>
                                            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4 flex items-center">
                                                "📋 Табличные части (" {tabular_sections_len} ")"
                                            </h3>
                                            <div class="space-y-4">
                                                {tabular_sections.iter().map(|ts| {
                                                    let ts_name = ts.name.clone();
                                                    let ts_attrs = ts.attributes.clone();
                                                    let attrs_count = ts_attrs.len();

                                                    view! {
                                                        <div class="p-4 bg-gray-50 dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
                                                            <h4 class="text-base font-semibold text-gray-900 dark:text-white mb-3">
                                                                "📄 " {ts_name.clone()} " (" {attrs_count} " атрибутов)"
                                                            </h4>
                                                            <div class="grid grid-cols-1 md:grid-cols-2 gap-2">
                                                                {ts_attrs.iter().map(|attr| {
                                                                    let attr_name = attr.name.clone();
                                                                    let attr_type = attr.attr_type.clone().unwrap_or_else(|| "?".to_string());

                                                                    view! {
                                                                        <div class="flex items-center justify-between p-2 bg-white dark:bg-gray-900 rounded border border-gray-100 dark:border-gray-700">
                                                                            <span class="font-mono text-sm text-gray-900 dark:text-white font-medium">
                                                                                {attr_name}
                                                                            </span>
                                                                            <span class="font-mono text-xs text-blue-600 dark:text-blue-400">
                                                                                {attr_type}
                                                                            </span>
                                                                        </div>
                                                                    }
                                                                }).collect::<Vec<_>>()}
                                                            </div>
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

                                // Methods and properties section
                                {if has_metadata {
                                    view! {
                                        <section class={detail_section_classes(false)}>
                                            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4 flex items-center">
                                                "🔧 Методы и свойства"
                                            </h3>

                                            // Methods subsection
                                            {if !methods_empty {
                                                view! {
                                                    <div class="mb-6">
                                                        <h4 class="text-base font-semibold text-gray-900 dark:text-white mb-3">
                                                            "📋 Методы (" {methods_len} ")"
                                                        </h4>
                                                        <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                                                            {methods.iter().map(|method| {
                                                                let signature = format_method_signature(method);
                                                                let tooltip = method.english_name.clone().unwrap_or_default();
                                                                view! {
                                                                    <div
                                                                        class="p-3 bg-gray-50 dark:bg-gray-800 rounded-lg border-l-4 border-blue-500 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors duration-200 cursor-default"
                                                                        title={tooltip}
                                                                    >
                                                                        <div class="font-mono text-sm text-blue-700 dark:text-blue-300 font-medium break-words">
                                                                            {method.name.clone()}
                                                                        </div>
                                                                        <div class="font-mono text-xs text-gray-600 dark:text-gray-400 mt-1 break-all">
                                                                            {signature}
                                                                        </div>
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
                                                    <div class="mb-6">
                                                        <h4 class="text-base font-semibold text-gray-900 dark:text-white mb-3">
                                                            "📌 Свойства (" {properties_len} ")"
                                                        </h4>
                                                        <div class="flex flex-wrap gap-2">
                                                            {properties.iter().map(|property| {
                                                                view! {
                                                                    <span class="px-3 py-1.5 bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200 rounded-lg text-sm font-mono font-medium hover:bg-green-200 dark:hover:bg-green-800 transition-colors duration-200">
                                                                        {property.clone()}
                                                                    </span>
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
                                                        <div class="p-4 bg-blue-50 dark:bg-blue-900/20 rounded-lg border border-blue-200 dark:border-blue-800">
                                                            <h4 class="text-base font-semibold text-gray-900 dark:text-white mb-2">
                                                                "📦 Атрибуты (" {attrs_count} ")"
                                                            </h4>
                                                            <p class="text-sm text-gray-600 dark:text-gray-400">
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
                                        <section class={detail_section_classes(true)}>
                                            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4 flex items-center">
                                                "🔧 Методы и свойства"
                                            </h3>
                                            <div class="p-4 bg-gray-50 dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 text-center">
                                                <p class="text-sm text-gray-600 dark:text-gray-400">
                                                    "Для этого типа нет доступной информации о методах и свойствах."
                                                </p>
                                            </div>
                                        </section>
                                    }.into_any()
                                }}
                            </div>

                            // Footer
                            <div class="flex justify-end gap-3 px-6 py-4 border-t border-gray-200 dark:border-gray-700">
                                <button
                                    class="px-4 py-2 rounded-lg font-medium transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2 dark:focus:ring-offset-gray-900 bg-gray-100 text-gray-700 hover:bg-gray-200 dark:bg-gray-800 dark:text-gray-300 dark:hover:bg-gray-700 border border-gray-300 dark:border-gray-600 focus:ring-gray-500"
                                    on:click=move |_| {
                                        let name_copy = name.clone();

                                        if let Some(window) = web_sys::window() {
                                            if let Some(navigator) = window.navigator().clipboard() {
                                                let _ = navigator.write_text(&name_copy);
                                                web_sys::console::log_1(&"Имя типа скопировано в буфер обмена".into());
                                            }
                                        }
                                    }
                                    aria-label="Скопировать имя типа в буфер обмена"
                                >
                                    "📋 Скопировать имя"
                                </button>
                                <button
                                    class="px-4 py-2 rounded-lg font-medium transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2 dark:focus:ring-offset-gray-900 bg-blue-600 text-white hover:bg-blue-700 focus:ring-blue-500"
                                    on:click=move |_| on_close.run(())
                                    aria-label="Закрыть модальное окно"
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
