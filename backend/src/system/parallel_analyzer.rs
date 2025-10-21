//! Parallel Project Analyzer - Milestone 2.4 Task 2
//!
//! Multi-threaded анализ файлов через rayon:
//! - Параллельный анализ файлов проекта
//! - Прогресс-бар для больших проектов (indicatif)
//! - Graceful degradation при ошибках
//! - Интеграция с PersistentCache
//!
//! Цель: Анализ 1000 файлов < 30 секунд

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::{debug, info, warn};

use bsl_shared::domain::types::TypeResolution;

use super::persistent_cache::{PersistentCache, SerializableTypeResolution};

/// Результат параллельного анализа проекта
#[derive(Debug, Clone)]
pub struct ProjectAnalysisResult {
    /// Количество успешно проанализированных файлов
    pub files_analyzed: usize,
    /// Количество файлов с ошибками
    pub files_failed: usize,
    /// Количество файлов загруженных из кеша
    pub files_from_cache: usize,
    /// Общее время анализа
    pub total_duration_ms: u64,
    /// Результаты анализа по файлам
    pub file_results: HashMap<String, FileAnalysisResult>,
    /// Ошибки при анализе
    pub errors: Vec<AnalysisError>,
}

/// Результат анализа одного файла
#[derive(Debug, Clone)]
pub struct FileAnalysisResult {
    pub file_path: String,
    pub type_count: usize,
    pub analysis_duration_ms: u64,
    pub from_cache: bool,
}

/// Ошибка анализа файла
#[derive(Debug, Clone)]
pub struct AnalysisError {
    pub file_path: String,
    pub error_message: String,
}

/// Parallel Project Analyzer
pub struct ParallelAnalyzer {
    /// Persistent cache для межсессионного кеширования
    cache: Arc<PersistentCache>,
    /// Включить прогресс-бар
    show_progress: bool,
    /// Количество потоков (None = auto)
    num_threads: Option<usize>,
}

impl ParallelAnalyzer {
    /// Создать новый parallel analyzer с persistent cache
    pub fn new(cache: Arc<PersistentCache>) -> Self {
        Self {
            cache,
            show_progress: true,
            num_threads: None,
        }
    }

    /// Установить количество потоков
    pub fn with_threads(mut self, num_threads: usize) -> Self {
        self.num_threads = Some(num_threads);
        self
    }

    /// Включить/выключить прогресс-бар
    pub fn with_progress(mut self, show_progress: bool) -> Self {
        self.show_progress = show_progress;
        self
    }

    /// Анализировать весь проект параллельно
    pub fn analyze_project(&self, project_root: &Path) -> Result<ProjectAnalysisResult> {
        let start = Instant::now();

        info!(
            "Starting parallel project analysis: {}",
            project_root.display()
        );

        // 1. Найти все .bsl файлы
        let bsl_files = self.find_bsl_files(project_root)?;
        let total_files = bsl_files.len();

        info!("Found {} BSL files to analyze", total_files);

        if total_files == 0 {
            return Ok(ProjectAnalysisResult {
                files_analyzed: 0,
                files_failed: 0,
                files_from_cache: 0,
                total_duration_ms: start.elapsed().as_millis() as u64,
                file_results: HashMap::new(),
                errors: Vec::new(),
            });
        }

        // 2. Настроить thread pool если указано
        if let Some(threads) = self.num_threads {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .context("Failed to configure thread pool")?;
        }

        // 3. Создать прогресс-бар
        let progress = if self.show_progress {
            let pb = ProgressBar::new(total_files as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template(
                        "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}",
                    )
                    .expect("Invalid progress template")
                    .progress_chars("#>-"),
            );
            Some(pb)
        } else {
            None
        };

        // 4. Shared state для результатов
        let file_results = Arc::new(Mutex::new(HashMap::new()));
        let errors = Arc::new(Mutex::new(Vec::new()));
        let cache_hits = Arc::new(Mutex::new(0usize));

        // 5. Параллельный анализ через rayon
        bsl_files.par_iter().for_each(|file_path| {
            match self.analyze_file(file_path) {
                Ok(result) => {
                    if result.from_cache {
                        let mut hits = cache_hits.lock().unwrap();
                        *hits += 1;
                    }

                    file_results
                        .lock()
                        .unwrap()
                        .insert(file_path.display().to_string(), result);
                }
                Err(e) => {
                    errors.lock().unwrap().push(AnalysisError {
                        file_path: file_path.display().to_string(),
                        error_message: e.to_string(),
                    });
                }
            }

            if let Some(pb) = &progress {
                pb.inc(1);
            }
        });

        // 6. Финализация прогресс-бара
        if let Some(pb) = progress {
            pb.finish_with_message("Analysis complete");
        }

        // 7. Собрать результаты
        let file_results_map = match Arc::try_unwrap(file_results) {
            Ok(mutex) => mutex.into_inner().unwrap(),
            Err(arc) => arc.lock().unwrap().clone(),
        };

        let errors_vec = match Arc::try_unwrap(errors) {
            Ok(mutex) => mutex.into_inner().unwrap(),
            Err(arc) => arc.lock().unwrap().clone(),
        };

        let files_from_cache = match Arc::try_unwrap(cache_hits) {
            Ok(mutex) => mutex.into_inner().unwrap(),
            Err(arc) => *arc.lock().unwrap(),
        };

        let files_analyzed = file_results_map.len();
        let files_failed = errors_vec.len();

        let result = ProjectAnalysisResult {
            files_analyzed,
            files_failed,
            files_from_cache,
            total_duration_ms: start.elapsed().as_millis() as u64,
            file_results: file_results_map,
            errors: errors_vec,
        };

        info!(
            "Project analysis complete: {} analyzed, {} from cache, {} failed in {}ms",
            result.files_analyzed,
            result.files_from_cache,
            result.files_failed,
            result.total_duration_ms
        );

        Ok(result)
    }

    /// Найти все .bsl файлы в проекте
    fn find_bsl_files(&self, root: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        self.walk_directory(root, &mut files)?;
        Ok(files)
    }

    /// Рекурсивно обойти директорию
    fn walk_directory(&self, dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Пропустить .bsl_cache и другие системные директории
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.') || name == "target" || name == "node_modules" {
                        continue;
                    }
                }
                self.walk_directory(&path, files)?;
            } else if path.extension().and_then(|e| e.to_str()) == Some("bsl") {
                files.push(path);
            }
        }

        Ok(())
    }

    /// Анализировать один файл с поддержкой кеша
    fn analyze_file(&self, file_path: &Path) -> Result<FileAnalysisResult> {
        let start = Instant::now();

        // 1. Прочитать содержимое файла
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        // 2. Вычислить hash для cache lookup
        let content_hash = PersistentCache::compute_content_hash(&content);
        let file_path_str = file_path.display().to_string();

        // 3. Попытаться загрузить из persistent cache
        if let Ok(Some(cached)) = self.cache.load_analysis(&file_path_str, &content_hash) {
            debug!("Cache hit for {}", file_path_str);

            return Ok(FileAnalysisResult {
                file_path: file_path_str,
                type_count: cached.type_resolutions.len(),
                analysis_duration_ms: start.elapsed().as_millis() as u64,
                from_cache: true,
            });
        }

        // 4. Выполнить анализ (простая эвристика для демонстрации)
        let type_resolutions = self.simple_analysis(&content)?;

        // 5. Сохранить в persistent cache
        let analysis_duration_ms = start.elapsed().as_millis() as u64;

        // Конвертировать HashMap<String, TypeResolution> для сохранения
        // Для демонстрации создаём пустой HashMap - в реальной интеграции
        // здесь будут реальные TypeResolution из AnalysisEngine
        let empty_resolutions: HashMap<String, TypeResolution> = HashMap::new();

        if let Err(e) = self.cache.store_analysis(
            &file_path_str,
            &content_hash,
            &empty_resolutions,
            analysis_duration_ms,
        ) {
            warn!("Failed to store cache for {}: {}", file_path_str, e);
        }

        Ok(FileAnalysisResult {
            file_path: file_path_str,
            type_count: type_resolutions.len(),
            analysis_duration_ms,
            from_cache: false,
        })
    }

    /// Простой анализ для демонстрации (будет заменён на AnalysisEngine)
    fn simple_analysis(
        &self,
        content: &str,
    ) -> Result<HashMap<String, SerializableTypeResolution>> {
        let mut resolutions = HashMap::new();

        // Простая эвристика: найти определения функций
        for line in content.lines() {
            let trimmed = line.trim();

            // Ищем "Функция" или "Function" и извлекаем имя после них
            let func_start_pos = if let Some(pos) = trimmed.find("Функция") {
                Some(pos + "Функция".len())
            } else if let Some(pos) = trimmed.find("Function") {
                Some(pos + "Function".len())
            } else {
                None
            };

            if let Some(start_pos) = func_start_pos {
                let after_keyword = &trimmed[start_pos..];
                // Пропустить пробелы после ключевого слова
                if let Some(name_start) = after_keyword.find(|c: char| c.is_alphabetic()) {
                    let name_part = &after_keyword[name_start..];
                    // Найти конец имени (до скобки)
                    if let Some(paren_pos) = name_part.find('(') {
                        let func_name = name_part[..paren_pos].trim();
                        if !func_name.is_empty() {
                            resolutions.insert(
                                func_name.to_string(),
                                SerializableTypeResolution {
                                    type_string: "Function".to_string(),
                                    certainty: 0.9,
                                    source: "Static".to_string(),
                                },
                            );
                        }
                    }
                }
            }
        }

        Ok(resolutions)
    }

    /// Получить статистику производительности
    pub fn get_performance_stats(&self, result: &ProjectAnalysisResult) -> PerformanceStats {
        let cache_hit_rate = if result.files_analyzed + result.files_failed > 0 {
            (result.files_from_cache as f64 / (result.files_analyzed + result.files_failed) as f64)
                * 100.0
        } else {
            0.0
        };

        let avg_file_time_ms = if result.files_analyzed > 0 {
            result.total_duration_ms / result.files_analyzed as u64
        } else {
            0
        };

        let files_per_second = if result.total_duration_ms > 0 {
            (result.files_analyzed as f64 / (result.total_duration_ms as f64 / 1000.0)) as usize
        } else {
            0
        };

        PerformanceStats {
            total_files: result.files_analyzed + result.files_failed,
            cache_hit_rate,
            avg_file_time_ms,
            files_per_second,
            total_duration_ms: result.total_duration_ms,
        }
    }
}

/// Статистика производительности анализа
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub total_files: usize,
    pub cache_hit_rate: f64,
    pub avg_file_time_ms: u64,
    pub files_per_second: usize,
    pub total_duration_ms: u64,
}

impl PerformanceStats {
    /// Проверить, выполняется ли цель "1000 файлов < 30 секунд"
    pub fn meets_performance_goal(&self) -> bool {
        if self.total_files >= 1000 {
            self.total_duration_ms < 30_000
        } else {
            // Экстраполяция для меньшего количества файлов
            let estimated_1000_files_ms =
                (1000.0 / self.total_files as f64) * self.total_duration_ms as f64;
            estimated_1000_files_ms < 30_000.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parallel_analyzer_creation() {
        let temp_cache = TempDir::new().unwrap();
        let cache = Arc::new(PersistentCache::new(Some(temp_cache.path().to_path_buf())).unwrap());
        let analyzer = ParallelAnalyzer::new(cache);

        assert!(analyzer.show_progress);
        assert!(analyzer.num_threads.is_none());
    }

    #[test]
    fn test_find_bsl_files() {
        let temp_dir = TempDir::new().unwrap();
        let cache = Arc::new(PersistentCache::new(None).unwrap());
        let analyzer = ParallelAnalyzer::new(cache);

        // Создать тестовые файлы
        fs::write(
            temp_dir.path().join("test1.bsl"),
            "Функция Тест1() КонецФункции",
        )
        .unwrap();
        fs::write(
            temp_dir.path().join("test2.bsl"),
            "Функция Тест2() КонецФункции",
        )
        .unwrap();
        fs::write(temp_dir.path().join("readme.txt"), "not a bsl file").unwrap();

        let files = analyzer.find_bsl_files(temp_dir.path()).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_simple_analysis() {
        let cache = Arc::new(PersistentCache::new(None).unwrap());
        let analyzer = ParallelAnalyzer::new(cache);

        let content = r#"
            Функция ПолучитьДанные()
                Возврат Новый Массив;
            КонецФункции

            Функция Тест()
                Возврат 42;
            КонецФункции
        "#;

        let result = analyzer.simple_analysis(content).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.contains_key("ПолучитьДанные"));
        assert!(result.contains_key("Тест"));
    }

    #[test]
    fn test_performance_stats() {
        let result = ProjectAnalysisResult {
            files_analyzed: 100,
            files_failed: 0,
            files_from_cache: 50,
            total_duration_ms: 1000,
            file_results: HashMap::new(),
            errors: Vec::new(),
        };

        let cache = Arc::new(PersistentCache::new(None).unwrap());
        let analyzer = ParallelAnalyzer::new(cache);
        let stats = analyzer.get_performance_stats(&result);

        assert_eq!(stats.cache_hit_rate, 50.0);
        assert_eq!(stats.avg_file_time_ms, 10);
        assert_eq!(stats.files_per_second, 100);
    }
}
