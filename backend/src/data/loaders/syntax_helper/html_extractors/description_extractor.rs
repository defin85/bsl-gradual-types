//! Извлечение описаний, примеров и метаданных из HTML документов синтакс-помощника

use scraper::{Html, Selector};

use super::super::type_parser::TypeParser;
use super::super::types::CodeExample;
use super::title_extractor::TitleExtractor;

/// Экстрактор описаний и метаданных
pub struct DescriptionExtractor;

impl DescriptionExtractor {
    /// Извлекает описание документа
    pub fn extract_description(document: &Html) -> String {
        if let Ok(selector) = Selector::parse("div.V8SH_descr p, p") {
            document
                .select(&selector)
                .map(|e| e.text().collect::<String>().trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            String::new()
        }
    }

    /// Извлекает примеры кода
    pub fn extract_examples(document: &Html) -> Vec<CodeExample> {
        let mut examples = Vec::new();

        if let Ok(selector) = Selector::parse("pre.V8SH_code, pre, code") {
            for elem in document.select(&selector) {
                let code = elem.text().collect::<String>().trim().to_string();
                if !code.is_empty() {
                    examples.push(CodeExample {
                        description: None,
                        code,
                        language: "bsl".to_string(),
                    });
                }
            }
        }

        examples
    }

    /// Извлекает информацию о доступности (Сервер, Клиент, Мобильный)
    pub fn extract_availability(document: &Html) -> Vec<String> {
        let mut availability = Vec::new();

        if let Ok(selector) = Selector::parse("div.V8SH_availability, div.availability") {
            if let Some(avail_div) = document.select(&selector).next() {
                let text = avail_div.text().collect::<String>();
                if text.contains("Сервер") || text.contains("Server") {
                    availability.push("Сервер".to_string());
                }
                if text.contains("Клиент") || text.contains("Client") {
                    availability.push("Клиент".to_string());
                }
                if text.contains("Мобильный") || text.contains("Mobile") {
                    availability.push("Мобильный".to_string());
                }
            }
        }

        if availability.is_empty() {
            availability = vec!["Сервер".to_string(), "Клиент".to_string()];
        }

        availability
    }

    /// Извлекает версию платформы
    pub fn extract_version(document: &Html) -> String {
        TitleExtractor::extract_element_text(document, "span.V8SH_version, span.version")
            .unwrap_or_else(|| "8.3.0+".to_string())
    }

    /// Извлекает английское название
    pub fn extract_english_name(document: &Html) -> Option<String> {
        TitleExtractor::extract_element_text(document, "span.V8SH_english, span.english")
    }

    /// Извлекает информацию о возвращаемом типе
    ///
    /// Возвращает: (тип, описание)
    pub fn extract_return_info(document: &Html) -> (Option<String>, Option<String>) {
        let html_text = document.html();
        TypeParser::parse_return_type(&html_text)
    }

    /// Извлекает только тип возвращаемого значения
    pub fn extract_return_type(document: &Html) -> String {
        Self::extract_return_info(document).0.unwrap_or_default()
    }

    /// Извлекает тип свойства
    pub fn extract_property_type(document: &Html) -> Option<String> {
        // 1) Прямой span (редкий формат)
        if let Some(t) = TitleExtractor::extract_element_text(document, "span.V8SH_type, span.type")
        {
            let t = t.trim().to_string();
            if !t.is_empty() {
                return Some(t);
            }
        }

        // 2) Тип внутри "Описание:" блока (типичный формат синтакс-помощника):
        // "Описание: Тип: <a>КоллекцияКолонокТаблицыЗначений</a>. <br>..."
        let html_text = document.html();

        // Находим секцию "Описание:" / "Description:" и берём её до следующего заголовка.
        const RU_DESC: &str = "Описание:";
        const EN_DESC: &str = "Description:";
        let section_start = html_text
            .find(RU_DESC)
            .map(|p| p + RU_DESC.len())
            .or_else(|| html_text.find(EN_DESC).map(|p| p + EN_DESC.len()))?;
        let remaining = &html_text[section_start..];
        let section_end = remaining
            .find("<p class=\"V8SH_chapter\">")
            .or_else(|| remaining.find("</body>"))
            .unwrap_or(remaining.len());
        let section = &remaining[..section_end];

        let (type_line, _description) = TypeParser::extract_type_line(section);
        if type_line.is_empty() {
            return None;
        }

        let fragments = TypeParser::parse_type_fragments(&type_line);
        if fragments.is_empty() {
            return None;
        }

        Some(TypeParser::assemble_types(&fragments))
    }

    /// Извлекает ссылки из документа
    pub fn extract_links(document: &Html) -> Vec<String> {
        let mut links = Vec::new();

        if let Ok(selector) = Selector::parse("a.V8SH_link, a") {
            for link in document.select(&selector) {
                if let Some(href) = link.value().attr("href") {
                    links.push(href.to_string());
                }
            }
        }

        links
    }

    /// Извлекает список типов
    pub fn extract_type_list(document: &Html) -> Vec<String> {
        let mut types = Vec::new();

        if let Ok(selector) = Selector::parse("ul.V8SH_types li, ul li") {
            for item in document.select(&selector) {
                let text = item.text().collect::<String>().trim().to_string();
                if !text.is_empty() {
                    types.push(text);
                }
            }
        }

        types
    }
}
