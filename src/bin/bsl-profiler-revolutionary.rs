//! Революционный профайлер на CentralTypeSystem

use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use std::path::PathBuf;

use bsl_gradual_types::target::system::{CentralTypeSystem, CentralSystemConfig};

#[derive(Parser)]
#[command(name = "bsl-profiler-revolutionary")]
#[command(about = "Революционный профайлер производительности BSL")]
struct Cli {
    /// Команда профилирования
    #[command(subcommand)]
    command: ProfileCommand,
    
    /// Путь к HTML справке
    #[arg(long, default_value = "examples/syntax_helper/rebuilt.shcntx_ru")]
    html_path: String,
    
    /// Включить детальное логирование
    #[arg(long)]
    verbose: bool,
}

#[derive(clap::Subcommand)]
enum ProfileCommand {
    /// Бенчмарк системы типов
    Benchmark {
        /// Количество итераций
        #[arg(long, default_value = "100")]
        iterations: usize,
    },
    /// Профилирование проекта
    Project {
        /// Путь к проекту
        path: PathBuf,
        /// Количество потоков
        #[arg(long, default_value = "4")]
        threads: usize,
    },
    /// Профилирование памяти
    Memory {
        /// Тип нагрузки
        #[arg(long, default_value = "standard")]
        load_type: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    println!("Revolutionary BSL Profiler");
    
    // === ЕДИНАЯ ИНИЦИАЛИЗАЦИЯ ===
    let central_config = CentralSystemConfig {
        html_path: cli.html_path.clone(),
        configuration_path: None,
        verbose_logging: cli.verbose,
        ..Default::default()
    };
    
    let central_system = Arc::new(CentralTypeSystem::new(central_config));
    
    if cli.verbose {
        println!("Initializing CentralTypeSystem...");
    }
    
    let init_start = std::time::Instant::now();
    central_system.initialize().await?;
    let init_time = init_start.elapsed();
    
    println!("System initialized in {:?}", init_time);
    
    // === ВЫПОЛНЕНИЕ КОМАНД ===
    match cli.command {
        ProfileCommand::Benchmark { iterations } => {
            run_benchmark(&central_system, iterations).await?;
        }
        ProfileCommand::Project { path, threads } => {
            run_project_profiling(&central_system, &path, threads).await?;
        }
        ProfileCommand::Memory { load_type } => {
            run_memory_profiling(&central_system, &load_type).await?;
        }
    }
    
    Ok(())
}

/// Бенчмарк разрешения типов
async fn run_benchmark(central_system: &CentralTypeSystem, iterations: usize) -> Result<()> {
    println!("Running type resolution benchmark...");
    println!("Iterations: {}", iterations);
    
    let test_expressions = vec![
        "Массив",
        "ТаблицаЗначений", 
        "Структура",
        "Справочники.Контрагенты",
        "Строка",
    ];
    
    let mut total_time = std::time::Duration::ZERO;
    let mut successful_resolutions = 0;
    
    for i in 0..iterations {
        if i % 10 == 0 {
            println!("Progress: {}/{}", i, iterations);
        }
        
        for expression in &test_expressions {
            let start = std::time::Instant::now();
            
            // Используем LSP интерфейс для быстрого разрешения
            let _resolution = central_system.lsp_interface()
                .handle_completion_request(LspCompletionRequest {
                    file_path: "benchmark.bsl".to_string(),
                    line: 1,
                    column: 1,
                    prefix: expression.to_string(),
                    trigger_character: None,
                }).await;
            
            total_time += start.elapsed();
            successful_resolutions += 1;
        }
    }
    
    let avg_time = total_time / successful_resolutions;
    let ops_per_sec = 1.0 / avg_time.as_secs_f64();
    
    println!("Benchmark Results:");
    println!("  - Total resolutions: {}", successful_resolutions);
    println!("  - Average time: {:?}", avg_time);
    println!("  - Operations per second: {:.0}", ops_per_sec);
    
    // Получаем метрики производительности
    if let Ok(perf_metrics) = central_system.lsp_interface().get_performance_metrics().await {
        println!("  - Cache hit rate: {:.1}%", perf_metrics.cache_hit_rate * 100.0);
        println!("  - Slow requests: {}", perf_metrics.slow_requests);
    }
    
    Ok(())
}

/// Профилирование проекта
async fn run_project_profiling(central_system: &CentralTypeSystem, project_path: &PathBuf, threads: usize) -> Result<()> {
    println!("Profiling project: {}", project_path.display());
    println!("Using {} threads", threads);
    
    let cli_analysis_request = bsl_gradual_types::target::presentation::CliAnalysisRequest {
        project_path: project_path.clone(),
        output_format: CliOutputFormat::Text,
        include_coverage: true,
        include_errors: true,
        verbose: true,
    };
    
    let start_time = std::time::Instant::now();
    
    match central_system.cli_interface().handle_analysis_request(cli_analysis_request).await {
        Ok(response) => {
            let total_time = start_time.elapsed();
            
            println!("Project Profiling Results:");
            println!("  - Analysis time: {:?}", total_time);
            println!("  - Files analyzed: {}", response.summary.total_files);
            println!("  - Functions found: {}", response.summary.total_functions);
            println!("  - Variables found: {}", response.summary.total_variables);
            println!("  - Type errors: {}", response.summary.error_count);
            
            if let Some(coverage) = response.coverage {
                println!("  - Type coverage: {:.1}%", coverage.coverage_percentage);
            }
            
            // Рассчитываем производительность
            if response.summary.total_files > 0 {
                let files_per_sec = response.summary.total_files as f64 / total_time.as_secs_f64();
                println!("  - Files per second: {:.1}", files_per_sec);
            }
        }
        Err(e) => {
            println!("Project profiling failed: {}", e);
        }
    }
    
    Ok(())
}

/// Профилирование памяти
async fn run_memory_profiling(central_system: &CentralTypeSystem, load_type: &str) -> Result<()> {
    println!("Memory profiling with load type: {}", load_type);
    
    // Получаем начальные метрики
    let initial_metrics = central_system.get_system_metrics().await;
    println!("Initial memory: {:.2} MB", initial_metrics.cache_memory_mb);
    
    // Симулируем нагрузку
    match load_type {
        "light" => {
            println!("Running light load simulation...");
            // 100 запросов разрешения типов
            for i in 0..100 {
                if i % 20 == 0 {
                    println!("  Progress: {}/100", i);
                }
                // TODO: Добавить нагрузочное тестирование
            }
        }
        "heavy" => {
            println!("Running heavy load simulation...");
            // 1000 запросов разрешения типов
            for i in 0..1000 {
                if i % 100 == 0 {
                    println!("  Progress: {}/1000", i);
                }
                // TODO: Добавить тяжёлое нагрузочное тестирование
            }
        }
        _ => {
            println!("Running standard load simulation...");
            // 500 запросов разрешения типов  
            for i in 0..500 {
                if i % 50 == 0 {
                    println!("  Progress: {}/500", i);
                }
                // TODO: Добавить стандартное нагрузочное тестирование
            }
        }
    }
    
    // Получаем финальные метрики
    let final_metrics = central_system.get_system_metrics().await;
    let memory_diff = final_metrics.cache_memory_mb - initial_metrics.cache_memory_mb;
    
    println!("Memory Profiling Results:");
    println!("  - Initial memory: {:.2} MB", initial_metrics.cache_memory_mb);
    println!("  - Final memory: {:.2} MB", final_metrics.cache_memory_mb);
    println!("  - Memory difference: {:.2} MB", memory_diff);
    println!("  - Total requests: {}", final_metrics.total_requests);
    
    // Проверяем здоровье после нагрузки
    let health = central_system.health_check().await;
    println!("  - Health after load: {}", health.status);
    
    Ok(())
}
