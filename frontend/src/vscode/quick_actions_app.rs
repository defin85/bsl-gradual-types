//! VSCode-specific entry point for Quick Actions Panel
//!
//! This is a standalone WASM application that runs in VSCode webview.
//! It communicates with the extension via postMessage API.

use leptos::prelude::*;
use web_sys::console;

#[cfg(all(target_arch = "wasm32", feature = "vscode"))]
use wasm_bindgen::prelude::wasm_bindgen;

use super::common::{send_to_vscode, setup_vscode_listener, VsCodeMessage};
use super::quick_actions_panel::{QuickActionsPanel, QuickActionsSidebarSnapshot, SearchResult};

/// Main VSCode webview component for Quick Actions
#[component]
pub fn VsCodeQuickActionsApp() -> impl IntoView {
    let (search_results, set_search_results) = signal(Vec::<SearchResult>::new());
    let (is_searching, set_is_searching) = signal(false);
    let (sidebar_snapshot, set_sidebar_snapshot) = signal(None::<QuickActionsSidebarSnapshot>);

    // Setup message listener from VSCode
    Effect::new(move |_| {
        let listener = setup_vscode_listener(move |msg: VsCodeMessage<serde_json::Value>| {
            console::log_1(&format!("Received message type: {}", msg.msg_type).into());

            match msg.msg_type.as_str() {
                "searchResults" => {
                    if let Some(data) = msg.data {
                        match serde_json::from_value::<Vec<SearchResult>>(data) {
                            Ok(results) => {
                                set_search_results.set(results);
                                set_is_searching.set(false);
                            }
                            Err(err) => {
                                console::warn_1(
                                    &format!("Failed to decode searchResults payload: {:?}", err)
                                        .into(),
                                );
                            }
                        }
                    }
                }
                "sidebarSnapshot" => {
                    if let Some(data) = msg.data {
                        match serde_json::from_value::<QuickActionsSidebarSnapshot>(data) {
                            Ok(snapshot) => {
                                set_sidebar_snapshot.set(Some(snapshot));
                            }
                            Err(err) => {
                                console::warn_1(
                                    &format!("Failed to decode sidebarSnapshot payload: {:?}", err)
                                        .into(),
                                );
                            }
                        }
                    }
                }
                _ => {
                    console::warn_1(&format!("Unknown message type: {}", msg.msg_type).into());
                }
            }
        });

        // ✅ NOTE: Используем forget() вместо StoredValue из-за ограничений WASM
        // Closure не реализует Clone, что требуется для StoredValue в CSR режиме
        // В WASM окружении нет thread safety issues, closure живёт весь lifecycle
        if let Ok(closure) = listener {
            std::mem::forget(closure);
        }
    });

    // Send "ready" signal to VSCode when mounted
    Effect::new(move |_| {
        let ready_msg = VsCodeMessage::<()>::simple("ready");
        if let Err(e) = send_to_vscode(ready_msg) {
            console::error_1(&format!("Failed to send ready signal: {:?}", e).into());
        }
    });

    view! {
        <QuickActionsPanel
            search_results=search_results
            is_searching=is_searching
            sidebar_snapshot=sidebar_snapshot
        />
    }
}

// ============================================================================
// WASM Entry Point for Quick Actions webview
// ============================================================================

/// Auto-start function for Quick Actions webview
/// This is called automatically when the WASM module is loaded
#[cfg(all(target_arch = "wasm32", feature = "vscode"))]
#[wasm_bindgen]
pub fn start_quick_actions_app() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(VsCodeQuickActionsApp);
}
