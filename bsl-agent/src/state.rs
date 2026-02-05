use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use bsl_runtime::system::runtime_config::{global_runtime_config, RuntimeKey};

pub const STATE_NAMESPACE: &str = "bsl-agent-state/v1";

pub fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn resolve_cache_root() -> PathBuf {
    if let Some(dir) = global_runtime_config().get_pathbuf(RuntimeKey::CacheDir) {
        return dir;
    }
    if let Ok(dir) = std::env::var("XDG_CACHE_HOME") {
        return PathBuf::from(dir).join("bsl-gradual-types");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".cache").join("bsl-gradual-types");
    }
    PathBuf::from(".bsl_cache")
}

pub fn state_root() -> PathBuf {
    resolve_cache_root().join(STATE_NAMESPACE)
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().unwrap_or(Path::new("."));
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("state");
    let temp_path = parent.join(format!(".{filename}.{}.tmp", uuid::Uuid::new_v4()));
    fs::write(&temp_path, bytes)?;
    fs::rename(temp_path, path)?;
    Ok(())
}
