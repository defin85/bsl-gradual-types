use std::path::{Path, PathBuf};
use std::sync::Arc;

use bsl_backend::application::TypeSystemService;
use bsl_backend::helpers::hover_formatter::{HoverFormatConfig, HoverOutputFormat};
use bsl_backend::system::SystemCoordinator;
use bsl_shared::formatting::DetailLevel;

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

async fn init_service_with_config(config_path: &Path) -> Arc<TypeSystemService> {
    let coordinator = SystemCoordinator::new();
    coordinator
        .start_with_paths(None, Some(config_path), Some("8.3.25"), None)
        .await
        .expect("SystemCoordinator start_with_paths");
    coordinator
        .type_service()
        .expect("TypeSystemService should be available")
}

#[tokio::test]
async fn hover_for_object_tabular_section_in_form_module_has_type() {
    let service = init_service_with_config(&test_config_path()).await;

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

    let hover = service
        .get_hover_info_for_file(
            &content,
            &file_path.to_string_lossy(),
            line,
            column,
            Some(hover_config),
        )
        .await
        .expect("get_hover_info_for_file should succeed")
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
