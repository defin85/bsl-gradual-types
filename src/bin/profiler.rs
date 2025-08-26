//! CLI инструмент для профилирования производительности BSL Type System

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;

use bsl_gradual_types::system::analysis::{
    ParallelAnalysisCLI, ParallelAnalysisConfig, ParallelAnalyzer,
};
use bsl_gradual_types::system::performance::{global_profiler, BenchmarkSuite, PerformanceOptimizer};
use bsl_gradual_types::parsing::bsl::common::ParserFactory;

#[derive(Parser)]
#[command(name = "bsl-profiler")]
#[command(about = "Профайлер производительности для BSL Type System")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Запустить полный набор бенчмарков
    Benchmark {
        /// Количество итераций для каждого бенчмарка
        #[arg(short, long, default_value = "10")]
        iterations: usize,

        /// Сохранить результаты в JSON файл
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Профилировать конкретный BSL файл
    Profile {
        /// Путь к BSL файлу
        file: PathBuf,

        /// Включить детальное профилирование
        #[arg(short, long)]
        verbose: bool,

        /// Сохранить результаты в JSON файл  
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Сравнить производительность разных версий
    Compare {
        /// Базовый BSL файл
        baseline: PathBuf,

        /// Измененный BSL файл
        modified: PathBuf,

        /// Количество итераций
        #[arg(short, long, default_value = "5")]
        iterations: usize,
    },

    /// Анализировать производительность и дать рекомендации
    Analyze {
        /// JSON файл с результатами профилирования
        report: PathBuf,
    },

    /// Параллельный анализ проекта 1С
    Project {
        /// Путь к корню проекта 1С
        path: PathBuf,

        /// Количество потоков (по умолчанию = CPU cores)
        #[arg(short, long)]
        threads: Option<usize>,

        /// Показать бенчмарк последовательный vs параллельный
        #[arg(short, long)]
        benchmark: bool,

        /// Отключить кеширование
        #[arg(long)]
        no_cache: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Настраиваем логирование
    tracing_subscriber::fmt()
        .with_env_filter("bsl_gradual_types=info")
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Benchmark {
            iterations: _,
            output,
        } => {
            println!("{}", "🔍 Запуск полного набора бенчмарков...".cyan().bold());

            let report = BenchmarkSuite::run_full_benchmark_suite();

            // Выводим результаты
            println!("{}", report.format_human_readable());

            // Сохраняем в файл если указан
            if let Some(output_path) = output {
                let json_report = serde_json::to_string_pretty(&report)?;
                std::fs::write(&output_path, json_report)?;
                println!("\n💾 Результаты сохранены в: {}", output_path.display());
            }

            // Выводим рекомендации
            let suggestions = PerformanceOptimizer::analyze_and_suggest(&report);
            if !suggestions.is_empty() {
                println!("\n{}", "💡 Рекомендации по оптимизации:".yellow().bold());
                for (i, suggestion) in suggestions.iter().enumerate() {
                    let priority_color = match suggestion.priority {
                        bsl_gradual_types::system::performance::OptimizationPriority::Critical => {
                            "red"
                        }
                        bsl_gradual_types::system::performance::OptimizationPriority::High => {
                            "yellow"
                        }
                        bsl_gradual_types::system::performance::OptimizationPriority::Medium => {
                            "blue"
                        }
                        bsl_gradual_types::system::performance::OptimizationPriority::Low => "green",
                    };

                    println!(
                        "  {}. {} [{}]",
                        i + 1,
                        suggestion.suggestion,
                        format!("{:?}", suggestion.priority).color(priority_color)
                    );
                }
            }
        }

        Commands::Profile {
            file,
            verbose,
            output,
        } => {
            println!(
                "{}",
                format!("🔬 Профилирование файла: {}", file.display())
                    .cyan()
                    .bold()
            );

            // Читаем файл
            let source_code = std::fs::read_to_string(&file)?;

            // Включаем профилирование
            let profiler = global_profiler();
            profiler.enable();

            // Профилируем парсинг
            let parsing_time = std::time::Instant::now();
            let mut parser = ParserFactory::create();
            let program = parser.parse(&source_code)?;
            let parsing_elapsed = parsing_time.elapsed();

            println!("📝 Парсинг: {:.2?}", parsing_elapsed);

            // Профилируем type checking
            let type_check_time = std::time::Instant::now();
            let type_checker = bsl_gradual_types::domain::analysis::TypeChecker::new(
                file.file_name().unwrap().to_str().unwrap().to_string(),
            );
            let (context, diagnostics) = type_checker.check(&program);
            let type_check_elapsed = type_check_time.elapsed();

            println!("🔍 Type checking: {:.2?}", type_check_elapsed);
            println!("📊 Найдено переменных: {}", context.variables.len());
            println!("📊 Найдено функций: {}", context.functions.len());
            println!("🚨 Диагностики: {}", diagnostics.len());

            if verbose {
                println!("\n{}", "📋 Детальная информация:".yellow().bold());

                if !context.variables.is_empty() {
                    println!("📦 Переменные:");
                    for (name, type_res) in context.variables.iter().take(10) {
                        println!("  • {} -> {:?}", name, type_res.result);
                    }
                }

                if !context.functions.is_empty() {
                    println!("🔧 Функции:");
                    for (name, sig) in context.functions.iter().take(5) {
                        println!("  • {} -> {:?}", name, sig.return_type.result);
                    }
                }

                if !diagnostics.is_empty() {
                    println!("🚨 Диагностики:");
                    for diag in diagnostics.iter().take(5) {
                        let severity_color = match diag.severity {
                            bsl_gradual_types::domain::analysis::DiagnosticSeverity::Error => {
                                "red"
                            }
                            bsl_gradual_types::core::type_checker::DiagnosticSeverity::Warning => {
                                "yellow"
                            }
                            _ => "blue",
                        };
                        println!(
                            "  • {} [{}:{}] {}",
                            diag.message.color(severity_color),
                            diag.line,
                            diag.column,
                            format!("{:?}", diag.severity).color(severity_color)
                        );
                    }
                }
            }

            // Генерируем отчет
            let report = profiler.generate_report();

            if let Some(output_path) = output {
                let json_report = serde_json::to_string_pretty(&report)?;
                std::fs::write(&output_path, json_report)?;
                println!("\n💾 Отчет сохранен в: {}", output_path.display());
            }
        }

        Commands::Compare {
            baseline,
            modified,
            iterations,
        } => {
            println!("{}", "⚖️ Сравнение производительности...".cyan().bold());

            let baseline_code = std::fs::read_to_string(&baseline)?;
            let modified_code = std::fs::read_to_string(&modified)?;

            println!("📊 Бенчмарк базовой версии ({} итераций)...", iterations);
            let baseline_metrics = BenchmarkSuite::benchmark_parsing(&baseline_code, iterations);

            println!("📊 Бенчмарк измененной версии ({} итераций)...", iterations);
            let modified_metrics = BenchmarkSuite::benchmark_parsing(&modified_code, iterations);

            // Сравнение результатов
            let speedup = baseline_metrics.avg_time.as_nanos() as f64
                / modified_metrics.avg_time.as_nanos() as f64;

            println!("\n{}", "📈 Результаты сравнения:".green().bold());
            println!(
                "  Базовая версия: {:.2?} (среднее)",
                baseline_metrics.avg_time
            );
            println!(
                "  Измененная версия: {:.2?} (среднее)",
                modified_metrics.avg_time
            );

            if speedup > 1.0 {
                println!("  🚀 Ускорение: {:.2}x", speedup.to_string().green().bold());
            } else if speedup < 1.0 {
                println!(
                    "  🐌 Замедление: {:.2}x",
                    (1.0 / speedup).to_string().red().bold()
                );
            } else {
                println!("  ⚖️ Без изменений");
            }
        }

        Commands::Analyze { report } => {
            println!(
                "{}",
                "🔍 Анализ отчета о производительности...".cyan().bold()
            );

            let report_json = std::fs::read_to_string(&report)?;
            let perf_report: bsl_gradual_types::core::performance::PerformanceReport =
                serde_json::from_str(&report_json)?;

            // Выводим отчет
            println!("{}", perf_report.format_human_readable());

            // Анализируем и выводим рекомендации
            let suggestions = PerformanceOptimizer::analyze_and_suggest(&perf_report);
            let cache_recommendations = PerformanceOptimizer::cache_recommendations(&perf_report);

            if !suggestions.is_empty() {
                println!("\n{}", "💡 Рекомендации по оптимизации:".yellow().bold());
                for (i, suggestion) in suggestions.iter().enumerate() {
                    println!("  {}. {}", i + 1, suggestion.suggestion);
                }
            }

            if !cache_recommendations.is_empty() {
                println!("\n{}", "🗄️ Рекомендации по кешированию:".blue().bold());
                for (i, rec) in cache_recommendations.iter().enumerate() {
                    println!(
                        "  {}. {}: {} ({:?})",
                        i + 1,
                        rec.component,
                        rec.reason,
                        rec.strategy
                    );
                }
            }
        }

        Commands::Project {
            path,
            threads,
            benchmark,
            no_cache,
        } => {
            println!(
                "{}",
                format!("🚀 Параллельный анализ проекта: {}", path.display())
                    .cyan()
                    .bold()
            );

            let config = ParallelAnalysisConfig {
                num_threads: threads,
                use_cache: !no_cache,
                show_progress: true,
                ..Default::default()
            };

            if benchmark {
                // Запускаем бенчмарк
                let analyzer = ParallelAnalyzer::new(config.clone())?;
                let files = ParallelAnalyzer::find_bsl_files(&path)?;

                if files.len() < 2 {
                    println!("⚠️ Недостаточно файлов для бенчмарка (нужно минимум 2)");
                } else {
                    let files_sample = files.into_iter().take(10).collect(); // Берем первые 10 файлов
                    println!("📊 Бенчмарк параллельного vs последовательного анализа...");

                    let benchmark_result =
                        analyzer.benchmark_parallel_vs_sequential(files_sample)?;
                    println!("\n{}", benchmark_result.format_results());
                }
            }

            // Обычный анализ проекта
            ParallelAnalysisCLI::run_project_analysis(path, config)?;
        }
    }

    Ok(())
}
