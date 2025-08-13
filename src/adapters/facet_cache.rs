//! Кеширование фасетных шаблонов из синтакс-помощника

use std::collections::HashMap;
use std::path::Path;
use std::fs;
use anyhow::Result;
use serde::{Serialize, Deserialize};
use crate::core::types::{Method, Property, FacetKind};
use crate::core::facets::FacetRegistry;

/// Кеш фасетных шаблонов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetCache {
    /// Версия платформы для которой построен кеш
    pub platform_version: String,
    
    /// Время создания кеша (Unix timestamp)
    pub created_at: u64,
    
    /// Кешированные фасеты: тип -> фасет -> (методы, свойства)
    pub facets: HashMap<String, HashMap<FacetKind, CachedFacet>>,
}

/// Кешированный фасет
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedFacet {
    pub methods: Vec<Method>,
    pub properties: Vec<Property>,
}

impl FacetCache {
    /// Создаёт новый пустой кеш
    pub fn new(platform_version: String) -> Self {
        Self {
            platform_version,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            facets: HashMap::new(),
        }
    }
    
    /// Добавляет фасет в кеш
    pub fn add_facet(&mut self, type_name: &str, facet_kind: FacetKind, methods: Vec<Method>, properties: Vec<Property>) {
        let type_facets = self.facets.entry(type_name.to_string()).or_insert_with(HashMap::new);
        type_facets.insert(facet_kind, CachedFacet { methods, properties });
    }
    
    /// Получает фасет из кеша
    pub fn get_facet(&self, type_name: &str, facet_kind: FacetKind) -> Option<&CachedFacet> {
        self.facets.get(type_name)?.get(&facet_kind)
    }
    
    /// Сохраняет кеш в файл
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
    
    /// Загружает кеш из файла
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let json = fs::read_to_string(path)?;
        let cache = serde_json::from_str(&json)?;
        Ok(cache)
    }
    
    /// Проверяет актуальность кеша (не старше 30 дней)
    pub fn is_valid(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let age_seconds = now - self.created_at;
        let max_age_seconds = 30 * 24 * 60 * 60; // 30 дней
        
        age_seconds < max_age_seconds
    }
    
    /// Заполняет FacetRegistry из кеша
    pub fn populate_registry(&self, registry: &mut FacetRegistry) {
        for (type_name, type_facets) in &self.facets {
            for (facet_kind, cached_facet) in type_facets {
                registry.register_facet(
                    type_name,
                    *facet_kind,
                    cached_facet.methods.clone(),
                    cached_facet.properties.clone(),
                );
            }
        }
    }
    
    /// Создаёт кеш из FacetRegistry
    pub fn from_registry(registry: &FacetRegistry, platform_version: String) -> Self {
        let cache = Self::new(platform_version);
        
        // TODO: Добавить метод в FacetRegistry для итерации по всем фасетам
        // Пока возвращаем пустой кеш
        
        cache
    }
}

/// Менеджер кеша фасетов
pub struct FacetCacheManager {
    cache_dir: String,
    current_cache: Option<FacetCache>,
}

impl FacetCacheManager {
    /// Создаёт новый менеджер кеша
    pub fn new(cache_dir: impl AsRef<Path>) -> Self {
        Self {
            cache_dir: cache_dir.as_ref().to_string_lossy().to_string(),
            current_cache: None,
        }
    }
    
    /// Получает или создаёт кеш для версии платформы
    pub fn get_or_create_cache(&mut self, platform_version: &str) -> Result<&mut FacetCache> {
        let cache_file = format!("{}/facet_cache_{}.json", self.cache_dir, platform_version);
        let cache_path = Path::new(&cache_file);
        
        // Пытаемся загрузить существующий кеш
        if cache_path.exists() {
            if let Ok(cache) = FacetCache::load_from_file(cache_path) {
                if cache.is_valid() && cache.platform_version == platform_version {
                    self.current_cache = Some(cache);
                    return Ok(self.current_cache.as_mut().unwrap());
                }
            }
        }
        
        // Создаём новый кеш
        self.current_cache = Some(FacetCache::new(platform_version.to_string()));
        Ok(self.current_cache.as_mut().unwrap())
    }
    
    /// Сохраняет текущий кеш
    pub fn save_cache(&self) -> Result<()> {
        if let Some(ref cache) = self.current_cache {
            let cache_file = format!("{}/facet_cache_{}.json", self.cache_dir, cache.platform_version);
            cache.save_to_file(cache_file)?;
        }
        Ok(())
    }
    
    /// Очищает устаревшие кеши
    pub fn cleanup_old_caches(&self) -> Result<()> {
        let cache_dir = Path::new(&self.cache_dir);
        
        if !cache_dir.exists() {
            return Ok(());
        }
        
        for entry in fs::read_dir(cache_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().is_some_and(|ext| ext == "json") {
                if let Some(filename) = path.file_name() {
                    let filename_str = filename.to_string_lossy();
                    if filename_str.starts_with("facet_cache_") {
                        // Проверяем возраст файла
                        if let Ok(metadata) = fs::metadata(&path) {
                            if let Ok(modified) = metadata.modified() {
                                if let Ok(age) = modified.elapsed() {
                                    // Удаляем файлы старше 60 дней
                                    if age.as_secs() > 60 * 24 * 60 * 60 {
                                        let _ = fs::remove_file(&path);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_facet_cache_creation() {
        let cache = FacetCache::new("8.3.25".to_string());
        assert_eq!(cache.platform_version, "8.3.25");
        assert!(cache.facets.is_empty());
    }
    
    #[test]
    fn test_facet_cache_add_and_get() {
        let mut cache = FacetCache::new("8.3.25".to_string());
        
        let methods = vec![Method {
            name: "Test".to_string(),
            parameters: vec![],
            return_type: None,
        }];
        
        let properties = vec![Property {
            name: "TestProp".to_string(),
            type_: "String".to_string(),
            readonly: false,
        }];
        
        cache.add_facet("TestType", FacetKind::Manager, methods.clone(), properties.clone());
        
        let facet = cache.get_facet("TestType", FacetKind::Manager).unwrap();
        assert_eq!(facet.methods.len(), 1);
        assert_eq!(facet.properties.len(), 1);
    }
}