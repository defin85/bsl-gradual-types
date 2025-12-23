//! TypeMetadataLookup - мост между TypeResolution и RawTypeData
//!
//! Этот модуль предоставляет сервис для получения полной документации типа
//! на основе результата статического анализа (TypeResolution).
//!
//! # Структура модуля
//!
//! - `core` - основные методы получения данных (get_raw_type, get_methods, get_properties)
//! - `facets` - логика работы с фасетами типов (Manager, Object, Reference, etc.)
//! - `generic` - обработка Generic типов с подстановкой типовых параметров
//! - `search` - API поиска и валидации метаданных (Milestone 3.16)

mod core;
mod facets;
mod generic;
mod search;

#[cfg(test)]
mod tests;

use crate::domain::repository::TypeRepository;
use std::sync::Arc;

// Re-export key types
pub use crate::domain::types::{
    RawMethodData, RawPropertyData, RawTabularSectionData, RawTypeData,
};

/// Сервис для получения метаданных типа по TypeResolution
///
/// # Назначение
///
/// TypeMetadataLookup является мостом между двумя концепциями:
/// - **TypeResolution** - результат статического анализа (что мы вывели о типе)
/// - **RawTypeData** - полная документация типа (что мы знаем из справки)
///
/// # Разделение ответственностей
///
/// - TypeResolution содержит только результат анализа (легковесный Value Object)
/// - RawTypeData содержит полную документацию (Single Source of Truth в Repository)
/// - TypeMetadataLookup предоставляет явный способ получить документацию по результату анализа
///
/// # Примеры использования
///
/// ```ignore
/// use bsl_gradual_types::domain::metadata_lookup::TypeMetadataLookup;
/// use bsl_gradual_types::domain::repository::TypeRepository;
/// use std::sync::Arc;
///
/// // Получить методы для TypeResolution
/// let lookup = TypeMetadataLookup::new(repository);
/// let resolution = resolver.resolve_expression_sync("Массив");
/// let methods = lookup.get_methods(&resolution);
///
/// // Проверить существование метода
/// if !lookup.has_member(&resolution, "НеСуществующийМетод") {
///     println!("Метод не найден!");
/// }
///
/// // Получить полную RawTypeData
/// if let Some(raw_type) = lookup.get_raw_type(&resolution) {
///     println!("Описание: {}", raw_type.description);
/// }
/// ```
#[derive(Clone)]
pub struct TypeMetadataLookup {
    repository: Arc<dyn TypeRepository>,
}

impl TypeMetadataLookup {
    /// Создать новый экземпляр TypeMetadataLookup
    ///
    /// # Параметры
    ///
    /// * `repository` - хранилище типов с RawTypeData
    pub fn new(repository: Arc<dyn TypeRepository>) -> Self {
        Self { repository }
    }

    /// Получить ссылку на репозиторий
    #[allow(dead_code)]
    pub(crate) fn repository(&self) -> &Arc<dyn TypeRepository> {
        &self.repository
    }

    /// Проверить, загружена ли документация платформы (Syntax Helper)
    pub fn platform_docs_loaded(&self) -> bool {
        self.repository.platform_docs_loaded()
    }
}
