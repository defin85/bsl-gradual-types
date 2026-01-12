use std::path::{Path, PathBuf};

use bsl_backend::helpers::hover_formatter::{HoverFormatConfig, HoverOutputFormat};
use bsl_shared::formatting::DetailLevel;

mod support;

fn workspace_root() -> PathBuf {
    let backend_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    backend_root.parent().expect("workspace root").to_path_buf()
}

fn test_config_path() -> PathBuf {
    workspace_root()
        .join("examples")
        .join("conf")
        .join("conf_test")
}

fn target_file_path() -> PathBuf {
    workspace_root()
        .join("examples")
        .join("conf")
        .join("conf_test")
        .join("Documents")
        .join("ЗаказНаряды")
        .join("Forms")
        .join("ФормаДокумента")
        .join("Ext")
        .join("Form")
        .join("Module.bsl")
}

#[tokio::test]
async fn hover_for_object_tabular_section_in_form_module_has_type() {
    let deps_bundle = support::deps_bundle_v2_for_paths(None, Some(&test_config_path()), Some("8.3.25"));

    let file_path = target_file_path();
    let content = std::fs::read_to_string(&file_path).expect("read Module.bsl");

    // В файле строка 7 (1-based): "    Работы = Объект.Работы;"
    // Нас интересует второе "Работы" (после точки), старт на колонке 21 (1-based).
    // Hover API использует 0-based line и UTF-16 column.
    let line = 6u32; // 7-я строка
    let column = 20u32; // 21-я колонка

    let hover_config = HoverFormatConfig {
        detail_level: DetailLevel::Full,
        output_format: HoverOutputFormat::Markdown,
        ..Default::default()
    };

    let hover = support::hover_for_code_with_config(
        deps_bundle.as_ref(),
        file_path.to_string_lossy().as_ref(),
        &content,
        line,
        column,
        Some(hover_config),
    )
    .expect("hover should exist");

    // Должны увидеть тип табличной части (из синтетических типов форм).
    assert!(
        hover.contains("ДанныеФормыКоллекция<СтрокаРаботы>"),
        "Expected hover to contain form tabular section type, got:\n{}",
        hover
    );

    // Полезно для ручной проверки через `cargo test -- --nocapture`
    println!("HOVER:\n{}", hover);
}
