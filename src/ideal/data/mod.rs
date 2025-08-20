//! Data Layer - слой данных идеальной архитектуры
//! 
//! Отвечает за хранение и предоставление сырых данных о типах
//! Принципы: Single Source of Truth, абстракция от конкретных источников

use async_trait::async_trait;
use anyhow::Result;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

use crate::core::types::FacetKind;

/// Сырые данные о типе из парсеров
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTypeData {
    /// Уникальный идентификатор типа
    pub id: String,
    
    /// Русское название
    pub russian_name: String,
    
    /// Английское название
    pub english_name: String,
    
    /// Источник данных
    pub source: TypeSource,
    
    /// Категория/путь в иерархии
    pub category_path: Vec<String>,
    
    /// Методы типа
    pub methods: Vec<RawMethodData>,
    
    /// Свойства типа  
    pub properties: Vec<RawPropertyData>,
    
    /// Документация
    pub documentation: String,
    
    /// Примеры использования
    pub examples: Vec<String>,
    
    /// Доступные фасеты
    pub available_facets: Vec<FacetKind>,
    
    /// Метаданные парсинга
    pub parse_metadata: ParseMetadata,
}

/// Источник типов
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TypeSource {
    /// Платформенные типы из HTML справки
    Platform { version: String },
    
    /// Конфигурационные типы из XML
    Configuration { config_version: String },
    
    /// Пользовательские типы из BSL кода
    UserDefined { file_path: String },
}

/// Данные о методе
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawMethodData {
    pub name: String,
    pub parameters: Vec<RawParameterData>,
    pub return_type: Option<String>,
    pub description: String,
    pub examples: Vec<String>,
}

/// Данные о параметре
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawParameterData {
    pub name: String,
    pub type_name: Option<String>,
    pub is_optional: bool,
    pub default_value: Option<String>,
    pub description: String,
}

/// Данные о свойстве
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawPropertyData {
    pub name: String,
    pub type_name: String,
    pub is_readonly: bool,
    pub description: String,
}

/// Метаданные парсинга
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseMetadata {
    pub source_file: Option<String>,
    pub parse_time: Option<std::time::SystemTime>,
    pub parser_version: String,
    pub quality_score: f32, // 0.0-1.0 качество извлечённых данных
}

/// Фильтр для поиска типов
#[derive(Debug, Clone, Default)]
pub struct TypeFilter {
    pub source: Option<TypeSource>,
    pub category: Option<String>,
    pub name_pattern: Option<String>,
    pub has_methods: Option<bool>,
    pub facets: Vec<FacetKind>,
}

/// Статистика репозитория
#[derive(Debug, Clone, Serialize, Default)]
pub struct RepositoryStats {
    pub total_types: usize,
    pub platform_types: usize,
    pub configuration_types: usize,
    pub user_defined_types: usize,
    pub total_methods: usize,
    pub total_properties: usize,
    pub memory_usage_mb: f64,
    pub last_updated: Option<std::time::SystemTime>,
}

/// Абстракция репозитория типов
#[async_trait]
pub trait TypeRepository: Send + Sync {
    /// Сохранить типы в репозиторий
    async fn save_types(&self, types: Vec<RawTypeData>) -> Result<()>;
    
    /// Загрузить все типы
    async fn load_all_types(&self) -> Result<Vec<RawTypeData>>;
    
    /// Загрузить типы по фильтру
    async fn load_types_filtered(&self, filter: &TypeFilter) -> Result<Vec<RawTypeData>>;
    
    /// Получить тип по имени
    async fn get_type_by_name(&self, name: &str) -> Result<Option<RawTypeData>>;
    
    /// Получить типы по категории
    async fn get_types_by_category(&self, category: &str) -> Result<Vec<RawTypeData>>;
    
    /// Поиск типов по паттерну
    async fn search_types(&self, pattern: &str) -> Result<Vec<RawTypeData>>;
    
    /// Получить статистику репозитория
    async fn get_stats(&self) -> Result<RepositoryStats>;
    
    /// Очистить репозиторий
    async fn clear(&self) -> Result<()>;
}

/// In-memory реализация репозитория типов
pub struct InMemoryTypeRepository {
    /// Основное хранилище типов (по имени)
    types_by_name: Arc<RwLock<HashMap<String, RawTypeData>>>,
    
    /// Индекс по категориям (для быстрого поиска)
    types_by_category: Arc<RwLock<HashMap<String, Vec<String>>>>,
    
    /// Индекс по источникам
    types_by_source: Arc<RwLock<HashMap<TypeSource, Vec<String>>>>,
    
    /// Полнотекстовый индекс (упрощённый)
    fulltext_index: Arc<RwLock<HashMap<String, Vec<String>>>>,
    
    /// Статистика
    stats: Arc<RwLock<RepositoryStats>>,
}

use std::sync::Arc;
use tokio::sync::RwLock;

impl InMemoryTypeRepository {
    /// Создать новый репозиторий
    pub fn new() -> Self {
        Self {
            types_by_name: Arc::new(RwLock::new(HashMap::new())),
            types_by_category: Arc::new(RwLock::new(HashMap::new())),
            types_by_source: Arc::new(RwLock::new(HashMap::new())),
            fulltext_index: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(RepositoryStats {
                total_types: 0,
                platform_types: 0,
                configuration_types: 0,
                user_defined_types: 0,
                total_methods: 0,
                total_properties: 0,
                memory_usage_mb: 0.0,
                last_updated: Some(std::time::SystemTime::now()),
            })),
        }
    }
    
    /// Построить индексы после загрузки данных
    async fn rebuild_indexes(&self) -> Result<()> {
        let types = self.types_by_name.read().await;
        
        // Индекс по категориям
        {
            let mut category_index = self.types_by_category.write().await;
            category_index.clear();
            
            for (name, type_data) in types.iter() {
                for category in &type_data.category_path {
                    category_index.entry(category.clone())
                        .or_insert_with(Vec::new)
                        .push(name.clone());
                }
            }
        }
        
        // Индекс по источникам
        {
            let mut source_index = self.types_by_source.write().await;
            source_index.clear();
            
            for (name, type_data) in types.iter() {
                source_index.entry(type_data.source.clone())
                    .or_insert_with(Vec::new)
                    .push(name.clone());
            }
        }
        
        // Полнотекстовый индекс
        {
            let mut fulltext = self.fulltext_index.write().await;
            fulltext.clear();
            
            for (name, type_data) in types.iter() {
                // Индексируем по словам в названии и документации
                let searchable_text = format!("{} {} {}", 
                    type_data.russian_name, 
                    type_data.english_name, 
                    type_data.documentation
                );
                
                for word in searchable_text.split_whitespace() {
                    let word_lower = word.to_lowercase();
                    fulltext.entry(word_lower)
                        .or_insert_with(Vec::new)
                        .push(name.clone());
                }
            }
        }
        
        // Обновляем статистику
        self.update_stats().await?;
        
        Ok(())
    }
    
    /// Обновить статистику репозитория
    async fn update_stats(&self) -> Result<()> {
        let types = self.types_by_name.read().await;
        let mut stats = self.stats.write().await;
        
        stats.total_types = types.len();
        stats.total_methods = types.values().map(|t| t.methods.len()).sum();
        stats.total_properties = types.values().map(|t| t.properties.len()).sum();
        
        // Подсчёт по источникам
        stats.platform_types = types.values()
            .filter(|t| matches!(t.source, TypeSource::Platform { .. }))
            .count();
        stats.configuration_types = types.values()
            .filter(|t| matches!(t.source, TypeSource::Configuration { .. }))
            .count();
        stats.user_defined_types = types.values()
            .filter(|t| matches!(t.source, TypeSource::UserDefined { .. }))
            .count();
            
        stats.last_updated = Some(std::time::SystemTime::now());
        
        // Приблизительная оценка памяти
        stats.memory_usage_mb = (types.len() * 1024) as f64 / (1024.0 * 1024.0);
        
        Ok(())
    }
}

#[async_trait]
impl TypeRepository for InMemoryTypeRepository {
    async fn save_types(&self, types: Vec<RawTypeData>) -> Result<()> {
        println!("💾 Сохранение {} типов в репозиторий...", types.len());
        
        {
            let mut types_map = self.types_by_name.write().await;
            
            for type_data in types {
                types_map.insert(type_data.russian_name.clone(), type_data.clone());
                
                // Также добавляем по английскому имени если есть
                if !type_data.english_name.is_empty() && 
                   type_data.english_name != type_data.russian_name {
                    types_map.insert(type_data.english_name.clone(), type_data);
                }
            }
        }
        
        // Перестраиваем индексы
        self.rebuild_indexes().await?;
        
        println!("✅ Типы сохранены, индексы обновлены");
        Ok(())
    }
    
    async fn load_all_types(&self) -> Result<Vec<RawTypeData>> {
        let types = self.types_by_name.read().await;
        Ok(types.values().cloned().collect())
    }
    
    async fn load_types_filtered(&self, filter: &TypeFilter) -> Result<Vec<RawTypeData>> {
        let types = self.types_by_name.read().await;
        
        let filtered: Vec<RawTypeData> = types.values()
            .filter(|type_data| {
                // Фильтр по источнику
                if let Some(ref source_filter) = filter.source {
                    if std::mem::discriminant(&type_data.source) != std::mem::discriminant(source_filter) {
                        return false;
                    }
                }
                
                // Фильтр по категории
                if let Some(ref category_filter) = filter.category {
                    if !type_data.category_path.iter().any(|cat| cat == category_filter) {
                        return false;
                    }
                }
                
                // Фильтр по паттерну имени
                if let Some(ref pattern) = filter.name_pattern {
                    if !type_data.russian_name.to_lowercase().contains(&pattern.to_lowercase()) &&
                       !type_data.english_name.to_lowercase().contains(&pattern.to_lowercase()) {
                        return false;
                    }
                }
                
                // Фильтр по наличию методов
                if let Some(has_methods) = filter.has_methods {
                    if (type_data.methods.is_empty()) == has_methods {
                        return false;
                    }
                }
                
                // Фильтр по фасетам
                if !filter.facets.is_empty() {
                    if !filter.facets.iter().any(|facet| type_data.available_facets.contains(facet)) {
                        return false;
                    }
                }
                
                true
            })
            .cloned()
            .collect();
            
        Ok(filtered)
    }
    
    async fn get_type_by_name(&self, name: &str) -> Result<Option<RawTypeData>> {
        let types = self.types_by_name.read().await;
        Ok(types.get(name).cloned())
    }
    
    async fn get_types_by_category(&self, category: &str) -> Result<Vec<RawTypeData>> {
        let category_index = self.types_by_category.read().await;
        let types_map = self.types_by_name.read().await;
        
        if let Some(type_names) = category_index.get(category) {
            let types = type_names.iter()
                .filter_map(|name| types_map.get(name).cloned())
                .collect();
            Ok(types)
        } else {
            Ok(Vec::new())
        }
    }
    
    async fn search_types(&self, pattern: &str) -> Result<Vec<RawTypeData>> {
        let pattern_lower = pattern.to_lowercase();
        let fulltext_index = self.fulltext_index.read().await;
        let types_map = self.types_by_name.read().await;
        
        let mut found_names = std::collections::HashSet::new();
        
        // Поиск в полнотекстовом индексе
        for (word, type_names) in fulltext_index.iter() {
            if word.contains(&pattern_lower) {
                for type_name in type_names {
                    found_names.insert(type_name.clone());
                }
            }
        }
        
        // Получаем полные данные типов
        let types = found_names.iter()
            .filter_map(|name| types_map.get(name).cloned())
            .collect();
            
        Ok(types)
    }
    
    async fn get_stats(&self) -> Result<RepositoryStats> {
        Ok(self.stats.read().await.clone())
    }
    
    async fn clear(&self) -> Result<()> {
        {
            let mut types = self.types_by_name.write().await;
            types.clear();
        }
        
        {
            let mut category_index = self.types_by_category.write().await;
            category_index.clear();
        }
        
        {
            let mut source_index = self.types_by_source.write().await;
            source_index.clear();
        }
        
        {
            let mut fulltext = self.fulltext_index.write().await;
            fulltext.clear();
        }
        
        self.update_stats().await?;
        
        println!("🗑️ Репозиторий типов очищен");
        Ok(())
    }
}

impl Default for InMemoryTypeRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_repository_basic_operations() {
        let repo = InMemoryTypeRepository::new();
        
        // Создаём тестовые данные
        let test_type = RawTypeData {
            id: "test_type".to_string(),
            russian_name: "ТестовыйТип".to_string(),
            english_name: "TestType".to_string(),
            source: TypeSource::Platform { version: "8.3".to_string() },
            category_path: vec!["Платформа".to_string(), "Коллекции".to_string()],
            methods: vec![],
            properties: vec![],
            documentation: "Тестовый тип для проверки".to_string(),
            examples: vec![],
            available_facets: vec![FacetKind::Object],
            parse_metadata: ParseMetadata {
                source_file: None,
                parse_time: Some(std::time::SystemTime::now()),
                parser_version: "test".to_string(),
                quality_score: 1.0,
            },
        };
        
        // Тестируем сохранение
        repo.save_types(vec![test_type.clone()]).await.unwrap();
        
        // Тестируем загрузку
        let loaded = repo.get_type_by_name("ТестовыйТип").await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().russian_name, "ТестовыйТип");
        
        // Тестируем статистику
        let stats = repo.get_stats().await.unwrap();
        assert_eq!(stats.total_types, 1);
        assert_eq!(stats.platform_types, 1);
    }
    
    #[tokio::test]
    async fn test_repository_search() {
        let repo = InMemoryTypeRepository::new();
        
        let test_types = vec![
            RawTypeData {
                id: "array".to_string(),
                russian_name: "Массив".to_string(),
                english_name: "Array".to_string(),
                source: TypeSource::Platform { version: "8.3".to_string() },
                category_path: vec!["Коллекции".to_string()],
                documentation: "Массив значений".to_string(),
                methods: vec![],
                properties: vec![],
                examples: vec![],
                available_facets: vec![FacetKind::Object],
                parse_metadata: ParseMetadata {
                    source_file: None,
                    parse_time: Some(std::time::SystemTime::now()),
                    parser_version: "test".to_string(),
                    quality_score: 1.0,
                },
            }
        ];
        
        repo.save_types(test_types).await.unwrap();
        
        // Тестируем поиск
        let search_results = repo.search_types("массив").await.unwrap();
        assert_eq!(search_results.len(), 1);
        assert_eq!(search_results[0].russian_name, "Массив");
        
        // Тестируем поиск по категории
        let category_results = repo.get_types_by_category("Коллекции").await.unwrap();
        assert_eq!(category_results.len(), 1);
    }
}