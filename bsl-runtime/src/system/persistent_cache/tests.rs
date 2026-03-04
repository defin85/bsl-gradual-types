use super::*;
use tempfile::TempDir;

#[test]
fn test_persistent_cache_creation() {
    let temp_dir = TempDir::new().unwrap();
    let cache = PersistentCache::new(Some(temp_dir.path().to_path_buf())).unwrap();

    assert!(cache.ast_cache_dir.exists());
    assert!(cache.analysis_cache_dir.exists());
}

#[test]
fn test_content_hash() {
    let hash1 = PersistentCache::compute_content_hash("test content");
    let hash2 = PersistentCache::compute_content_hash("test content");
    let hash3 = PersistentCache::compute_content_hash("different");

    assert_eq!(hash1, hash2);
    assert_ne!(hash1, hash3);
}

#[test]
fn test_sanitize_filename() {
    let sanitized = PersistentCache::sanitize_filename("C:\\Users\\Test\\file.bsl");
    assert!(!sanitized.contains('\\'));
    assert!(!sanitized.contains(':'));
}

#[test]
fn test_analysis_cache_roundtrip() {
    let temp_dir = TempDir::new().unwrap();
    let cache = PersistentCache::new(Some(temp_dir.path().to_path_buf())).unwrap();

    let file_path = "test.bsl";
    let content = "Функция Тест() КонецФункции";
    let hash = PersistentCache::compute_content_hash(content);

    // Store
    let type_resolutions = HashMap::new();
    cache
        .store_analysis(file_path, &hash, &type_resolutions, 100)
        .unwrap();

    // Load
    let loaded = cache.load_analysis(file_path, &hash).unwrap();
    assert!(loaded.is_some());

    let cached = loaded.unwrap();
    assert_eq!(cached.file_path, file_path);
    assert_eq!(cached.content_hash, hash);
}

#[test]
fn test_cache_invalidation_on_hash_mismatch() {
    let temp_dir = TempDir::new().unwrap();
    let cache = PersistentCache::new(Some(temp_dir.path().to_path_buf())).unwrap();

    let file_path = "test.bsl";
    let old_hash = "old_hash";
    let new_hash = "new_hash";

    // Store with old hash
    cache
        .store_analysis(file_path, old_hash, &HashMap::new(), 100)
        .unwrap();

    // Try to load with new hash - should return None
    let loaded = cache.load_analysis(file_path, new_hash).unwrap();
    assert!(loaded.is_none());
}
