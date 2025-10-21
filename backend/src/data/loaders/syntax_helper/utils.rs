//! Вспомогательные функции для парсера

use scraper::{Html, Selector};
use std::path::Path;

use bsl_shared::domain::types::FacetKind;

/// Тип файла для парсинга
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Type,
    Method,
    Property,
    Category,
    Constructor,
    GlobalFunction,
}

/// Определяет тип файла по пути и содержимому
pub fn detect_file_type(path: &Path, document: &Html) -> FileType {
    // Проверяем, является ли это файлом категории в корне objects
    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
        if let Some(parent) = path.parent() {
            // Проверяем, что файл находится в корне objects
            if parent.file_name().and_then(|n| n.to_str()) == Some("objects") {
                // Для файлов в корне objects проверяем наличие одноименной директории
                if file_name.ends_with(".html") {
                    let dir_name = file_name.trim_end_matches(".html");
                    let potential_dir = parent.join(dir_name);

                    // Если есть одноименная директория или это Global context - это категория
                    if potential_dir.exists() && potential_dir.is_dir() {
                        return FileType::Category;
                    }
                    // Специальная обработка для Global context.html
                    if file_name == "Global context.html" {
                        return FileType::Category;
                    }
                }
            }

            // Для вложенных catalog*.html тоже проверяем
            if file_name.starts_with("catalog") && file_name.ends_with(".html") {
                let catalog_name = file_name.trim_end_matches(".html");
                let catalog_dir = parent.join(catalog_name);
                if catalog_dir.exists() && catalog_dir.is_dir() {
                    return FileType::Category;
                }
            }
        }
    }

    // Специальная проверка для глобальных функций в Global context
    if let Some(path_str) = path.to_str() {
        if path_str.contains("Global context") && path_str.contains("methods") {
            // Проверяем заголовок документа
            let title_selector = Selector::parse("h1.V8SH_pagetitle")
                .unwrap_or_else(|_| Selector::parse("h1").unwrap());

            if let Some(title_elem) = document.select(&title_selector).next() {
                let title = title_elem.text().collect::<String>();
                // Если заголовок начинается с "Глобальный контекст." или "Global context."
                // это глобальная функция
                if title.starts_with("Глобальный контекст.") || title.starts_with("Global context.")
                {
                    return FileType::GlobalFunction;
                }
            }
        }
    }

    // Проверяем по пути
    if let Some(parent) = path.parent() {
        if let Some(dir_name) = parent.file_name().and_then(|n| n.to_str()) {
            match dir_name {
                "methods" => return FileType::Method,
                "properties" => return FileType::Property,
                "constructors" => return FileType::Constructor,
                _ => {}
            }
        }
    }

    // Проверяем по содержимому
    let title_selector =
        Selector::parse("h1.V8SH_pagetitle").unwrap_or_else(|_| Selector::parse("h1").unwrap());

    if let Some(title_elem) = document.select(&title_selector).next() {
        let title = title_elem.text().collect::<String>();

        // Если в заголовке есть скобки - это тип
        if title.contains('(') && title.contains(')') {
            return FileType::Type;
        }

        // Если заголовок содержит "." - это метод
        if title.contains('.') && !title.contains("...") {
            return FileType::Method;
        }
    }

    // По умолчанию считаем типом
    FileType::Type
}

pub fn determine_facet_kind(_type_name: &str) -> Option<FacetKind> {
    // Заглушка - будет реализовано при необходимости
    None
}

pub fn build_path(path: &Path) -> String {
    path.to_string_lossy().to_string()
}
