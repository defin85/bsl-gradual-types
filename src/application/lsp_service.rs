//! LSP Type Service - оптимизированный для скорости сервис для LSP

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

// Временно используем domain пока не завершим миграцию
use crate::domain::analysis::type_checker::TypeContext;
use crate::domain::resolution_service::TypeResolver;
use crate::domain::resolvers::platform::{CompletionItem, CompletionKind};
use crate::domain::types::{FacetKind, TypeResolution};

// TODO: Добавить когда будут реализованы
// TypeCheckerService, TypeSearchResult, TypeSourceStub, RawTypeDataForResult

/// Сервис типов для LSP (оптимизирован для скорости)
pub struct LspTypeService {
    /// Центральный сервис разрешения - TODO: restore after migration
    // resolution_service: Arc<dyn TypeResolver>,

    /// LSP-специфичный кеш (быстрые операции)
    lsp_cache: Arc<RwLock<LspCache>>,
}

/// LSP кеш для быстрых операций
#[derive(Debug)]
struct LspCache {
    /// Кеш автодополнений
    completions: HashMap<String, Vec<CompletionItem>>,

    /// Кеш типов
    types: HashMap<String, TypeResolution>,
}

impl Default for LspCache {
    fn default() -> Self {
        Self::new()
    }
}

impl LspCache {
    pub fn new() -> Self {
        Self {
            completions: HashMap::new(),
            types: HashMap::new(),
        }
    }
}

impl LspTypeService {
    /// Создать новый LSP сервис
    pub fn new() -> Self {
        Self {
            lsp_cache: Arc::new(RwLock::new(LspCache::new())),
        }
    }

    /// Получить автодополнения для позиции (оптимизировано для LSP)
    pub async fn get_completions(&self, position: &str) -> Result<Vec<CompletionItem>> {
        // Проверяем кеш
        {
            let cache = self.lsp_cache.read().await;
            if let Some(cached) = cache.completions.get(position) {
                return Ok(cached.clone());
            }
        }

        // Получаем из основного сервиса
        let completions = self.resolution_service.get_completions(position).await?;

        // Кешируем
        {
            let mut cache = self.lsp_cache.write().await;
            cache
                .completions
                .insert(position.to_string(), completions.clone());
        }

        Ok(completions)
    }

    /// Разрешить тип для выражения (быстрая версия для LSP)
    pub async fn resolve_type_fast(&self, expression: &str) -> Result<Option<TypeResolution>> {
        // Проверяем кеш
        {
            let cache = self.lsp_cache.read().await;
            if let Some(cached) = cache.types.get(expression) {
                return Ok(Some(cached.clone()));
            }
        }

        // Получаем из основного сервиса
        if let Some(resolution) = self
            .resolution_service
            .resolve_expression(expression)
            .await?
        {
            // Кешируем
            {
                let mut cache = self.lsp_cache.write().await;
                cache
                    .types
                    .insert(expression.to_string(), resolution.clone());
            }
            Ok(Some(resolution))
        } else {
            Ok(None)
        }
    }

    /// Очистить кеш
    pub async fn clear_cache(&self) {
        let mut cache = self.lsp_cache.write().await;
        cache.completions.clear();
        cache.types.clear();
    }
}
