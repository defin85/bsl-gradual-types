//! BSL Gradual Type System CLI Tool
//!
//! Архитектура: CLI Tool (Presentation Layer) -> AnalysisHostV2/AnalysisV2 (bsl-analysis-v2)
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
use bsl_shared::formatting::DetailLevel;
use formatters::CliFormatter;

use bsl_backend::application::type_system::web_api_service;
use bsl_backend::system::{build_deps_bundle_v2, SystemCoordinator};
use bsl_shared::domain::types::{DiagnosticSeverity, TypeResolution};
use bsl_shared::engine::CliAnalysisResult;
use bsl_shared::ir::SemanticNodeKind;

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

    if Path::new(path).is_file() {
        // Анализ одного файла
        let result = analyze_file_v2(path, DetailLevel::Full).await?;
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
    strict: bool,
) -> anyhow::Result<()> {
    println!("{} {}", "🔍 Проверка типов:".blue().bold(), path.cyan());

    let (analysis, file_id) = build_analysis_v2_for_path(path, DetailLevel::Full).await?;

    let syntax = analysis
        .syntax_diagnostics(file_id)
        .map_err(|_| anyhow::anyhow!("v2 syntax diagnostics cancelled"))?
        .unwrap_or_default();
    let semantic = analysis
        .semantic_diagnostics(file_id)
        .map_err(|_| anyhow::anyhow!("v2 semantic diagnostics cancelled"))?
        .unwrap_or_default();

    // Подсчет ошибок и предупреждений (v2 diagnostics)
    let mut errors = syntax.len();
    let mut warnings = 0usize;
    for diag in semantic.iter() {
        match diag.severity {
            DiagnosticSeverity::Error => errors += 1,
            DiagnosticSeverity::Warning => warnings += 1,
            DiagnosticSeverity::Info | DiagnosticSeverity::Hint => {}
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

    if verbose && (!syntax.is_empty() || !semantic.is_empty()) {
        if !syntax.is_empty() {
            println!("\n{}", "🧩 Syntax diagnostics:".red().bold());
            for diag in syntax.iter().take(50) {
                println!("  - {}", diag.message);
            }
        }

        if !semantic.is_empty() {
            println!("\n{}", "🧠 Semantic diagnostics:".red().bold());
            for diag in semantic.iter().take(50) {
                let severity = match diag.severity {
                    DiagnosticSeverity::Error => "error",
                    DiagnosticSeverity::Warning => "warning",
                    DiagnosticSeverity::Info => "info",
                    DiagnosticSeverity::Hint => "hint",
                };
                println!("  - [{}] {}", severity, diag.message);
            }
        }
    }

    if errors > 0 || (strict && warnings > 0) {
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

    let completions = web_api_service::get_type_completions(deps.as_ref(), expression).await?;

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

    let resolution = web_api_service::get_type_details(deps.as_ref(), expression)
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

    let type_resolutions = collect_ir_type_resolutions(&analysis, file_id, &ir)?;

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
                let params = sig
                    .params
                    .iter()
                    .map(|p| {
                        format!(
                            "{}: {}",
                            p.name,
                            p.type_hint
                                .as_deref()
                                .unwrap_or("?")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("     - Функция {}({})", name.cyan(), params);
            }

            for (name, sig) in &ir.symbols.global_procedures {
                let params = sig
                    .params
                    .iter()
                    .map(|p| {
                        format!(
                            "{}: {}",
                            p.name,
                            p.type_hint
                                .as_deref()
                                .unwrap_or("?")
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
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: V2FileId,
    ir: &bsl_shared::ir::SemanticProgram,
) -> anyhow::Result<HashMap<usize, TypeResolution>> {
    let mut resolutions = HashMap::new();
    for (idx, node) in ir.nodes.iter().enumerate() {
        let Some(resolution) = type_at_span(analysis, file_id, node.span)? else {
            continue;
        };
        resolutions.insert(idx, resolution);
    }
    Ok(resolutions)
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

fn compute_settings_id_v2(diagnostics_detail_level: DetailLevel) -> SettingsId {
    SettingsId::from_hash(format!(
        "cli;schema={};diagnostics.detail_level={:?}",
        bsl_analysis_v2::SETTINGS_SCHEMA_VERSION,
        diagnostics_detail_level
    ))
}

async fn build_analysis_v2_for_path(
    path: &str,
    diagnostics_detail_level: DetailLevel,
) -> anyhow::Result<(bsl_analysis_v2::AnalysisV2, V2FileId)> {
    let coordinator = SystemCoordinator::new();
    coordinator.start().await?;

    let deps_bundle = build_deps_bundle_v2(&coordinator, None, None)?;
    let content = std::fs::read_to_string(path)?;

    let file_id = V2FileId(1);
    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: deps_bundle.deps_id.clone(),
        deps: deps_bundle.semantic_deps.clone(),
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: compute_settings_id_v2(diagnostics_detail_level),
        diagnostics_detail_level,
    });
    host.apply_change(ChangeV2::SetFile {
        file_id,
        text: Arc::from(content),
        version: 0,
        path: Arc::from(path.to_string()),
    });

    Ok((host.analysis(), file_id))
}

async fn analyze_file_v2(
    path: &str,
    diagnostics_detail_level: DetailLevel,
) -> anyhow::Result<CliAnalysisResult> {
    let start_time = std::time::Instant::now();

    let (analysis, file_id) = build_analysis_v2_for_path(path, diagnostics_detail_level).await?;
    let ir = analysis
        .ir(file_id)
        .map_err(|_| anyhow::anyhow!("v2 ir cancelled"))?
        .ok_or_else(|| anyhow::anyhow!("v2 ir unavailable"))?;

    let mut vars: HashMap<String, TypeResolution> = HashMap::new();
    for node in &ir.nodes {
        match &node.kind {
            SemanticNodeKind::VariableDeclaration { name, type_hint, .. } => {
                let resolution = type_hint
                    .as_deref()
                    .map(TypeResolution::explicit)
                    .unwrap_or_else(TypeResolution::unknown);
                vars.entry(name.clone()).or_insert(resolution);
            }
            SemanticNodeKind::Assignment { variable, .. } => {
                if let Some(resolution) = analysis
                    .type_at_byte_offset(file_id, node.span.start)
                    .map_err(|_| anyhow::anyhow!("v2 type_at_byte_offset cancelled"))?
                {
                    vars.insert(variable.clone(), resolution);
                }
            }
            _ => {}
        }
    }

    let mut type_resolutions: Vec<(String, TypeResolution)> = vars.into_iter().collect();
    type_resolutions.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(CliAnalysisResult {
        file_path: path.to_string(),
        type_resolutions,
        analysis_duration_ms: start_time.elapsed().as_millis(),
    })
}

fn type_at_span(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: V2FileId,
    span: bsl_shared::ir::Span,
) -> anyhow::Result<Option<TypeResolution>> {
    if !span.is_empty() {
        let end_inclusive = span.end.saturating_sub(1);
        if let Some(found) = analysis
            .type_at_byte_offset(file_id, end_inclusive)
            .map_err(|_| anyhow::anyhow!("v2 type_at_byte_offset cancelled"))?
        {
            return Ok(Some(found));
        }
    }

    analysis
        .type_at_byte_offset(file_id, span.start)
        .map_err(|_| anyhow::anyhow!("v2 type_at_byte_offset cancelled"))
}
