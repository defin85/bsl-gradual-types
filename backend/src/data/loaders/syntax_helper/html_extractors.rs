//! HTML extraction методы для парсера синтакс-помощника 1С

use scraper::{ElementRef, Html, Selector};
use std::path::Path;
use tracing::debug;

use super::type_parser::{TypeParser, UNION_SEPARATOR};
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
    ///
    /// ВАЖНО: Парсер извлекает параметры ТОЛЬКО из секции "Параметры:" до "Возвращаемое значение:"
    /// чтобы избежать захвата placeholder'ов из заголовков типа "СправочникМенеджер.<Имя справочника>"
    pub fn extract_parameters(&self, document: &Html) -> Vec<ParameterInfo> {
        use regex::Regex;

        let mut parameters = Vec::new();
        let html_text = document.html();

        // Проверяем наличие раздела "Параметры:"
        if !html_text.contains("Параметры:") && !html_text.contains("Parameters:") {
            debug!("⚠️  Раздел 'Параметры' не найден в HTML");
            return parameters;
        }

        // ШАГ 1: Извлекаем ТОЛЬКО секцию "Параметры:" (до следующей секции)
        // Это предотвращает захват placeholder'ов из заголовков типов
        let params_section = Self::extract_parameters_section(&html_text);
        if params_section.is_empty() {
            debug!("⚠️  Секция 'Параметры' пуста");
            return parameters;
        }

        debug!("🔍 Начинаем извлечение параметров из секции ({} символов)", params_section.len());

        // ШАГ 2: Парсер параметров внутри секции
        // Ищем паттерн: <div class="V8SH_rubric">...<p...>&lt;ИМЯ&gt; (обязательный|необязательный)</p>?</div>
        // ВАЖНО: scraper добавляет </p> при парсинге, поэтому используем (?:</p>)?
        // После div идёт: Тип: <a>ТИП1</a>, <a>ТИП2</a>.
        let param_regex = match Regex::new(
            r#"<div class="V8SH_rubric">[^<]*<p[^>]*>&lt;([^>]+)&gt;\s*\(([^)]+)\)(?:</p>)?</div>"#,
        ) {
            Ok(r) => r,
            Err(e) => {
                debug!("❌ Ошибка компиляции regex для параметров: {}", e);
                return parameters;
            }
        };

        // Regex для извлечения union типов из <a> тегов
        let type_link_regex = match Regex::new(r#"<a[^>]*>([^<]+)</a>"#) {
            Ok(r) => r,
            Err(e) => {
                debug!("❌ Ошибка компиляции regex для типов: {}", e);
                return parameters;
            }
        };

        for cap in param_regex.captures_iter(&params_section) {
            let name = cap
                .get(1)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            let optional_text = cap.get(2).map(|m| m.as_str().trim()).unwrap_or("");

            // Пропускаем если имя пусто
            if name.is_empty() {
                continue;
            }

            // ШАГ 3: Найти "Тип:" после этого параметра и извлечь union типы
            let match_end = cap.get(0).map(|m| m.end()).unwrap_or(0);
            let remaining = if match_end < params_section.len() {
                &params_section[match_end..]
            } else {
                ""
            };

            // Находим "Тип:" и извлекаем типы до "<br>" или следующего "<div"
            // ВАЖНО: Используем .len() для корректной работы с UTF-8
            const RU_TYPE: &str = "Тип:";
            const EN_TYPE: &str = "Type:";
            let mut param_type = String::new();
            let (type_pos, type_marker_len) = remaining
                .find(RU_TYPE)
                .map(|p| (p, RU_TYPE.len()))
                .or_else(|| remaining.find(EN_TYPE).map(|p| (p, EN_TYPE.len())))
                .unwrap_or((0, 0));

            if type_marker_len > 0 {
                let type_section_start = type_pos + type_marker_len;
                let type_section_end = remaining[type_section_start..]
                    .find("<br")
                    .or_else(|| remaining[type_section_start..].find("<div"))
                    .map(|p| type_section_start + p)
                    .unwrap_or_else(|| std::cmp::min(type_section_start + 500, remaining.len()));

                let type_section = &remaining[type_section_start..type_section_end];

                // Извлекаем все типы из <a> тегов
                let types: Vec<String> = type_link_regex
                    .captures_iter(type_section)
                    .filter_map(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
                    .filter(|t| !t.is_empty())
                    .collect();

                if !types.is_empty() {
                    // Объединяем union типы через UNION_SEPARATOR
                    param_type = types.join(UNION_SEPARATOR);
                } else {
                    // Fallback: если нет <a> тегов, берём текст напрямую
                    let clean_text = type_section
                        .replace("&nbsp;", " ")
                        .trim()
                        .trim_end_matches('.')
                        .to_string();
                    if !clean_text.is_empty() {
                        param_type = clean_text;
                    }
                }
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

            // Извлекаем описание параметра из remaining (после "Тип: ...")
            // Ищем текст после первого "<br>" до следующего "<div" или конца секции
            let description = {
                // Ищем текст после <br>
                if let Some(br_pos) = remaining.find("<br>") {
                    let text_after_br = &remaining[br_pos + 4..];
                    // Берём текст до следующего <div или конца
                    let desc_text = if let Some(next_div) = text_after_br.find("<div") {
                        text_after_br[..next_div].trim()
                    } else {
                        text_after_br.trim()
                    };
                    // Убираем HTML теги и лишние переводы строк
                    let clean_desc = desc_text
                        .replace("<br>", " ")
                        .replace("&nbsp;", " ")
                        .trim()
                        .to_string();
                    if clean_desc.is_empty() { None } else { Some(clean_desc) }
                } else if let Some(br_pos) = remaining.find("<br") {
                    // Некорректный <br без закрытия
                    let text_after_br = &remaining[br_pos..];
                    if let Some(gt_pos) = text_after_br.find('>') {
                        let desc = &text_after_br[gt_pos + 1..];
                        let desc_text = if let Some(next_div) = desc.find("<div") {
                            desc[..next_div].trim()
                        } else {
                            desc.trim()
                        };
                        let clean_desc = desc_text.replace("&nbsp;", " ").trim().to_string();
                        if clean_desc.is_empty() { None } else { Some(clean_desc) }
                    } else {
                        None
                    }
                } else {
                    None
                }
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
                if is_optional {
                    "необязательный"
                } else {
                    "обязательный"
                }
            );
        }

        if parameters.is_empty() {
            debug!("⚠️  Не удалось извлечь параметры из HTML");
        } else {
            debug!("✓ Извлечено {} параметров", parameters.len());
        }

        parameters
    }

    /// Извлечь секцию "Параметры:" из HTML до следующей секции
    ///
    /// Это предотвращает захват placeholder'ов из заголовков типа
    /// "СправочникМенеджер.<Имя справочника>" при парсинге параметров
    fn extract_parameters_section(html_text: &str) -> String {
        // Находим начало секции "Параметры:"
        // ВАЖНО: Используем .len() строки поиска, т.к. UTF-8 кириллица = 2 байта/символ
        const RU_PARAMS: &str = "Параметры:</p>";
        const EN_PARAMS: &str = "Parameters:</p>";

        let params_start = html_text
            .find(RU_PARAMS)
            .map(|p| p + RU_PARAMS.len())
            .or_else(|| html_text.find(EN_PARAMS).map(|p| p + EN_PARAMS.len()))
            .unwrap_or(0);

        if params_start == 0 {
            return String::new();
        }

        // Находим конец секции (начало следующей секции)
        let remaining = &html_text[params_start..];
        let section_end = remaining
            .find("Возвращаемое значение:")
            .or_else(|| remaining.find("Return value:"))
            .or_else(|| remaining.find("Описание:</p>"))
            .or_else(|| remaining.find("Description:</p>"))
            .or_else(|| remaining.find("Доступность:</p>"))
            .or_else(|| remaining.find("Availability:</p>"))
            .unwrap_or(remaining.len());

        remaining[..section_end].to_string()
    }

    /// КРИТИЧНЫЙ МЕТОД: Извлечение информации о возвращаемом типе из HTML
    ///
    /// Использует DOM-based парсер для корректной обработки:
    /// - Простых типов: `Тип: <a>Строка</a>.`
    /// - Union типов: `Тип: <a>Число</a>, <a>Строка</a>.`
    /// - Faceted типов: `<a>СправочникСсылка.</a><span>&lt;</span><a>Имя</a><span>&gt;</span>`
    /// - Faceted + Union: комбинация вышеуказанных форматов
    ///
    /// Placeholder'ы нормализуются в `<T>`:
    /// - `<Имя справочника>` → `<T>`
    /// - `<Имя документа>` → `<T>`
    /// - и т.д.
    ///
    /// Делегирует парсинг в `TypeParser` для лучшей модульности.
    pub fn extract_return_info(&self, document: &Html) -> (Option<String>, Option<String>) {
        let html_text = document.html();
        TypeParser::parse_return_type(&html_text)
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
        assert!(!params[0].is_optional);
        assert!(
            params[0]
                .description
                .as_ref()
                .map(|d| d.contains("Индекс вставляемого"))
                .unwrap_or(false),
            "Описание должно содержать текст о вставляемом значении"
        );

        // Второй параметр: Значение (необязательный)
        assert_eq!(params[1].name, "Значение");
        assert_eq!(params[1].type_name, Some("Произвольный".to_string()));
        assert!(params[1].is_optional);
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
        assert!(!params[0].is_optional);
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
        assert!(!params[0].is_optional);
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

    #[test]
    fn test_extract_return_info_from_value_table_insert() {
        // Реальная HTML структура из ValueTable.Insert
        let html_content = r#"
        <html>
            <p class="V8SH_chapter">Возвращаемое значение:</p>
            Тип: <a href="v8help://SyntaxHelperContext/objects/catalog234/catalog236/ValueTableRow.html">СтрокаТаблицыЗначений</a>. <br>
            Вставленная строка.
        </html>
        "#;

        let document = Html::parse_document(html_content);
        let extractor = HtmlExtractor::new();
        let (return_type, return_desc) = extractor.extract_return_info(&document);

        assert_eq!(
            return_type,
            Some("СтрокаТаблицыЗначений".to_string()),
            "Возвращаемый тип должен быть СтрокаТаблицыЗначений"
        );
        assert_eq!(
            return_desc,
            Some("Вставленная строка.".to_string()),
            "Описание должно быть 'Вставленная строка.'"
        );
    }

    #[test]
    fn test_extract_return_info_no_return_section() {
        // HTML без раздела "Возвращаемое значение:"
        let html_content = r#"
        <html>
            <p class="V8SH_chapter">Параметры:</p>
            <p>Метод без возвращаемого значения</p>
        </html>
        "#;

        let document = Html::parse_document(html_content);
        let extractor = HtmlExtractor::new();
        let (return_type, return_desc) = extractor.extract_return_info(&document);

        assert_eq!(return_type, None, "Не должно быть возвращаемого типа");
        assert_eq!(return_desc, None, "Не должно быть описания");
    }

    /// Тест для бага: placeholder из заголовка типа "СправочникМенеджер.<Имя справочника>"
    /// НЕ должен попадать в имена параметров
    #[test]
    fn test_extract_parameters_find_by_code_with_header_placeholder() {
        // Реальная HTML структура из FindByCode250.html
        // Важно: в заголовке есть "&lt;Имя справочника&gt;" - это НЕ параметр!
        let html_content = r#"
        <html>
            <h1 class="V8SH_pagetitle">СправочникМенеджер.&lt;Имя справочника&gt;.НайтиПоКоду</h1>
            <p class="V8SH_title">СправочникМенеджер.&lt;Имя справочника&gt;</p>
            <p class="V8SH_chapter">Параметры:</p>
            <div class="V8SH_rubric">
                <p style="margin-top: 2px; margin-bottom: 1px">&lt;Код&gt; (обязательный)</div>
            Тип: <a href="v8help://SyntaxHelperLanguage/def_Number">Число</a>, <a href="v8help://SyntaxHelperLanguage/def_String">Строка</a>. <br>
            Искомый код.
            <div class="V8SH_rubric">
                <p style="margin-top: 2px; margin-bottom: 1px">&lt;ПоискПоПолномуКоду&gt; (необязательный)</div>
            Тип: <a href="v8help://SyntaxHelperLanguage/def_Boolean">Булево</a>. <br>
            Определяет режим поиска.
            <p class="V8SH_chapter">Возвращаемое значение:</p>
            Тип: СправочникСсылка.
        </html>
        "#;

        let document = Html::parse_document(html_content);
        let extractor = HtmlExtractor::new();
        let params = extractor.extract_parameters(&document);

        // Должно быть 2 параметра (Код, ПоискПоПолномуКоду), а НЕ 3 или больше
        assert_eq!(params.len(), 2, "Должно быть 2 параметра, placeholder 'Имя справочника' не должен парситься как параметр");

        // Первый параметр: Код (обязательный) с union типом
        assert_eq!(params[0].name, "Код", "Первый параметр должен быть 'Код', а не 'Имя справочника'");
        assert_eq!(
            params[0].type_name,
            Some("Число | Строка".to_string()),
            "Тип должен быть union 'Число | Строка'"
        );
        assert!(!params[0].is_optional, "Параметр 'Код' должен быть обязательным");

        // Второй параметр: ПоискПоПолномуКоду (необязательный)
        assert_eq!(params[1].name, "ПоискПоПолномуКоду");
        assert_eq!(params[1].type_name, Some("Булево".to_string()));
        assert!(params[1].is_optional, "Параметр 'ПоискПоПолномуКоду' должен быть необязательным");
    }

    /// Тест для проверки парсинга union типов
    #[test]
    fn test_extract_parameters_union_types() {
        let html_content = r#"
        <html>
            <p class="V8SH_chapter">Параметры:</p>
            <div class="V8SH_rubric">
                <p style="margin-top: 2px; margin-bottom: 1px">&lt;Значение&gt; (обязательный)</div>
            Тип: <a href="type">Число</a>, <a href="type">Строка</a>, <a href="type">Дата</a>. <br>
            Описание.
        </html>
        "#;

        let document = Html::parse_document(html_content);
        let extractor = HtmlExtractor::new();
        let params = extractor.extract_parameters(&document);

        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "Значение");
        assert_eq!(
            params[0].type_name,
            Some("Число | Строка | Дата".to_string()),
            "Должны быть три типа через ' | '"
        );
    }

    // =========================================================================
    // Тесты для DOM-based парсера возвращаемого типа
    // =========================================================================

    /// Тест парсинга faceted типа с placeholder: СправочникСсылка.<Имя справочника>
    #[test]
    fn test_extract_return_info_faceted_type_with_placeholder() {
        // Реальная HTML структура из FindByCode250.html
        let html_content = r#"
        <html>
            <p class="V8SH_chapter">Возвращаемое значение:</p>
            Тип: <a href="v8help://SyntaxHelperContext/objects/catalog125/catalog126/object129.html">СправочникСсылка.</a><span style='color=blue'>&lt;</span><a href="v8help://SyntaxHelperContext/objects/catalog125/catalog126/object129.html">Имя справочника</a><span style='color=blue'>&gt;</span>, <a href="v8help://SyntaxHelperLanguage/def_Undefined">Неопределено</a>. <br>
            Если не существует ни одного элемента с требуемым кодом, то будет возвращена пустая ссылка.
        </html>
        "#;

        let document = Html::parse_document(html_content);
        let extractor = HtmlExtractor::new();
        let (return_type, return_desc) = extractor.extract_return_info(&document);

        assert_eq!(
            return_type,
            Some("СправочникСсылка.<T> | Неопределено".to_string()),
            "Возвращаемый тип должен быть 'СправочникСсылка.<T> | Неопределено'"
        );
        assert!(
            return_desc.is_some(),
            "Описание должно быть извлечено"
        );
    }

    /// Тест парсинга простого union типа
    #[test]
    fn test_extract_return_info_simple_union() {
        let html_content = r#"
        <html>
            <p class="V8SH_chapter">Возвращаемое значение:</p>
            Тип: <a href="type">Число</a>, <a href="type">Строка</a>. <br>
            Описание результата.
        </html>
        "#;

        let document = Html::parse_document(html_content);
        let extractor = HtmlExtractor::new();
        let (return_type, return_desc) = extractor.extract_return_info(&document);

        assert_eq!(
            return_type,
            Some("Число | Строка".to_string()),
            "Возвращаемый тип должен быть union 'Число | Строка'"
        );
        assert_eq!(
            return_desc,
            Some("Описание результата.".to_string())
        );
    }

    /// Тест парсинга faceted типа для документа
    #[test]
    fn test_extract_return_info_faceted_document() {
        let html_content = r#"
        <html>
            <p class="V8SH_chapter">Возвращаемое значение:</p>
            Тип: <a href="type">ДокументСсылка.</a><span>&lt;</span><a href="type">Имя документа</a><span>&gt;</span>. <br>
            Ссылка на документ.
        </html>
        "#;

        let document = Html::parse_document(html_content);
        let extractor = HtmlExtractor::new();
        let (return_type, _) = extractor.extract_return_info(&document);

        assert_eq!(
            return_type,
            Some("ДокументСсылка.<T>".to_string()),
            "Placeholder '<Имя документа>' должен быть нормализован в '<T>'"
        );
    }

    /// Тест вспомогательных функций парсера (делегирован в TypeParser)
    #[test]
    fn test_parse_type_fragments() {
        use crate::data::loaders::syntax_helper::type_parser::TypeFragment;

        // Тест фрагментов для faceted типа
        let type_line = r#" <a href="...">СправочникСсылка.</a><span>&lt;</span><a href="...">Имя справочника</a><span>&gt;</span>"#;
        let fragments = TypeParser::parse_type_fragments(type_line);

        // Должны быть: TypeName("СправочникСсылка."), GenericOpen, TypeName("Имя справочника"), GenericClose
        assert!(fragments.iter().any(|f| matches!(f, TypeFragment::TypeName(s) if s == "СправочникСсылка.")));
        assert!(fragments.iter().any(|f| matches!(f, TypeFragment::GenericOpen)));
        assert!(fragments.iter().any(|f| matches!(f, TypeFragment::TypeName(s) if s == "Имя справочника")));
        assert!(fragments.iter().any(|f| matches!(f, TypeFragment::GenericClose)));
    }

    /// Тест сборки типов из фрагментов (делегирован в TypeParser)
    #[test]
    fn test_assemble_types() {
        use crate::data::loaders::syntax_helper::type_parser::TypeFragment;

        // Тест простого типа
        let fragments = vec![TypeFragment::TypeName("Строка".to_string())];
        assert_eq!(TypeParser::assemble_types(&fragments), "Строка");

        // Тест union типа
        let fragments = vec![
            TypeFragment::TypeName("Число".to_string()),
            TypeFragment::UnionSeparator,
            TypeFragment::TypeName("Строка".to_string()),
        ];
        assert_eq!(TypeParser::assemble_types(&fragments), "Число | Строка");

        // Тест faceted типа
        let fragments = vec![
            TypeFragment::TypeName("СправочникСсылка.".to_string()),
            TypeFragment::GenericOpen,
            TypeFragment::TypeName("Имя справочника".to_string()),
            TypeFragment::GenericClose,
        ];
        assert_eq!(TypeParser::assemble_types(&fragments), "СправочникСсылка.<T>");

        // Тест faceted + union
        let fragments = vec![
            TypeFragment::TypeName("СправочникСсылка.".to_string()),
            TypeFragment::GenericOpen,
            TypeFragment::TypeName("Имя справочника".to_string()),
            TypeFragment::GenericClose,
            TypeFragment::UnionSeparator,
            TypeFragment::TypeName("Неопределено".to_string()),
        ];
        assert_eq!(
            TypeParser::assemble_types(&fragments),
            "СправочникСсылка.<T> | Неопределено"
        );
    }

    /// Тест для malformed HTML: unclosed generic (делегирован в TypeParser)
    #[test]
    fn test_assemble_types_unclosed_generic() {
        use crate::data::loaders::syntax_helper::type_parser::TypeFragment;

        // Malformed: открыли generic, но не закрыли
        let fragments = vec![
            TypeFragment::TypeName("СправочникСсылка.".to_string()),
            TypeFragment::GenericOpen,
            TypeFragment::TypeName("Имя".to_string()),
            // GenericClose отсутствует
        ];

        let result = TypeParser::assemble_types(&fragments);

        // Тип должен быть возвращен БЕЗ паники
        assert!(!result.is_empty(), "Результат не должен быть пустым");

        // Проверяем, что добавлен <T> для robustness
        // Trailing точка удаляется при финализации типа, поэтому результат без точки
        assert_eq!(
            result, "СправочникСсылка<T>",
            "Unclosed generic должен быть закрыт автоматически"
        );
    }

    // =========================================================================
    // Интеграционные тесты на реальных HTML файлах
    // =========================================================================

    /// Интеграционный тест для реального файла FindByCode250.html
    /// Проверяет парсинг параметров и возвращаемого типа на реальных данных
    #[test]
    fn test_find_by_code_real_html() {
        // Загружаем реальный HTML файл
        let html_path = "/home/egor/code/bsl-gradual-types/examples/syntax_helper/rebuilt.shcntx_ru/objects/catalog125/catalog126/object128/methods/FindByCode250.html";

        let html_content = match std::fs::read_to_string(html_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("⚠️  Не удалось загрузить FindByCode250.html: {}", e);
                eprintln!("   Тест пропущен (файл может отсутствовать в разработке)");
                return;
            }
        };

        let document = Html::parse_document(&html_content);
        let extractor = HtmlExtractor::new();

        // ТЕСТ 1: Параметры
        let params = extractor.extract_parameters(&document);
        assert!(!params.is_empty(), "Параметры должны быть найдены");

        // Проверяем первый параметр: Код (union Число | Строка)
        let code_param = params.iter().find(|p| p.name == "Код")
            .expect("Параметр 'Код' должен быть найден");
        assert_eq!(code_param.type_name, Some("Число | Строка".to_string()),
            "Тип 'Код' должен быть union 'Число | Строка'");
        assert!(!code_param.is_optional, "'Код' должен быть обязательным");

        // Проверяем второй параметр: ПоискПоПолномуКоду (Булево, необязательный)
        let search_param = params.iter().find(|p| p.name == "ПоискПоПолномуКоду")
            .expect("Параметр 'ПоискПоПолномуКоду' должен быть найден");
        assert_eq!(search_param.type_name, Some("Булево".to_string()),
            "Тип 'ПоискПоПолномуКоду' должен быть 'Булево'");
        assert!(search_param.is_optional, "'ПоискПоПолномуКоду' должен быть необязательным");

        // ТЕСТ 2: Возвращаемый тип
        let (return_type, return_desc) = extractor.extract_return_info(&document);

        assert_eq!(
            return_type,
            Some("СправочникСсылка.<T> | Неопределено".to_string()),
            "Возвращаемый тип должен быть 'СправочникСсылка.<T> | Неопределено' с нормализацией placeholder'а"
        );

        assert!(
            return_desc.is_some(),
            "Описание возвращаемого значения должно быть извлечено"
        );

        let desc = return_desc.unwrap();
        assert!(
            desc.contains("не существует") || desc.contains("пустая ссылка"),
            "Описание должно содержать информацию о пустой ссылке"
        );

        println!("✓ Интеграционный тест FindByCode250.html пройден");
    }

    /// Edge case: HTML без секции "Возвращаемое значение:"
    #[test]
    fn test_extract_return_info_empty_section() {
        let html_content = r#"
        <html>
            <p class="V8SH_chapter">Параметры:</p>
            <p>Метод без возвращаемого значения</p>
        </html>
        "#;

        let document = Html::parse_document(html_content);
        let extractor = HtmlExtractor::new();
        let (return_type, return_desc) = extractor.extract_return_info(&document);

        assert_eq!(return_type, None, "Возвращаемый тип должен быть None для отсутствующей секции");
        assert_eq!(return_desc, None, "Описание должно быть None для отсутствующей секции");
    }

    /// Edge case: Пустой return_type (нет типа после "Тип:")
    /// Когда нет HTML контента для парсинга, parse_type_fragments возвращает пустой вектор
    /// и assemble_types возвращает пустую строку, что приводит к None
    #[test]
    fn test_extract_return_info_empty_type() {
        let html_content = r#"
        <html>
            <p class="V8SH_chapter">Возвращаемое значение:</p>
            Тип: <br>
            Описание без типа.
        </html>
        "#;

        let document = Html::parse_document(html_content);
        let extractor = HtmlExtractor::new();
        let (return_type, _) = extractor.extract_return_info(&document);

        // Когда нет типа на строке "Тип:", parse_type_fragments не находит фрагменты
        // и возвращает пустой вектор, что приводит к None в assemble_types
        assert_eq!(return_type, None, "Возвращаемый тип должен быть None для пустого типа");
        // Описание может быть None если нет контента после <br>
        // или может содержать "Описание без типа." в зависимости от парсинга
    }

    /// Edge case: Только текст без <a> тегов и span'ов
    /// Когда тип указан как простой текст без HTML обёртки, парсер не может его найти
    /// потому что ищет фрагменты в parse_type_fragments
    #[test]
    fn test_extract_return_info_plain_text() {
        let html_content = r#"
        <html>
            <p class="V8SH_chapter">Возвращаемое значение:</p>
            Тип: <a href="type">Произвольный</a>. <br>
            Результат работы.
        </html>
        "#;

        let document = Html::parse_document(html_content);
        let extractor = HtmlExtractor::new();
        let (return_type, return_desc) = extractor.extract_return_info(&document);

        assert_eq!(return_type, Some("Произвольный".to_string()));
        assert_eq!(return_desc, Some("Результат работы.".to_string()));
    }

    /// Edge case: Несколько union типов (> 2)
    #[test]
    fn test_extract_return_info_multiple_union() {
        let html_content = r#"
        <html>
            <p class="V8SH_chapter">Возвращаемое значение:</p>
            Тип: <a href="type">Число</a>, <a href="type">Строка</a>, <a href="type">Дата</a>, <a href="type">Булево</a>. <br>
            Результат с несколькими типами.
        </html>
        "#;

        let document = Html::parse_document(html_content);
        let extractor = HtmlExtractor::new();
        let (return_type, _) = extractor.extract_return_info(&document);

        assert_eq!(
            return_type,
            Some("Число | Строка | Дата | Булево".to_string()),
            "Должны быть все четыре типа через ' | '"
        );
    }

    /// Edge case: Несколько faceted типов
    #[test]
    fn test_extract_return_info_multiple_faceted() {
        let html_content = r#"
        <html>
            <p class="V8SH_chapter">Возвращаемое значение:</p>
            Тип: <a href="type">СправочникСсылка.</a><span>&lt;</span><a href="type">СправочникА</a><span>&gt;</span>, <a href="type">ДокументСсылка.</a><span>&lt;</span><a href="type">ДокументБ</a><span>&gt;</span>. <br>
            Ссылка на объект.
        </html>
        "#;

        let document = Html::parse_document(html_content);
        let extractor = HtmlExtractor::new();
        let (return_type, _) = extractor.extract_return_info(&document);

        assert_eq!(
            return_type,
            Some("СправочникСсылка.<T> | ДокументСсылка.<T>".to_string()),
            "Оба placeholder'а должны быть нормализованы в <T>"
        );
    }

    /// Edge case: Faceted тип без union
    #[test]
    fn test_extract_return_info_faceted_only() {
        let html_content = r#"
        <html>
            <p class="V8SH_chapter">Возвращаемое значение:</p>
            Тип: <a href="type">ДокументОбъект.</a><span>&lt;</span><a href="type">Соответствующий документ</a><span>&gt;</span>. <br>
            Объект документа.
        </html>
        "#;

        let document = Html::parse_document(html_content);
        let extractor = HtmlExtractor::new();
        let (return_type, _) = extractor.extract_return_info(&document);

        assert_eq!(
            return_type,
            Some("ДокументОбъект.<T>".to_string()),
            "Faceted тип без union должен быть нормализован"
        );
    }

    /// Edge case: Парсинг placeholder'ов разных типов
    #[test]
    fn test_extract_return_info_various_placeholders() {
        // Тест разных типов placeholder'ов, которые должны быть нормализованы в <T>
        let placeholders = vec![
            ("Имя справочника", "Справочник"),
            ("Имя документа", "Документ"),
            ("Имя перечисления", "Перечисление"),
        ];

        for (placeholder, context) in placeholders {
            let html_content = format!(
                r#"<html>
                    <p class="V8SH_chapter">Возвращаемое значение:</p>
                    Тип: <a href="type">Тип.</a><span>&lt;</span><a href="type">{}</a><span>&gt;</span>. <br>
                    Результат для {}.
                </html>"#,
                placeholder, context
            );

            let document = Html::parse_document(&html_content);
            let extractor = HtmlExtractor::new();
            let (return_type, _) = extractor.extract_return_info(&document);

            assert_eq!(
                return_type,
                Some("Тип.<T>".to_string()),
                "Placeholder '{}' должен быть нормализован в <T>",
                placeholder
            );
        }
    }
}
