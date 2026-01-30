//! Извлечение заголовков из HTML документов синтакс-помощника

use scraper::{Html, Selector};

/// Экстрактор заголовков из HTML
pub struct TitleExtractor;

impl TitleExtractor {
    /// Извлекает заголовок документа
    pub fn extract_title(document: &Html) -> String {
        Self::extract_element_text(document, "h1.V8SH_pagetitle")
            .or_else(|| Self::extract_element_text(document, "h1"))
            .unwrap_or_default()
    }

    /// Парсит заголовок в формате "Русское (English)"
    pub fn parse_title(title: &str) -> (String, String) {
        if let Some(open) = title.find('(') {
            if let Some(close) = title.find(')') {
                let russian = title[..open].trim().to_string();
                let english = title[open + 1..close].trim().to_string();
                return (russian, english);
            }
        }
        (title.trim().to_string(), String::new())
    }

    /// Извлекает текст элемента по CSS селектору
    pub fn extract_element_text(document: &Html, selector_str: &str) -> Option<String> {
        Selector::parse(selector_str).ok().and_then(|selector| {
            document
                .select(&selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
        })
    }
}
