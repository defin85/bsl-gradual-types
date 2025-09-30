//! BSL Gradual Type System CLI Tool
//!
//! Архитектура: CLI Tool (Presentation Layer) -> AnalysisEngine (Application Layer)
//! Соответствует архитектурной диаграмме из simplified_architecture.md

mod args;
mod formatters;

use clap::Parser;
use colored::*;
use std::path::Path;

use args::{CliArgs, Commands, OutputFormat};
use formatters::CliFormatter;
use bsl_shared::engine::AnalysisEngine;

#[tokio::main]
async fn main() {
    let args = CliArgs::parse();

    // Инициализация логирования
    if args.verbose {
        tracing_subscriber::fmt()
            .with_env_filter("debug")
            .init();
    }

    // Выполнение команд
    let result = match args.command {
        Commands::Analyze { path, errors_only, show_inference } => {
            analyze_command(&path, &args.format, args.verbose, errors_only, show_inference).await
        }
        Commands::Check { path, strict } => {
            check_command(&path, &args.format, args.verbose, strict).await
        }
        Commands::Complete { expression, limit } => {
            complete_command(&expression, &args.format, limit).await
        }
        Commands::Info { expression } => {
            info_command(&expression, &args.format).await
        }
    };

    if let Err(e) = result {
        eprintln!("{} {}", "❌ Ошибка:".red().bold(), e);
        std::process::exit(1);
    }
}

/// Команда анализа файлов
async fn analyze_command(
    path: &str,
    format: &OutputFormat,
    verbose: bool,
    errors_only: bool,
    _show_inference: bool,
) -> anyhow::Result<()> {
    println!("{} {}", "🚀 Анализ BSL кода:".green().bold(), path.cyan());

    // Создаем AnalysisEngine согласно архитектурной диаграмме
    let engine = create_analysis_engine().await?;

    if Path::new(path).is_file() {
        // Анализ одного файла
        let result = engine.analyze_file(path).await?;
        let output = CliFormatter::format_analysis(&result, format, verbose, errors_only);
        println!("{}", output);
    } else if Path::new(path).is_dir() {
        // Анализ директории (будущая функциональность)
        println!("{}", "📁 Анализ директорий будет реализован в будущих версиях".yellow());
    } else {
        return Err(anyhow::anyhow!("Путь не существует: {}", path));
    }

    Ok(())
}

/// Команда проверки типов
async fn check_command(
    path: &str,
    _format: &OutputFormat,
    verbose: bool,
    _strict: bool,
) -> anyhow::Result<()> {
    println!("{} {}", "🔍 Проверка типов:".blue().bold(), path.cyan());

    let engine = create_analysis_engine().await?;
    let result = engine.analyze_file(path).await?;

    // Подсчет ошибок и предупреждений
    let mut errors = 0;
    let mut warnings = 0;

    for (_, resolution) in &result.type_resolutions {
        match resolution.certainty {
            bsl_shared::domain::types::Certainty::Unknown => errors += 1,
            bsl_shared::domain::types::Certainty::Inferred(confidence) if confidence < 0.7 => warnings += 1,
            _ => {}
        }
    }

    println!("📊 Результаты проверки:");
    println!("   • Ошибок: {}", if errors > 0 { errors.to_string().red() } else { errors.to_string().green() });
    println!("   • Предупреждений: {}", if warnings > 0 { warnings.to_string().yellow() } else { warnings.to_string().green() });

    if verbose && (errors > 0 || warnings > 0) {
        let output = CliFormatter::format_analysis(&result, &OutputFormat::Table, true, true);
        println!("\n{}", output);
    }

    if errors > 0 {
        std::process::exit(1);
    }

    println!("{}", "✅ Проверка завершена успешно!".green());
    Ok(())
}

/// Команда автодополнения
async fn complete_command(
    expression: &str,
    format: &OutputFormat,
    limit: usize,
) -> anyhow::Result<()> {
    println!("{} {}", "💡 Автодополнения для:".yellow().bold(), expression.cyan());

    // Phase 4+: используем SystemCoordinator и TypeSystemService
    let coordinator = bsl_backend::system::SystemCoordinator::new();
    coordinator.start().await?;

    let type_service = coordinator.type_service()
        .ok_or_else(|| anyhow::anyhow!("TypeSystemService не доступен"))?;

    let completions = type_service.get_type_completions(expression).await?;

    let output = CliFormatter::format_completions(&completions, format, limit);
    println!("{}", output);

    Ok(())
}

/// Команда получения информации о типе
async fn info_command(expression: &str, format: &OutputFormat) -> anyhow::Result<()> {
    println!("{} {}", "ℹ️  Информация о типе:".blue().bold(), expression.cyan());

    // Phase 4+: используем SystemCoordinator и TypeSystemService
    let coordinator = bsl_backend::system::SystemCoordinator::new();
    coordinator.start().await?;

    let type_service = coordinator.type_service()
        .ok_or_else(|| anyhow::anyhow!("TypeSystemService не доступен"))?;

    let resolution = type_service.get_type_details(expression).await?
        .ok_or_else(|| anyhow::anyhow!("Тип '{}' не найден", expression))?;

    let output = CliFormatter::format_type_info(expression, &resolution, format);
    println!("{}", output);

    Ok(())
}

/// Создание AnalysisEngine через SystemCoordinator (Phase 3)
/// Возвращает Arc для избежания клонирования тяжелых объектов
async fn create_analysis_engine() -> anyhow::Result<std::sync::Arc<AnalysisEngine>> {
    // Phase 3: используем SystemCoordinator для инициализации Infrastructure
    let coordinator = bsl_backend::system::SystemCoordinator::new();

    // Инициализация системы
    coordinator.start().await?;

    // Получаем AnalysisEngine из координатора
    let engine = coordinator.analysis_engine()
        .ok_or_else(|| anyhow::anyhow!("AnalysisEngine не доступен"))?;

    Ok(engine)
}