//! Интеграционный тест для HBK Recovery
//!
//! Проверяет базовую функциональность восстановления .hbk файлов

use bsl_backend::data::loaders::hbk_recovery;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    // Инициализация логирования
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Путь к директории с .hbk файлами
    let syntax_helper_dir = Path::new("examples/syntax_helper");

    println!("🔍 Тестирование HBK Recovery...");
    println!("📂 Директория: {:?}", syntax_helper_dir);

    // Проверяем что директория существует
    if !syntax_helper_dir.exists() {
        eprintln!("❌ Директория не существует: {:?}", syntax_helper_dir);
        std::process::exit(1);
    }

    // Запускаем автоматическое восстановление
    println!("\n🔧 Запускаем auto_recover_directory()...");
    match hbk_recovery::auto_recover_directory(syntax_helper_dir) {
        Ok(results) => {
            println!("\n✅ Восстановление завершено!");
            println!("📊 Всего восстановлено: {} файлов\n", results.len());

            for (i, result) in results.iter().enumerate() {
                println!("Файл #{}", i + 1);
                println!("  ZIP:         {:?}", result.repaired_zip_path);
                println!("  Распаковано: {:?}", result.extracted_dir);
                println!("  Offset:      {} байт", result.signature_offset);
                println!("  Размер ZIP:  {} байт\n", result.recovered_size);
            }
        }
        Err(e) => {
            eprintln!("❌ Ошибка восстановления: {}", e);
            std::process::exit(1);
        }
    }

    println!("✅ Тест завершён успешно!");
    Ok(())
}
