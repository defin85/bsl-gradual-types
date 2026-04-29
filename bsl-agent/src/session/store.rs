use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::state::{now_unix_secs, state_root, write_atomic};

const SESSIONS_DIR: &str = "sessions";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedSession {
    pub session_id: String,
    pub roots: Vec<String>,
    #[serde(default)]
    pub platform_docs_archive: Option<String>,
    #[serde(default)]
    pub platform_version: Option<String>,
    #[serde(default)]
    pub configuration_path: Option<String>,
    #[serde(default)]
    pub rules_config_path: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    pub analysis_revision: u64,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(default)]
    pub startup_job_id: Option<String>,
    #[serde(default)]
    pub env_overrides: HashMap<String, JsonValue>,
    #[serde(default)]
    pub dev_env_overrides: HashMap<String, JsonValue>,
    #[serde(default)]
    pub allow_dev_overrides: bool,
}

#[derive(Clone)]
pub struct SessionStore {
    sessions_dir: Arc<PathBuf>,
}

impl SessionStore {
    pub fn new() -> Option<Self> {
        let sessions_dir = state_root().join(SESSIONS_DIR);
        if let Err(err) = fs::create_dir_all(&sessions_dir) {
            tracing::warn!(
                "Failed to create sessions state dir {}: {}",
                sessions_dir.display(),
                err
            );
            return None;
        }
        Some(Self {
            sessions_dir: Arc::new(sessions_dir),
        })
    }

    pub fn load(&self, session_id: &str) -> Option<PersistedSession> {
        let path = self.session_path(session_id);
        let data = fs::read(path).ok()?;
        serde_json::from_slice(&data).ok()
    }

    pub fn list(&self) -> Vec<PersistedSession> {
        let mut sessions = Vec::new();
        let Ok(entries) = fs::read_dir(self.sessions_dir.as_ref()) else {
            return sessions;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension() != Some(OsStr::new("json")) {
                continue;
            }
            let Ok(data) = fs::read(&path) else {
                continue;
            };
            let Ok(session) = serde_json::from_slice::<PersistedSession>(&data) else {
                continue;
            };
            sessions.push(session);
        }

        sessions.sort_by(|a, b| a.updated_at.cmp(&b.updated_at).reverse());
        sessions
    }

    pub fn upsert(&self, session: &mut PersistedSession) {
        session.updated_at = now_unix_secs();
        let path = self.session_path(&session.session_id);
        if let Ok(bytes) = serde_json::to_vec(session) {
            if let Err(err) = write_atomic(&path, &bytes) {
                tracing::warn!("Failed to persist session {}: {}", path.display(), err);
            }
        }
    }

    fn session_path(&self, session_id: &str) -> PathBuf {
        self.sessions_dir.join(format!("{session_id}.json"))
    }
}
