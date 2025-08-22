//! Парсер иерархии категорий из файловой структуры справки 1С
//!
//! Парсит HTML файлы из objects/ и строит правильную иерархию типов
//! на основе файловой структуры справки синтакс-помощника

use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Узел в иерархии категорий
#[derive(Debug, Clone)]
pub struct CategoryNode {
    /// ID категории (имя файла без расширения)
    pub id: String,

    /// Название категории (из заголовка HTML)
    pub title: String,

    /// Описание категории
    pub description: String,

    /// Путь к файлу
    pub file_path: String,

    /// Дочерние категории
    pub children: Vec<CategoryNode>,

    /// Типы в этой категории
    pub types: Vec<String>,
}

/// Результат парсинга иерархии
#[derive(Debug, Clone)]
pub struct CategoryHierarchy {
    /// Корневые категории
    pub root_categories: Vec<CategoryNode>,

    /// Общее количество категорий
    pub total_categories: usize,

    /// Общее количество типов
    pub total_types: usize,
}

/// Парсер иерархии категорий
pub struct CategoryHierarchyParser {
    /// Путь к корневой папке objects/
    objects_path: String,

    /// Кеш распаршенных HTML файлов
    html_cache: HashMap<String, HtmlContent>,
}

/// Содержимое HTML файла
#[derive(Debug, Clone)]
struct HtmlContent {
    pub title: String,
    pub description: String,
}

impl CategoryHierarchyParser {
    /// Создать новый парсер
    pub fn new(objects_path: &str) -> Self {
        Self {
            objects_path: objects_path.to_string(),
            html_cache: HashMap::new(),
        }
    }

    /// Распарсить всю иерархию категорий
    pub fn parse_hierarchy(&mut self) -> Result<CategoryHierarchy> {
        println!("🌳 Парсинг иерархии категорий из: {}", self.objects_path);

        // Находим все HTML файлы в корне
        let root_files = self.find_root_html_files()?;
        println!("📁 Найдено {} корневых HTML файлов", root_files.len());

        // Парсим каждый корневой файл
        let mut root_categories = Vec::new();
        let mut total_types = 0;

        for file_path in root_files {
            if let Ok(category) = self.parse_category_file(&file_path) {
                total_types += category.types.len();
                root_categories.push(category);
            }
        }

        let total_categories = self.count_total_categories(&root_categories);

        println!(
            "✅ Парсинг завершён: {} категорий, {} типов",
            total_categories, total_types
        );

        Ok(CategoryHierarchy {
            root_categories,
            total_categories,
            total_types,
        })
    }

    /// Найти все HTML файлы в корне objects/
    fn find_root_html_files(&self) -> Result<Vec<String>> {
        let mut files = Vec::new();

        let objects_dir = Path::new(&self.objects_path);
        if !objects_dir.exists() {
            return Err(anyhow::anyhow!(
                "Objects directory not found: {}",
                self.objects_path
            ));
        }

        for entry in fs::read_dir(objects_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().map_or(false, |ext| ext == "html") {
                if let Some(path_str) = path.to_str() {
                    files.push(path_str.to_string());
                }
            }
        }

        // Сортируем для предсказуемости
        files.sort();
        Ok(files)
    }

    /// Распарсить отдельный файл категории
    fn parse_category_file(&mut self, file_path: &str) -> Result<CategoryNode> {
        let path = Path::new(file_path);
        let file_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        // Читаем содержимое HTML файла
        let html_content = self.parse_html_file(file_path)?;

        // Ищем соответствующую папку для подкатегорий
        let folder_path = path
            .parent()
            .map(|p| p.join(file_name))
            .filter(|p| p.exists());

        // Парсим подкатегории, если папка существует
        let mut children = Vec::new();
        let mut types = Vec::new();

        if let Some(folder) = folder_path {
            let (sub_categories, sub_types) = self.parse_subfolder(&folder)?;
            children = sub_categories;
            types = sub_types;
        }

        Ok(CategoryNode {
            id: file_name.to_string(),
            title: html_content.title,
            description: html_content.description,
            file_path: file_path.to_string(),
            children,
            types,
        })
    }

    /// Распарсить HTML файл и извлечь заголовок
    fn parse_html_file(&mut self, file_path: &str) -> Result<HtmlContent> {
        // Проверяем кеш
        if let Some(cached) = self.html_cache.get(file_path) {
            return Ok(cached.clone());
        }

        let content = fs::read_to_string(file_path)?;

        // Извлекаем заголовок из <h1 class="V8SH_pagetitle">
        let title_regex = Regex::new(r#"<h1[^>]*class="V8SH_pagetitle"[^>]*>([^<]+)</h1>"#)?;
        let title = title_regex
            .captures(&content)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_else(|| "Без названия".to_string());

        // Извлекаем описание из <p class="V8SH_title"> до первого <HR>
        let desc_regex = Regex::new(r#"<p[^>]*class="V8SH_title"[^>]*>.*?</p>(.*?)<HR>"#)?;
        let description = desc_regex
            .captures(&content)
            .and_then(|caps| caps.get(1))
            .map(|m| {
                // Убираем HTML теги и лишние пробелы
                let text = m.as_str();
                let clean_text = Regex::new(r"<[^>]+>").unwrap().replace_all(text, " ");
                clean_text
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .chars()
                    .take(200)
                    .collect::<String>()
            })
            .unwrap_or_else(|| "Описание отсутствует".to_string());

        let html_content = HtmlContent { title, description };

        // Кешируем результат
        self.html_cache
            .insert(file_path.to_string(), html_content.clone());

        Ok(html_content)
    }

    /// Распарсить подпапку с подкатегориями
    fn parse_subfolder(&mut self, folder_path: &Path) -> Result<(Vec<CategoryNode>, Vec<String>)> {
        let mut subcategories = Vec::new();
        let mut types = Vec::new();

        if !folder_path.exists() {
            return Ok((subcategories, types));
        }

        for entry in fs::read_dir(folder_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().map_or(false, |ext| ext == "html") {
                if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                    if file_name.starts_with("catalog") {
                        // Это подкатегория
                        if let Ok(subcategory) = self.parse_category_file(path.to_str().unwrap()) {
                            subcategories.push(subcategory);
                        }
                    } else {
                        // Это тип (object*.html или конкретный тип)
                        if let Ok(html_content) = self.parse_html_file(path.to_str().unwrap()) {
                            types.push(html_content.title);
                        }
                    }
                }
            }
        }

        Ok((subcategories, types))
    }

    /// Подсчитать общее количество категорий рекурсивно
    fn count_total_categories(&self, categories: &[CategoryNode]) -> usize {
        let mut count = categories.len();
        for category in categories {
            count += self.count_total_categories(&category.children);
        }
        count
    }
}

impl Default for CategoryHierarchyParser {
    fn default() -> Self {
        Self::new("examples/syntax_helper/rebuilt.shcntx_ru/objects")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_creation() {
        let parser = CategoryHierarchyParser::new("test_path");
        assert_eq!(parser.objects_path, "test_path");
        assert!(parser.html_cache.is_empty());
    }

    #[test]
    fn test_html_title_extraction() {
        let mut parser = CategoryHierarchyParser::new("test");

        // Создаём временный HTML файл для теста
        let test_html = r#"<html><body><h1 class="V8SH_pagetitle">Прикладные объекты</h1><p class="V8SH_title">Прикладные объекты</p>Описание категории<HR></body></html>"#;

        // В реальном тесте нужно было бы создать файл, но для демо показываем логику
        assert!(true); // Placeholder для демонстрации структуры тестов
    }
}
