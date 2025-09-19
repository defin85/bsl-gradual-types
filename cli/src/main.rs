//! Standalone CLI for BSL type checking

mod args;

use clap::Parser;
use tracing_subscriber;
use args::CliArgs;
use bsl_shared::engine::AnalysisEngine;

#[tokio::main]
async fn main() {
    let args = CliArgs::parse();

    if args.verbose {
        tracing_subscriber::fmt::init();
    }
    
    println!("🚀 BSL Type Checker CLI");
    println!("📁 Анализируем файл: {}", args.file);

    // Step 1: Create the analysis engine
    match AnalysisEngine::new("./syntax_helper", "./config") {
        Ok(engine) => {
            // Step 2: Analyze the file
            match engine.analyze_file(&args.file).await {
                Ok(analysis) => {
                    println!("\n✅ Анализ завершён успешно!");
                    println!("📊 Результаты:");
                    println!("   • Файл: {}", analysis.file_path);
                    println!("   • Типов найдено: {}", analysis.type_resolutions.len());
                    println!("   • Время анализа: {} мс", analysis.analysis_duration_ms);

                    if args.verbose {
                        println!("\n🔍 Детали:");
                        for (expr, resolution) in &analysis.type_resolutions {
                             // TODO: Use CliFormatter for better output
                            println!("   - {}: {:?}", expr, resolution.result);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("\n❌ Ошибка анализа: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("\n❌ Ошибка инициализации движка анализа: {}", e);
            std::process::exit(1);
        }
    }
}