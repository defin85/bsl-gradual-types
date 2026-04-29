//! BSL Gradual Type System CLI Tool
//!
//! Архитектура: CLI Tool (Presentation Layer) -> AnalysisHostV2/AnalysisV2 (bsl-analysis-v2)
//! Соответствует архитектурной диаграмме из simplified_architecture.md

mod args;
mod formatters;
mod runtime;

use anyhow::Context;
use clap::Parser;
use colored::*;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use args::{CacheCommand, CliArgs, CliOutputFormat, Commands};
use bsl_backend::application::{
    completion_member_access_owner_type_hints_from_analysis,
    get_completion_with_semantic_program_snapshot_with_trigger_hint_and_owner_hints_with_snapshot_ids,
    SemanticOperation,
};
use bsl_shared::domain::types::{DiagnosticSeverity, TypeResolution};
use bsl_shared::domain::CompletionItem;
use bsl_shared::engine::CliAnalysisResult;
use bsl_shared::formatting::DetailLevel;
use bsl_shared::ir::SemanticNodeKind;
use formatters::CliFormatter;
use runtime::{
    prepare_cli_file_operation, prepare_cli_file_operation_with_rules_config,
    prepare_cli_text_operation, prepare_cli_text_operation_with_rules_config,
    CliPreparedFileOperation,
};

#[tokio::main]
async fn main() {
    let args = CliArgs::parse();
    let rules_config = args.rules_config.clone();

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
                rules_config.as_deref(),
            )
            .await
        }
        Commands::Check { path, strict } => {
            check_command(
                &path,
                &args.format,
                args.verbose,
                strict,
                rules_config.as_deref(),
            )
            .await
        }
        Commands::Complete {
            expression,
            path,
            limit,
        } => {
            complete_command(
                &expression,
                path.as_deref(),
                &args.format,
                limit,
                rules_config.as_deref(),
            )
            .await
        }
        Commands::Info { expression, path } => {
            info_command(
                &expression,
                path.as_deref(),
                &args.format,
                rules_config.as_deref(),
            )
            .await
        }
        Commands::AnalyzeIr {
            path,
            show_ir,
            show_symbols,
        } => {
            analyze_ir_command(
                &path,
                &args.format,
                args.verbose,
                show_ir,
                show_symbols,
                rules_config.as_deref(),
            )
            .await
        }
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
    rules_config: Option<&str>,
) -> anyhow::Result<()> {
    println!("{} {}", "🚀 Анализ BSL кода:".green().bold(), path.cyan());

    if Path::new(path).is_file() {
        // Анализ одного файла
        let result = analyze_file_v2(path, DetailLevel::Full, rules_config).await?;
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
    rules_config: Option<&str>,
) -> anyhow::Result<()> {
    println!("{} {}", "🔍 Проверка типов:".blue().bold(), path.cyan());

    let prepared = prepare_cli_file_operation_with_rules_config(
        path,
        SemanticOperation::Diagnostics,
        DetailLevel::Full,
        rules_config,
    )
    .await?;
    let syntax = prepared.syntax_diagnostics()?;
    let semantic = prepared.semantic_diagnostics(false)?;

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
    file_path: Option<&str>,
    format: &CliOutputFormat,
    limit: usize,
    rules_config: Option<&str>,
) -> anyhow::Result<()> {
    println!(
        "{} {}",
        "💡 Автодополнения для:".yellow().bold(),
        expression.cyan()
    );

    let completions = collect_cli_completion_items(expression, file_path, rules_config).await?;

    let output = CliFormatter::format_completions(&completions, format, limit);
    println!("{}", output);

    Ok(())
}

/// Команда получения информации о типе
async fn info_command(
    expression: &str,
    file_path: Option<&str>,
    format: &CliOutputFormat,
    rules_config: Option<&str>,
) -> anyhow::Result<()> {
    println!(
        "{} {}",
        "ℹ️  Информация о типе:".blue().bold(),
        expression.cyan()
    );

    let resolution = resolve_cli_expression_type(expression, file_path, rules_config).await?;

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
    rules_config: Option<&str>,
) -> anyhow::Result<()> {
    println!("{} {}", "🎯 IR-based анализ:".green().bold(), path.cyan());

    let prepared = prepare_cli_file_operation_with_rules_config(
        path,
        SemanticOperation::TypeAtPosition,
        DetailLevel::Full,
        rules_config,
    )
    .await?;

    println!("📝 Парсинг → IR...");

    let parse_start = Instant::now();
    prepared
        .analysis()
        .parse_result(prepared.file_id)
        .map_err(|_| anyhow::anyhow!("v2 parse_result cancelled"))?
        .ok_or_else(|| anyhow::anyhow!("v2 parse_result unavailable"))?;
    let parse_duration_ms = parse_start.elapsed().as_millis();

    let analysis_start = Instant::now();
    let ir = prepared.ir_program()?;
    let analysis_duration_ms = analysis_start.elapsed().as_millis();

    let type_resolutions = collect_ir_type_resolutions(&prepared, &ir)?;

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
                    .map(|p| format!("{}: {}", p.name, p.type_hint.as_deref().unwrap_or("?")))
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("     - Функция {}({})", name.cyan(), params);
            }

            for (name, sig) in &ir.symbols.global_procedures {
                let params = sig
                    .params
                    .iter()
                    .map(|p| format!("{}: {}", p.name, p.type_hint.as_deref().unwrap_or("?")))
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
    prepared: &CliPreparedFileOperation,
    ir: &bsl_shared::ir::SemanticProgram,
) -> anyhow::Result<HashMap<usize, TypeResolution>> {
    let mut resolutions = HashMap::new();
    for (idx, node) in ir.nodes.iter().enumerate() {
        let Some(resolution) = type_at_span(prepared, node.span)? else {
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

async fn analyze_file_v2(
    path: &str,
    diagnostics_detail_level: DetailLevel,
    rules_config: Option<&str>,
) -> anyhow::Result<CliAnalysisResult> {
    let start_time = std::time::Instant::now();

    let prepared = prepare_cli_file_operation_with_rules_config(
        path,
        SemanticOperation::TypeAtPosition,
        diagnostics_detail_level,
        rules_config,
    )
    .await?;
    let ir = prepared.ir_program()?;

    let mut vars: HashMap<String, TypeResolution> = HashMap::new();
    for node in &ir.nodes {
        match &node.kind {
            SemanticNodeKind::VariableDeclaration {
                name, type_hint, ..
            } => {
                let resolution = type_hint
                    .as_deref()
                    .map(TypeResolution::explicit)
                    .unwrap_or_else(TypeResolution::unknown);
                vars.entry(name.clone()).or_insert(resolution);
            }
            SemanticNodeKind::Assignment { variable, .. } => {
                if let Some(resolution) =
                    prepared.serve_only_type_at_byte_offset(node.span.start)?
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
    prepared: &CliPreparedFileOperation,
    span: bsl_shared::ir::Span,
) -> anyhow::Result<Option<TypeResolution>> {
    if !span.is_empty() {
        let end_inclusive = span.end.saturating_sub(1);
        if let Some(found) = prepared.serve_only_type_at_byte_offset(end_inclusive)? {
            return Ok(Some(found));
        }
    }

    prepared.serve_only_type_at_byte_offset(span.start)
}

#[derive(Debug)]
#[cfg(test)]
struct CliDiagnosticsSummary {
    syntax_messages: Vec<String>,
    semantic_messages: Vec<String>,
}

struct InlineCliExpression {
    file_text: Arc<str>,
    file_path: Arc<str>,
    line: u32,
    cursor_column: u32,
}

fn inline_cli_file_path(file_path: Option<&str>, default_path: &str) -> Arc<str> {
    file_path
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(|path| Arc::<str>::from(path.to_string()))
        .unwrap_or_else(|| Arc::<str>::from(default_path.to_string()))
}

fn inline_cli_expression(
    expression: &str,
    file_path: Option<&str>,
) -> anyhow::Result<InlineCliExpression> {
    let expression = expression.trim();
    if expression.is_empty() {
        return Err(anyhow::anyhow!("CLI expression must not be empty"));
    }

    let expression_line = format!("    {expression}");
    let cursor_column =
        bsl_analysis_v2::byte_offset_to_utf16(&expression_line, expression_line.len());
    let file_text = Arc::<str>::from(format!(
        "Процедура Test()\n{expression_line}\nКонецПроцедуры\n"
    ));

    Ok(InlineCliExpression {
        file_text,
        file_path: inline_cli_file_path(file_path, "/virtual/cli-inline-expression.bsl"),
        line: 1,
        cursor_column,
    })
}

fn inline_cli_resolution_probe(
    expression: &str,
    file_path: Option<&str>,
) -> anyhow::Result<(Arc<str>, Arc<str>, u32)> {
    let expression = expression.trim();
    if expression.is_empty() {
        return Err(anyhow::anyhow!("CLI expression must not be empty"));
    }

    let file_text = Arc::<str>::from(format!(
        "Процедура Test()\n    Arr = {expression};\n    ForType = Arr;\nКонецПроцедуры\n"
    ));
    let probe_offset = file_text
        .rfind("Arr")
        .ok_or_else(|| anyhow::anyhow!("resolution probe marker missing"))?
        .min(u32::MAX as usize) as u32;

    Ok((
        file_text,
        inline_cli_file_path(file_path, "/virtual/cli-inline-resolution.bsl"),
        probe_offset,
    ))
}

fn inline_cli_completion_expression(
    expression: &str,
    file_path: Option<&str>,
) -> anyhow::Result<InlineCliExpression> {
    let expression = expression.trim();
    if !expression_targets_member_access(expression) {
        return inline_cli_expression(expression, file_path);
    }

    member_access_receiver_expression(expression)
        .ok_or_else(|| anyhow::anyhow!("member access expression receiver must not be empty"))?;

    let completion_line = format!("    {expression}");
    let cursor_column =
        bsl_analysis_v2::byte_offset_to_utf16(&completion_line, completion_line.len());
    let file_text = Arc::<str>::from(format!(
        "Процедура Test()\n{completion_line}\nКонецПроцедуры\n"
    ));
    Ok(InlineCliExpression {
        file_text,
        file_path: inline_cli_file_path(file_path, "/virtual/cli-inline-completion.bsl"),
        line: 1,
        cursor_column,
    })
}

fn expression_targets_member_access(expression: &str) -> bool {
    let trimmed = expression.trim_end();
    let Some(dot_pos) = trimmed.rfind('.') else {
        return false;
    };
    let after_dot = trimmed[dot_pos + 1..].trim_start();
    after_dot.is_empty()
        || after_dot
            .chars()
            .all(|ch| ch == '_' || ch.is_alphanumeric())
}

fn member_access_receiver_expression(expression: &str) -> Option<&str> {
    let trimmed = expression.trim_end();
    let dot_pos = trimmed.rfind('.')?;
    let receiver = trimmed[..dot_pos].trim_end();
    (!receiver.is_empty()).then_some(receiver)
}

fn cli_completion_owner_hints(
    expression: &str,
    inline: &InlineCliExpression,
    prepared: &CliPreparedFileOperation,
) -> Vec<TypeResolution> {
    let member_access_request = expression_targets_member_access(expression);
    let owner_hints = if member_access_request {
        completion_member_access_owner_type_hints_from_analysis(
            prepared.analysis(),
            prepared.file_id,
            inline.file_text.as_ref(),
            inline.line,
            inline.cursor_column,
        )
    } else {
        Vec::new()
    };

    owner_hints
        .into_iter()
        .filter(|hint| !hint.is_unknown() && !hint.is_dynamic())
        .collect()
}

async fn collect_cli_completion_items(
    expression: &str,
    file_path: Option<&str>,
    rules_config: Option<&str>,
) -> anyhow::Result<Vec<CompletionItem>> {
    let inline = inline_cli_completion_expression(expression, file_path)?;
    let prepared = prepare_cli_text_operation_with_rules_config(
        inline.file_text.clone(),
        inline.file_path.clone(),
        SemanticOperation::Completion,
        DetailLevel::Full,
        rules_config,
    )
    .await?;
    let ir_program = prepared.ir_program()?;
    let trigger_char_hint = expression.trim_end().chars().last().filter(|ch| *ch == '.');

    let owner_hints = cli_completion_owner_hints(expression, &inline, &prepared);
    let completions =
        get_completion_with_semantic_program_snapshot_with_trigger_hint_and_owner_hints_with_snapshot_ids(
            inline.file_text.as_ref(),
            inline.line,
            inline.cursor_column,
            None,
            prepared.index_snapshot(),
            &prepared.metadata_lookup,
            inline.file_path.as_ref(),
            prepared.resolver.as_ref(),
            ir_program,
            owner_hints,
            false,
            prepared.context.expected_deps_id.as_ref(),
            Some(&prepared.context.settings.settings_id),
            trigger_char_hint,
        )
        .await
        .context("cli shared-runtime completion query failed")?;

    Ok(completions
        .items
        .into_iter()
        .map(|candidate| candidate.item)
        .collect())
}

async fn resolve_cli_expression_type(
    expression: &str,
    file_path: Option<&str>,
    rules_config: Option<&str>,
) -> anyhow::Result<TypeResolution> {
    let (file_text, file_path, probe_offset) = inline_cli_resolution_probe(expression, file_path)?;
    let prepared = prepare_cli_text_operation_with_rules_config(
        file_text,
        file_path,
        SemanticOperation::TypeAtPosition,
        DetailLevel::Full,
        rules_config,
    )
    .await?;

    prepared
        .serve_only_type_at_byte_offset(probe_offset)?
        .ok_or_else(|| anyhow::anyhow!("Тип '{}' не найден", expression.trim()))
}

#[cfg(test)]
async fn collect_cli_file_diagnostics(
    path: &str,
    diagnostics_detail_level: DetailLevel,
) -> anyhow::Result<CliDiagnosticsSummary> {
    let prepared = prepare_cli_file_operation(
        path,
        SemanticOperation::Diagnostics,
        diagnostics_detail_level,
    )
    .await?;
    let syntax_messages = prepared
        .syntax_diagnostics()?
        .iter()
        .map(|diag| diag.message.clone())
        .collect();
    let semantic_messages = prepared
        .semantic_diagnostics(false)?
        .iter()
        .map(|diag| diag.message.clone())
        .collect();

    Ok(CliDiagnosticsSummary {
        syntax_messages,
        semantic_messages,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsl_backend::system::{
        IndexItem, IndexItemKind, IndexKind, IndexSnapshot, IndexSnapshotId, TypeKind,
    };
    use bsl_shared::domain::types::FacetKind;
    use bsl_shared::formatting::user_facing_resolution_type_name;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn cli_inline_completion_uses_shared_runtime_snapshot() {
        let completions = collect_cli_completion_items("Новый Массив.", None, None)
            .await
            .expect("cli completions");
        let labels: Vec<_> = completions.into_iter().map(|item| item.label).collect();
        assert!(
            labels.iter().any(|label| label == "Добавить"),
            "expected canonical completion items, got {labels:?}"
        );
    }

    #[tokio::test]
    async fn cli_inline_completion_preserves_canonical_generic_owner_hint() {
        let completions = collect_cli_completion_items("(Новый Массив()).", None, None)
            .await
            .expect("cli parenthesized completions");
        let labels: Vec<_> = completions.into_iter().map(|item| item.label).collect();
        assert!(
            labels.iter().any(|label| label == "Добавить"),
            "CLI production completion path must preserve canonical generic owner semantics, labels={labels:?}"
        );
    }

    #[tokio::test]
    async fn cli_inline_completion_does_not_backfill_from_polluted_search_index() {
        let inline =
            inline_cli_completion_expression("Несуществующий.", None).expect("inline expr");
        let mut prepared = prepare_cli_text_operation(
            inline.file_text.clone(),
            inline.file_path.clone(),
            SemanticOperation::Completion,
            DetailLevel::Full,
        )
        .await
        .expect("prepare cli completion");
        let ir_program = prepared.ir_program().expect("ir program");

        let mut polluted_snapshot =
            IndexSnapshot::empty(IndexSnapshotId::from_hash("cli-search-only-snapshot"));
        Arc::make_mut(&mut polluted_snapshot.type_index).insert(
            "SearchOnlyType".to_string(),
            Arc::new(IndexItem::new(
                "SearchOnlyType".to_string(),
                IndexItemKind::Type(TypeKind::Generic),
                IndexKind::Type,
            )),
        );
        prepared.prepared.index_snapshot = Arc::new(polluted_snapshot);

        let owner_hints = completion_member_access_owner_type_hints_from_analysis(
            prepared.analysis(),
            prepared.file_id,
            inline.file_text.as_ref(),
            inline.line,
            inline.cursor_column,
        );
        assert!(
            owner_hints.is_empty(),
            "test precondition: canonical CLI owner hints must be absent for unresolved receiver"
        );

        let completions =
            get_completion_with_semantic_program_snapshot_with_trigger_hint_and_owner_hints_with_snapshot_ids(
                inline.file_text.as_ref(),
                inline.line,
                inline.cursor_column,
                None,
                prepared.index_snapshot(),
                &prepared.metadata_lookup,
                inline.file_path.as_ref(),
                prepared.resolver.as_ref(),
                ir_program,
                owner_hints,
                false,
                prepared.context.expected_deps_id.as_ref(),
                Some(&prepared.context.settings.settings_id),
                Some('.'),
            )
            .await
            .expect("cli completion query");
        let labels: Vec<_> = completions
            .items
            .into_iter()
            .map(|candidate| candidate.item.label)
            .collect();
        assert!(
            labels.is_empty(),
            "CLI completion must stay fail-closed when only polluted search index is available, labels={labels:?}"
        );
    }

    #[tokio::test]
    async fn cli_inline_type_info_uses_shared_runtime_snapshot() {
        let resolution = resolve_cli_expression_type("Новый Массив", None, None)
            .await
            .expect("cli type info");
        assert!(
            user_facing_resolution_type_name(&resolution).starts_with("Массив"),
            "expected shared runtime array resolution, got {:?}",
            resolution
        );
    }

    #[tokio::test]
    async fn cli_inline_completion_preserves_object_module_binding_facets() {
        let without_path = collect_cli_completion_items("Объект.", None, None)
            .await
            .expect("cli completion without module path");
        let without_path_labels: Vec<_> = without_path.into_iter().map(|item| item.label).collect();
        assert!(
            without_path_labels.is_empty(),
            "synthetic inline completion must stay unresolved for object-module-only binding without --path, labels={without_path_labels:?}"
        );

        let completions = collect_cli_completion_items(
            "Объект.",
            Some("Documents/Док1/Ext/ObjectModule.bsl"),
            None,
        )
        .await
        .expect("cli object module completions");
        let labels: Vec<_> = completions.into_iter().map(|item| item.label).collect();

        assert!(
            labels.iter().any(|label| label == "Записать"),
            "CLI completion must honor public --path module context for object-module bindings, labels={labels:?}"
        );
        assert!(
            labels.iter().any(|label| label == "ЭтоНовый"),
            "CLI completion must expose object-module members on the public --path path, labels={labels:?}"
        );
    }

    #[tokio::test]
    async fn cli_type_info_preserves_object_module_binding_facets() {
        let resolution = resolve_cli_expression_type(
            "Объект",
            Some("Documents/Док1/Ext/ObjectModule.bsl"),
            None,
        )
        .await
        .expect("cli object module type info");

        assert!(
            user_facing_resolution_type_name(&resolution).contains("Док1"),
            "expected object module binding to preserve configuration identity, got {:?}",
            resolution
        );
        assert_eq!(resolution.active_facet, Some(FacetKind::Object));
        assert!(
            resolution.available_facets.contains(&FacetKind::Object),
            "expected object facet to survive CLI shared runtime path, got {:?}",
            resolution.available_facets
        );
    }

    #[tokio::test]
    async fn cli_file_diagnostics_use_shared_runtime_snapshot() {
        let file = NamedTempFile::new().expect("temp file");
        std::fs::write(
            file.path(),
            "Процедура Тест()\n    x = 1;\n    x.UnknownMethod();\nКонецПроцедуры\n",
        )
        .expect("write fixture");

        let result = collect_cli_file_diagnostics(
            file.path().to_str().expect("fixture path"),
            DetailLevel::Full,
        )
        .await
        .expect("cli diagnostics");

        assert_eq!(
            result.syntax_messages.len(),
            0,
            "unexpected syntax: {result:#?}"
        );
        assert!(
            result
                .semantic_messages
                .iter()
                .any(|message| message.contains("UnknownMethod")),
            "expected semantic diagnostics from shared runtime, got {result:#?}"
        );
    }
}
