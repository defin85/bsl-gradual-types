//! Application Layer: Type Inference Service
//!
//! Высокоуровневая бизнес-логика для разрешения типов и автодополнения.
//! Использует чистый Domain TypeResolver для основной логики типизации.

use bsl_shared::domain::repository::{CompletionItem, CompletionKind, TypeRepository};
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::types::{
    Attribute, ConcreteType, ConfigurationType, MetadataKind, RawAttributeData, RawDataSource,
    RawTabularSectionData, ResolutionResult, TabularSection, TypeResolution,
};
use std::sync::Arc;

/// Сервис вывода типов - Application Layer оркестрация
pub struct TypeInferenceService {
    resolver: Arc<TypeResolver>,
    repository: Arc<dyn TypeRepository>,
}

impl TypeInferenceService {
    pub fn new(resolver: Arc<TypeResolver>, repository: Arc<dyn TypeRepository>) -> Self {
        Self {
            resolver,
            repository,
        }
    }

    /// Асинхронное разрешение выражений (Application оркестрация)
    pub async fn resolve_expression_async(&self, expression: &str) -> TypeResolution {
        // Делегируем в Domain resolver
        self.resolver.resolve_expression_sync(expression)
    }

    /// Получить автодополнения для запроса (Application логика)
    pub fn get_completions(&self, query: &str) -> Vec<CompletionItem> {
        let all_types = self.repository.get_all_types();
        let mut completions = Vec::new();

        for raw_type in all_types {
            if raw_type.name.to_lowercase().contains(&query.to_lowercase()) {
                // Создаём правильный тип на основе источника
                let concrete_type = match raw_type.source {
                    RawDataSource::Platform => {
                        ConcreteType::Platform(bsl_shared::domain::types::PlatformType {
                            name: raw_type.name.clone(),
                        })
                    }
                    RawDataSource::Configuration => {
                        let attributes = Self::convert_raw_attributes(&raw_type.attributes);
                        let tabular_sections =
                            Self::convert_raw_tabular_sections(&raw_type.tabular_sections);

                        ConcreteType::Configuration(ConfigurationType {
                            kind: raw_type.kind.unwrap_or(MetadataKind::Unknown),
                            name: raw_type.name.clone(),
                            facet: raw_type.facets.first().copied(),
                            attributes,
                            tabular_sections,
                        })
                    }
                    RawDataSource::UserDefined => {
                        ConcreteType::Platform(bsl_shared::domain::types::PlatformType {
                            name: raw_type.name.clone(),
                        })
                    }
                };

                let resolution = TypeResolution::known(concrete_type);

                let item = CompletionItem::with_details(
                    raw_type.name.clone(),
                    self.determine_completion_kind(&resolution),
                    Some(format!("{:?}", resolution.result)),
                    resolution.metadata.notes.first().cloned(),
                );
                completions.push(item);
            }
        }
        completions
    }

    /// Поиск типов по запросу (Application логика)
    pub fn search_types(&self, query: &str) -> Vec<String> {
        let completions = self.get_completions(query);
        completions.into_iter().map(|c| c.label).collect()
    }

    /// Получить все типы как HashMap (Application логика)
    /// Поддерживает Platform, Configuration и UserDefined источники
    pub fn get_all_platform_globals(&self) -> std::collections::HashMap<String, TypeResolution> {
        let raw_types = self.repository.get_all_types();
        let mut result = std::collections::HashMap::new();

        for raw_type in raw_types {
            let concrete_type = match raw_type.source {
                RawDataSource::Platform => {
                    ConcreteType::Platform(bsl_shared::domain::types::PlatformType {
                        name: raw_type.name.clone(),
                    })
                }
                RawDataSource::Configuration => {
                    // Конвертируем атрибуты и табличные части
                    let attributes = Self::convert_raw_attributes(&raw_type.attributes);
                    let tabular_sections =
                        Self::convert_raw_tabular_sections(&raw_type.tabular_sections);

                    ConcreteType::Configuration(ConfigurationType {
                        kind: raw_type.kind.unwrap_or(MetadataKind::Unknown),
                        name: raw_type.name.clone(),
                        facet: raw_type.facets.first().copied(),
                        attributes,
                        tabular_sections,
                    })
                }
                RawDataSource::UserDefined => {
                    // Пользовательские типы пока как Platform
                    ConcreteType::Platform(bsl_shared::domain::types::PlatformType {
                        name: raw_type.name.clone(),
                    })
                }
            };

            let mut resolution = TypeResolution::known(concrete_type);
            // Копируем фасеты из RawTypeData
            resolution.available_facets = raw_type.facets.clone();

            result.insert(raw_type.name, resolution);
        }

        result
    }

    /// Конвертация RawAttributeData -> Attribute
    fn convert_raw_attributes(raw_attrs: &[RawAttributeData]) -> Vec<Attribute> {
        raw_attrs
            .iter()
            .map(|ra| Attribute {
                name: ra.name.clone(),
                type_: ra.attr_type.clone(),
                is_composite: false,
                types: vec![ra.attr_type.clone()],
            })
            .collect()
    }

    /// Конвертация RawTabularSectionData -> TabularSection
    fn convert_raw_tabular_sections(raw_ts: &[RawTabularSectionData]) -> Vec<TabularSection> {
        raw_ts
            .iter()
            .map(|rts| TabularSection {
                name: rts.name.clone(),
                synonym: None,
                attributes: Self::convert_raw_attributes(&rts.attributes),
            })
            .collect()
    }

    /// Определить тип автодополнения на основе TypeResolution
    fn determine_completion_kind(&self, resolution: &TypeResolution) -> CompletionKind {
        use bsl_shared::domain::types::MetadataKind;

        match &resolution.result {
            ResolutionResult::Concrete(ConcreteType::Platform(_)) => CompletionKind::Global,
            ResolutionResult::Concrete(ConcreteType::Configuration(config)) => match config.kind {
                MetadataKind::Catalog => CompletionKind::Catalog,
                MetadataKind::Document => CompletionKind::Document,
                MetadataKind::Enum => CompletionKind::Enum,
                _ => CompletionKind::Global,
            },
            _ => CompletionKind::Global,
        }
    }
}
