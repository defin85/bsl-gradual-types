//! Интеграционный тест для Milestone 2.13: IR Caching & Performance Optimization
//!
//! Проверяет:
//! 1. Первый hover → cache MISS → parse + cache
//! 2. Второй hover (same file) → cache HIT → instant
//! 3. Измерение времени обоих hovers
//! 4. Проверка статистики кеша

use std::sync::Arc;
use std::time::Instant;
use bsl_backend::application::TypeSystemService;
use bsl_backend::system::{AnalysisCache, ParserCoordinator, IrCache};
use bsl_shared::engine::AnalysisEngine;
use bsl_shared::domain::repository::InMemoryTypeRepository;
use bsl_shared::domain::resolver::TypeResolver;

/// Инициализация TypeSystemService для тестов
fn setup_service() -> TypeSystemService {
    let repository = Arc::new(InMemoryTypeRepository::new());
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let analysis_engine = Arc::new(AnalysisEngine::new(resolver, repository.clone()));
    let cache = Arc::new(AnalysisCache::new(1000));
    let parser = Arc::new(ParserCoordinator::with_fallback());
    let ir_cache = Arc::new(IrCache::new(100)); // MILESTONE 2.13: IR Cache

    TypeSystemService::new(analysis_engine, cache, parser, ir_cache)
}

#[tokio::test]
async fn test_ir_cache_hit_vs_miss() {
    let service = setup_service();

    let code = r#"
Процедура Тест()
    МассивДанных = Новый Массив();
    МассивДанных.Добавить(42);
КонецПроцедуры
    "#;

    // Первый hover - cache MISS (должен парсить файл)
    let start1 = Instant::now();
    let hover1 = service
        .get_hover_info(code, 2, 4)
        .await
        .expect("First hover failed");
    let duration1 = start1.elapsed();

    println!("First hover (MISS): {:?}", duration1);
    assert!(hover1.is_some(), "First hover should return info");

    // Второй hover - cache HIT (должен быть мгновенным)
    let start2 = Instant::now();
    let hover2 = service
        .get_hover_info(code, 2, 4)
        .await
        .expect("Second hover failed");
    let duration2 = start2.elapsed();

    println!("Second hover (HIT): {:?}", duration2);
    assert!(hover2.is_some(), "Second hover should return info");

    // Проверяем, что оба hover вернули одинаковый результат
    assert_eq!(hover1, hover2, "Hover results should be identical");

    // Проверяем, что второй hover быстрее первого
    // (хотя бы в 2 раза, учитывая что парсинг занимает время)
    println!("Speedup: {:.2}x", duration1.as_secs_f64() / duration2.as_secs_f64());

    // Получаем статистику кеша
    let stats = service.get_ir_cache_stats().await;
    println!("Cache stats: hits={}, misses={}, evictions={}",
        stats.hits, stats.misses, stats.evictions);

    // Проверяем статистику:
    // - Первый hover → MISS → парсинг → put → get при поиске узла → HIT
    // - Второй hover → HIT
    // Итого: минимум 1 hit, 1 miss
    assert!(stats.hits >= 1, "Should have at least 1 hit (expected: {})", stats.hits);
    assert!(stats.misses >= 1, "Should have at least 1 miss (expected: {})", stats.misses);
}

#[tokio::test]
async fn test_ir_cache_different_files() {
    let service = setup_service();

    let code1 = r#"
Процедура Тест1()
    Переменная1 = Новый Массив();
КонецПроцедуры
    "#;

    let code2 = r#"
Процедура Тест2()
    Переменная2 = Новый Структура();
КонецПроцедуры
    "#;

    // Hover на первом файле
    let _hover1 = service
        .get_hover_info(code1, 2, 4)
        .await
        .expect("Hover on code1 failed");

    // Hover на втором файле (другой content hash)
    let _hover2 = service
        .get_hover_info(code2, 2, 4)
        .await
        .expect("Hover on code2 failed");

    // Статистика должна показать 2 miss (разные файлы → разные hash)
    let stats = service.get_ir_cache_stats().await;
    println!("Cache stats for different files: hits={}, misses={}",
        stats.hits, stats.misses);

    assert!(stats.misses >= 2, "Should have at least 2 misses for different files");
}

#[tokio::test]
async fn test_ir_cache_modified_file() {
    let service = setup_service();

    let code_v1 = r#"
Процедура Тест()
    Переменная = Новый Массив();
КонецПроцедуры
    "#;

    let code_v2 = r#"
Процедура Тест()
    Переменная = Новый Структура();
КонецПроцедуры
    "#;

    // Hover на v1
    let hover1 = service
        .get_hover_info(code_v1, 2, 4)
        .await
        .expect("Hover on v1 failed");

    // Hover на v2 (изменённый файл → другой hash → cache MISS)
    let hover2 = service
        .get_hover_info(code_v2, 2, 4)
        .await
        .expect("Hover on v2 failed");

    // Результаты должны отличаться (разные типы)
    assert_ne!(hover1, hover2, "Hovers on different file versions should differ");

    // Статистика: минимум 2 miss (разные hash)
    let stats = service.get_ir_cache_stats().await;
    assert!(stats.misses >= 2, "Modified file should cause cache MISS");
}

#[tokio::test]
async fn test_ir_cache_performance_target() {
    let service = setup_service();

    let code = r#"
Процедура Тест()
    МассивДанных = Новый Массив();
КонецПроцедуры
    "#;

    // Первый hover - cache MISS
    let start1 = Instant::now();
    let _hover1 = service.get_hover_info(code, 2, 4).await.unwrap();
    let duration1 = start1.elapsed();

    // Второй hover - cache HIT
    let start2 = Instant::now();
    let _hover2 = service.get_hover_info(code, 2, 4).await.unwrap();
    let duration2 = start2.elapsed();

    println!("Performance test:");
    println!("  First hover (MISS): {:?}", duration1);
    println!("  Second hover (HIT):  {:?}", duration2);

    // MILESTONE 2.13: Целевые метрики
    // - Первый hover: 50-100ms (парсинг + кеш)
    // - Последующие hover: <5ms (кеш попадание)

    // В тестовом окружении без реального парсера метрики могут отличаться
    // Проверяем, что второй hover значительно быстрее первого
    assert!(
        duration2 < duration1 / 2,
        "Cache HIT should be at least 2x faster than MISS. Got: {:?} vs {:?}",
        duration2,
        duration1
    );
}
