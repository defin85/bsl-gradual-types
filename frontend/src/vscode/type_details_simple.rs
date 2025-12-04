//! Simple Type Details component for VSCode webview
//!
//! Uses compact text layout similar to hover for clean display in VSCode panels.

use leptos::prelude::*;

use crate::api::TypeDto;

/// Simple Type Details display for VSCode (compact hover-like style)
#[component]
pub fn SimpleTypeDetails(
    type_info: Signal<Option<TypeDto>>,
    on_close: Callback<()>,
) -> impl IntoView {
    view! {
        <Show when=move || type_info.get().is_some()>
            {move || {
                let info = type_info.get().unwrap();

                let name = info.name.clone();
                let category = info.category.clone();
                let certainty = info.certainty;
                let certainty_text = info.certainty_text.clone();
                let source = info.source.clone();
                let description = info.description.clone();
                let facets = info.facets.clone();
                let methods = info.methods.clone();
                let properties = info.properties.clone();

                let methods_count = methods.len();
                let properties_count = properties.len();

                view! {
                    <div class="p-3 text-xs leading-normal">
                        // Header with close button
                        <div class="flex justify-between items-center mb-3 pb-2 border-b border-vscode-fg/20">
                            <h1 class="text-sm font-semibold">{name.clone()}</h1>
                            <button
                                class="text-base opacity-50 hover:opacity-100 p-1 rounded hover:bg-vscode-list-hover"
                                on:click=move |_| on_close.run(())
                                title="Закрыть (Esc)"
                            >
                                "×"
                            </button>
                        </div>

                        // General Info section
                        <section class="mb-3">
                            <h2 class="text-[10px] font-medium mb-1.5 opacity-50 uppercase tracking-wide">
                                "Общая информация"
                            </h2>
                            <div class="space-y-0.5">
                                <div class="flex">
                                    <span class="opacity-50 w-28 shrink-0">"Категория"</span>
                                    <span class="font-medium">{category}</span>
                                </div>
                                <div class="flex">
                                    <span class="opacity-50 w-28 shrink-0">"Определённость"</span>
                                    <span class="font-medium">
                                        {certainty_icon(certainty)}
                                        " "
                                        {certainty_text}
                                        " ("
                                        {certainty}
                                        "%)"
                                    </span>
                                </div>
                                <div class="flex">
                                    <span class="opacity-50 w-28 shrink-0">"Источник"</span>
                                    <span class="opacity-80">{source}</span>
                                </div>
                            </div>
                        </section>

                        // Description section
                        {if !description.is_empty() {
                            view! {
                                <section class="mb-3">
                                    <h2 class="text-[10px] font-medium mb-1.5 opacity-50 uppercase tracking-wide">
                                        "Описание"
                                    </h2>
                                    <p class="opacity-70 break-words leading-relaxed">{description}</p>
                                </section>
                            }.into_any()
                        } else {
                            ().into_any()
                        }}

                        // Facets section
                        {if !facets.is_empty() {
                            view! {
                                <section class="mb-3">
                                    <h2 class="text-[10px] font-medium mb-1.5 opacity-50 uppercase tracking-wide">
                                        "Фасеты"
                                    </h2>
                                    <div class="flex flex-wrap gap-1">
                                        {facets.iter().map(|facet| {
                                            view! {
                                                <span class="px-1.5 py-0.5 bg-vscode-button-bg text-vscode-button-fg rounded text-[10px] font-medium">
                                                    {facet.clone()}
                                                </span>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                </section>
                            }.into_any()
                        } else {
                            ().into_any()
                        }}

                        // Properties section
                        {if properties_count > 0 {
                            view! {
                                <section class="mb-3">
                                    <h2 class="text-[10px] font-medium mb-1.5 opacity-50 uppercase tracking-wide">
                                        "Свойства (" {properties_count} ")"
                                    </h2>
                                    <div class="space-y-0.5 font-mono">
                                        {properties.iter().map(|prop| {
                                            view! {
                                                <div class="flex items-baseline">
                                                    <span class="opacity-40 mr-1">"•"</span>
                                                    <span class="font-semibold">{prop.clone()}</span>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                </section>
                            }.into_any()
                        } else {
                            ().into_any()
                        }}

                        // Methods section
                        {if methods_count > 0 {
                            view! {
                                <section class="mb-3">
                                    <h2 class="text-[10px] font-medium mb-1.5 opacity-50 uppercase tracking-wide">
                                        "Методы (" {methods_count} ")"
                                    </h2>
                                    <div class="space-y-0.5 font-mono">
                                        {methods.iter().map(|method| {
                                            let params_str = method.params.iter()
                                                .map(|p| {
                                                    let optional = if p.is_optional { "?" } else { "" };
                                                    format!("{}{}: {}", p.name, optional, p.param_type)
                                                })
                                                .collect::<Vec<_>>()
                                                .join(", ");

                                            let return_type = method.return_type.clone()
                                                .unwrap_or_else(|| "void".to_string());

                                            view! {
                                                <div class="flex items-baseline flex-wrap">
                                                    <span class="opacity-40 mr-1">"•"</span>
                                                    <span class="font-semibold">{method.name.clone()}</span>
                                                    <span class="opacity-50">"("</span>
                                                    <span class="opacity-70">{params_str}</span>
                                                    <span class="opacity-50">")"</span>
                                                    <span class="opacity-50">" → "</span>
                                                    <span class="opacity-70">{return_type}</span>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                </section>
                            }.into_any()
                        } else {
                            ().into_any()
                        }}

                        // Footer
                        <div class="mt-4 pt-2 border-t border-vscode-fg/20 flex justify-end">
                            <button
                                class="px-3 py-1 bg-vscode-button-bg text-vscode-button-fg rounded text-xs hover:bg-vscode-button-hover"
                                on:click=move |_| on_close.run(())
                            >
                                "Закрыть"
                            </button>
                        </div>
                    </div>
                }
            }}
        </Show>
    }
}

/// Returns certainty icon based on percentage
fn certainty_icon(certainty: u8) -> &'static str {
    match certainty {
        90..=100 => "🟢",
        50..=89 => "🟡",
        _ => "⚪",
    }
}
