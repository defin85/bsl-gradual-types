//! Построение индексов для быстрого поиска типов

use dashmap::DashMap;
use rayon::prelude::*;
use std::sync::Arc;

use super::types::*;

/// Builder для построения индексов типов
pub struct IndexBuilder;

impl IndexBuilder {
    pub fn new() -> Self {
        Self
    }

    /// Строит индексы для типов (однопоточно)
    pub fn build_indexes(nodes: &Arc<DashMap<String, SyntaxNode>>) -> TypeIndex {
        let mut index = TypeIndex::default();

        for entry in nodes.iter() {
            let (path, node) = entry.pair();

            if let SyntaxNode::Type(type_info) = node {
                // Индекс по русскому имени
                index
                    .by_russian
                    .insert(type_info.identity.russian_name.clone(), path.clone());

                // Индекс по английскому имени
                if !type_info.identity.english_name.is_empty() {
                    index
                        .by_english
                        .insert(type_info.identity.english_name.clone(), path.clone());
                }

                // Индекс по фасетам
                for facet in &type_info.metadata.available_facets {
                    index.by_facet.entry(*facet).or_default().push(path.clone());
                }

                // Индекс по категориям
                if !type_info.identity.category_path.is_empty() {
                    index
                        .by_category
                        .entry(type_info.identity.category_path.clone())
                        .or_default()
                        .push(path.clone());
                }
            }
        }

        index
    }

    /// Параллельное построение индексов
    pub fn build_indexes_parallel(nodes: &Arc<DashMap<String, SyntaxNode>>) -> TypeIndex {
        // Создаём параллельные индексы
        let by_russian = Arc::new(DashMap::new());
        let by_english = Arc::new(DashMap::new());
        let by_facet = Arc::new(DashMap::new());
        let by_category = Arc::new(DashMap::new());

        // Параллельно обрабатываем все узлы
        nodes.iter().par_bridge().for_each(|entry| {
            let (path, node) = entry.pair();

            if let SyntaxNode::Type(type_info) = node {
                // Индекс по русскому имени
                by_russian.insert(type_info.identity.russian_name.clone(), path.clone());

                // Индекс по английскому имени
                if !type_info.identity.english_name.is_empty() {
                    by_english.insert(type_info.identity.english_name.clone(), path.clone());
                }

                // Индекс по фасетам
                for facet in &type_info.metadata.available_facets {
                    by_facet
                        .entry(*facet)
                        .or_insert_with(Vec::new)
                        .push(path.clone());
                }

                // Индекс по категориям
                if !type_info.identity.category_path.is_empty() {
                    by_category
                        .entry(type_info.identity.category_path.clone())
                        .or_insert_with(Vec::new)
                        .push(path.clone());
                }
            }
        });

        // Конвертируем в обычный индекс
        let mut index = TypeIndex::default();

        for entry in by_russian.iter() {
            index
                .by_russian
                .insert(entry.key().clone(), entry.value().clone());
        }

        for entry in by_english.iter() {
            index
                .by_english
                .insert(entry.key().clone(), entry.value().clone());
        }

        for entry in by_facet.iter() {
            index.by_facet.insert(*entry.key(), entry.value().clone());
        }

        for entry in by_category.iter() {
            index
                .by_category
                .insert(entry.key().clone(), entry.value().clone());
        }

        index
    }
}

impl Default for IndexBuilder {
    fn default() -> Self {
        Self::new()
    }
}
