//! Discovery-based парсер синтакс-помощника 1С
//! 
//! Вместо ожидания конкретных типов в конкретных местах,
//! этот парсер обнаруживает структуру по содержимому файлов

use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use scraper::{Html, Selector};
use tracing::{info, debug};
use super::syntax_helper_parser::{
    SyntaxHelperDatabase, ObjectInfo, MethodInfo, PropertyInfo, 
    ParameterInfo, TypeRef
};

/// Типы узлов в иерархии синтакс-помощника
#[derive(Debug, Clone, PartialEq)]
enum NodeType {
    /// Категория с подкатегориями/объектами (например catalog234)
    Category { 
        name: String,
        description: Option<String>,
    },
    /// Объект с методами и свойствами (например ValueTable) 
    Object {
        name: String,
        description: Option<String>,
        has_methods: bool,
        has_properties: bool,
        has_constructors: bool,
    },
    /// Метод объекта
    Method {
        object_path: String,
        name: String,
    },
    /// Свойство объекта
    Property {
        object_path: String,
        name: String,
    },
    /// Конструктор объекта
    Constructor {
        object_path: String,
        name: String,
    },
    /// Неизвестный тип (для отладки)
    Unknown,
}

/// Discovery-based парсер
pub struct SyntaxHelperParserV2 {
    /// База данных для хранения результатов
    database: SyntaxHelperDatabase,
    /// Базовый путь к распакованному архиву
    base_path: PathBuf,
    /// Счётчики для статистики
    processed_files: usize,
    discovered_objects: usize,
    discovered_methods: usize,
}

impl SyntaxHelperParserV2 {
    /// Создаёт новый парсер
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        Self {
            database: SyntaxHelperDatabase::default(),
            base_path: base_path.as_ref().to_path_buf(),
            processed_files: 0,
            discovered_objects: 0,
            discovered_methods: 0,
        }
    }
    
    /// Запускает discovery-based парсинг
    pub fn parse(&mut self) -> Result<()> {
        info!("Начинаем discovery-based парсинг синтакс-помощника");
        
        // Начинаем с корневой папки objects
        let objects_path = self.base_path.join("objects");
        if objects_path.exists() {
            self.discover_directory(&objects_path, "")?;
        }
        
        info!("Парсинг завершён. Статистика:");
        info!("  Обработано файлов: {}", self.processed_files);
        info!("  Обнаружено объектов: {}", self.discovered_objects);
        info!("  Обнаружено методов: {}", self.discovered_methods);
        info!("  Глобальных функций: {}", self.database.global_functions.len());
        info!("  Глобальных объектов: {}", self.database.global_objects.len());
        info!("  Методов объектов: {}", self.database.object_methods.len());
        info!("  Свойств объектов: {}", self.database.object_properties.len());
        
        Ok(())
    }
    
    /// Рекурсивно обходит директорию и обнаруживает структуру
    pub fn discover_directory(&mut self, dir: &Path, parent_path: &str) -> Result<()> {
        debug!("Обход директории: {:?}", dir);
        
        // Выводим прогресс каждые 100 файлов
        if self.processed_files % 100 == 0 && self.processed_files > 0 {
            info!("Обработано файлов: {}", self.processed_files);
        }
        
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();
            
            // Пропускаем служебные файлы
            if file_name.starts_with("__") || file_name.ends_with(".st") {
                continue;
            }
            
            if path.is_dir() {
                // Это поддиректория - определяем её тип
                let dir_type = self.determine_directory_type(&path);
                
                match dir_type {
                    DirType::Methods => {
                        // Папка с методами - парсим все методы внутри
                        self.discover_methods_directory(&path, parent_path)?;
                    }
                    DirType::Properties => {
                        // Папка со свойствами
                        self.discover_properties_directory(&path, parent_path)?;
                    }
                    DirType::Constructors => {
                        // Папка с конструкторами
                        self.discover_constructors_directory(&path, parent_path)?;
                    }
                    DirType::Regular => {
                        // Обычная папка - может содержать объекты или подкатегории
                        let new_parent = if parent_path.is_empty() {
                            file_name.clone()
                        } else {
                            format!("{}/{}", parent_path, file_name)
                        };
                        self.discover_directory(&path, &new_parent)?;
                    }
                }
            } else if path.extension().map_or(false, |ext| ext == "html") {
                // HTML файл - анализируем его содержимое
                self.process_html_file(&path, parent_path)?;
            }
        }
        
        Ok(())
    }
    
    /// Определяет тип директории по имени
    fn determine_directory_type(&self, dir: &Path) -> DirType {
        let name = dir.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
            
        match name {
            "methods" => DirType::Methods,
            "properties" => DirType::Properties,
            "ctors" => DirType::Constructors,
            _ => DirType::Regular,
        }
    }
    
    /// Обрабатывает HTML файл и определяет его тип по содержимому
    fn process_html_file(&mut self, file_path: &Path, parent_path: &str) -> Result<()> {
        self.processed_files += 1;
        
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("Не удалось прочитать файл {:?}", file_path))?;
        
        let document = Html::parse_document(&content);
        let node_type = self.analyze_html_content(&document, file_path);
        
        match node_type {
            NodeType::Object { name, has_methods, has_properties, has_constructors, .. } => {
                debug!("Обнаружен объект: {} (методы: {}, свойства: {})", 
                    name, has_methods, has_properties);
                
                self.discovered_objects += 1;
                
                // Создаём ObjectInfo
                let object_key = if parent_path.is_empty() {
                    name.clone()
                } else {
                    format!("{}/{}", parent_path, name)
                };
                
                let mut object_info = ObjectInfo {
                    name: name.clone(),
                    object_type: object_key.clone(),
                    description: self.extract_description(&document),
                    methods: vec![],
                    properties: vec![],
                    constructors: vec![],
                };
                
                // Извлекаем ссылки на методы и свойства
                if has_methods || has_properties {
                    self.extract_object_members(&document, &mut object_info);
                }
                
                self.database.global_objects.insert(object_key, object_info);
            }
            
            NodeType::Category { name, .. } => {
                debug!("Обнаружена категория: {}", name);
                // Категории просто обходим рекурсивно
            }
            
            _ => {
                // Другие типы обрабатываются в специализированных методах
            }
        }
        
        Ok(())
    }
    
    /// Анализирует HTML содержимое и определяет тип узла
    fn analyze_html_content(&self, document: &Html, file_path: &Path) -> NodeType {
        // Проверяем наличие характерных элементов
        
        // Есть ли ссылки на методы/свойства?
        let link_selector = Selector::parse("a").unwrap();
        let mut has_method_links = false;
        let mut has_property_links = false;
        let mut has_ctor_links = false;
        
        for element in document.select(&link_selector) {
            if let Some(href) = element.value().attr("href") {
                if href.contains("/methods/") {
                    has_method_links = true;
                } else if href.contains("/properties/") {
                    has_property_links = true;
                } else if href.contains("/ctors/") {
                    has_ctor_links = true;
                }
            }
        }
        
        // Извлекаем заголовок
        let title_selector = Selector::parse("h1.V8SH_pagetitle").unwrap();
        let title = document.select(&title_selector)
            .next()
            .map(|e| e.text().collect::<String>());
        
        // Определяем имя из пути файла
        let name = file_path.file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();
        
        if has_method_links || has_property_links || has_ctor_links {
            // Это объект с членами
            NodeType::Object {
                name,
                description: title,
                has_methods: has_method_links,
                has_properties: has_property_links,
                has_constructors: has_ctor_links,
            }
        } else if name.starts_with("catalog") {
            // Вероятно категория
            NodeType::Category {
                name,
                description: title,
            }
        } else {
            NodeType::Unknown
        }
    }
    
    /// Извлекает методы и свойства объекта из HTML
    fn extract_object_members(&self, document: &Html, object_info: &mut ObjectInfo) {
        let link_selector = Selector::parse("a").unwrap();
        
        for element in document.select(&link_selector) {
            if let Some(href) = element.value().attr("href") {
                let text = element.text().collect::<String>();
                
                if href.contains("/methods/") {
                    object_info.methods.push(text);
                } else if href.contains("/properties/") {
                    object_info.properties.push(text);
                } else if href.contains("/ctors/") {
                    object_info.constructors.push(text);
                }
            }
        }
    }
    
    /// Обходит директорию с методами
    fn discover_methods_directory(&mut self, dir: &Path, parent_path: &str) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().map_or(false, |ext| ext == "html") {
                self.discovered_methods += 1;
                self.parse_method_file(&path, parent_path)?;
            }
        }
        Ok(())
    }
    
    /// Обходит директорию со свойствами
    fn discover_properties_directory(&mut self, dir: &Path, parent_path: &str) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().map_or(false, |ext| ext == "html") {
                self.parse_property_file(&path, parent_path)?;
            }
        }
        Ok(())
    }
    
    /// Обходит директорию с конструкторами
    fn discover_constructors_directory(&mut self, dir: &Path, parent_path: &str) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().map_or(false, |ext| ext == "html") {
                // Конструкторы можно обрабатывать как специальные методы
                self.parse_method_file(&path, parent_path)?;
            }
        }
        Ok(())
    }
    
    /// Парсит файл метода
    fn parse_method_file(&mut self, file_path: &Path, object_path: &str) -> Result<()> {
        let content = fs::read_to_string(file_path)?;
        let document = Html::parse_document(&content);
        
        // Извлекаем имя из имени файла (например Add110.html -> Add110)
        let file_name = file_path.file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        
        // Извлекаем русское и английское имя из документа
        let method_name = self.extract_method_name(&document).unwrap_or(file_name.clone());
        let english_name = self.extract_english_name(&document);
        
        // Извлекаем информацию о методе
        let method_info = MethodInfo {
            name: method_name.clone(),
            object_type: object_path.to_string(),
            english_name,
            description: self.extract_description(&document),
            syntax: self.extract_syntax(&document).map(|s| vec![s]).unwrap_or_default(),
            parameters: self.extract_parameters(&document),
            return_type: self.extract_return_type(&document),
            return_description: None,
            examples: vec![],
            availability: vec![],
            facet: None,
        };
        
        // Используем имя файла как часть ключа (Add110, а не "Добавить (Add)")
        let key = format!("{}.{}", object_path, file_name);
        self.database.object_methods.insert(key, method_info);
        
        Ok(())
    }
    
    /// Парсит файл свойства
    fn parse_property_file(&mut self, file_path: &Path, object_path: &str) -> Result<()> {
        let content = fs::read_to_string(file_path)?;
        let document = Html::parse_document(&content);
        
        let property_name = file_path.file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        
        // Извлекаем информацию о свойстве
        let property_info = PropertyInfo {
            name: self.extract_property_name(&document).unwrap_or(property_name.clone()),
            object_type: object_path.to_string(),
            property_type: self.extract_property_type(&document),
            is_readonly: self.is_readonly_property(&document),
            description: self.extract_description(&document),
            availability: vec![],
            facet: None,
        };
        
        let key = format!("{}.{}", object_path, property_name);
        self.database.object_properties.insert(key, property_info);
        
        Ok(())
    }
    
    // === Вспомогательные методы для извлечения информации из HTML ===
    
    fn extract_description(&self, document: &Html) -> Option<String> {
        let selector = Selector::parse("p").unwrap();
        for element in document.select(&selector) {
            let text = element.text().collect::<String>().trim().to_string();
            if !text.is_empty() && !text.starts_with("Доступен") {
                return Some(text);
            }
        }
        None
    }
    
    fn extract_method_name(&self, document: &Html) -> Option<String> {
        let selector = Selector::parse("p.V8SH_heading").unwrap();
        document.select(&selector)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
    }
    
    fn extract_property_name(&self, document: &Html) -> Option<String> {
        self.extract_method_name(document) // Используем тот же селектор
    }
    
    fn extract_english_name(&self, document: &Html) -> Option<String> {
        // Ищем английское название в скобках
        let selector = Selector::parse("p.V8SH_heading, h1").unwrap();
        for element in document.select(&selector) {
            let text = element.text().collect::<String>();
            if let Some(start) = text.find('(') {
                if let Some(end) = text.find(')') {
                    let english = text[start + 1..end].trim();
                    if !english.is_empty() {
                        return Some(english.to_string());
                    }
                }
            }
        }
        None
    }
    
    fn extract_syntax(&self, document: &Html) -> Option<String> {
        // Ищем секцию "Синтаксис:"
        let text_selector = Selector::parse("p").unwrap();
        let mut found_syntax = false;
        
        for element in document.select(&text_selector) {
            let text = element.text().collect::<String>();
            if text.contains("Синтаксис:") {
                found_syntax = true;
            } else if found_syntax && !text.is_empty() {
                return Some(text.trim().to_string());
            }
        }
        None
    }
    
    fn extract_parameters(&self, document: &Html) -> Vec<ParameterInfo> {
        // TODO: Реализовать извлечение параметров
        vec![]
    }
    
    fn extract_return_type(&self, document: &Html) -> Option<TypeRef> {
        // TODO: Реализовать извлечение типа возвращаемого значения
        None
    }
    
    fn extract_property_type(&self, document: &Html) -> Option<TypeRef> {
        // TODO: Реализовать извлечение типа свойства
        None
    }
    
    fn is_readonly_property(&self, document: &Html) -> bool {
        // TODO: Определить, является ли свойство только для чтения
        false
    }
    
    /// Возвращает базу данных
    pub fn database(&self) -> &SyntaxHelperDatabase {
        &self.database
    }
    
    /// Возвращает изменяемую ссылку на базу данных
    pub fn database_mut(&mut self) -> &mut SyntaxHelperDatabase {
        &mut self.database
    }
}

/// Тип директории
#[derive(Debug, PartialEq)]
enum DirType {
    Methods,
    Properties,
    Constructors,
    Regular,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_discovery_parser() {
        let parser = SyntaxHelperParserV2::new("examples/syntax_helper/rebuilt.shcntx_ru");
        // Тесты...
    }
}