//! Параллельный анализ модулей с использованием rayon
//!
//! Этот модуль предоставляет возможности для параллельной обработки
//! множественных BSL файлов с оптимальным использованием CPU ресурсов.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use anyhow::Result;
use rayon::prelude::*;
use indicatif::{ProgressBar, ProgressStyle};

use crate::parser::common::{Parser, ParserFactory};
use crate::core::type_checker::{TypeChecker, TypeContext, TypeDiagnostic};
use crate::core::analysis_cache::AnalysisCacheManager;

/// Результат анализа одного файла
#[derive(Debug, Clone)]
pub struct FileAnalysisResult {
    /// Путь к файлу
    pub file_path: PathBuf,
    /// Контекст типов
    pub type_context: TypeContext,
    /// Диагностики
    pub diagnostics: Vec<TypeDiagnostic>,
    /// Время анализа
    pub analysis_time: std::time::Duration,
    /// Успешность анализа
    pub success: bool,
    /// Сообщение об ошибке (если не успешно)
    pub error_message: Option<String>,
}

/// Результат пакетного анализа
#[derive(Debug)]
pub struct BatchAnalysisResult {
    /// Результаты по файлам
    pub file_results: Vec<FileAnalysisResult>,
    /// Общее время анализа
    pub total_time: std::time::Duration,
    /// Агрегированная статистика
    pub stats: BatchAnalysisStats,
}

/// Статистика пакетного анализа
#[derive(Debug, Clone)]
pub struct BatchAnalysisStats {
    /// Общее количество файлов
    pub total_files: usize,
    /// Успешно проанализированных файлов
    pub successful_files: usize,
    /// Файлов с ошибками
    pub failed_files: usize,
    /// Общее количество найденных функций
    pub total_functions: usize,
    /// Общее количество переменных
    pub total_variables: usize,
    /// Общее количество диагностик
    pub total_diagnostics: usize,
    /// Среднее время анализа на файл
    pub avg_analysis_time: std::time::Duration,
}

/// Конфигурация параллельного анализа
#[derive(Debug, Clone)]
pub struct ParallelAnalysisConfig {
    /// Количество рабочих потоков (None = автоматически)
    pub num_threads: Option<usize>,
    /// Размер chunk для обработки файлов
    pub chunk_size: usize,
    /// Показывать прогресс бар
    pub show_progress: bool,
    /// Использовать кеширование
    pub use_cache: bool,
    /// Директория для кеша
    pub cache_dir: Option<PathBuf>,
}

impl Default for ParallelAnalysisConfig {
    fn default() -> Self {
        Self {
            num_threads: None, // Автоматически
            chunk_size: 10,
            show_progress: true,
            use_cache: true,
            cache_dir: Some(PathBuf::from(".bsl_cache")),
        }
    }
}

/// Параллельный анализатор BSL файлов
pub struct ParallelAnalyzer {
    config: ParallelAnalysisConfig,
    cache_manager: Option<Arc<Mutex<AnalysisCacheManager>>>,
}

impl ParallelAnalyzer {
    /// Создать новый параллельный анализатор
    pub fn new(config: ParallelAnalysisConfig) -> Result<Self> {
        let cache_manager = if config.use_cache {
            if let Some(cache_dir) = &config.cache_dir {
                let manager = AnalysisCacheManager::new(cache_dir, env!("CARGO_PKG_VERSION"))?;
                Some(Arc::new(Mutex::new(manager)))
            } else {
                None
            }
        } else {
            None
        };
        
        Ok(Self {
            config,
            cache_manager,
        })
    }
    
    /// Проанализировать множество файлов параллельно
    pub fn analyze_files<P: AsRef<Path> + Send + Sync>(
        &self,
        file_paths: Vec<P>,
    ) -> Result<BatchAnalysisResult> {
        let start_time = std::time::Instant::now();
        
        // Настраиваем thread pool если указано
        if let Some(num_threads) = self.config.num_threads {
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build_global()?;
        }
        
        // Создаем прогресс бар
        let progress = if self.config.show_progress {
            let pb = ProgressBar::new(file_paths.len() as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                    .unwrap()
                    .progress_chars("#>-")
            );
            Some(pb)
        } else {
            None
        };
        
        // Параллельно анализируем файлы
        let file_results: Vec<FileAnalysisResult> = file_paths
            .par_iter()
            .map(|file_path| {
                let result = self.analyze_single_file(file_path.as_ref());
                
                // Обновляем прогресс
                if let Some(pb) = &progress {
                    pb.inc(1);
                    if let Ok(ref res) = result {
                        pb.set_message(format!("Анализ: {}", res.file_path.display()));
                    }
                }
                
                result.unwrap_or_else(|e| FileAnalysisResult {
                    file_path: file_path.as_ref().to_path_buf(),
                    type_context: TypeContext {
                        variables: HashMap::new(),
                        functions: HashMap::new(),
                        current_scope: crate::core::dependency_graph::Scope::Global,
                        scope_stack: vec![],
                    },
                    diagnostics: vec![],
                    analysis_time: std::time::Duration::ZERO,
                    success: false,
                    error_message: Some(e.to_string()),
                })
            })
            .collect();
        
        if let Some(pb) = progress {
            pb.finish_with_message("Анализ завершен");
        }
        
        let total_time = start_time.elapsed();
        let stats = self.calculate_stats(&file_results, total_time);
        
        Ok(BatchAnalysisResult {
            file_results,
            total_time,
            stats,
        })
    }
    
    /// Проанализировать один файл
    fn analyze_single_file(&self, file_path: &Path) -> Result<FileAnalysisResult> {
        let analysis_start = std::time::Instant::now();
        
        // Читаем файл
        let content = std::fs::read_to_string(file_path)?;
        
        // Проверяем кеш если включен
        if let Some(cache_manager) = &self.cache_manager {
            let cache_key = crate::core::analysis_cache::CacheKey::from_content(
                &content, 
                env!("CARGO_PKG_VERSION")
            );
            
            if let Ok(mut manager) = cache_manager.lock() {
                if let Some(cached) = manager.get(&cache_key) {
                    return Ok(FileAnalysisResult {
                        file_path: file_path.to_path_buf(),
                        type_context: TypeContext {
                            variables: HashMap::new(),
                            functions: cached.function_signatures,
                            current_scope: crate::core::dependency_graph::Scope::Global,
                            scope_stack: vec![],
                        },
                        diagnostics: vec![], // Кешированные диагностики не сохраняем пока
                        analysis_time: std::time::Duration::from_micros(10), // Очень быстро из кеша
                        success: true,
                        error_message: None,
                    });
                }
            }
        }
        
        // Анализируем файл
        let mut parser = ParserFactory::create();
        let program = parser.parse(&content)?;
        
        let file_name = file_path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown.bsl")
            .to_string();
        
        let type_checker = TypeChecker::new(file_name);
        let (context, diagnostics) = type_checker.check(&program);
        
        let analysis_time = analysis_start.elapsed();
        
        // Сохраняем в кеш если включен
        if let Some(cache_manager) = &self.cache_manager {
            let cache_key = crate::core::analysis_cache::CacheKey::from_content(
                &content, 
                env!("CARGO_PKG_VERSION")
            );
            
            let cached_results = crate::core::analysis_cache::CachedInterproceduralResults {
                function_results: HashMap::new(), // TODO: Извлечь из context
                function_signatures: context.functions.clone(),
                call_graph_summary: crate::core::analysis_cache::CallGraphSummary {
                    function_call_counts: HashMap::new(),
                    topological_order: vec![],
                    recursive_functions: vec![],
                },
                created_at: std::time::SystemTime::now(),
                ttl: std::time::Duration::from_secs(3600),
            };
            
            if let Ok(mut manager) = cache_manager.lock() {
                let _ = manager.put(cache_key, cached_results);
            }
        }
        
        Ok(FileAnalysisResult {
            file_path: file_path.to_path_buf(),
            type_context: context,
            diagnostics,
            analysis_time,
            success: true,
            error_message: None,
        })
    }
    
    /// Вычислить статистику
    fn calculate_stats(&self, results: &[FileAnalysisResult], total_time: std::time::Duration) -> BatchAnalysisStats {
        let total_files = results.len();
        let successful_files = results.iter().filter(|r| r.success).count();
        let failed_files = total_files - successful_files;
        
        let total_functions: usize = results.iter()
            .map(|r| r.type_context.functions.len())
            .sum();
        
        let total_variables: usize = results.iter()
            .map(|r| r.type_context.variables.len())
            .sum();
        
        let total_diagnostics: usize = results.iter()
            .map(|r| r.diagnostics.len())
            .sum();
        
        let avg_analysis_time = if total_files > 0 {
            total_time / total_files as u32
        } else {
            std::time::Duration::ZERO
        };
        
        BatchAnalysisStats {
            total_files,
            successful_files,
            failed_files,
            total_functions,
            total_variables,
            total_diagnostics,
            avg_analysis_time,
        }
    }
    
    /// Найти все BSL файлы в директории
    pub fn find_bsl_files<P: AsRef<Path>>(root_dir: P) -> Result<Vec<PathBuf>> {
        let mut bsl_files = Vec::new();
        
        for entry in walkdir::WalkDir::new(root_dir) {
            let entry = entry?;
            if entry.file_type().is_file() {
                if let Some(extension) = entry.path().extension() {
                    if extension == "bsl" || extension == "os" {
                        bsl_files.push(entry.path().to_path_buf());
                    }
                }
            }
        }
        
        Ok(bsl_files)
    }
    
    /// Анализировать проект 1С целиком
    pub fn analyze_project<P: AsRef<Path>>(&self, project_root: P) -> Result<BatchAnalysisResult> {
        let bsl_files = Self::find_bsl_files(project_root)?;
        
        if bsl_files.is_empty() {
            return Ok(BatchAnalysisResult {
                file_results: vec![],
                total_time: std::time::Duration::ZERO,
                stats: BatchAnalysisStats {
                    total_files: 0,
                    successful_files: 0,
                    failed_files: 0,
                    total_functions: 0,
                    total_variables: 0,
                    total_diagnostics: 0,
                    avg_analysis_time: std::time::Duration::ZERO,
                },
            });
        }
        
        println!("🔍 Найдено {} BSL файлов для анализа", bsl_files.len());
        
        self.analyze_files(bsl_files)
    }
    
    /// Получить информацию о производительности
    pub fn benchmark_parallel_vs_sequential<P: AsRef<Path> + Send + Sync + Clone>(
        &self,
        file_paths: Vec<P>,
    ) -> Result<ParallelBenchmarkResult> {
        let files_clone = file_paths.clone();
        
        // Последовательный анализ
        let sequential_start = std::time::Instant::now();
        let _sequential_results: Vec<_> = files_clone.iter()
            .map(|path| self.analyze_single_file(path.as_ref()))
            .collect::<Result<Vec<_>>>()?;
        let sequential_time = sequential_start.elapsed();
        
        // Параллельный анализ
        let parallel_start = std::time::Instant::now();
        let _parallel_results: Vec<_> = file_paths.par_iter()
            .map(|path| self.analyze_single_file(path.as_ref()))
            .collect::<Result<Vec<_>>>()?;
        let parallel_time = parallel_start.elapsed();
        
        let speedup = sequential_time.as_secs_f64() / parallel_time.as_secs_f64();
        let efficiency = speedup / rayon::current_num_threads() as f64;
        
        Ok(ParallelBenchmarkResult {
            sequential_time,
            parallel_time,
            speedup,
            efficiency,
            files_count: file_paths.len(),
            threads_used: rayon::current_num_threads(),
        })
    }
}

/// Результат бенчмарка параллельного анализа
#[derive(Debug, Clone)]
pub struct ParallelBenchmarkResult {
    pub sequential_time: std::time::Duration,
    pub parallel_time: std::time::Duration,
    pub speedup: f64,
    pub efficiency: f64,
    pub files_count: usize,
    pub threads_used: usize,
}

impl ParallelBenchmarkResult {
    /// Форматировать результаты бенчмарка
    pub fn format_results(&self) -> String {
        format!(
            "🏁 Результаты параллельного бенчмарка:\n\
             📁 Файлов: {}\n\
             🧵 Потоков: {}\n\
             ⏱️  Последовательно: {:.2?}\n\
             🚀 Параллельно: {:.2?}\n\
             📈 Ускорение: {:.2}x\n\
             ⚡ Эффективность: {:.1}%\n\
             💡 Рекомендация: {}",
            self.files_count,
            self.threads_used,
            self.sequential_time,
            self.parallel_time,
            self.speedup,
            self.efficiency * 100.0,
            if self.efficiency > 0.7 {
                "Отличная параллелизация"
            } else if self.efficiency > 0.5 {
                "Хорошая параллелизация"
            } else {
                "Параллелизация не эффективна для таких файлов"
            }
        )
    }
}

/// Утилиты для работы с большими проектами
pub struct ProjectAnalysisUtils;

impl ProjectAnalysisUtils {
    /// Найти конфигурацию 1С в проекте
    pub fn find_1c_config<P: AsRef<Path>>(project_root: P) -> Option<PathBuf> {
        let root = project_root.as_ref();
        
        // Стандартные места расположения конфигурации
        let candidates = [
            root.join("src").join("cf"),
            root.join("Configuration.xml"),
            root.join("Ext").join("Configuration.xml"),
            root.join("ConfigDumpInfo.xml"),
        ];
        
        for candidate in &candidates {
            if candidate.exists() {
                return Some(candidate.clone());
            }
        }
        
        None
    }
    
    /// Группировать файлы по подсистемам
    pub fn group_files_by_subsystem(file_paths: &[PathBuf]) -> HashMap<String, Vec<PathBuf>> {
        let mut groups = HashMap::new();
        
        for file_path in file_paths {
            let subsystem = Self::extract_subsystem_name(file_path);
            groups.entry(subsystem).or_insert_with(Vec::new).push(file_path.clone());
        }
        
        groups
    }
    
    /// Извлечь имя подсистемы из пути файла
    fn extract_subsystem_name(file_path: &Path) -> String {
        // Простая эвристика: берем название родительской директории
        file_path.parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("Common")
            .to_string()
    }
    
    /// Приоритизировать файлы для анализа
    pub fn prioritize_files(file_paths: &[PathBuf]) -> Vec<PathBuf> {
        let mut prioritized = file_paths.to_vec();
        
        // Сортируем по приоритету:
        // 1. CommonModules (общие модули)
        // 2. Catalogs, Documents (основная функциональность)
        // 3. Reports, DataProcessors (вторичная функциональность)
        prioritized.sort_by(|a, b| {
            let priority_a = Self::get_file_priority(a);
            let priority_b = Self::get_file_priority(b);
            priority_a.cmp(&priority_b)
        });
        
        prioritized
    }
    
    /// Получить приоритет файла для анализа
    fn get_file_priority(file_path: &Path) -> u32 {
        let path_str = file_path.to_string_lossy().to_lowercase();
        
        if path_str.contains("commonmodules") {
            1 // Высший приоритет
        } else if path_str.contains("catalogs") || path_str.contains("documents") {
            2
        } else if path_str.contains("reports") || path_str.contains("dataprocessors") {
            3
        } else {
            4 // Низший приоритет
        }
    }
}

/// CLI интерфейс для параллельного анализа
pub struct ParallelAnalysisCLI;

impl ParallelAnalysisCLI {
    /// Запустить анализ проекта с подробным выводом
    pub fn run_project_analysis<P: AsRef<Path>>(
        project_root: P,
        config: ParallelAnalysisConfig,
    ) -> Result<()> {
        println!("🚀 Запуск параллельного анализа проекта: {}", 
                 project_root.as_ref().display());
        
        let analyzer = ParallelAnalyzer::new(config)?;
        let result = analyzer.analyze_project(project_root)?;
        
        // Выводим результаты
        Self::print_analysis_results(&result);
        
        Ok(())
    }
    
    /// Напечатать результаты анализа
    fn print_analysis_results(result: &BatchAnalysisResult) {
        println!("\n📊 Результаты анализа:");
        println!("⏱️  Общее время: {:.2?}", result.total_time);
        println!("📁 Всего файлов: {}", result.stats.total_files);
        println!("✅ Успешно: {}", result.stats.successful_files);
        println!("❌ С ошибками: {}", result.stats.failed_files);
        println!("🔧 Функций найдено: {}", result.stats.total_functions);
        println!("📦 Переменных найдено: {}", result.stats.total_variables);
        println!("🚨 Диагностик: {}", result.stats.total_diagnostics);
        println!("📈 Среднее время на файл: {:.2?}", result.stats.avg_analysis_time);
        
        if result.stats.failed_files > 0 {
            println!("\n❌ Файлы с ошибками:");
            for file_result in &result.file_results {
                if !file_result.success {
                    println!("  • {} - {}", 
                             file_result.file_path.display(),
                             file_result.error_message.as_ref().unwrap_or(&"Неизвестная ошибка".to_string())
                    );
                }
            }
        }
        
        // Топ файлов по количеству диагностик
        let mut files_by_diagnostics: Vec<_> = result.file_results.iter()
            .filter(|r| r.success && !r.diagnostics.is_empty())
            .collect();
        files_by_diagnostics.sort_by(|a, b| b.diagnostics.len().cmp(&a.diagnostics.len()));
        
        if !files_by_diagnostics.is_empty() {
            println!("\n🔍 Файлы с наибольшим количеством диагностик:");
            for (i, file_result) in files_by_diagnostics.iter().take(5).enumerate() {
                println!("  {}. {} - {} диагностик", 
                         i + 1,
                         file_result.file_path.display(),
                         file_result.diagnostics.len()
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[test]
    fn test_parallel_analysis_config() {
        let config = ParallelAnalysisConfig::default();
        assert_eq!(config.chunk_size, 10);
        assert!(config.show_progress);
        assert!(config.use_cache);
    }
    
    #[test]
    fn test_file_priority() {
        let common_module = PathBuf::from("/src/CommonModules/TestModule.bsl");
        let catalog = PathBuf::from("/src/Catalogs/Items/Ext/ObjectModule.bsl");
        let report = PathBuf::from("/src/Reports/TestReport/Ext/ObjectModule.bsl");
        
        assert_eq!(ProjectAnalysisUtils::get_file_priority(&common_module), 1);
        assert_eq!(ProjectAnalysisUtils::get_file_priority(&catalog), 2);
        assert_eq!(ProjectAnalysisUtils::get_file_priority(&report), 3);
    }
    
    #[test]
    fn test_find_bsl_files() -> Result<()> {
        let temp_dir = TempDir::new()?;
        
        // Создаем тестовые файлы
        let bsl_file = temp_dir.path().join("test.bsl");
        let os_file = temp_dir.path().join("test.os");
        let txt_file = temp_dir.path().join("test.txt");
        
        fs::write(&bsl_file, "Процедура Тест() КонецПроцедуры")?;
        fs::write(&os_file, "Процедура ТестОС() КонецПроцедуры")?;
        fs::write(&txt_file, "not bsl file")?;
        
        let found_files = ParallelAnalyzer::find_bsl_files(temp_dir.path())?;
        
        assert_eq!(found_files.len(), 2);
        assert!(found_files.contains(&bsl_file));
        assert!(found_files.contains(&os_file));
        assert!(!found_files.contains(&txt_file));
        
        Ok(())
    }
    
    #[test]
    fn test_parallel_analyzer_creation() -> Result<()> {
        let config = ParallelAnalysisConfig {
            num_threads: Some(2),
            show_progress: false,
            use_cache: false,
            ..Default::default()
        };
        
        let analyzer = ParallelAnalyzer::new(config)?;
        assert!(analyzer.cache_manager.is_none()); // Кеш выключен
        
        Ok(())
    }
    
    #[test]
    fn test_group_files_by_subsystem() {
        let files = vec![
            PathBuf::from("/project/CommonModules/Module1.bsl"),
            PathBuf::from("/project/CommonModules/Module2.bsl"),
            PathBuf::from("/project/Catalogs/Items.bsl"),
            PathBuf::from("/project/Reports/Report1.bsl"),
        ];
        
        let groups = ProjectAnalysisUtils::group_files_by_subsystem(&files);
        
        assert_eq!(groups.get("CommonModules").unwrap().len(), 2);
        assert_eq!(groups.get("Catalogs").unwrap().len(), 1);
        assert_eq!(groups.get("Reports").unwrap().len(), 1);
    }
}