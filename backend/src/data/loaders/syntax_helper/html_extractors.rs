//! HTML extraction методы для парсера синтакс-помощника 1С

use scraper::{ElementRef, Html, Selector};
use std::path::Path;
use tracing::debug;

use super::types::*;
use bsl_shared::domain::types::FacetKind;

/// Экстрактор данных из HTML документов синтакс-помощника
pub struct HtmlExtractor;

impl HtmlExtractor {
    pub fn new() -> Self {
        Self
    }

    // =========================================================================
    // Основные методы извлечения
    // =========================================================================

    pub fn extract_title(&self, document: &Html) -> String {
        self.extract_element_text(document, "h1.V8SH_pagetitle")
            .or_else(|| self.extract_element_text(document, "h1"))
            .unwrap_or_default()
    }

    pub fn parse_title(&self, title: &str) -> (String, String) {
        if let Some(open) = title.find('(') {
            if let Some(close) = title.find(')') {
                let russian = title[..open].trim().to_string();
                let english = title[open + 1..close].trim().to_string();
                return (russian, english);
            }
        }
        (title.trim().to_string(), String::new())
    }

    pub fn extract_element_text(&self, document: &Html, selector_str: &str) -> Option<String> {
        Selector::parse(selector_str).ok().and_then(|selector| {
            document
                .select(&selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
        })
    }

    pub fn extract_description(&self, document: &Html) -> String {
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

    pub fn extract_examples(&self, document: &Html) -> Vec<CodeExample> {
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

    /// КРИТИЧНЫЙ МЕТОД: Извлечение параметров методов из HTML
    ///
    /// Извлекает параметры из HTML в формате:
    /// ```html
    /// <p class="V8SH_chapter">Параметры:</p>
    /// <div class="V8SH_rubric">
    ///     <p>&lt;Индекс&gt; (обязательный)</div>
    /// Тип: Число. <br>
    /// Описание параметра.
    /// ```
    ///
    /// Используется более надёжный парсер на основе regex с учётом реальной структуры HTML
    pub fn extract_parameters(&self, document: &Html) -> Vec<ParameterInfo> {
        use regex::Regex;

        let mut parameters = Vec::new();
        let html_text = document.html();

        // Проверяем наличие раздела "Параметры:"
        if !html_text.contains("Параметры:") && !html_text.contains("Parameters:") {
            debug!("⚠️  Раздел 'Параметры' не найден в HTML");
            return parameters;
        }

        // Парсер параметров - ищет паттерн:
        // &lt;ИМЯ&gt; (обязательный) ... Тип: ТИП
        //
        // [\s\S]{0,500}? - матчит любые символы в non-greedy режиме до "Тип:"
        // [^<\n]+ - захватить тип GREEDY (не содержит < или перевод строки)
        // Точка после типа может быть или опциональна
        let param_regex = match Regex::new(
            r#"&lt;([^>]+)&gt;\s*\(([^)]+)\)[\s\S]{0,500}?Тип:\s*(?:<a[^>]*>)?([^<\n]+)(?:</a>)?\.?"#
        ) {
            Ok(r) => r,
            Err(e) => {
                debug!("❌ Ошибка компиляции regex для параметров: {}", e);
                return parameters;
            }
        };

        debug!("🔍 Начинаем извлечение параметров с помощью regex");

        for cap in param_regex.captures_iter(&html_text) {
            let name = cap.get(1).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
            let optional_text = cap.get(2).map(|m| m.as_str().trim()).unwrap_or("");
            let mut param_type = cap.get(3).map(|m| m.as_str().trim().to_string()).unwrap_or_default();

            // Пропускаем если имя пусто
            if name.is_empty() {
                continue;
            }

            // Удаляем точку в конце типа если она есть
            if param_type.ends_with('.') {
                param_type.pop();
            }
            param_type = param_type.trim().to_string();

            // Определяем опциональность
            let is_optional = optional_text.contains("необязательный")
                || optional_text.contains("optional")
                || optional_text.to_lowercase().contains("optional");

            // Извлекаем описание параметра
            // Ищем текст после "<br>" до "<div" или следующего параметра
            let description = {
                // Берём текст начиная с конца типа параметра
                let search_start = cap.get(3).map(|m| m.end()).unwrap_or(0);
                // Убедимся что search_start - это valid char boundary
                let search_start = if html_text.is_char_boundary(search_start) {
                    search_start
                } else {
                    // Если нет, то идём назад до valid boundary
                    (0..search_start).rev().find(|&i| html_text.is_char_boundary(i)).unwrap_or(0)
                };

                let search_end = std::cmp::min(search_start + 500, html_text.len());
                let search_end = if html_text.is_char_boundary(search_end) {
                    search_end
                } else {
                    // Если нет, то идём назад до valid boundary
                    (0..=search_end).rev().find(|&i| html_text.is_char_boundary(i)).unwrap_or(0)
                };

                let search_text = &html_text[search_start..search_end];

                // Ищем текст после <br>
                if let Some(br_pos) = search_text.find("<br>") {
                    let text_after_br = &search_text[br_pos + 4..];
                    // Берём текст до следующего <div или конца
                    if let Some(next_div) = text_after_br.find("<div") {
                        text_after_br[..next_div].trim().to_string()
                    } else {
                        text_after_br.trim().to_string()
                    }
                } else if let Some(br_pos) = search_text.find("<br") {
                    // Некорректный <br без закрытия
                    let text_after_br = &search_text[br_pos..];
                    if let Some(gt_pos) = text_after_br.find('>') {
                        let desc = &text_after_br[gt_pos + 1..];
                        if let Some(next_div) = desc.find("<div") {
                            desc[..next_div].trim().to_string()
                        } else {
                            desc.trim().to_string()
                        }
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            };

            let description = if description.is_empty() {
                None
            } else {
                Some(description.replace("&nbsp;", " "))
            };

            parameters.push(ParameterInfo {
                name: name.clone(),
                type_name: Some(param_type.clone()),
                is_optional,
                default_value: None,
                description,
            });

            debug!(
                "✓ Извлечён параметр: {} : {} ({})",
                name,
                param_type,
                if is_optional { "необязательный" } else { "обязательный" }
            );
        }

        if parameters.is_empty() {
            debug!("⚠️  Не удалось извлечь параметры из HTML");
        } else {
            debug!("✓ Извлечено {} параметров", parameters.len());
        }

        parameters
    }

    pub fn extract_return_info(&self, document: &Html) -> (Option<String>, Option<String>) {
        // Ищем информацию о возвращаемом значении
        if let Ok(selector) = Selector::parse("div.V8SH_return, div.return") {
            if let Some(return_div) = document.select(&selector).next() {
                let text = return_div.text().collect::<String>();
                // Разделяем тип и описание
                if let Some(colon) = text.find(':') {
                    let return_type = text[..colon].trim().to_string();
                    let return_desc = text[colon + 1..].trim().to_string();
                    return (Some(return_type), Some(return_desc));
                }
                return (Some(text.trim().to_string()), None);
            }
        }
        (None, None)
    }

    pub fn extract_return_type(&self, document: &Html) -> String {
        self.extract_return_info(document).0.unwrap_or_default()
    }

    pub fn extract_property_type(&self, document: &Html) -> Option<String> {
        self.extract_element_text(document, "span.V8SH_type, span.type")
    }

    pub fn extract_english_name(&self, document: &Html) -> Option<String> {
        self.extract_element_text(document, "span.V8SH_english, span.english")
    }

    pub fn extract_availability(&self, document: &Html) -> Vec<String> {
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

    pub fn extract_version(&self, document: &Html) -> String {
        self.extract_element_text(document, "span.V8SH_version, span.version")
            .unwrap_or_else(|| "8.3.0+".to_string())
    }

    pub fn extract_aliases(&self, _document: &Html) -> Vec<String> {
        // Извлекаем альтернативные имена из текста
        Vec::new() // TODO: Implement alias extraction
    }

    pub fn extract_collection_element(&self, _document: &Html) -> Option<String> {
        // Извлекаем тип элемента коллекции
        None // TODO: Implement collection element extraction
    }

    /// КРИТИЧНЫЙ МЕТОД: Извлечение enum значений для платформенных перечислений
    ///
    /// Извлекает значения перечислений из HTML в формате:
    /// ```html
    /// <p class="V8SH_chapter">Значения</p>
    /// <a href="...">Авто (Auto)</a><br>
    /// <a href="...">ИспользоватьЕслиВозможно (UseIfPossible)</a><br>
    /// ```
    pub fn extract_enum_values_from_html(&self, document: &Html) -> Vec<String> {
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
                    debug!("🔍 Найден раздел 'Значения' в HTML");
                    break;
                }
            }

            if !found_values_section {
                debug!("⚠️  Раздел 'Значения' не найден в HTML");
                return enum_values;
            }

            // Извлекаем все <a> элементы после заголовка "Значения"
            // до следующего заголовка или конца документа
            if let Some(chapter_elem) = current_chapter {
                if let Ok(_link_selector) = Selector::parse("a") {
                    // Получаем следующие элементы после заголовка
                    let mut collect = false;

                    // Обходим все элементы документа
                    for element in document.root_element().descendants() {
                        // Проверяем, достигли ли мы заголовка "Значения"
                        if let Some(elem_ref) = ElementRef::wrap(element) {
                            if elem_ref == chapter_elem {
                                collect = true;
                                continue;
                            }

                            // Если встретили следующий заголовок, прекращаем сбор
                            if collect {
                                if let Some(elem) = ElementRef::wrap(element) {
                                    if elem.value().name() == "p" {
                                        if let Some(class) = elem.value().attr("class") {
                                            if class.contains("V8SH_chapter") {
                                                break;
                                            }
                                        }
                                    }

                                    // Собираем текст из <a> элементов
                                    if elem.value().name() == "a" {
                                        let link_text =
                                            elem.text().collect::<String>().trim().to_string();
                                        if !link_text.is_empty() {
                                            enum_values.push(link_text.clone());
                                            debug!("✓ Извлечено enum значение: {}", link_text);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if enum_values.is_empty() {
            debug!("⚠️  Не удалось извлечь enum значения из HTML");
        } else {
            debug!("✓ Извлечено {} enum значений", enum_values.len());
        }

        enum_values
    }

    /// КРИТИЧНЫЙ МЕТОД: Извлечение методов из HTML
    ///
    /// Извлекает методы из HTML в формате:
    /// ```html
    /// <p class="V8SH_chapter">Методы:</p>
    /// <a href="Array/methods/UBound770.html">ВГраница (UBound)</a><br>
    /// <a href="Array/methods/Add772.html">Добавить (Add)</a><br>
    /// ```
    pub fn extract_methods_from_html(&self, document: &Html) -> Vec<(String, String)> {
        self.extract_section_links(document, "Методы")
    }

    /// КРИТИЧНЫЙ МЕТОД: Извлечение свойств из HTML
    ///
    /// Извлекает свойства из HTML в формате:
    /// ```html
    /// <p class="V8SH_chapter">Свойства:</p>
    /// <a href="...">Имя (Name)</a><br>
    /// ```
    pub fn extract_properties_from_html(&self, document: &Html) -> Vec<(String, String)> {
        self.extract_section_links(document, "Свойства")
    }

    /// Общий метод для извлечения ссылок из раздела (Методы, Свойства, Конструкторы)
    ///
    /// Возвращает: Vec<(русское_имя, английское_имя)>
    fn extract_section_links(&self, document: &Html, section_name: &str) -> Vec<(String, String)> {
        let mut items = Vec::new();

        // Ищем заголовок раздела
        if let Ok(chapter_selector) = Selector::parse("p.V8SH_chapter") {
            let mut found_section = false;
            let mut current_chapter: Option<ElementRef> = None;

            for chapter in document.select(&chapter_selector) {
                let chapter_text = chapter.text().collect::<String>().trim().to_string();

                if chapter_text == section_name || chapter_text.starts_with(section_name) {
                    found_section = true;
                    current_chapter = Some(chapter);
                    debug!("🔍 Найден раздел '{}' в HTML", section_name);
                    break;
                }
            }

            if !found_section {
                debug!("⚠️  Раздел '{}' не найден в HTML", section_name);
                return items;
            }

            // Извлекаем все <a> элементы после заголовка до следующего заголовка
            if let Some(chapter_elem) = current_chapter {
                let mut collect = false;

                // Обходим все элементы документа
                for element in document.root_element().descendants() {
                    // Проверяем, достигли ли мы нужного заголовка
                    if let Some(elem_ref) = ElementRef::wrap(element) {
                        if elem_ref == chapter_elem {
                            collect = true;
                            continue;
                        }

                        // Если встретили следующий заголовок, прекращаем сбор
                        if collect {
                            if let Some(elem) = ElementRef::wrap(element) {
                                if elem.value().name() == "p" {
                                    if let Some(class) = elem.value().attr("class") {
                                        if class.contains("V8SH_chapter") {
                                            break;
                                        }
                                    }
                                }

                                // Собираем текст из <a> элементов
                                if elem.value().name() == "a" {
                                    let link_text =
                                        elem.text().collect::<String>().trim().to_string();
                                    if !link_text.is_empty() {
                                        // Парсим формат "Добавить (Add)" -> ("Добавить", "Add")
                                        let (russian, english) =
                                            self.parse_bilingual_name(&link_text);
                                        items.push((russian, english));
                                        debug!(
                                            "✓ Извлечено из '{}': {} ({})",
                                            section_name,
                                            items.last().unwrap().0,
                                            items.last().unwrap().1
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if items.is_empty() {
            debug!(
                "⚠️  Не удалось извлечь элементы из раздела '{}'",
                section_name
            );
        } else {
            debug!(
                "✓ Извлечено {} элементов из '{}'",
                items.len(),
                section_name
            );
        }

        items
    }

    /// Парсит двуязычное имя формата "Русское (English)" -> ("Русское", "English")
    fn parse_bilingual_name(&self, text: &str) -> (String, String) {
        // Ищем паттерн "Русское (English)"
        if let Some(pos) = text.rfind('(') {
            if let Some(end_pos) = text.rfind(')') {
                if end_pos > pos {
                    let russian = text[..pos].trim().to_string();
                    let english = text[pos + 1..end_pos].trim().to_string();
                    return (russian, english);
                }
            }
        }
        // Если формат не распознан, возвращаем как есть
        (text.to_string(), String::new())
    }

    pub fn extract_links(&self, document: &Html) -> Vec<String> {
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

    pub fn extract_type_list(&self, document: &Html) -> Vec<String> {
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

    pub fn extract_category_path(&self, _path: &Path) -> String {
        // Пока возвращаем пустую строку, так как категории будут установлены
        // позже в link_types_to_categories на основе каталогов категорий
        String::new()
    }

    // =========================================================================
    // Вспомогательные методы проверки
    // =========================================================================

    pub fn is_readonly(&self, document: &Html) -> bool {
        let text = document.root_element().text().collect::<String>();
        text.contains("Только чтение") || text.contains("Read only")
    }

    pub fn is_iterable(&self, description: &str) -> bool {
        description.contains("Для каждого")
            || description.contains("For each")
            || description.contains("итерация")
            || description.contains("iteration")
    }

    pub fn is_indexable(&self, description: &str) -> bool {
        description.contains("индекс")
            || description.contains("index")
            || description.contains("[]")
    }

    pub fn is_serializable(&self, document: &Html) -> bool {
        let text = document.root_element().text().collect::<String>();
        text.contains("Сериализуемый")
            || text.contains("Serializable")
            || text.contains("XML")
            || text.contains("JSON")
    }

    pub fn is_exchangeable(&self, document: &Html) -> bool {
        let text = document.root_element().text().collect::<String>();
        text.contains("Обмен данными") || text.contains("Data exchange") || text.contains("XDTO")
    }

    pub fn detect_facets(&self, type_name: &str, description: &str) -> Vec<FacetKind> {
        let mut facets = vec![];

        // Определяем фасеты по имени типа
        if type_name.ends_with("Manager") || type_name.contains("Менеджер") {
            facets.push(FacetKind::Manager);
        }

        if type_name.ends_with("Object") || type_name.contains("Объект") {
            facets.push(FacetKind::Object);
        }

        if type_name.ends_with("Ref") || type_name.contains("Ссылка") {
            facets.push(FacetKind::Reference);
        }

        // Определяем фасеты по описанию
        if description.contains("коллекция")
            || description.contains("collection")
            || description.contains("Для каждого")
            || type_name.contains("Таблица")
            || type_name.contains("Table")
            || type_name.contains("Массив")
            || type_name.contains("Array")
        {
            facets.push(FacetKind::Collection);
        }

        if description.contains("создать")
            || description.contains("create")
            || description.contains("конструктор")
        {
            facets.push(FacetKind::Constructor);
        }

        facets
    }

    pub fn build_path(&self, path: &Path) -> String {
        // Строим путь относительно корня синтакс-помощника
        path.components()
            .filter_map(|c| {
                if let std::path::Component::Normal(name) = c {
                    name.to_str()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("/")
    }
}

impl Default for HtmlExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_parameters_from_real_html_insert() {
        // Реальная HTML структура из Array.Insert
        let html_content = r#"
        <html>
            <p class="V8SH_chapter">Параметры:</p>
            <div class="V8SH_rubric">
                <p style="margin-top: 2px; margin-bottom: 1px">&lt;Индекс&gt; (обязательный)</div>
            Тип: <a href="v8help://SyntaxHelperLanguage/def_Number">Число</a>. <br>
            Индекс вставляемого значения.
            <div class="V8SH_rubric">
                <p style="margin-top: 2px; margin-bottom: 1px">&lt;Значение&gt; (необязательный)</div>
            Тип: Произвольный. <br>
            Вставляемое значение. Если не указан, то будет добавлено значение типа Неопределено.
        </html>
        "#;

        let document = Html::parse_document(html_content);
        let extractor = HtmlExtractor::new();
        let params = extractor.extract_parameters(&document);

        assert_eq!(params.len(), 2, "Должно быть 2 параметра");

        // Первый параметр: Индекс (обязательный)
        assert_eq!(params[0].name, "Индекс");
        assert_eq!(params[0].type_name, Some("Число".to_string()));
        assert_eq!(params[0].is_optional, false);
        assert!(
            params[0].description.as_ref().map(|d| d.contains("Индекс вставляемого")).unwrap_or(false),
            "Описание должно содержать текст о вставляемом значении"
        );

        // Второй параметр: Значение (необязательный)
        assert_eq!(params[1].name, "Значение");
        assert_eq!(params[1].type_name, Some("Произвольный".to_string()));
        assert_eq!(params[1].is_optional, true);
    }

    #[test]
    fn test_extract_parameters_from_real_html_find() {
        // Реальная HTML структура из Array.Find
        let html_content = r#"
        <html>
            <p class="V8SH_chapter">Параметры:</p>
            <div class="V8SH_rubric">
                <p style="margin-top: 2px; margin-bottom: 1px">&lt;Значение&gt; (обязательный)</div>
            Тип: Произвольный. <br>
            Искомое значение.
        </html>
        "#;

        let document = Html::parse_document(html_content);
        let extractor = HtmlExtractor::new();
        let params = extractor.extract_parameters(&document);

        assert_eq!(params.len(), 1, "Должен быть 1 параметр");
        assert_eq!(params[0].name, "Значение");
        assert_eq!(params[0].type_name, Some("Произвольный".to_string()));
        assert_eq!(params[0].is_optional, false);
    }

    #[test]
    fn test_extract_parameters_from_real_html_get() {
        // Реальная HTML структура из Array.Get
        let html_content = r#"
        <html>
            <p class="V8SH_chapter">Параметры:</p>
            <div class="V8SH_rubric">
                <p style="margin-top: 2px; margin-bottom: 1px">&lt;Индекс&gt; (обязательный)</div>
            Тип: <a href="v8help://SyntaxHelperLanguage/def_Number">Число</a>. <br>
            Индекс элемента.
        </html>
        "#;

        let document = Html::parse_document(html_content);
        let extractor = HtmlExtractor::new();
        let params = extractor.extract_parameters(&document);

        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "Индекс");
        assert_eq!(params[0].type_name, Some("Число".to_string()));
        assert_eq!(params[0].is_optional, false);
    }

    #[test]
    fn test_extract_parameters_no_params_section() {
        // HTML без раздела "Параметры:"
        let html_content = r#"
        <html>
            <p class="V8SH_chapter">Описание:</p>
            <p>Метод без параметров</p>
        </html>
        "#;

        let document = Html::parse_document(html_content);
        let extractor = HtmlExtractor::new();
        let params = extractor.extract_parameters(&document);

        assert_eq!(params.len(), 0, "Не должно быть параметров");
    }

    #[test]
    fn test_parse_bilingual_name() {
        let extractor = HtmlExtractor::new();

        let (ru, en) = extractor.parse_bilingual_name("Добавить (Add)");
        assert_eq!(ru, "Добавить");
        assert_eq!(en, "Add");

        let (ru, en) = extractor.parse_bilingual_name("ВГраница (UBound)");
        assert_eq!(ru, "ВГраница");
        assert_eq!(en, "UBound");

        let (ru, _) = extractor.parse_bilingual_name("Метод без английского");
        assert_eq!(ru, "Метод без английского");
    }
}
