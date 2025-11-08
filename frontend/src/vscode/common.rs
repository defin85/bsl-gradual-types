//! Common utilities for VSCode webview integration

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::{console, window};

/// VSCode API handle for postMessage communication
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window"], js_name = acquireVsCodeApi)]
    fn acquire_vscode_api() -> VsCodeApi;
}

#[wasm_bindgen]
extern "C" {
    pub type VsCodeApi;

    #[wasm_bindgen(method, structural, js_name = postMessage)]
    pub fn post_message(this: &VsCodeApi, message: &JsValue);

    #[wasm_bindgen(method, structural, js_name = getState)]
    pub fn get_state(this: &VsCodeApi) -> JsValue;

    #[wasm_bindgen(method, structural, js_name = setState)]
    pub fn set_state(this: &VsCodeApi, state: &JsValue);
}

/// Generic message structure for VSCode communication
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VsCodeMessage<T> {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T: Serialize> VsCodeMessage<T> {
    /// Create a new message with data
    pub fn new(msg_type: impl Into<String>, data: T) -> Self {
        Self {
            msg_type: msg_type.into(),
            data: Some(data),
            error: None,
        }
    }

    /// Create a new message without data
    pub fn simple(msg_type: impl Into<String>) -> VsCodeMessage<()> {
        VsCodeMessage {
            msg_type: msg_type.into(),
            data: None,
            error: None,
        }
    }

    /// Create an error message
    pub fn error(msg_type: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            msg_type: msg_type.into(),
            data: None,
            error: Some(error.into()),
        }
    }
}

/// Send message to VSCode extension via postMessage API
pub fn send_to_vscode<T: Serialize>(msg: VsCodeMessage<T>) -> Result<(), JsValue> {
    let vscode_api = acquire_vscode_api();
    let js_value = serde_wasm_bindgen::to_value(&msg)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {:?}", e)))?;

    vscode_api.post_message(&js_value);

    // Log for debugging
    console::log_2(&JsValue::from_str("Sent to VSCode:"), &js_value);

    Ok(())
}

/// Setup message listener from VSCode
///
/// Returns a Closure that must be kept alive (call `.forget()` on it)
pub fn setup_vscode_listener<F, T>(
    mut callback: F,
) -> Result<Closure<dyn FnMut(web_sys::MessageEvent)>, JsValue>
where
    F: FnMut(VsCodeMessage<T>) + 'static,
    T: for<'de> Deserialize<'de>,
{
    let closure = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
        match serde_wasm_bindgen::from_value::<VsCodeMessage<T>>(event.data()) {
            Ok(msg) => {
                // Log received message
                console::log_2(
                    &JsValue::from_str("Received from VSCode:"),
                    &JsValue::from_str(&format!("{:?}", msg.msg_type)),
                );
                callback(msg);
            }
            Err(e) => {
                console::error_2(
                    &JsValue::from_str("Failed to parse message:"),
                    &JsValue::from_str(&format!("{:?}", e)),
                );
            }
        }
    }) as Box<dyn FnMut(_)>);

    let window = window().ok_or_else(|| JsValue::from_str("No window object"))?;
    window.add_event_listener_with_callback("message", closure.as_ref().unchecked_ref())?;

    Ok(closure)
}
