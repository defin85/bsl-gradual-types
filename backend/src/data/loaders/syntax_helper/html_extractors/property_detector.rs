//! Детектор свойств типов из HTML документов синтакс-помощника

use scraper::Html;
use std::path::Path;

use bsl_shared::domain::types::FacetKind;

/// Детектор свойств и фасетов типов
pub struct PropertyDetector;

impl PropertyDetector {
    /// Проверяет, является ли тип read-only
    pub fn is_readonly(document: &Html) -> bool {
        let text = document.root_element().text().collect::<String>();
        text.contains("Только чтение") || text.contains("Read only")
    }

    /// Проверяет, поддерживает ли тип итерацию
    pub fn is_iterable(description: &str) -> bool {
        description.contains("Для каждого")
            || description.contains("For each")
            || description.contains("итерация")
            || description.contains("iteration")
    }

    /// Проверяет, поддерживает ли тип индексацию
    pub fn is_indexable(description: &str) -> bool {
        description.contains("индекс")
            || description.contains("index")
            || description.contains("[]")
    }

    /// Проверяет, является ли тип сериализуемым
    pub fn is_serializable(document: &Html) -> bool {
        let text = document.root_element().text().collect::<String>();
        text.contains("Сериализуемый")
            || text.contains("Serializable")
            || text.contains("XML")
            || text.contains("JSON")
    }

    /// Проверяет, поддерживает ли тип обмен данными
    pub fn is_exchangeable(document: &Html) -> bool {
        let text = document.root_element().text().collect::<String>();
        text.contains("Обмен данными") || text.contains("Data exchange") || text.contains("XDTO")
    }

    /// Определяет фасеты типа на основе имени и описания
    pub fn detect_facets(type_name: &str, description: &str) -> Vec<FacetKind> {
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

    /// Извлекает альтернативные имена типа
    pub fn extract_aliases(_document: &Html) -> Vec<String> {
        // TODO: Implement alias extraction
        Vec::new()
    }

    /// Извлекает тип элемента коллекции
    pub fn extract_collection_element(document: &Html) -> Option<String> {
        // В реальном Syntax Helper часто встречается строка вида:
        // "Элементы коллекции: <Type>" или "Collection elements: <Type>"
        //
        // Извлекаем из plain text, чтобы не зависеть от разметки (p/a/br и т.п.).
        //
        // В rebuilt.shcntx_ru это часто выглядит так:
        // <p class="V8SH_chapter">Элементы коллекции:</p>
        // <a>ДанныеФормыЭлементКоллекции</a><br>
        let raw_text = document.root_element().text().collect::<Vec<_>>().join(" ");
        let normalized = raw_text.split_whitespace().collect::<Vec<_>>().join(" ");

        let candidates = [
            "Элементы коллекции:",
            "Элементы коллекции",
            "Collection elements:",
            "Collection elements",
        ];

        for marker in candidates {
            if let Some(pos) = normalized.find(marker) {
                let mut rest = normalized[pos + marker.len()..].trim();
                if rest.starts_with(':') {
                    rest = rest[1..].trim();
                }

                if rest.is_empty() {
                    continue;
                }

                // Ограничиваем сегмент списком типов до описательного блока ("Для объекта ...").
                // Это снижает риск захвата служебного текста.
                let end_markers = ["Для объекта", "For object", "For the object"];
                let mut segment = rest;
                for end_marker in end_markers {
                    if let Some(end_pos) = segment.find(end_marker) {
                        segment = segment[..end_pos].trim();
                    }
                }

                // В некоторых типах элементы коллекции перечислены несколькими типами (гетерогенная коллекция),
                // например: "ГруппаФормы, ДекорацияФормы, ..."
                // Для таких случаев НЕ возвращаем один конкретный тип (чтобы не вводить в заблуждение).
                let mut found: Vec<String> = Vec::new();
                for part in segment.split(',') {
                    let part = part.trim();
                    if part.is_empty() {
                        continue;
                    }
                    // Берём первый токен из части.
                    let token_end = part
                        .find(|c: char| c.is_whitespace() || c == '(')
                        .unwrap_or(part.len());
                    let mut token = part[..token_end].trim().to_string();
                    while token.ends_with(['.', ',', ';', ':']) {
                        token.pop();
                    }
                    if token.is_empty() {
                        continue;
                    }
                    if !found.contains(&token) {
                        found.push(token);
                    }
                }

                if found.len() == 1 {
                    return Some(found.remove(0));
                }

                // 0 — не смогли извлечь; >1 — гетерогенная коллекция (возвращаем None).
                return None;
            }
        }

        None
    }

    /// Извлекает путь к категории из пути файла
    pub fn extract_category_path(_path: &Path) -> String {
        // Пока возвращаем пустую строку, так как категории будут установлены
        // позже в link_types_to_categories на основе каталогов категорий
        String::new()
    }

    /// Строит путь относительно корня синтакс-помощника
    pub fn build_path(path: &Path) -> String {
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
