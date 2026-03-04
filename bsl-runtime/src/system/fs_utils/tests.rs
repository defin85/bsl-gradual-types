use super::*;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn read_bsl_file_strips_utf8_bom() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(
        b"\xEF\xBB\xBF&\xD0\x9D\xD0\xB0\xD0\xA1\xD0\xB5\xD1\x80\xD0\xB2\xD0\xB5\xD1\x80\xD0\xB5\n",
    )
    .unwrap();

    let content = read_bsl_file(file.path()).unwrap();
    assert!(!content.starts_with('\u{FEFF}'), "BOM should be stripped");
    assert!(
        content.starts_with("&НаСервере"),
        "expected content after BOM"
    );
}

#[test]
fn read_bsl_file_without_bom_is_unchanged() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "Процедура П() Экспорт").unwrap();

    let content = read_bsl_file(file.path()).unwrap();
    assert!(
        content.starts_with("Процедура"),
        "expected original content"
    );
}
