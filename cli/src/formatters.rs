//! Output formatters for CLI Tool
//!
//! Согласно архитектурной диаграмме: CLI Tool (Presentation Layer)

use colored::*;
use bsl_shared::engine::CliAnalysisResult;
use bsl_shared::domain::types::{TypeResolution, Certainty, ResolutionResult};
use crate::args::OutputFormat;

/// Форматтер для CLI вывода
pub struct CliFormatter;

impl CliFormatter {
    /// Форматировать результат анализа
    pub fn format_analysis(
        result: &CliAnalysisResult,
        format: &OutputFormat,
        verbose: bool,
        errors_only: bool,
    ) -> String {
        match format {
            OutputFormat::Table => Self::format_table(result, verbose, errors_only),
            OutputFormat::Json => Self::format_json(result),
            OutputFormat::Plain => Self::format_plain(result, verbose, errors_only),
        }
    }

    /// Форматировать автодополнения
    pub fn format_completions(
        completions: &[bsl_shared::domain::repository::CompletionItem],
        format: &OutputFormat,
        limit: usize,
    ) -> String {
        let limited_completions: Vec<_> = completions.iter().take(limit).collect();

        match format {
            OutputFormat::Table => Self::format_completions_table(&limited_completions),
            OutputFormat::Json => serde_json::to_string_pretty(&limited_completions).unwrap_or_default(),
            OutputFormat::Plain => Self::format_completions_plain(&limited_completions),
        }
    }

    /// Табличный формат для анализа
    fn format_table(result: &CliAnalysisResult, verbose: bool, errors_only: bool) -> String {
        let mut output = String::new();

        // Заголовок
        output.push_str(&format!("📁 {}\n", result.file_path.bold()));
        output.push_str(&format!("⏱️  Время анализа: {} мс\n", result.analysis_duration_ms));
        output.push_str(&format!("🔍 Найдено типов: {}\n\n", result.type_resolutions.len()));

        if result.type_resolutions.is_empty() {
            output.push_str("ℹ️  Типы не найдены\n");
            return output;
        }

        // Таблица типов
        output.push_str(&format!("{:<30} {:<20} {:<15}\n", "Выражение", "Тип", "Уверенность"));
        output.push_str(&"-".repeat(70));
        output.push('\n');

        for (expr, resolution) in &result.type_resolutions {
            let should_show = if errors_only {
                matches!(resolution.certainty, Certainty::Unknown)
            } else {
                true
            };

            if should_show {
                let type_str = Self::format_type_result(&resolution.result);
                let certainty_str = Self::format_certainty(&resolution.certainty);

                output.push_str(&format!("{:<30} {:<20} {:<15}\n",
                    expr.cyan(),
                    type_str,
                    certainty_str
                ));

                if verbose && !resolution.metadata.notes.is_empty() {
                    for note in &resolution.metadata.notes {
                        output.push_str(&format!("   💡 {}\n", note.dimmed()));
                    }
                }
            }
        }

        output
    }

    /// JSON формат
    fn format_json(result: &CliAnalysisResult) -> String {
        serde_json::to_string_pretty(result).unwrap_or_else(|e| {
            format!("{{\"error\": \"Failed to serialize result: {}\"}}", e)
        })
    }

    /// Простой текстовый формат
    fn format_plain(result: &CliAnalysisResult, verbose: bool, errors_only: bool) -> String {
        let mut output = String::new();

        output.push_str(&format!("File: {}\n", result.file_path));
        output.push_str(&format!("Analysis time: {} ms\n", result.analysis_duration_ms));
        output.push_str(&format!("Types found: {}\n", result.type_resolutions.len()));

        for (expr, resolution) in &result.type_resolutions {
            let should_show = if errors_only {
                matches!(resolution.certainty, Certainty::Unknown)
            } else {
                true
            };

            if should_show {
                output.push_str(&format!("{}: {} ({})\n",
                    expr,
                    Self::format_type_result(&resolution.result),
                    Self::format_certainty(&resolution.certainty)
                ));

                if verbose && !resolution.metadata.notes.is_empty() {
                    for note in &resolution.metadata.notes {
                        output.push_str(&format!("  Note: {}\n", note));
                    }
                }
            }
        }

        output
    }

    /// Табличный формат для автодополнений
    fn format_completions_table(completions: &[&bsl_shared::domain::repository::CompletionItem]) -> String {
        let mut output = String::new();

        if completions.is_empty() {
            output.push_str("ℹ️  Автодополнения не найдены\n");
            return output;
        }

        output.push_str(&format!("🔍 Найдено автодополнений: {}\n\n", completions.len()));
        output.push_str(&format!("{:<25} {:<15} {:<30}\n", "Название", "Тип", "Описание"));
        output.push_str(&"-".repeat(70));
        output.push('\n');

        for completion in completions {
            let description = completion.detail.as_deref().unwrap_or("");
            output.push_str(&format!("{:<25} {:<15} {:<30}\n",
                completion.label.cyan(),
                format!("{:?}", completion.kind).yellow(),
                description.dimmed()
            ));
        }

        output
    }

    /// Простой формат для автодополнений
    fn format_completions_plain(completions: &[&bsl_shared::domain::repository::CompletionItem]) -> String {
        let mut output = String::new();

        for completion in completions {
            output.push_str(&completion.label);
            if let Some(detail) = &completion.detail {
                output.push_str(&format!(" - {}", detail));
            }
            output.push('\n');
        }

        output
    }

    /// Форматировать результат типа
    fn format_type_result(result: &ResolutionResult) -> String {
        match result {
            ResolutionResult::Concrete(concrete_type) => {
                format!("{}", concrete_type).green().to_string()
            }
            ResolutionResult::Union(types) => {
                format!("Union[{}]", types.len()).yellow().to_string()
            }
            ResolutionResult::Dynamic => "Dynamic".magenta().to_string(),
        }
    }

    /// Форматировать уверенность
    fn format_certainty(certainty: &Certainty) -> String {
        match certainty {
            Certainty::Known => "Known".green().to_string(),
            Certainty::Inferred(confidence) => {
                format!("Inferred({:.2})", confidence).yellow().to_string()
            }
            Certainty::Unknown => "Unknown".red().to_string(),
        }
    }

    /// Форматировать информацию о типе
    pub fn format_type_info(
        expression: &str,
        resolution: &TypeResolution,
        format: &OutputFormat,
    ) -> String {
        match format {
            OutputFormat::Table | OutputFormat::Plain => {
                let mut output = String::new();
                output.push_str(&format!("🔍 {}\n", expression.bold()));
                output.push_str(&format!("Тип: {}\n", Self::format_type_result(&resolution.result)));
                output.push_str(&format!("Уверенность: {}\n", Self::format_certainty(&resolution.certainty)));

                if !resolution.metadata.notes.is_empty() {
                    output.push_str("\n💡 Заметки:\n");
                    for note in &resolution.metadata.notes {
                        output.push_str(&format!("  • {}\n", note));
                    }
                }

                output
            }
            OutputFormat::Json => {
                let info = serde_json::json!({
                    "expression": expression,
                    "type": resolution.result,
                    "certainty": resolution.certainty,
                    "notes": resolution.metadata.notes
                });
                serde_json::to_string_pretty(&info).unwrap_or_default()
            }
        }
    }
}