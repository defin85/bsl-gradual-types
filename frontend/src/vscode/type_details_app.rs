//! VSCode-specific entry point for Type Details Modal
//!
//! This is a standalone WASM application that runs in VSCode webview.
//! It communicates with the extension via postMessage API.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::console;

// Import components from parent modules
use super::common::{send_to_vscode, setup_vscode_listener, VsCodeMessage};
use crate::api::{MethodDto, ParamDto, TypeDto};
use crate::components::TypeDetailsModal;

/// VSCode-specific TypeInfo structure (matches typeDetailsWebview.ts format)
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VsCodeTypeInfo {
    pub name: String,
    pub certainty: String,
    pub facet: String,
    pub methods: Vec<VsCodeMethod>,
    pub properties: Vec<VsCodeProperty>,
    pub documentation: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VsCodeMethod {
    pub name: String,
    pub description: String,
    pub returns: String,
    pub parameters: Vec<VsCodeParameter>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VsCodeParameter {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VsCodeProperty {
    pub name: String,
    #[serde(rename = "type")]
    pub prop_type: String,
    pub readonly: bool,
}

/// Convert VSCode format to TypeDto format
fn convert_to_type_dto(vscode_info: VsCodeTypeInfo) -> TypeDto {
    // ✅ Extract values для переиспользования
    let name = vscode_info.name;
    let facet = vscode_info.facet;
    let certainty = vscode_info.certainty;

    TypeDto {
        id: name.clone(),              // ✅ 1 clone (нужен для id и name)
        name,                          // ✅ Move
        category: facet.clone(),       // ✅ 1 clone (нужен для category и facets)
        certainty: match certainty.as_str() {
            "High" | "Высокая" => 90,
            "Medium" | "Средняя" => 60,
            "Low" | "Низкая" => 30,
            _ => 50,
        },
        certainty_text: certainty,     // ✅ Move
        facets: vec![facet],           // ✅ Move
        methods_count: Some(vscode_info.methods.len()),
        methods: vscode_info
            .methods
            .into_iter()
            .map(|m| MethodDto {
                name: m.name,
                english_name: None,
                description: Some(m.description),
                is_deprecated: false,
                is_constructor: false,
                params: m
                    .parameters
                    .into_iter()
                    .map(|p| ParamDto {
                        name: p.name,
                        param_type: p.param_type,
                        is_optional: false,
                        default_value: None,
                    })
                    .collect(),
                return_type: Some(m.returns),
            })
            .collect(),
        attributes_count: None,
        properties: vscode_info.properties.into_iter().map(|p| p.name).collect(),
        enum_values: None,
        tabular_sections: vec![],
        source: "VSCode Extension".to_string(),
        flow_sensitive: false,
        description: vscode_info.documentation,
        union_types: None,
        flow_analysis: None,
        connections: None,
        warning: None,
        recommendation: None,
    }
}

/// Main VSCode webview component for Type Details
#[component]
pub fn VsCodeTypeDetailsApp() -> impl IntoView {
    let type_info = RwSignal::new(None::<TypeDto>);
    let is_visible = RwSignal::new(true);

    // Setup message listener from VSCode
    Effect::new(move |_| {
        let listener = setup_vscode_listener(move |msg: VsCodeMessage<VsCodeTypeInfo>| {
            match msg.msg_type.as_str() {
                "updateTypeInfo" => {
                    if let Some(data) = msg.data {
                        let type_dto = convert_to_type_dto(data);
                        type_info.set(Some(type_dto));
                        is_visible.set(true);
                    }
                }
                "error" => {
                    if let Some(error) = msg.error {
                        log_error(&format!("VSCode error: {}", error));
                    }
                }
                _ => {
                    log_warning(&format!("Unknown message type: {}", msg.msg_type));
                }
            }
        });

        // ✅ NOTE: Используем forget() - это ПРАВИЛЬНОЕ решение для WASM CSR
        // Closure не реализует Send+Sync+Clone, что требуется для StoredValue
        // В WASM single-threaded окружении нет проблем с памятью
        // Closure должен жить весь lifecycle компонента (до unmount webview целиком)
        if let Ok(closure) = listener {
            std::mem::forget(closure);
        }
    });

    // Send "ready" signal to VSCode when mounted
    Effect::new(move |_| {
        let ready_msg = VsCodeMessage::<()>::simple("ready");
        if let Err(e) = send_to_vscode(ready_msg) {
            log_error(&format!("Failed to send ready signal: {:?}", e));
        }
    });

    // Close handler
    let on_close = Callback::new(move |_| {
        is_visible.set(false);
        let close_msg = VsCodeMessage::<()>::simple("close");
        if let Err(e) = send_to_vscode(close_msg) {
            log_error(&format!("Failed to send close signal: {:?}", e));
        }
    });

    view! {
        <Show when=move || is_visible.get()>
            <TypeDetailsModal
                type_info=Signal::derive(move || type_info.get())
                on_close=on_close
            />
        </Show>
    }
}

// ============================================================================
// Logging utilities
// ============================================================================

fn log_error(msg: &str) {
    console::error_1(&JsValue::from_str(msg));
}

fn log_warning(msg: &str) {
    console::warn_1(&JsValue::from_str(msg));
}

// ============================================================================
// WASM Entry Point (only for VSCode feature)
// ============================================================================

#[cfg(all(target_arch = "wasm32", feature = "vscode"))]
#[wasm_bindgen]
pub fn start_type_details_app() {
    // Setup panic hook for better error messages
    console_error_panic_hook::set_once();

    // Mount Leptos app to #root
    leptos::mount::mount_to_body(VsCodeTypeDetailsApp);
}
