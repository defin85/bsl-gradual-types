use super::stats::RepositoryStats;
use super::RawTypeData;
use crate::core::types::TypeResolution;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;

#[async_trait]
pub trait TypeRepository: Send + Sync {
    fn add_resolution(&self, resolution: TypeResolution);
    fn get_stats(&self) -> RepositoryStats;
    async fn clear(&self) -> Result<()>;
    async fn save_types(&self, types: Vec<RawTypeData>) -> Result<()>;
    async fn search_types(&self, query: &str) -> Result<Vec<RawTypeData>>;
    async fn load_all_types(&self) -> Result<Vec<RawTypeData>>;
    async fn load_types_filtered(
        &self,
        filter: &super::filters::TypeFilter,
    ) -> Result<Vec<RawTypeData>>;
}

pub struct InMemoryTypeRepository {
    resolutions_by_name: Mutex<HashMap<String, TypeResolution>>,
}

impl InMemoryTypeRepository {
    pub fn new() -> Self {
        Self {
            resolutions_by_name: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl TypeRepository for InMemoryTypeRepository {
    fn add_resolution(&self, resolution: TypeResolution) {
        if let Some(name) = resolution.get_name() {
            if let Ok(mut map) = self.resolutions_by_name.lock() {
                map.insert(name, resolution);
            }
        }
    }

    fn get_stats(&self) -> RepositoryStats {
        if let Ok(map) = self.resolutions_by_name.lock() {
            let mut platform = 0usize;
            let mut configuration = 0usize;
            let mut user_defined = 0usize;
            let total = map.len();

            for res in map.values() {
                let raw = res.to_raw_data();
                match raw.source {
                    super::TypeSource::Platform { .. } => platform += 1,
                    super::TypeSource::Configuration { .. } => configuration += 1,
                    super::TypeSource::UserDefined { .. } => user_defined += 1,
                }
            }

            RepositoryStats {
                total_types: total,
                platform_types: platform,
                configuration_types: configuration,
                user_defined_types: user_defined,
                types_count: total,
            }
        } else {
            RepositoryStats {
                total_types: 0,
                platform_types: 0,
                configuration_types: 0,
                user_defined_types: 0,
                types_count: 0,
            }
        }
    }

    async fn clear(&self) -> Result<()> {
        if let Ok(mut map) = self.resolutions_by_name.lock() {
            map.clear();
        }
        Ok(())
    }

    async fn save_types(&self, types: Vec<RawTypeData>) -> Result<()> {
        if let Ok(mut map) = self.resolutions_by_name.lock() {
            for raw_type in types {
                // Конвертируем RawTypeData в TypeResolution
                let resolution = TypeResolution::from_raw_data(&raw_type);
                if let Some(name) = resolution.get_name() {
                    map.insert(name, resolution);
                }
            }
        }
        Ok(())
    }

    async fn search_types(&self, query: &str) -> Result<Vec<RawTypeData>> {
        if let Ok(map) = self.resolutions_by_name.lock() {
            let filtered_types: Vec<RawTypeData> = map
                .values()
                .filter(|resolution| {
                    if let Some(name) = resolution.get_name() {
                        name.to_lowercase().contains(&query.to_lowercase())
                    } else {
                        false
                    }
                })
                .map(|resolution| resolution.to_raw_data())
                .collect();
            Ok(filtered_types)
        } else {
            Ok(Vec::new())
        }
    }

    async fn load_all_types(&self) -> Result<Vec<RawTypeData>> {
        if let Ok(map) = self.resolutions_by_name.lock() {
            let all: Vec<RawTypeData> = map.values().map(|r| r.to_raw_data()).collect();
            Ok(all)
        } else {
            Ok(Vec::new())
        }
    }

    async fn load_types_filtered(
        &self,
        _filter: &super::filters::TypeFilter,
    ) -> Result<Vec<RawTypeData>> {
        let filter = _filter;
        if let Ok(map) = self.resolutions_by_name.lock() {
            let mut out = Vec::new();
            for res in map.values() {
                let raw = res.to_raw_data();
                // Фильтрация по источнику
                if let Some(src) = &filter.source {
                    let matches_source = match (&raw.source, src) {
                        (
                            super::TypeSource::Platform { .. },
                            super::TypeSource::Platform { .. },
                        ) => true,
                        (
                            super::TypeSource::Configuration { .. },
                            super::TypeSource::Configuration { .. },
                        ) => true,
                        (
                            super::TypeSource::UserDefined { .. },
                            super::TypeSource::UserDefined { .. },
                        ) => true,
                        _ => false,
                    };
                    if !matches_source {
                        continue;
                    }
                }

                // Фильтр по категории (грубый по вхождению в путь категории)
                if let Some(cat) = &filter.category {
                    if !raw.category_path.iter().any(|c| c.contains(cat)) {
                        continue;
                    }
                }

                // Фильтр по имени (подстрока, регистр игнорируется)
                if let Some(name_substr) = &filter.name_contains {
                    let nn = name_substr.to_lowercase();
                    if !raw.russian_name.to_lowercase().contains(&nn)
                        && !raw.english_name.to_lowercase().contains(&nn)
                    {
                        continue;
                    }
                }

                // Фильтр по фасете (должна присутствовать среди доступных)
                if let Some(facet_kind) = filter.facet {
                    if !raw.available_facets.iter().any(|f| f.kind == facet_kind) {
                        continue;
                    }
                }

                // Фильтр по наличию методов/свойств
                if let Some(has_methods) = filter.has_methods {
                    if has_methods && raw.methods.is_empty() {
                        continue;
                    }
                    if !has_methods && !raw.methods.is_empty() {
                        continue;
                    }
                }
                if let Some(has_properties) = filter.has_properties {
                    if has_properties && raw.properties.is_empty() {
                        continue;
                    }
                    if !has_properties && !raw.properties.is_empty() {
                        continue;
                    }
                }

                out.push(raw);
            }
            Ok(out)
        } else {
            Ok(Vec::new())
        }
    }
}
