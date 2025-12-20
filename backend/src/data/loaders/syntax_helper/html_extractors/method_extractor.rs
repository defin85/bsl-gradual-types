//! Извлечение методов, свойств и enum значений из HTML документов синтакс-помощника

use scraper::{ElementRef, Html, Selector};
use tracing::debug;

/// Экстрактор методов, свойств и enum значений
pub struct MethodExtractor;

impl MethodExtractor {
    /// Извлекает методы из HTML документа
    ///
    /// Возвращает: Vec<(русское_имя, английское_имя)>
    pub fn extract_methods_from_html(document: &Html) -> Vec<(String, String)> {
        Self::extract_section_links(document, "Методы")
    }

    /// Извлекает свойства из HTML документа
    ///
    /// Возвращает: Vec<(русское_имя, английское_имя)>
    pub fn extract_properties_from_html(document: &Html) -> Vec<(String, String)> {
        Self::extract_section_links(document, "Свойства")
    }

    /// Извлекает enum значения из HTML документа
    pub fn extract_enum_values_from_html(document: &Html) -> Vec<String> {
        let mut enum_values = Vec::new();

        // Ищем заголовок "Значения"
        if let Ok(chapter_selector) = Selector::parse("p.V8SH_chapter") {
            let mut found_values_section = false;
            let mut current_chapter: Option<ElementRef> = None;

            for chapter in document.select(&chapter_selector) {
                let chapter_text = chapter.text().collect::<String>().trim().to_string();

                if chapter_text == "Значения" || chapter_text.starts_with("Значения")
                {
                    found_values_section = true;
                    current_chapter = Some(chapter);
                    debug!("Found 'Значения' section in HTML");
                    break;
                }
            }

            if !found_values_section {
                debug!("Section 'Значения' not found in HTML");
                return enum_values;
            }

            // Извлекаем все <a> элементы после заголовка "Значения"
            if let Some(chapter_elem) = current_chapter {
                let mut collect = false;

                for element in document.root_element().descendants() {
                    if let Some(elem_ref) = ElementRef::wrap(element) {
                        if elem_ref == chapter_elem {
                            collect = true;
                            continue;
                        }

                        if collect {
                            if let Some(elem) = ElementRef::wrap(element) {
                                if elem.value().name() == "p" {
                                    if let Some(class) = elem.value().attr("class") {
                                        if class.contains("V8SH_chapter") {
                                            break;
                                        }
                                    }
                                }

                                if elem.value().name() == "a" {
                                    let link_text =
                                        elem.text().collect::<String>().trim().to_string();
                                    if !link_text.is_empty() {
                                        enum_values.push(link_text.clone());
                                        debug!("Extracted enum value: {}", link_text);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if enum_values.is_empty() {
            debug!("No enum values extracted from HTML");
        } else {
            debug!("Extracted {} enum values", enum_values.len());
        }

        enum_values
    }

    /// Общий метод для извлечения ссылок из раздела (Методы, Свойства, Конструкторы)
    ///
    /// Возвращает: Vec<(русское_имя, английское_имя)>
    fn extract_section_links(document: &Html, section_name: &str) -> Vec<(String, String)> {
        let mut items = Vec::new();

        if let Ok(chapter_selector) = Selector::parse("p.V8SH_chapter") {
            let mut found_section = false;
            let mut current_chapter: Option<ElementRef> = None;

            for chapter in document.select(&chapter_selector) {
                let chapter_text = chapter.text().collect::<String>().trim().to_string();

                if chapter_text == section_name || chapter_text.starts_with(section_name) {
                    found_section = true;
                    current_chapter = Some(chapter);
                    debug!("Found section '{}' in HTML", section_name);
                    break;
                }
            }

            if !found_section {
                debug!("Section '{}' not found in HTML", section_name);
                return items;
            }

            if let Some(chapter_elem) = current_chapter {
                let mut collect = false;

                for element in document.root_element().descendants() {
                    if let Some(elem_ref) = ElementRef::wrap(element) {
                        if elem_ref == chapter_elem {
                            collect = true;
                            continue;
                        }

                        if collect {
                            if let Some(elem) = ElementRef::wrap(element) {
                                if elem.value().name() == "p" {
                                    if let Some(class) = elem.value().attr("class") {
                                        if class.contains("V8SH_chapter") {
                                            break;
                                        }
                                    }
                                }

                                if elem.value().name() == "a" {
                                    let link_text =
                                        elem.text().collect::<String>().trim().to_string();
                                    if !link_text.is_empty() {
                                        let (russian, english) =
                                            Self::parse_bilingual_name(&link_text);
                                        items.push((russian, english));
                                        if let Some((ru, en)) = items.last() {
                                            debug!(
                                                "Extracted from '{}': {} ({})",
                                                section_name,
                                                ru,
                                                en
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if items.is_empty() {
            debug!("No items extracted from section '{}'", section_name);
        } else {
            debug!("Extracted {} items from '{}'", items.len(), section_name);
        }

        items
    }

    /// Парсит двуязычное имя формата "Русское (English)" -> ("Русское", "English")
    fn parse_bilingual_name(text: &str) -> (String, String) {
        if let Some(pos) = text.rfind('(') {
            if let Some(end_pos) = text.rfind(')') {
                if end_pos > pos {
                    let russian = text[..pos].trim().to_string();
                    let english = text[pos + 1..end_pos].trim().to_string();
                    return (russian, english);
                }
            }
        }
        (text.to_string(), String::new())
    }
}
