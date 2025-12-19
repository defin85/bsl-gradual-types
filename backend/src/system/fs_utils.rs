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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn read_bsl_file_strips_utf8_bom() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"\xEF\xBB\xBF&\xD0\x9D\xD0\xB0\xD0\xA1\xD0\xB5\xD1\x80\xD0\xB2\xD0\xB5\xD1\x80\xD0\xB5\n").unwrap();

        let content = read_bsl_file(file.path()).unwrap();
        assert!(!content.starts_with('\u{FEFF}'), "BOM should be stripped");
        assert!(content.starts_with("&НаСервере"), "expected content after BOM");
    }

    #[test]
    fn read_bsl_file_without_bom_is_unchanged() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Процедура П() Экспорт").unwrap();

        let content = read_bsl_file(file.path()).unwrap();
        assert!(content.starts_with("Процедура"), "expected original content");
    }
}
