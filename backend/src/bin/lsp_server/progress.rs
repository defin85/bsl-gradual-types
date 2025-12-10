//! Progress logging utilities for BSL Language Server
//!
//! Contains helper functions for logging progress to debug files.

use std::fs::OpenOptions;
use std::io::Write;

/// Write message to progress_debug.log with timestamp
/// Logs are written to current working directory (where LSP server is running)
pub fn log_progress_to_file(message: &str) {
    let log_path = "progress_debug.log";
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let log_line = format!("[{}] {}\n", timestamp, message);

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
        let _ = file.write_all(log_line.as_bytes());
        let _ = file.flush();
    }
}
