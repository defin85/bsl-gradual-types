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
