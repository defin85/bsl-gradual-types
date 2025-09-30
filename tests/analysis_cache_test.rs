//! Unit-тесты для AnalysisCache (Phase 4)
//!
//! Тестирование String-based API и hit rate tracking

use bsl_backend::system::simple_cache::{AnalysisCache, AnalysisResult};
use std::collections::HashMap;
use std::time::Instant;

#[test]
fn test_string_cache_get_and_store() {
    // Arrange
    let cache = AnalysisCache::new(10);

    let key = "test.bsl:hash123";
    let analysis = AnalysisResult {
        file_path: "test.bsl".to_string(),
        type_resolutions: HashMap::new(),
        analysis_duration_ms: 42,
        cached_at: Instant::now(),
    };

    // Act: store
    cache.store_analysis(key.to_string(), analysis.clone());

    // Assert: get
    let retrieved = cache.get_analysis(key);
    assert!(retrieved.is_some(), "Should retrieve stored analysis");

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.file_path, "test.bsl");
    assert_eq!(retrieved.analysis_duration_ms, 42);
}

#[test]
fn test_string_cache_miss() {
    // Arrange
    let cache = AnalysisCache::new(10);

    // Act: пытаемся получить несуществующий ключ
    let result = cache.get_analysis("nonexistent");

    // Assert
    assert!(result.is_none(), "Should return None for missing key");
}

#[test]
fn test_hit_rate_tracking() {
    // Arrange
    let cache = AnalysisCache::new(10);

    let analysis = AnalysisResult {
        file_path: "test.bsl".to_string(),
        type_resolutions: HashMap::new(),
        analysis_duration_ms: 100,
        cached_at: Instant::now(),
    };

    // Act: store one item
    cache.store_analysis("key1".to_string(), analysis);

    // Perform cache operations
    let _ = cache.get_analysis("key1"); // hit
    let _ = cache.get_analysis("key1"); // hit
    let _ = cache.get_analysis("key2"); // miss
    let _ = cache.get_analysis("key3"); // miss

    // Assert: hit rate should be 50% (2 hits, 2 misses)
    let hit_rate = cache.get_hit_rate();
    assert!(
        (hit_rate - 50.0).abs() < 0.1,
        "Hit rate should be ~50%, got {}",
        hit_rate
    );
}

#[test]
fn test_hit_rate_all_hits() {
    // Arrange
    let cache = AnalysisCache::new(10);

    let analysis = AnalysisResult {
        file_path: "test.bsl".to_string(),
        type_resolutions: HashMap::new(),
        analysis_duration_ms: 100,
        cached_at: Instant::now(),
    };

    cache.store_analysis("key1".to_string(), analysis);

    // Act: only hits
    let _ = cache.get_analysis("key1");
    let _ = cache.get_analysis("key1");
    let _ = cache.get_analysis("key1");

    // Assert: 100% hit rate
    let hit_rate = cache.get_hit_rate();
    assert!(
        (hit_rate - 100.0).abs() < 0.1,
        "Hit rate should be 100%, got {}",
        hit_rate
    );
}

#[test]
fn test_hit_rate_all_misses() {
    // Arrange
    let cache = AnalysisCache::new(10);

    // Act: only misses
    let _ = cache.get_analysis("nonexistent1");
    let _ = cache.get_analysis("nonexistent2");
    let _ = cache.get_analysis("nonexistent3");

    // Assert: 0% hit rate
    let hit_rate = cache.get_hit_rate();
    assert!(
        hit_rate.abs() < 0.1,
        "Hit rate should be 0%, got {}",
        hit_rate
    );
}

#[test]
fn test_hit_rate_empty_cache() {
    // Arrange
    let cache = AnalysisCache::new(10);

    // Act: no operations
    let hit_rate = cache.get_hit_rate();

    // Assert: should return 0 for empty cache
    assert_eq!(hit_rate, 0.0, "Empty cache should have 0% hit rate");
}

#[test]
fn test_string_cache_overwrite() {
    // Arrange
    let cache = AnalysisCache::new(10);

    let analysis1 = AnalysisResult {
        file_path: "test.bsl".to_string(),
        type_resolutions: HashMap::new(),
        analysis_duration_ms: 100,
        cached_at: Instant::now(),
    };

    let analysis2 = AnalysisResult {
        file_path: "test.bsl".to_string(),
        type_resolutions: HashMap::new(),
        analysis_duration_ms: 200,
        cached_at: Instant::now(),
    };

    // Act: store twice with same key
    cache.store_analysis("key1".to_string(), analysis1);
    cache.store_analysis("key1".to_string(), analysis2);

    // Assert: should get latest value
    let retrieved = cache.get_analysis("key1").unwrap();
    assert_eq!(
        retrieved.analysis_duration_ms, 200,
        "Should get updated value"
    );
}

#[test]
fn test_string_cache_lru_eviction() {
    // Arrange: small cache
    let cache = AnalysisCache::new(3);

    let make_analysis = |duration: u64| AnalysisResult {
        file_path: format!("test_{}.bsl", duration),
        type_resolutions: HashMap::new(),
        analysis_duration_ms: duration,
        cached_at: Instant::now(),
    };

    // Act: fill cache beyond capacity
    cache.store_analysis("key1".to_string(), make_analysis(1));
    cache.store_analysis("key2".to_string(), make_analysis(2));
    cache.store_analysis("key3".to_string(), make_analysis(3));

    // Access key1 and key2 to make them recently used
    let _ = cache.get_analysis("key1");
    let _ = cache.get_analysis("key2");

    // Add new item - should evict key3 (least recently used)
    cache.store_analysis("key4".to_string(), make_analysis(4));

    // Assert: key3 should be evicted
    assert!(cache.get_analysis("key1").is_some(), "key1 should still exist");
    assert!(cache.get_analysis("key2").is_some(), "key2 should still exist");
    assert!(cache.get_analysis("key3").is_none(), "key3 should be evicted");
    assert!(cache.get_analysis("key4").is_some(), "key4 should exist");
}

#[test]
fn test_multiple_caches_independent_hit_rates() {
    // Arrange: two independent caches
    let cache1 = AnalysisCache::new(10);
    let cache2 = AnalysisCache::new(10);

    let analysis = AnalysisResult {
        file_path: "test.bsl".to_string(),
        type_resolutions: HashMap::new(),
        analysis_duration_ms: 100,
        cached_at: Instant::now(),
    };

    // Act: different operations on each cache
    cache1.store_analysis("key1".to_string(), analysis.clone());
    let _ = cache1.get_analysis("key1"); // hit
    let _ = cache1.get_analysis("key1"); // hit

    cache2.store_analysis("key2".to_string(), analysis);
    let _ = cache2.get_analysis("key3"); // miss
    let _ = cache2.get_analysis("key3"); // miss

    // Assert: hit rates should be independent
    let hit_rate1 = cache1.get_hit_rate();
    let hit_rate2 = cache2.get_hit_rate();

    assert!(
        (hit_rate1 - 100.0).abs() < 0.1,
        "Cache1 should have 100% hit rate"
    );
    assert!(
        hit_rate2.abs() < 0.1,
        "Cache2 should have 0% hit rate"
    );
}