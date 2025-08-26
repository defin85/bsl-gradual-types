//! Провайдер документации конфигурационных типов

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::core::hierarchy::{
    CategoryStatistics, DocumentationNode, DocumentationSourceType, RootCategoryNode,
    TypeDocumentationFull, UiMetadata,
};
use super::core::providers::{DocumentationProvider, ProviderConfig};
use super::core::statistics::{InitializationStatus, ProviderStatistics};
use super::search::AdvancedSearchQuery;
use crate::data::loaders::config_parser_quick_xml::ConfigurationQuickXmlParser;
use crate::data::loaders::config_parser_xml::ConfigParserXml;
use crate::domain::types::{MetadataKind, TypeResolution};

/// Провайдер документации конфигурационных типов
pub struct ConfigurationDocumentationProvider {
    /// Парсер конфигурации XML (старый)
    config_parser: Arc<RwLock<Option<ConfigParserXml>>>,

    /// Улучшенный парсер с quick-xml
    quick_parser: Arc<RwLock<Option<ConfigurationQuickXmlParser>>>,

    /// Статус инициализации
    initialization_status: Arc<RwLock<InitializationStatus>>,

    /// Кеш конфигурационных типов
    configuration_cache: Arc<RwLock<std::collections::HashMap<String, TypeDocumentationFull>>>,

    /// Корневая категория конфигурации
    root_category_cache: Arc<RwLock<Option<RootCategoryNode>>>,
}

impl ConfigurationDocumentationProvider {
    /// Создать новый провайдер
    pub fn new() -> Self {
        Self {
            config_parser: Arc::new(RwLock::new(None)),
            quick_parser: Arc::new(RwLock::new(None)),
            initialization_status: Arc::new(RwLock::new(InitializationStatus::default())),
            configuration_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
            root_category_cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Анализ конфигурации и построение документации
    async fn analyze_configuration(&self, config_path: &str) -> Result<()> {
        println!("📁 Анализ конфигурации: {}", config_path);

        // Создаём улучшенный парсер
        let mut quick_parser = ConfigurationQuickXmlParser::new(config_path);

        match quick_parser.parse_configuration() {
            Ok(parsed_config) => {
                println!(
                    "✅ Quick XML парсер обработал {} типов",
                    parsed_config.len()
                );

                // Сохраняем парсер
                *self.quick_parser.write().await = Some(quick_parser);

                // Строим документацию из новых TypeResolution
                self.build_configuration_documentation(&parsed_config)
                    .await?;
            }
            Err(e) => {
                println!("⚠️ Ошибка quick XML парсинга: {}", e);

                // Fallback на старый парсер
                println!("🔄 Fallback на старый XML парсер...");
                let mut old_parser = ConfigParserXml::new(config_path);

                match old_parser.parse_configuration() {
                    Ok(parsed_config) => {
                        println!("✅ Старый парсер обработал {} типов", parsed_config.len());
                        *self.config_parser.write().await = Some(old_parser);
                        self.build_configuration_documentation(&parsed_config)
                            .await?;
                    }
                    Err(e2) => {
                        println!("❌ Оба парсера failed: quick={}, old={}", e, e2);
                    }
                }
            }
        }

        Ok(())
    }

    /// Построить документацию конфигурационных объектов
    async fn build_configuration_documentation(
        &self,
        config_types: &[TypeResolution],
    ) -> Result<()> {
        use crate::domain::types::{ConcreteType, ResolutionResult};

        let mut cache = self.configuration_cache.write().await;

        // Получаем доступ к парсеру для извлечения реальных имен
        let quick_parser = self.quick_parser.read().await;

        for type_resolution in config_types {
            if let ResolutionResult::Concrete(ConcreteType::Configuration(config_type)) =
                &type_resolution.result
            {
                // Получаем реальные метаданные из парсера
                let qualified_name = format!(
                    "{}.{}",
                    self.get_kind_prefix(&config_type.kind),
                    &config_type.name
                );
                let metadata = quick_parser
                    .as_ref()
                    .and_then(|parser| parser.get_metadata(&qualified_name));

                let real_name = metadata
                    .map(|m| m.name.clone())
                    .unwrap_or_else(|| config_type.name.clone());
                let synonym = metadata.and_then(|m| m.synonym.clone());
                let attributes_count = metadata.map(|m| m.attributes.len()).unwrap_or(0);
                let ts_count = metadata.map(|m| m.tabular_sections.len()).unwrap_or(0);

                let type_doc = TypeDocumentationFull {
                    id: format!(
                        "config_{}_{}",
                        self.get_kind_prefix(&config_type.kind),
                        real_name
                    ),
                    russian_name: real_name.clone(),
                    english_name: real_name.clone(),
                    aliases: if let Some(ref syn) = synonym {
                        vec![syn.clone()]
                    } else {
                        Vec::new()
                    },
                    source_type: DocumentationSourceType::UserDefined {
                        module_path: format!("{}.xml", real_name),
                    },
                    hierarchy_path: vec![
                        "Конфигурация".to_string(),
                        self.get_kind_prefix(&config_type.kind).to_string(),
                        real_name.clone(),
                    ],
                    type_resolution: type_resolution.clone(),
                    available_facets: type_resolution.available_facets.clone(),
                    active_facet: type_resolution.active_facet,
                    methods: Vec::new(),    // TODO: добавить реальные методы
                    properties: Vec::new(), // TODO: добавить реальные свойства из атрибутов
                    constructors: Vec::new(),
                    description: format!(
                        "{} {} {}",
                        self.get_kind_display_name(&config_type.kind),
                        real_name,
                        if let Some(ref syn) = synonym {
                            format!("({})", syn)
                        } else {
                            String::new()
                        }
                    ),
                    examples: Vec::new(),
                    availability: Vec::new(),
                    since_version: "8.0".to_string(),
                    notes: vec![
                        format!("Атрибутов: {}", attributes_count),
                        format!("Табличных частей: {}", ts_count),
                    ],
                    related_types: Vec::new(),
                    parent_type: None,
                    child_types: Vec::new(),
                    source_file: Some(format!("{}.xml", real_name)),
                    ui_metadata: UiMetadata {
                        icon: self.get_icon_for_kind(&config_type.kind),
                        color: self.get_color_for_kind(&config_type.kind),
                        tree_path: vec![
                            "Конфигурация".to_string(),
                            self.get_kind_prefix(&config_type.kind).to_string(),
                            real_name.clone(),
                        ],
                        expanded: false,
                        sort_weight: 0,
                        css_classes: vec![
                            "config-type".to_string(),
                            format!("{:?}-type", config_type.kind),
                        ],
                    },
                };

                cache.insert(type_doc.id.clone(), type_doc);
            }
        }

        println!(
            "📊 Построена документация для {} конфигурационных типов",
            cache.len()
        );
        Ok(())
    }

    /// Получить отображаемое название типа
    fn get_kind_display_name(&self, kind: &MetadataKind) -> &str {
        match kind {
            MetadataKind::Catalog => "Справочник",
            MetadataKind::Document => "Документ",
            MetadataKind::Register => "Регистр сведений",
            MetadataKind::Enum => "Перечисление",
            _ => "Объект конфигурации",
        }
    }

    /// Получить префикс для типа
    fn get_kind_prefix(&self, kind: &MetadataKind) -> &str {
        match kind {
            MetadataKind::Catalog => "Справочники",
            MetadataKind::Document => "Документы",
            MetadataKind::Register => "РегистрыСведений",
            MetadataKind::Enum => "Перечисления",
            _ => "ПрочиеОбъекты",
        }
    }

    /// Получить иконку для типа
    fn get_icon_for_kind(&self, kind: &MetadataKind) -> String {
        match kind {
            MetadataKind::Catalog => "📁".to_string(),
            MetadataKind::Document => "📄".to_string(),
            MetadataKind::Register => "📊".to_string(),
            MetadataKind::Enum => "📋".to_string(),
            _ => "⚙️".to_string(),
        }
    }

    /// Получить цвет для типа
    fn get_color_for_kind(&self, kind: &MetadataKind) -> String {
        match kind {
            MetadataKind::Catalog => "#4CAF50".to_string(),
            MetadataKind::Document => "#2196F3".to_string(),
            MetadataKind::Register => "#FF9800".to_string(),
            MetadataKind::Enum => "#9C27B0".to_string(),
            _ => "#9E9E9E".to_string(),
        }
    }

    /// Построить корневую категорию конфигурации
    async fn build_configuration_root_category(&self) -> Result<RootCategoryNode> {
        Ok(RootCategoryNode {
            id: "configuration_root".to_string(),
            name: "Конфигурация".to_string(),
            description: "Объекты конфигурации (справочники, документы, регистры)".to_string(),
            children: Vec::new(),
            ui_metadata: UiMetadata {
                icon: "⚙️".to_string(),
                color: "#FF9800".to_string(),
                tree_path: vec!["Конфигурация".to_string()],
                expanded: false,
                sort_weight: 200,
                css_classes: vec![
                    "root-category".to_string(),
                    "configuration-root".to_string(),
                ],
            },
            statistics: CategoryStatistics {
                child_types_count: self.configuration_cache.read().await.len(),
                total_methods_count: 0,
                total_properties_count: 0,
                most_popular_type: None,
            },
        })
    }
}

#[async_trait]
impl DocumentationProvider for ConfigurationDocumentationProvider {
    fn provider_id(&self) -> &str {
        "configuration_types"
    }

    fn display_name(&self) -> &str {
        "Конфигурационные типы"
    }

    async fn initialize(&self, config: &ProviderConfig) -> Result<()> {
        if !config.data_source.is_empty() && std::path::Path::new(&config.data_source).exists() {
            self.analyze_configuration(&config.data_source).await?;
        } else {
            println!("⚠️ Конфигурация не найдена: {}", config.data_source);
        }
        Ok(())
    }

    async fn get_root_category(&self) -> Result<RootCategoryNode> {
        let cache = self.root_category_cache.read().await;
        match cache.as_ref() {
            Some(category) => Ok(category.clone()),
            None => {
                drop(cache);
                let category = self.build_configuration_root_category().await?;
                *self.root_category_cache.write().await = Some(category.clone());
                Ok(category)
            }
        }
    }

    async fn get_type_details(&self, type_id: &str) -> Result<Option<TypeDocumentationFull>> {
        let cache = self.configuration_cache.read().await;
        Ok(cache.get(type_id).cloned())
    }

    async fn search_types(&self, _query: &AdvancedSearchQuery) -> Result<Vec<DocumentationNode>> {
        Ok(Vec::new())
    }

    async fn get_all_types(&self) -> Result<Vec<TypeDocumentationFull>> {
        let cache = self.configuration_cache.read().await;
        Ok(cache.values().cloned().collect())
    }

    async fn get_statistics(&self) -> Result<ProviderStatistics> {
        let cache = self.configuration_cache.read().await;

        Ok(ProviderStatistics {
            total_types: cache.len(),
            total_methods: cache.values().map(|t| t.methods.len()).sum(),
            total_properties: cache.values().map(|t| t.properties.len()).sum(),
            last_load_time_ms: 0,
            memory_usage_mb: (cache.len() * 512) as f64 / (1024.0 * 1024.0),
        })
    }

    async fn get_initialization_status(&self) -> Result<InitializationStatus> {
        Ok(self.initialization_status.read().await.clone())
    }

    async fn check_availability(&self) -> Result<bool> {
        Ok(self.config_parser.read().await.is_some())
    }

    async fn refresh(&self) -> Result<()> {
        self.configuration_cache.write().await.clear();
        *self.root_category_cache.write().await = None;
        Ok(())
    }
}
