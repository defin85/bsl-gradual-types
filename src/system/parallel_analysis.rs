//! DEPRECATED: Параллельный анализ модулей с использованием rayon
//!
//! ⚠️ УСТАРЕВШИЙ КОД - НЕ ИСПОЛЬЗУЕТСЯ В SystemCoordinator
//! Этот модуль заменен на упрощенную архитектуру SystemCoordinator.
//! Сохранен для совместимости со старыми бинарными файлами.

#![allow(dead_code, unused_imports)]

use anyhow::Result;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::system::simple_cache::AnalysisCache; // Используем новый простой кеш
use crate::domain::analysis::type_checker::{TypeChecker, TypeContext, TypeDiagnostic};
use crate::parsing::bsl::common::ParserFactory;

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

/// Результат параллельного анализа проекта
#[derive(Debug, Clone)]
pub struct ProjectAnalysisResult {
    /// Результаты анализа файлов
    pub file_results: Vec<FileAnalysisResult>,
    /// Общее время анализа
    pub total_time: std::time::Duration,
    /// Статистика
    pub stats: AnalysisStatistics,
    /// Общие ошибки проекта
    pub project_errors: Vec<String>,
}

/// Статистика анализа
#[derive(Debug, Clone)]
pub struct AnalysisStatistics {
    /// Количество файлов
    pub total_files: usize,
    /// Успешно проанализировано
    pub successful_files: usize,
    /// Файлы с ошибками
    pub failed_files: usize,
    /// Общее количество диагностик
    pub total_diagnostics: usize,
    /// Диагностики по типам
    pub diagnostics_by_severity: HashMap<String, usize>,
    /// Среднее время анализа на файл
    pub avg_analysis_time: std::time::Duration,
}

/// Конфигурация параллельного анализа
#[derive(Debug, Clone)]
pub struct ParallelAnalysisConfig {
    /// Количество потоков (None = auto)
    pub thread_count: Option<usize>,
    /// Включить прогресс бар
    pub show_progress: bool,
    /// Остановить при первой ошибке
    pub fail_fast: bool,
    /// Максимальная глубина рекурсии
    pub max_depth: Option<usize>,
    /// Паттерны файлов для игнорирования
    pub ignore_patterns: Vec<String>,
    /// Использовать кеш
    pub use_cache: bool,
}

impl Default for ParallelAnalysisConfig {
    fn default() -> Self {
        Self {
            thread_count: None,
            show_progress: true,
            fail_fast: false,
            max_depth: Some(10),
            ignore_patterns: vec![
                "*.bak".to_string(),
                "*.tmp".to_string(),
                "*~".to_string(),
            ],
            use_cache: true,
        }
    }
}

/// Параллельный анализатор BSL проектов
pub struct ParallelAnalyzer {
    /// Конфигурация
    #[allow(dead_code)]
    config: ParallelAnalysisConfig,
    /// Кеш результатов (DEPRECATED)
    #[allow(dead_code)]
    cache: Option<String>, // Заглушка вместо AnalysisCacheManager
    /// Type checker
    #[allow(dead_code)]
    type_checker: Arc<TypeChecker>,
}

impl ParallelAnalyzer {
    /// Создать новый параллельный анализатор
    pub fn new(config: ParallelAnalysisConfig) -> Result<Self> {
        let cache = if config.use_cache {
            // TODO: Initialize cache manager
            None
        } else {
            None
        };

        // TODO: Initialize type checker properly
        let type_checker = Arc::new(TypeChecker::new("default".to_string()));

        Ok(Self {
            config,
            cache,
            type_checker,
        })
    }

    /// Найти все BSL файлы в директории
    pub fn find_bsl_files<P: AsRef<Path>>(root_dir: P) -> Result<Vec<PathBuf>> {
        let mut bsl_files = Vec::new();

        for entry in std::fs::read_dir(root_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if extension == "bsl" || extension == "os" {
                        bsl_files.push(path);
                    }
                }
            } else if path.is_dir() {
                // Рекурсивно ищем в подпапках
                let mut sub_files = Self::find_bsl_files(&path)?;
                bsl_files.append(&mut sub_files);
            }
        }

        Ok(bsl_files)
    }

    /// Анализировать проект 1С целиком
    pub fn analyze_project<P: AsRef<Path>>(&self, project_root: P) -> Result<ProjectAnalysisResult> {
        let start_time = std::time::Instant::now();
        let bsl_files = Self::find_bsl_files(project_root)?;

        if bsl_files.is_empty() {
            return Ok(ProjectAnalysisResult {
                file_results: vec![],
                total_time: std::time::Duration::ZERO,
                stats: AnalysisStatistics {
                    total_files: 0,
                    successful_files: 0,
                    failed_files: 0,
                    total_diagnostics: 0,
                    diagnostics_by_severity: HashMap::new(),
                    avg_analysis_time: std::time::Duration::ZERO,
                },
                project_errors: vec![],
            });
        }

        println!("🔍 Найдено {} BSL файлов для анализа", bsl_files.len());

        // Простая параллельная обработка
        let file_results: Vec<FileAnalysisResult> = bsl_files
            .par_iter()
            .map(|file_path| self.analyze_single_file(file_path))
            .collect::<Result<Vec<_>>>()?;

        let total_time = start_time.elapsed();
        let stats = self.calculate_stats(&file_results, total_time);

        Ok(ProjectAnalysisResult {
            file_results,
            total_time,
            stats,
            project_errors: vec![],
        })
    }

    /// Проанализировать один файл
    fn analyze_single_file(&self, file_path: &Path) -> Result<FileAnalysisResult> {
        let analysis_start = std::time::Instant::now();

        // Читаем файл
        let content = std::fs::read_to_string(file_path)?;

        // Создаем простой парсер и анализатор
        let mut parser = ParserFactory::create();
        let program = parser.parse(&content)?;

        let file_name = file_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown.bsl")
            .to_string();

        let type_checker = TypeChecker::new(file_name);
        let (context, diagnostics) = type_checker.check(&program);

        let analysis_time = analysis_start.elapsed();

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
    fn calculate_stats(
        &self,
        results: &[FileAnalysisResult],
        total_time: std::time::Duration,
    ) -> AnalysisStatistics {
        let total_files = results.len();
        let successful_files = results.iter().filter(|r| r.success).count();
        let failed_files = total_files - successful_files;

        let total_diagnostics: usize = results.iter().map(|r| r.diagnostics.len()).sum();

        let avg_analysis_time = if total_files > 0 {
            total_time / total_files as u32
        } else {
            std::time::Duration::ZERO
        };

        AnalysisStatistics {
            total_files,
            successful_files,
            failed_files,
            total_diagnostics,
            diagnostics_by_severity: HashMap::new(), // TODO: группировать диагностики
            avg_analysis_time,
        }
    }
}
