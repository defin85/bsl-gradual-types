//! Парсинг различных типов документов из HTML

use anyhow::Result;
use scraper::{Html, Selector};
use std::path::Path;

use super::html_extractors::HtmlExtractor;
use super::types::*;

/// Парсер документов синтакс-помощника
pub struct DocumentParser {
    html_extractor: HtmlExtractor,
}

impl DocumentParser {
    pub fn new() -> Self {
        Self {
            html_extractor: HtmlExtractor::new(),
        }
    }

    /// Создаёт парсер с готовым экстрактором
    pub fn with_extractor(html_extractor: HtmlExtractor) -> Self {
        Self { html_extractor }
    }

    /// Получает ссылку на HTML экстрактор
    pub fn html_extractor(&self) -> &HtmlExtractor {
        &self.html_extractor
    }

    /// Парсит тип из документа
    pub fn parse_type(&self, path: &Path, document: &Html) -> Result<TypeInfo> {
        let title = self.html_extractor.extract_title(document);
        let (russian, english) = self.html_extractor.parse_title(&title);
        let description = self.html_extractor.extract_description(document);

        Ok(TypeInfo {
            identity: TypeIdentity {
                russian_name: russian.clone(),
                english_name: english,
                catalog_path: self.html_extractor.build_path(path),
                category_path: self.html_extractor.extract_category_path(path),
                aliases: self.html_extractor.extract_aliases(document),
            },
            documentation: TypeDocumentation {
                category_description: None,
                type_description: description.clone(),
                examples: self.html_extractor.extract_examples(document),
                availability: self.html_extractor.extract_availability(document),
                since_version: self.html_extractor.extract_version(document),
            },
            structure: TypeStructure {
                collection_element: self.html_extractor.extract_collection_element(document),
                // Извлекаем методы из HTML с двуязычными именами
                methods: self.html_extractor.extract_methods_from_html(document),
                // Извлекаем свойства из HTML с двуязычными именами
                properties: self.html_extractor.extract_properties_from_html(document),
                constructors: Vec::new(), // Будут заполнены позже
                iterable: self.html_extractor.is_iterable(&description),
                indexable: self.html_extractor.is_indexable(&description),
                enum_values: self.html_extractor.extract_enum_values_from_html(document),
            },
            metadata: TypeMetadata {
                available_facets: self.html_extractor.detect_facets(&russian, &description),
                default_facet: None,
                serializable: self.html_extractor.is_serializable(document),
                exchangeable: self.html_extractor.is_exchangeable(document),
                xdto_namespace: None,
                xdto_type: None,
            },
        })
    }

    /// Парсит метод из документа
    pub fn parse_method(&self, document: &Html) -> Result<MethodInfo> {
        let title = self.html_extractor.extract_title(document);
        let (name, _english_from_title) = self.html_extractor.parse_title(&title);
        let description = self.html_extractor.extract_description(document);
        let (return_type, return_description) = self.html_extractor.extract_return_info(document);
        let overloads = self.html_extractor.extract_method_overloads(document);
        let parameters = overloads
            .first()
            .map(|o| o.parameters.clone())
            .unwrap_or_else(|| self.html_extractor.extract_parameters(document));

        Ok(MethodInfo {
            name: name.clone(),
            english_name: self.html_extractor.extract_english_name(document),
            description: Some(description),
            overloads,
            parameters,
            return_type,
            return_description,
        })
    }

    /// Парсит глобальную функцию из документа
    pub fn parse_global_function(
        &self,
        path: &Path,
        document: &Html,
    ) -> Result<GlobalFunctionInfo> {
        // Извлекаем категорию из пути
        let category = Self::extract_function_category(path);

        // Извлекаем заголовок
        let title = self.html_extractor.extract_title(document);

        // Извлекаем имя функции из заголовка
        let (russian_name, english_name) = if title.contains('.') {
            let parts: Vec<&str> = title.split('.').collect();
            if parts.len() >= 2 {
                let func_part = parts[1..].join(".");
                if func_part.contains(" (") {
                    let russian = func_part.split(" (").next().unwrap_or(&func_part).trim();
                    let english = func_part.split(" (").nth(1).and_then(|s| {
                        if s.contains('.') {
                            s.split('.').nth(1).and_then(|name| {
                                name.split(')').next().map(|n| n.trim().to_string())
                            })
                        } else {
                            s.split(')').next().map(|n| n.trim().to_string())
                        }
                    });
                    (russian.to_string(), english)
                } else {
                    (func_part.trim().to_string(), None)
                }
            } else if title.contains(" (") {
                let russian = title.split(" (").next().unwrap_or(&title).trim();
                (russian.to_string(), None)
            } else {
                (title.clone(), None)
            }
        } else if title.contains(" (") {
            let russian = title.split(" (").next().unwrap_or(&title).trim();
            (russian.to_string(), None)
        } else {
            (title.clone(), None)
        };

        let description = self.html_extractor.extract_description(document);
        let parameters = self.html_extractor.extract_parameters(document);
        let (return_type, return_description) = self.html_extractor.extract_return_info(document);

        // Определяем характеристики функции
        let polymorphic = Self::is_polymorphic_function(&russian_name, &english_name);
        let pure = Self::is_pure_function(&russian_name, &english_name);
        let contexts = Self::extract_availability_contexts(document);

        Ok(GlobalFunctionInfo {
            name: russian_name,
            english_name,
            description: Some(description),
            parameters,
            return_type,
            return_description,
            polymorphic,
            pure,
            contexts,
            category,
        })
    }

    /// Парсит свойство из документа
    pub fn parse_property(&self, document: &Html) -> Result<PropertyInfo> {
        let title = self.html_extractor.extract_title(document);
        let (name, _english_from_title) = self.html_extractor.parse_title(&title);
        let description = self.html_extractor.extract_description(document);
        let property_type = self.html_extractor.extract_property_type(document);
        let is_readonly = self.html_extractor.is_readonly(document);

        Ok(PropertyInfo {
            name,
            property_type,
            is_readonly,
            description: Some(description),
        })
    }

    /// Парсит категорию из документа
    pub fn parse_category(&self, path: &Path, document: &Html) -> Result<CategoryInfo> {
        let name = self.html_extractor.extract_title(document);
        let description = self.html_extractor.extract_description(document);
        let related_links = self.html_extractor.extract_links(document);
        let types = self.html_extractor.extract_type_list(document);

        // Строим catalog_path относительно objects
        let catalog_id = if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
            let path_str = path.to_string_lossy();
            if let Some(objects_pos) = path_str.rfind("objects") {
                let after_objects = &path_str[objects_pos + 8..];
                let clean_path = after_objects
                    .trim_start_matches('/')
                    .trim_start_matches('\\')
                    .trim_end_matches(".html");

                if !clean_path.contains('/') && !clean_path.contains('\\') {
                    file_stem.to_string()
                } else {
                    clean_path
                        .replace('\\', "/")
                        .rsplit_once('/')
                        .map(|(dir, _)| format!("{}/{}", dir, file_stem))
                        .unwrap_or_else(|| file_stem.to_string())
                }
            } else {
                file_stem.to_string()
            }
        } else {
            "unknown".to_string()
        };

        Ok(CategoryInfo {
            name,
            catalog_path: catalog_id,
            description,
            related_links,
            types,
        })
    }

    /// Парсит конструктор из документа
    pub fn parse_constructor(&self, document: &Html) -> Result<ConstructorInfo> {
        let description = self.html_extractor.extract_description(document);
        let parameters = self.html_extractor.extract_parameters(document);

        Ok(ConstructorInfo {
            name: self.html_extractor.extract_title(document),
            description: Some(description),
            parameters,
        })
    }

    // ========== Вспомогательные методы для глобальных функций ==========

    /// Определяет, является ли функция полиморфной
    fn is_polymorphic_function(russian_name: &str, english_name: &Option<String>) -> bool {
        let polymorphic_functions = [
            "Мин",
            "Min",
            "Макс",
            "Max",
            "Строка",
            "String",
            "Число",
            "Number",
        ];

        polymorphic_functions.contains(&russian_name)
            || english_name
                .as_ref()
                .is_some_and(|en| polymorphic_functions.contains(&en.as_str()))
    }

    /// Определяет, является ли функция чистой
    fn is_pure_function(russian_name: &str, english_name: &Option<String>) -> bool {
        let pure_functions = vec![
            "Мин",
            "Min",
            "Макс",
            "Max",
            "Строка",
            "String",
            "Число",
            "Number",
            "Тип",
            "Type",
            "ТипЗнч",
            "TypeOf",
            "СтрДлина",
            "StrLen",
            "Лев",
            "Left",
            "Прав",
            "Right",
            "Окр",
            "Round",
            "Цел",
            "Int",
            "Лог",
            "Log",
        ];

        pure_functions.contains(&russian_name)
            || english_name
                .as_ref()
                .is_some_and(|en| pure_functions.contains(&en.as_str()))
    }

    /// Извлекает категорию функции из пути
    fn extract_function_category(path: &Path) -> Option<String> {
        let path_str = path.to_string_lossy();

        let pattern = if cfg!(windows) {
            r"methods\catalog"
        } else {
            "methods/catalog"
        };
        let separator = if cfg!(windows) { '\\' } else { '/' };

        if let Some(start) = path_str.find(pattern) {
            let skip_len = 8;
            let catalog_part = &path_str[start + skip_len..];
            if let Some(end) = catalog_part.find(separator) {
                let catalog_id = &catalog_part[..end];

                let category_file = path
                    .parent()
                    .and_then(|p| p.parent())
                    .map(|p| p.join(format!("{}.html", catalog_id)));

                if let Some(category_path) = category_file {
                    if category_path.exists() {
                        if let Ok(content) = std::fs::read_to_string(&category_path) {
                            if let Some(start) = content.find(r#"<h1 class="V8SH_pagetitle">"#) {
                                let start_pos = start + r#"<h1 class="V8SH_pagetitle">"#.len();
                                let title_part = &content[start_pos..];
                                if let Some(end) = title_part.find("</h1>") {
                                    return Some(title_part[..end].to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Извлекает контексты доступности функции
    fn extract_availability_contexts(document: &Html) -> Vec<String> {
        let Ok(availability_selector) = Selector::parse("p.V8SH_chapter + p") else {
            return Vec::new();
        };

        for element in document.select(&availability_selector) {
            let text = element.text().collect::<String>();
            if text.contains("клиент") || text.contains("сервер") || text.contains("соединение")
            {
                return text
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }

        Vec::new()
    }
}

impl Default for DocumentParser {
    fn default() -> Self {
        Self::new()
    }
}
