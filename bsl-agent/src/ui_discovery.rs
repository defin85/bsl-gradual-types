use bsl_shared::api::dtos::McpSessionsResponseDto;
use bsl_shared::api::dtos::McpStatusDto;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use crate::state::{now_unix_secs, state_root, write_atomic};

const RUNTIME_DIR: &str = "runtime";
const HTTP_UI_DIR: &str = "http-ui";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpUiDiscoveryRecord {
    pub instance_id: String,
    pub pid: u32,
    pub started_at: u64,
    pub addr: String,
    pub ui_url: String,
}

impl HttpUiDiscoveryRecord {
    pub fn new(instance_id: String, addr: String, ui_url: String) -> Self {
        Self {
            instance_id,
            pid: std::process::id(),
            started_at: now_unix_secs(),
            addr,
            ui_url,
        }
    }
}

pub fn registry_dir() -> PathBuf {
    state_root().join(RUNTIME_DIR).join(HTTP_UI_DIR)
}

pub fn registry_path(instance_id: &str) -> PathBuf {
    registry_dir().join(format!("{instance_id}.json"))
}

pub fn write_http_ui_registry(record: &HttpUiDiscoveryRecord) -> anyhow::Result<PathBuf> {
    let dir = registry_dir();
    fs::create_dir_all(&dir)?;

    let path = registry_path(&record.instance_id);
    let bytes = serde_json::to_vec(record)?;
    write_atomic(&path, &bytes)?;
    Ok(path)
}

pub fn read_all_registry_records() -> Vec<HttpUiDiscoveryRecord> {
    let dir = registry_dir();
    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };

    let mut records = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        let Ok(record) = serde_json::from_slice::<HttpUiDiscoveryRecord>(&bytes) else {
            continue;
        };
        records.push(record);
    }
    records.sort_by(|a, b| a.started_at.cmp(&b.started_at).reverse());
    records
}

pub async fn healthcheck_status(
    client: &Client,
    ui_url: &str,
    timeout: Duration,
) -> Option<McpStatusDto> {
    let url = format!("{ui_url}/api/mcp/status");
    let request = client.get(url).send();
    let response = tokio::time::timeout(timeout, request).await.ok()?.ok()?;
    if !response.status().is_success() {
        return None;
    }
    response.json::<McpStatusDto>().await.ok()
}

pub async fn is_live_instance(
    client: &Client,
    record: &HttpUiDiscoveryRecord,
    timeout: Duration,
) -> bool {
    let Some(status) = healthcheck_status(client, record.ui_url.as_str(), timeout).await else {
        return false;
    };

    status.instance_id.as_deref() == Some(record.instance_id.as_str())
        && status.ui_url.as_deref() == Some(record.ui_url.as_str())
        && status.supported
}

pub async fn healthcheck_sessions(
    client: &Client,
    ui_url: &str,
    timeout: Duration,
) -> Option<McpSessionsResponseDto> {
    let url = format!("{ui_url}/api/mcp/sessions");
    let request = client.get(url).send();
    let response = tokio::time::timeout(timeout, request).await.ok()?.ok()?;
    if !response.status().is_success() {
        return None;
    }
    response.json::<McpSessionsResponseDto>().await.ok()
}

pub async fn matches_root_path(
    client: &Client,
    record: &HttpUiDiscoveryRecord,
    root_path: &str,
    timeout: Duration,
) -> bool {
    let Some(resp) = healthcheck_sessions(client, record.ui_url.as_str(), timeout).await else {
        return false;
    };

    resp.sessions
        .iter()
        .flat_map(|s| s.roots.iter())
        .any(|root| root.path == root_path)
}
