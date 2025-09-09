//! Enhanced CLI for BSL type checking - Uses SystemCoordinator + new CLI layer

use bsl_backend::presentation::cli::TypeCheckArgs;
use bsl_backend::system::SystemCoordinator;
use clap::Parser;
use tracing_subscriber;

fn main() {
    let args = TypeCheckArgs::parse();

    // Инициализируем логирование если включен verbose режим
    if args.verbose {
        tracing_subscriber::fmt::init();
    }

    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    rt.block_on(async {
        // Создаём новый SystemCoordinator
        let coordinator = SystemCoordinator::new();

        println!("🚀 BSL Type Checker (SystemCoordinator)");
        println!("📁 Анализируем файл: {}", args.file);

        // Анализируем файл
        match coordinator.type_service().analyze_file(&args.file).await {
            Ok(analysis) => {
                println!("✅ Анализ завершён успешно!");
                println!("📊 Результаты:");
                println!("   • Файл: {}", analysis.file_path);
                println!("   • Типов найдено: {}", analysis.type_resolutions.len());
                println!("   • Время анализа: {} мс", analysis.analysis_duration_ms);

                if args.verbose {
                    println!("\n🔍 Детали:");
                    for (expr, resolution) in &analysis.type_resolutions {
                        println!("   {} → {:?}", expr, resolution);
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ Ошибка анализа: {}", e);
                std::process::exit(1);
            }
        }
    });
}
