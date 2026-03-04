//! Persistent Cache - Межсессионное кеширование для BSL Type System
//!
//! Реализует требования Milestone 2.4:
//! - Кеш AST деревьев в `.bsl_cache/ast/`
//! - Кеш результатов анализа в `.bsl_cache/analysis/`
//! - Инвалидация при изменении файлов (по hash)
//! - TTL для устаревших кешей
//!
//! Цель: Загрузка из кеша < 50ms

#![allow(clippy::only_used_in_recursion)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, info};

use bsl_shared::domain::types::TypeResolution;

/// Persistent cache manager для межсессионного кеширования
pub struct PersistentCache {
    /// Корневая директория кеша (.bsl_cache/)
    cache_root: PathBuf,
    /// Директория для AST деревьев
    ast_cache_dir: PathBuf,
    /// Директория для результатов анализа
    analysis_cache_dir: PathBuf,
    /// TTL для кеша (по умолчанию 24 часа)
    default_ttl: Duration,
}

/// Закешированный результат анализа файла
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedAnalysis {
    /// Путь к файлу
    pub file_path: String,
    /// SHA-256 хеш содержимого файла
    pub content_hash: String,
    /// Результаты анализа типов
    pub type_resolutions: HashMap<String, SerializableTypeResolution>,
    /// Timestamp создания кеша (Unix timestamp)
    pub cached_at: u64,
    /// Время анализа в миллисекундах
    pub analysis_duration_ms: u64,
}

/// Serializable wrapper для TypeResolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableTypeResolution {
    /// Тип в строковом представлении
    pub type_string: String,
    /// Certainty (0.0-1.0)
    pub certainty: f32,
    /// Source (Static, Inferred, Dynamic)
    pub source: String,
}

impl From<&TypeResolution> for SerializableTypeResolution {
    fn from(resolution: &TypeResolution) -> Self {
        use bsl_shared::domain::types::Certainty;

        let certainty = match resolution.certainty {
            Certainty::Known => 1.0,
            Certainty::Inferred => 0.8,
            Certainty::InferredWeak => 0.5,
            Certainty::Unknown => 0.0,
        };

        Self {
            type_string: format!("{:?}", resolution.result),
            certainty,
            source: format!("{:?}", resolution.source),
        }
    }
}

/// Metadata о кеше AST дерева
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedAstMetadata {
    pub file_path: String,
    pub content_hash: String,
    pub cached_at: u64,
    /// Размер AST дерева в байтах (для статистики)
    pub tree_size_bytes: usize,
}

impl PersistentCache {
    /// Создать новый persistent cache с автоматическим созданием директорий
    pub fn new(cache_root: Option<PathBuf>) -> Result<Self> {
        let cache_root = cache_root.unwrap_or_else(|| PathBuf::from(".bsl_cache"));
        let ast_cache_dir = cache_root.join("ast");
        let analysis_cache_dir = cache_root.join("analysis");

        // Создать директории если не существуют
        fs::create_dir_all(&ast_cache_dir).context("Failed to create AST cache directory")?;
        fs::create_dir_all(&analysis_cache_dir)
            .context("Failed to create analysis cache directory")?;

        info!("Persistent cache initialized at {}", cache_root.display());

        Ok(Self {
            cache_root,
            ast_cache_dir,
            analysis_cache_dir,
            default_ttl: Duration::from_secs(24 * 60 * 60), // 24 часа
        })
    }

    /// Получить путь к файлу кеша анализа
    fn get_analysis_cache_path(&self, file_path: &str) -> PathBuf {
        let safe_name = Self::sanitize_filename(file_path);
        self.analysis_cache_dir.join(format!("{}.json", safe_name))
    }

    /// Получить путь к файлу кеша AST
    fn get_ast_cache_path(&self, file_path: &str) -> PathBuf {
        let safe_name = Self::sanitize_filename(file_path);
        self.ast_cache_dir.join(format!("{}.json", safe_name))
    }

    /// Преобразовать путь файла в безопасное имя файла кеша
    fn sanitize_filename(file_path: &str) -> String {
        file_path
            .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
            .chars()
            .take(200) // Ограничение длины имени файла
            .collect()
    }

    /// Вычислить SHA-256 хеш содержимого файла
    pub fn compute_content_hash(content: &str) -> String {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Получить текущий Unix timestamp
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Проверить, не истёк ли TTL кеша
    fn is_cache_valid(&self, cached_at: u64) -> bool {
        let now = Self::current_timestamp();
        let age = now.saturating_sub(cached_at);
        age < self.default_ttl.as_secs()
    }

    // =========================================================================
    // Analysis Cache Operations
    // =========================================================================

    /// Сохранить результаты анализа в persistent cache
    pub fn store_analysis(
        &self,
        file_path: &str,
        content_hash: &str,
        type_resolutions: &HashMap<String, TypeResolution>,
        analysis_duration_ms: u64,
    ) -> Result<()> {
        let cache_path = self.get_analysis_cache_path(file_path);

        // Конвертировать TypeResolution в serializable формат
        let serializable_resolutions: HashMap<String, SerializableTypeResolution> =
            type_resolutions
                .iter()
                .map(|(k, v)| (k.clone(), SerializableTypeResolution::from(v)))
                .collect();

        let cached = CachedAnalysis {
            file_path: file_path.to_string(),
            content_hash: content_hash.to_string(),
            type_resolutions: serializable_resolutions,
            cached_at: Self::current_timestamp(),
            analysis_duration_ms,
        };

        let json = serde_json::to_string_pretty(&cached)?;
        fs::write(&cache_path, json).context("Failed to write analysis cache")?;

        debug!(
            "Stored analysis cache for {} at {}",
            file_path,
            cache_path.display()
        );

        Ok(())
    }

    /// Загрузить результаты анализа из persistent cache
    pub fn load_analysis(
        &self,
        file_path: &str,
        content_hash: &str,
    ) -> Result<Option<CachedAnalysis>> {
        let cache_path = self.get_analysis_cache_path(file_path);

        if !cache_path.exists() {
            debug!("No cache found for {}", file_path);
            return Ok(None);
        }

        let json = fs::read_to_string(&cache_path).context("Failed to read analysis cache")?;
        let cached: CachedAnalysis =
            serde_json::from_str(&json).context("Failed to deserialize analysis cache")?;

        // Проверка 1: Hash match (инвалидация при изменении файла)
        if cached.content_hash != content_hash {
            debug!(
                "Cache invalidated for {} (hash mismatch: {} != {})",
                file_path, cached.content_hash, content_hash
            );
            return Ok(None);
        }

        // Проверка 2: TTL (инвалидация устаревших кешей)
        if !self.is_cache_valid(cached.cached_at) {
            debug!("Cache expired for {} (TTL exceeded)", file_path);
            return Ok(None);
        }

        debug!(
            "Loaded analysis cache for {} (age: {}s)",
            file_path,
            Self::current_timestamp() - cached.cached_at
        );

        Ok(Some(cached))
    }

    // =========================================================================
    // AST Cache Operations
    // =========================================================================

    /// Сохранить metadata AST дерева
    pub fn store_ast_metadata(
        &self,
        file_path: &str,
        content_hash: &str,
        tree_size_bytes: usize,
    ) -> Result<()> {
        let cache_path = self.get_ast_cache_path(file_path);

        let metadata = CachedAstMetadata {
            file_path: file_path.to_string(),
            content_hash: content_hash.to_string(),
            cached_at: Self::current_timestamp(),
            tree_size_bytes,
        };

        let json = serde_json::to_string_pretty(&metadata)?;
        fs::write(&cache_path, json).context("Failed to write AST metadata")?;

        debug!("Stored AST metadata for {}", file_path);

        Ok(())
    }

    /// Загрузить metadata AST дерева
    pub fn load_ast_metadata(
        &self,
        file_path: &str,
        content_hash: &str,
    ) -> Result<Option<CachedAstMetadata>> {
        let cache_path = self.get_ast_cache_path(file_path);

        if !cache_path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&cache_path)?;
        let metadata: CachedAstMetadata = serde_json::from_str(&json)?;

        // Валидация hash и TTL
        if metadata.content_hash != content_hash || !self.is_cache_valid(metadata.cached_at) {
            return Ok(None);
        }

        Ok(Some(metadata))
    }

    // =========================================================================
    // Cache Management
    // =========================================================================

    /// Очистить весь persistent cache
    pub fn clear_all(&self) -> Result<()> {
        info!("Clearing persistent cache at {}", self.cache_root.display());

        if self.ast_cache_dir.exists() {
            fs::remove_dir_all(&self.ast_cache_dir)?;
            fs::create_dir_all(&self.ast_cache_dir)?;
        }

        if self.analysis_cache_dir.exists() {
            fs::remove_dir_all(&self.analysis_cache_dir)?;
            fs::create_dir_all(&self.analysis_cache_dir)?;
        }

        info!("Persistent cache cleared");
        Ok(())
    }

    /// Очистить устаревшие записи кеша (TTL expired)
    pub fn cleanup_expired(&self) -> Result<CacheCleanupStats> {
        let mut removed_analysis = 0;
        let mut removed_ast = 0;

        // Очистка analysis cache
        if let Ok(entries) = fs::read_dir(&self.analysis_cache_dir) {
            for entry in entries.flatten() {
                if let Ok(contents) = fs::read_to_string(entry.path()) {
                    if let Ok(cached) = serde_json::from_str::<CachedAnalysis>(&contents) {
                        if !self.is_cache_valid(cached.cached_at) {
                            fs::remove_file(entry.path())?;
                            removed_analysis += 1;
                        }
                    }
                }
            }
        }

        // Очистка AST cache
        if let Ok(entries) = fs::read_dir(&self.ast_cache_dir) {
            for entry in entries.flatten() {
                if let Ok(contents) = fs::read_to_string(entry.path()) {
                    if let Ok(metadata) = serde_json::from_str::<CachedAstMetadata>(&contents) {
                        if !self.is_cache_valid(metadata.cached_at) {
                            fs::remove_file(entry.path())?;
                            removed_ast += 1;
                        }
                    }
                }
            }
        }

        info!(
            "Cache cleanup: removed {} analysis + {} AST entries",
            removed_analysis, removed_ast
        );

        Ok(CacheCleanupStats {
            removed_analysis,
            removed_ast,
        })
    }

    /// Получить статистику кеша
    pub fn get_stats(&self) -> Result<CacheStats> {
        let analysis_count = self.count_files(&self.analysis_cache_dir)?;
        let ast_count = self.count_files(&self.ast_cache_dir)?;
        let total_size_bytes = self.calculate_dir_size(&self.cache_root)?;

        Ok(CacheStats {
            analysis_entries: analysis_count,
            ast_entries: ast_count,
            total_size_bytes,
            cache_root: self.cache_root.display().to_string(),
        })
    }

    /// Подсчитать количество файлов в директории
    fn count_files(&self, dir: &Path) -> Result<usize> {
        if !dir.exists() {
            return Ok(0);
        }

        Ok(fs::read_dir(dir)?
            .filter_map(Result::ok)
            .filter(|e| e.path().is_file())
            .count())
    }

    /// Вычислить размер директории рекурсивно
    fn calculate_dir_size(&self, _dir: &Path) -> Result<u64> {
        if !_dir.exists() {
            return Ok(0);
        }

        let mut total = 0;
        for entry in fs::read_dir(_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                total += fs::metadata(&path)?.len();
            } else if path.is_dir() {
                total += self.calculate_dir_size(&path)?;
            }
        }

        Ok(total)
    }
}

/// Статистика persistent cache
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub analysis_entries: usize,
    pub ast_entries: usize,
    pub total_size_bytes: u64,
    pub cache_root: String,
}

impl CacheStats {
    pub fn total_size_mb(&self) -> f64 {
        self.total_size_bytes as f64 / (1024.0 * 1024.0)
    }
}

/// Результат очистки кеша
#[derive(Debug, Clone)]
pub struct CacheCleanupStats {
    pub removed_analysis: usize,
    pub removed_ast: usize,
}

#[cfg(test)]
#[path = "persistent_cache/tests.rs"]
mod tests;
