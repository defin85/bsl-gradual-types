use serde::{Deserialize, Serialize};
use serde_json::Value;

/// DAP Request
#[derive(Debug, Serialize)]
pub struct DapRequest {
    pub seq: u32,
    #[serde(rename = "type")]
    pub type_: String, // "request"
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

/// DAP Response
#[derive(Debug, Deserialize)]
pub struct DapResponse {
    pub seq: u32,
    #[serde(rename = "type")]
    pub type_: String, // "response"
    pub request_seq: u32,
    pub success: bool,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
}

/// DAP Event
#[derive(Debug, Deserialize)]
pub struct DapEvent {
    pub seq: u32,
    #[serde(rename = "type")]
    pub type_: String, // "event"
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
}
