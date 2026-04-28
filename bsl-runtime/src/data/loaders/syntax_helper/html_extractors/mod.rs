//! HTML extraction модули для парсера синтакс-помощника 1С
//!
//! Модуль разбит на специализированные экстракторы:
//! - `title_extractor` - извлечение заголовков
//! - `parameter_extractor` - извлечение параметров методов
//! - `method_extractor` - извлечение методов, свойств, enum значений
//! - `description_extractor` - извлечение описаний и метаданных
//! - `property_detector` - детектор свойств и фасетов типов

mod description_extractor;
mod keyword_extractor;
mod method_extractor;
mod parameter_extractor;
mod property_detector;
mod title_extractor;

#[cfg(test)]
mod tests;

pub use description_extractor::DescriptionExtractor;
pub use keyword_extractor::KeywordExtractor;
pub use method_extractor::MethodExtractor;
pub use parameter_extractor::ParameterExtractor;
pub use property_detector::PropertyDetector;
pub use title_extractor::TitleExtractor;

use scraper::Html;
use std::path::Path;

use super::types::MethodOverloadInfo;
use super::types::{CodeExample, ParameterInfo};
use bsl_shared::domain::types::FacetKind;

/// Фасад для всех HTML экстракторов
///
/// Обеспечивает обратную совместимость с существующим кодом.
/// Все методы делегируют вызовы в соответствующие специализированные экстракторы.
pub struct HtmlExtractor;

impl HtmlExtractor {
    pub fn new() -> Self {
        Self
    }

    // =========================================================================
    // TitleExtractor делегаты
    // =========================================================================

    pub fn extract_title(&self, document: &Html) -> String {
        TitleExtractor::extract_title(document)
    }

    pub fn parse_title(&self, title: &str) -> (String, String) {
        TitleExtractor::parse_title(title)
    }

    pub fn extract_element_text(&self, document: &Html, selector_str: &str) -> Option<String> {
        TitleExtractor::extract_element_text(document, selector_str)
    }

    // =========================================================================
    // DescriptionExtractor делегаты
    // =========================================================================

    pub fn extract_description(&self, document: &Html) -> String {
        DescriptionExtractor::extract_description(document)
    }

    pub fn extract_examples(&self, document: &Html) -> Vec<CodeExample> {
        DescriptionExtractor::extract_examples(document)
    }

    pub fn extract_availability(&self, document: &Html) -> Vec<String> {
        DescriptionExtractor::extract_availability(document)
    }

    pub fn extract_version(&self, document: &Html) -> String {
        DescriptionExtractor::extract_version(document)
    }

    pub fn extract_english_name(&self, document: &Html) -> Option<String> {
        DescriptionExtractor::extract_english_name(document)
    }

    pub fn extract_return_info(&self, document: &Html) -> (Option<String>, Option<String>) {
        DescriptionExtractor::extract_return_info(document)
    }

    pub fn extract_return_type(&self, document: &Html) -> String {
        DescriptionExtractor::extract_return_type(document)
    }

    pub fn extract_property_type(&self, document: &Html) -> Option<String> {
        DescriptionExtractor::extract_property_type(document)
    }

    pub fn extract_property_description(&self, document: &Html) -> Option<String> {
        DescriptionExtractor::extract_property_description(document)
    }

    pub fn extract_metadata_collection_item_type(&self, document: &Html) -> Option<String> {
        DescriptionExtractor::extract_metadata_collection_item_type(document)
    }

    pub fn extract_links(&self, document: &Html) -> Vec<String> {
        DescriptionExtractor::extract_links(document)
    }

    pub fn extract_type_list(&self, document: &Html) -> Vec<String> {
        DescriptionExtractor::extract_type_list(document)
    }

    // =========================================================================
    // ParameterExtractor делегаты
    // =========================================================================

    pub fn extract_parameters(&self, document: &Html) -> Vec<ParameterInfo> {
        ParameterExtractor::extract_parameters(document)
    }

    /// Извлечь варианты синтаксиса (overloads) для метода.
    ///
    /// Для методов без "Вариант синтаксиса" вернёт 0 или 1 вариант в зависимости от наличия секции параметров.
    pub fn extract_method_overloads(&self, document: &Html) -> Vec<MethodOverloadInfo> {
        ParameterExtractor::extract_method_overloads(document)
    }

    // =========================================================================
    // MethodExtractor делегаты
    // =========================================================================

    pub fn extract_methods_from_html(&self, document: &Html) -> Vec<(String, String)> {
        MethodExtractor::extract_methods_from_html(document)
    }

    pub fn extract_properties_from_html(&self, document: &Html) -> Vec<(String, String)> {
        MethodExtractor::extract_properties_from_html(document)
    }

    pub fn extract_enum_values_from_html(&self, document: &Html) -> Vec<String> {
        MethodExtractor::extract_enum_values_from_html(document)
    }

    // =========================================================================
    // KeywordExtractor делегаты
    // =========================================================================

    pub fn extract_keywords(&self, document: &Html) -> Vec<String> {
        KeywordExtractor::extract_keywords(document)
    }

    // =========================================================================
    // PropertyDetector делегаты
    // =========================================================================

    pub fn is_readonly(&self, document: &Html) -> bool {
        PropertyDetector::is_readonly(document)
    }

    pub fn is_iterable(&self, description: &str) -> bool {
        PropertyDetector::is_iterable(description)
    }

    pub fn is_indexable(&self, description: &str) -> bool {
        PropertyDetector::is_indexable(description)
    }

    pub fn is_serializable(&self, document: &Html) -> bool {
        PropertyDetector::is_serializable(document)
    }

    pub fn is_exchangeable(&self, document: &Html) -> bool {
        PropertyDetector::is_exchangeable(document)
    }

    pub fn detect_facets(&self, type_name: &str, description: &str) -> Vec<FacetKind> {
        PropertyDetector::detect_facets(type_name, description)
    }

    pub fn extract_aliases(&self, document: &Html) -> Vec<String> {
        PropertyDetector::extract_aliases(document)
    }

    pub fn extract_collection_element(&self, document: &Html) -> Option<String> {
        PropertyDetector::extract_collection_element(document)
    }

    pub fn extract_category_path(&self, path: &Path) -> String {
        PropertyDetector::extract_category_path(path)
    }

    pub fn build_path(&self, path: &Path) -> String {
        PropertyDetector::build_path(path)
    }
}

impl Default for HtmlExtractor {
    fn default() -> Self {
        Self::new()
    }
}
