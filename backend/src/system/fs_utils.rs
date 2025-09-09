//! File system utilities for BSL files

use anyhow::Result;
use std::fs;
use std::path::Path;

/// Read a BSL file as UTF-8, stripping BOM if present
pub fn read_bsl_file(path: &Path) -> Result<String> {
    let mut content = fs::read_to_string(path)?;
    if content.starts_with('\u{FEFF}') {
        content = content.trim_start_matches('\u{FEFF}').to_string();
    }
    Ok(content)
}

/// Check if path is a BSL file
pub fn is_bsl_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("bsl") || ext.eq_ignore_ascii_case("os"))
        .unwrap_or(false)
}
