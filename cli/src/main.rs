//! BSL Gradual Type System CLI Tool
//!
//! Архитектура: CLI Tool (Presentation Layer) -> AnalysisEngine (Application Layer)
//! Соответствует архитектурной диаграмме из simplified_architecture.md

mod args;
mod formatters;

use clap::Parser;
use colored::*;
use std::path::Path;

use args::{CliArgs, CliOutputFormat, Commands};
use bsl_shared::engine::AnalysisEngine;
use formatters::CliFormatter;

#[tokio::main]
async fn main() {
    let args = CliArgs::parse();

    // Инициализация логирования
    if args.verbose {
        tracing_subscriber::fmt().with_env_filter("debug").init();
    }

    // Выполнение команд
    let result = match args.command {
        Commands::Analyze {
            path,
            errors_only,
            show_inference,
        } => {
            analyze_command(
                &path,
                &args.format,
                args.verbose,
                errors_only,
                show_inference,
            )
            .await
        }
        Commands::Check { path, strict } => {
            check_command(&path, &args.format, args.verbose, strict).await
        }
        Commands::Complete { expression, limit } => {
            complete_command(&expression, &args.format, limit).await
        }
        Commands::Info { expression } => info_command(&expression, &args.format).await,
        Commands::AnalyzeIr {
            path,
            show_ir,
            show_symbols,
        } => analyze_ir_command(&path, &args.format, args.verbose, show_ir, show_symbols).await,
    };

    if let Err(e) = result {
        eprintln!("{} {}", "❌ Ошибка:".red().bold(), e);
        std::process::exit(1);
    }
}

/// Команда анализа файлов
async fn analyze_command(
    path: &str,
    format: &CliOutputFormat,
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
        println!(
            "{}",
            "📁 Анализ директорий будет реализован в будущих версиях".yellow()
        );
    } else {
        return Err(anyhow::anyhow!("Путь не существует: {}", path));
    }

    Ok(())
}

/// Команда проверки типов
async fn check_command(
    path: &str,
    _format: &CliOutputFormat,
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
            bsl_shared::domain::types::Certainty::Inferred(confidence) if confidence < 0.7 => {
                warnings += 1
            }
            _ => {}
        }
    }

    println!("📊 Результаты проверки:");
    println!(
        "   • Ошибок: {}",
        if errors > 0 {
            errors.to_string().red()
        } else {
            errors.to_string().green()
        }
    );
    println!(
        "   • Предупреждений: {}",
        if warnings > 0 {
            warnings.to_string().yellow()
        } else {
            warnings.to_string().green()
        }
    );

    if verbose && (errors > 0 || warnings > 0) {
        let output = CliFormatter::format_analysis(&result, &CliOutputFormat::Table, true, true);
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
    format: &CliOutputFormat,
    limit: usize,
) -> anyhow::Result<()> {
    println!(
        "{} {}",
        "💡 Автодополнения для:".yellow().bold(),
        expression.cyan()
    );

    // Phase 4+: используем SystemCoordinator и TypeSystemService
    let coordinator = bsl_backend::system::SystemCoordinator::new();
    coordinator.start().await?;

    let type_service = coordinator
        .type_service()
        .ok_or_else(|| anyhow::anyhow!("TypeSystemService не доступен"))?;

    let completions = type_service.get_type_completions(expression).await?;

    let output = CliFormatter::format_completions(&completions, format, limit);
    println!("{}", output);

    Ok(())
}

/// Команда получения информации о типе
async fn info_command(expression: &str, format: &CliOutputFormat) -> anyhow::Result<()> {
    println!(
        "{} {}",
        "ℹ️  Информация о типе:".blue().bold(),
        expression.cyan()
    );

    // Phase 4+: используем SystemCoordinator и TypeSystemService
    let coordinator = bsl_backend::system::SystemCoordinator::new();
    coordinator.start().await?;

    let type_service = coordinator
        .type_service()
        .ok_or_else(|| anyhow::anyhow!("TypeSystemService не доступен"))?;

    let resolution = type_service
        .get_type_details(expression)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Тип '{}' не найден", expression))?;

    let output = CliFormatter::format_type_info(expression, &resolution, format);
    println!("{}", output);

    Ok(())
}

/// Milestone 2.8: Команда IR-based анализа (Task 6.2)
async fn analyze_ir_command(
    path: &str,
    format: &CliOutputFormat,
    verbose: bool,
    show_ir: bool,
    show_symbols: bool,
) -> anyhow::Result<()> {
    println!("{} {}", "🎯 IR-based анализ:".green().bold(), path.cyan());

    // 1. Создаем координатор
    let coordinator = bsl_backend::system::SystemCoordinator::new();
    coordinator.start().await?;

    let engine = coordinator
        .analysis_engine()
        .ok_or_else(|| anyhow::anyhow!("AnalysisEngine не доступен"))?;

    // 2. Получаем Parser через backend (полный ParserCoordinator)
    // Согласно CLAUDE.md: CLI всегда использует полный backend (~7-8 MB)
    let parser = std::sync::Arc::new(bsl_backend::system::ParserCoordinator::with_fallback());

    // 3. Читаем файл
    let content = std::fs::read_to_string(path)?;

    // 4. Парсинг → IR → Type Analysis через AnalysisEngine
    println!("📝 Парсинг → IR...");
    let ir_result = engine.parse_and_analyze(parser.as_ref(), &content, path)?;

    // 4. Вывод результатов
    println!("\n{}", "✅ Результаты анализа:".green().bold());
    println!(
        "   • Узлов IR: {}",
        ir_result.ir.nodes.len().to_string().cyan()
    );
    println!(
        "   • Типов разрешено: {}",
        ir_result.type_resolutions.len().to_string().cyan()
    );
    println!(
        "   • Время парсинга: {}ms",
        ir_result.parse_duration_ms.to_string().yellow()
    );
    println!(
        "   • Время анализа: {}ms",
        ir_result.analysis_duration_ms.to_string().yellow()
    );

    // 5. SymbolTable
    if show_symbols || verbose {
        println!("\n{}", "📋 Symbol Table:".blue().bold());
        println!("   • Scopes: {}", ir_result.ir.symbols.scopes.len());
        println!(
            "   • Функции: {}",
            ir_result.ir.symbols.global_functions.len()
        );
        println!(
            "   • Процедуры: {}",
            ir_result.ir.symbols.global_procedures.len()
        );

        if verbose {
            for (name, sig) in &ir_result.ir.symbols.global_functions {
                // Phase 3: type_hint теперь Option<TypeResolution>
                let params = sig
                    .params
                    .iter()
                    .map(|p| format!("{}: {}", p.name, p.type_hint.as_ref().map(|t| t.type_name()).unwrap_or_else(|| "?".to_string())))
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("     - Функция {}({})", name.cyan(), params);
            }

            for (name, sig) in &ir_result.ir.symbols.global_procedures {
                // Phase 3: type_hint теперь Option<TypeResolution>
                let params = sig
                    .params
                    .iter()
                    .map(|p| format!("{}: {}", p.name, p.type_hint.as_ref().map(|t| t.type_name()).unwrap_or_else(|| "?".to_string())))
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("     - Процедура {}({})", name.cyan(), params);
            }
        }
    }

    // 6. IR структура
    if show_ir {
        println!("\n{}", "🔍 IR Nodes:".magenta().bold());
        for (idx, node) in ir_result.ir.nodes.iter().enumerate().take(10) {
            let node_type = format!("{:?}", node.kind);
            let first_line = node_type.lines().next().unwrap_or("?");

            if let Some(resolution) = ir_result.type_resolutions.get(&idx) {
                println!(
                    "   [{:2}] {} → {:?}",
                    idx,
                    first_line.yellow(),
                    resolution.certainty
                );
            } else {
                println!("   [{:2}] {}", idx, first_line.dimmed());
            }
        }
        if ir_result.ir.nodes.len() > 10 {
            println!("   ... и ещё {} узлов", ir_result.ir.nodes.len() - 10);
        }
    }

    // 7. Типовая информация (как в старом analyze)
    if matches!(format, CliOutputFormat::Json) {
        let json = serde_json::json!({
            "file": path,
            "nodes": ir_result.ir.nodes.len(),
            "types_resolved": ir_result.type_resolutions.len(),
            "parse_time_ms": ir_result.parse_duration_ms,
            "analysis_time_ms": ir_result.analysis_duration_ms,
        });
        println!("\n{}", serde_json::to_string_pretty(&json)?);
    }

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
    let engine = coordinator
        .analysis_engine()
        .ok_or_else(|| anyhow::anyhow!("AnalysisEngine не доступен"))?;

    Ok(engine)
}
