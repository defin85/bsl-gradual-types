//! BSL Gradual Type System CLI Tool
//!
//! Архитектура: CLI Tool (Presentation Layer) -> AnalysisEngine (Application Layer)
//! Соответствует архитектурной диаграмме из simplified_architecture.md

mod args;
mod formatters;

use clap::Parser;
use colored::*;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use args::{CacheCommand, CliArgs, CliOutputFormat, Commands};
use bsl_analysis_v2::{AnalysisHostV2, Change as ChangeV2, FileId as V2FileId, SettingsId};
use bsl_shared::engine::AnalysisEngine;
use bsl_shared::formatting::DetailLevel;
use formatters::CliFormatter;

use bsl_backend::application::type_system::web_api_service;
use bsl_backend::application::TypeInferenceService;
use bsl_backend::system::{build_deps_bundle_v2, SystemCoordinator};
use bsl_shared::TypeResolver;

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
        Commands::Cache {
            config_path,
            action,
        } => cache_command(&config_path, action).await,
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
            bsl_shared::domain::types::Certainty::InferredWeak => warnings += 1,
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

    // v2-only: используем deps bundle (SemanticDeps snapshot) вместо legacy фасада системы типов.
    let coordinator = SystemCoordinator::new();
    coordinator.start().await?;

    let deps_bundle = build_deps_bundle_v2(&coordinator, None, None)?;
    let deps = deps_bundle.semantic_deps;
    let resolver = deps
        .resolver
        .clone()
        .unwrap_or_else(|| Arc::new(TypeResolver::new(deps.repository.clone())));
    let inference_service = TypeInferenceService::new(resolver, deps.repository.clone());

    let completions = web_api_service::get_type_completions(&inference_service, expression).await?;

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

    // v2-only: используем deps bundle (SemanticDeps snapshot) вместо legacy фасада системы типов.
    let coordinator = SystemCoordinator::new();
    coordinator.start().await?;

    let deps_bundle = build_deps_bundle_v2(&coordinator, None, None)?;
    let deps = deps_bundle.semantic_deps;
    let resolver = deps
        .resolver
        .clone()
        .unwrap_or_else(|| Arc::new(TypeResolver::new(deps.repository.clone())));
    let inference_service = TypeInferenceService::new(resolver, deps.repository.clone());

    let resolution = web_api_service::get_type_details(&inference_service, expression)
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
    let coordinator = SystemCoordinator::new();
    coordinator.start().await?;

    let deps_bundle = build_deps_bundle_v2(&coordinator, None, None)?;

    // 2. Читаем файл
    let content = std::fs::read_to_string(path)?;

    // 3. Парсинг → IR через v2 (salsa)
    println!("📝 Парсинг → IR...");

    let file_id = V2FileId(1);
    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: deps_bundle.deps_id.clone(),
        deps: deps_bundle.semantic_deps.clone(),
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("cli"),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(ChangeV2::SetFile {
        file_id,
        text: Arc::from(content),
        version: 0,
        path: Arc::from(path.to_string()),
    });

    let analysis = host.analysis();

    let parse_start = Instant::now();
    analysis
        .parse_result(file_id)
        .map_err(|_| anyhow::anyhow!("v2 parse_result cancelled"))?
        .ok_or_else(|| anyhow::anyhow!("v2 parse_result unavailable"))?;
    let parse_duration_ms = parse_start.elapsed().as_millis();

    let analysis_start = Instant::now();
    let ir = analysis
        .ir(file_id)
        .map_err(|_| anyhow::anyhow!("v2 ir cancelled"))?
        .ok_or_else(|| anyhow::anyhow!("v2 ir unavailable"))?;
    let analysis_duration_ms = analysis_start.elapsed().as_millis();

    let type_resolutions = collect_ir_type_resolutions(&ir);

    // 4. Вывод результатов
    println!("\n{}", "✅ Результаты анализа:".green().bold());
    println!("   • Узлов IR: {}", ir.nodes.len().to_string().cyan());
    println!(
        "   • Типов разрешено: {}",
        type_resolutions.len().to_string().cyan()
    );
    println!(
        "   • Время парсинга: {}ms",
        parse_duration_ms.to_string().yellow()
    );
    println!(
        "   • Время анализа: {}ms",
        analysis_duration_ms.to_string().yellow()
    );

    // 5. SymbolTable
    if show_symbols || verbose {
        println!("\n{}", "📋 Symbol Table:".blue().bold());
        println!("   • Scopes: {}", ir.symbols.scopes.len());
        println!("   • Функции: {}", ir.symbols.global_functions.len());
        println!("   • Процедуры: {}", ir.symbols.global_procedures.len());

        if verbose {
            for (name, sig) in &ir.symbols.global_functions {
                // Phase 3: type_hint теперь Option<TypeResolution>
                let params = sig
                    .params
                    .iter()
                    .map(|p| {
                        format!(
                            "{}: {}",
                            p.name,
                            p.type_hint
                                .as_ref()
                                .map(|t| t.type_name())
                                .unwrap_or_else(|| "?".to_string())
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("     - Функция {}({})", name.cyan(), params);
            }

            for (name, sig) in &ir.symbols.global_procedures {
                // Phase 3: type_hint теперь Option<TypeResolution>
                let params = sig
                    .params
                    .iter()
                    .map(|p| {
                        format!(
                            "{}: {}",
                            p.name,
                            p.type_hint
                                .as_ref()
                                .map(|t| t.type_name())
                                .unwrap_or_else(|| "?".to_string())
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("     - Процедура {}({})", name.cyan(), params);
            }
        }
    }

    // 6. IR структура
    if show_ir {
        println!("\n{}", "🔍 IR Nodes:".magenta().bold());
        for (idx, node) in ir.nodes.iter().enumerate().take(10) {
            let node_type = format!("{:?}", node.kind);
            let first_line = node_type.lines().next().unwrap_or("?");

            if let Some(resolution) = type_resolutions.get(&idx) {
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
        if ir.nodes.len() > 10 {
            println!("   ... и ещё {} узлов", ir.nodes.len() - 10);
        }
    }

    // 7. Типовая информация (как в старом analyze)
    if matches!(format, CliOutputFormat::Json) {
        let json = serde_json::json!({
            "file": path,
            "nodes": ir.nodes.len(),
            "types_resolved": type_resolutions.len(),
            "parse_time_ms": parse_duration_ms,
            "analysis_time_ms": analysis_duration_ms,
        });
        println!("\n{}", serde_json::to_string_pretty(&json)?);
    }

    Ok(())
}

fn collect_ir_type_resolutions(
    ir: &bsl_shared::ir::SemanticProgram,
) -> HashMap<usize, bsl_shared::domain::types::TypeResolution> {
    use bsl_shared::domain::types::TypeResolution;
    use bsl_shared::ir::SemanticNodeKind;

    let mut resolutions = HashMap::new();
    for (idx, node) in ir.nodes.iter().enumerate() {
        let resolution = match &node.kind {
            SemanticNodeKind::VariableDeclaration {
                type_hint: Some(type_hint),
                ..
            } => type_hint.clone(),
            SemanticNodeKind::VariableDeclaration {
                type_hint: None,
                initial_value_type: Some(initial_value_type),
                ..
            } => initial_value_type.clone(),
            SemanticNodeKind::VariableDeclaration { .. } => TypeResolution::unknown(),
            SemanticNodeKind::Assignment { value_type, .. } => value_type.clone(),
            SemanticNodeKind::FunctionDeclaration {
                return_type: Some(return_type),
                ..
            } => return_type.clone(),
            SemanticNodeKind::IfStatement { condition_type, .. } => condition_type.clone(),
            SemanticNodeKind::WhileLoop { condition_type, .. } => condition_type.clone(),
            SemanticNodeKind::ForLoop { range_type, .. } => range_type.clone(),
            SemanticNodeKind::ForEachLoop {
                collection_type, ..
            } => collection_type.clone(),
            SemanticNodeKind::Return {
                value_type: Some(value_type),
            } => value_type.clone(),
            SemanticNodeKind::GlobalPropertyAccess { result_type, .. } => result_type.clone(),
            SemanticNodeKind::MemberAccess { result_type, .. } => result_type.clone(),
            SemanticNodeKind::FunctionCall { result_type, .. } => result_type.clone(),
            SemanticNodeKind::NewExpression { result_type, .. } => result_type.clone(),
            _ => continue,
        };

        resolutions.insert(idx, resolution);
    }

    resolutions
}

async fn cache_command(config_path: &str, action: CacheCommand) -> anyhow::Result<()> {
    let coordinator = bsl_backend::system::SystemCoordinator::new();
    let scope = coordinator.cache_scope_for_config_path(Path::new(config_path))?;

    match action {
        CacheCommand::Stats => {
            let report = coordinator.cache_stats(&scope).await?;
            let output = serde_json::to_string_pretty(&report)?;
            println!("{}", output);
        }
        CacheCommand::Clear => {
            let report = coordinator.clear_cache_scope(&scope).await?;
            let output = serde_json::to_string_pretty(&report)?;
            println!("{}", output);
        }
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
