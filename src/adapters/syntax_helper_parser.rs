//! Парсер синтакс-помощника 1С для извлечения информации о типах платформы
//! 
//! Работает с файлами:
//! - rebuilt.shcntx_ru.zip - контекстная справка (методы, объекты, свойства)
//! - rebuilt.shlang_ru.zip - справка по языку (операторы, ключевые слова)
//!
//! Использует потоковый парсинг для минимизации потребления памяти

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, BufReader};
use std::path::Path;
use anyhow::{Result, Context};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use zip::ZipArchive;
use tracing::{debug, info, warn};

const MAX_HTML_SIZE: usize = 10 * 1024 * 1024; // 10MB лимит для HTML файла
const BUFFER_SIZE: usize = 8192; // 8KB буфер для чтения

/// База знаний синтакс-помощника
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyntaxHelperDatabase {
    /// Глобальные функции (Сообщить, Тип, XMLСтрока и т.д.)
    pub global_functions: HashMap<String, FunctionInfo>,
    
    /// Глобальные объекты (Справочники, Документы, РегистрыСведений и т.д.)
    pub global_objects: HashMap<String, ObjectInfo>,
    
    /// Методы объектов (ключ: "ОбъектТип.МетодИмя")
    pub object_methods: HashMap<String, MethodInfo>,
    
    /// Свойства объектов (ключ: "ОбъектТип.СвойствоИмя")
    pub object_properties: HashMap<String, PropertyInfo>,
    
    /// Системные перечисления (РежимЗаписиДокумента, ВидДвиженияНакопления и т.д.)
    pub system_enums: HashMap<String, EnumInfo>,
    
    /// Ключевые слова языка
    pub keywords: Vec<KeywordInfo>,
    
    /// Операторы языка
    pub operators: Vec<OperatorInfo>,
}

/// Информация о функции
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub english_name: Option<String>,
    pub description: Option<String>,
    pub syntax: Vec<String>,
    pub parameters: Vec<ParameterInfo>,
    pub return_type: Option<TypeRef>,  // Изменено на TypeRef
    pub return_description: Option<String>,
    pub examples: Vec<String>,
    pub availability: Vec<String>, // Клиент, Сервер, МобильноеПриложение и т.д.
}

/// Ссылка на тип (нормализованная)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeRef {
    pub id: String,              // "language:def_String" или "context:objects/catalog234/Array.html"
    pub name_ru: String,         // "Строка"
    pub name_en: Option<String>, // "String"
    pub kind: TypeRefKind,       // language, context, metadata_ref
}

/// Вид ссылки на тип
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeRefKind {
    Language,       // Языковые типы (Строка, Число, Булево)
    Context,        // Платформенные типы (Массив, СправочникСсылка)
    MetadataRef,    // Ссылки на метаданные (СправочникСсылка.Контрагенты)
}

/// Информация о параметре
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterInfo {
    pub name: String,
    pub type_ref: Option<TypeRef>,  // Изменено на TypeRef
    pub is_optional: bool,
    pub default_value: Option<String>,
    pub description: Option<String>,
}

/// Информация об объекте
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectInfo {
    pub name: String,
    pub object_type: String, // Manager, Object, Reference, Metadata
    pub description: Option<String>,
    pub methods: Vec<String>,
    pub properties: Vec<String>,
    pub constructors: Vec<String>,
}

/// Информация о методе
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodInfo {
    pub name: String,
    pub object_type: String,
    pub english_name: Option<String>,
    pub description: Option<String>,
    pub syntax: Vec<String>,
    pub parameters: Vec<ParameterInfo>,
    pub return_type: Option<TypeRef>,  // Изменено на TypeRef
    pub return_description: Option<String>,
    pub examples: Vec<String>,
    pub availability: Vec<String>,
    pub facet: Option<FacetKind>,  // Добавлено для определения фасета
}

/// Информация о свойстве
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyInfo {
    pub name: String,
    pub object_type: String,
    pub property_type: Option<TypeRef>,  // Изменено на TypeRef
    pub is_readonly: bool,
    pub description: Option<String>,
    pub availability: Vec<String>,
    pub facet: Option<FacetKind>,  // Добавлено для определения фасета
}

/// Вид фасета (для связи с основной системой типов)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FacetKind {
    Manager,     // CatalogManager, DocumentManager
    Object,      // CatalogObject, DocumentObject
    Reference,   // CatalogRef, DocumentRef
    Selection,   // CatalogSelection, DocumentSelection
    Metadata,    // ОбъектМетаданных
    Constructor, // Конструируемые типы (Массив, Структура)
}

/// Информация о перечислении
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumInfo {
    pub name: String,
    pub description: Option<String>,
    pub values: Vec<EnumValueInfo>,
}

/// Значение перечисления
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumValueInfo {
    pub name: String,
    pub description: Option<String>,
}

/// Информация о ключевом слове
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordInfo {
    pub russian: String,
    pub english: String,
    pub category: KeywordCategory,
    pub description: Option<String>,
}

/// Категория ключевого слова
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KeywordCategory {
    Structure,     // struct_ - управляющие конструкции (Если, Для, Пока)
    Definition,    // def_ - определения (Процедура, Функция, Перем)
    Root,          // root_ - корневые элементы (Новый, Выполнить)
    Operator,      // operator_ - операторы
    Instruction,   // Instructions_ - инструкции
    Other,         // Прочее
}

/// Информация об операторе
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorInfo {
    pub symbol: String,
    pub name: String,
    pub description: Option<String>,
    pub precedence: i32,
}

/// Парсер синтакс-помощника (потоковый)
pub struct SyntaxHelperParser {
    context_archive_path: Option<String>,
    lang_archive_path: Option<String>,
    database: SyntaxHelperDatabase,
    processed_files: usize,
    skipped_files: usize,
}

impl SyntaxHelperParser {
    /// Создаёт новый парсер
    pub fn new() -> Self {
        Self {
            context_archive_path: None,
            lang_archive_path: None,
            database: SyntaxHelperDatabase {
                global_functions: HashMap::new(),
                global_objects: HashMap::new(),
                object_methods: HashMap::new(),
                object_properties: HashMap::new(),
                system_enums: HashMap::new(),
                keywords: Vec::new(),
                operators: Vec::new(),
            },
            processed_files: 0,
            skipped_files: 0,
        }
    }
    
    /// Устанавливает путь к архиву контекстной справки
    pub fn with_context_archive<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.context_archive_path = Some(path.as_ref().to_string_lossy().to_string());
        self
    }
    
    /// Устанавливает путь к архиву справки по языку
    pub fn with_lang_archive<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.lang_archive_path = Some(path.as_ref().to_string_lossy().to_string());
        self
    }
    
    /// Парсит все доступные архивы и строит базу знаний (потоково)
    pub fn parse(&mut self) -> Result<()> {
        info!("Начинаем потоковый парсинг синтакс-помощника");
        
        // Парсим контекстную справку
        if let Some(ref path) = self.context_archive_path.clone() {
            info!("Парсинг контекстной справки: {}", path);
            self.parse_context_archive(path)?;
        }
        
        // Парсим справку по языку
        if let Some(ref path) = self.lang_archive_path.clone() {
            info!("Парсинг справки по языку: {}", path);
            self.parse_lang_archive(path)?;
        }
        
        info!("Парсинг завершён. Найдено:");
        info!("  - Глобальных функций: {}", self.database.global_functions.len());
        info!("  - Глобальных объектов: {}", self.database.global_objects.len());
        info!("  - Методов объектов: {}", self.database.object_methods.len());
        info!("  - Свойств объектов: {}", self.database.object_properties.len());
        info!("  - Системных перечислений: {}", self.database.system_enums.len());
        info!("  - Ключевых слов: {}", self.database.keywords.len());
        info!("  Обработано файлов: {}, пропущено: {}", self.processed_files, self.skipped_files);
        
        Ok(())
    }
    
    /// Парсит архив контекстной справки потоково
    fn parse_context_archive(&mut self, path: &str) -> Result<()> {
        let file = File::open(path)
            .with_context(|| format!("Не удалось открыть файл: {}", path))?;
            
        let reader = BufReader::new(file);
        let mut archive = ZipArchive::new(reader)?;
        
        debug!("Потоковый парсинг архива: {} файлов", archive.len());
        
        // Ищем только нужные файлы
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name().to_string();
            
            // Обрабатываем только нужные файлы
            if name == "objects/Global context.html" {
                info!("Найден файл глобального контекста");
                self.parse_html_file(&mut file, &name)?;
                self.processed_files += 1;
            } else if name.contains("/properties/") && name.ends_with(".html") {
                // Обрабатываем свойства объектов
                debug!("Обрабатываем свойство: {}", name);
                self.parse_property_file(&mut file, &name)?;
                self.processed_files += 1;
            } else if name.contains("/methods/") && name.ends_with(".html") {
                // Обрабатываем методы объектов  
                debug!("Обрабатываем метод: {}", name);
                self.parse_method_file(&mut file, &name)?;
                self.processed_files += 1;
            } else if name.starts_with("objects/") && name.ends_with(".html") && !name.contains("Global context") {
                // Обрабатываем объекты по необходимости
                if self.should_process_object_file(&name) {
                    debug!("Обрабатываем объект: {}", name);
                    self.parse_object_file(&mut file, &name)?;
                    self.processed_files += 1;
                } else {
                    self.skipped_files += 1;
                }
            } else {
                self.skipped_files += 1;
            }
        }
        
        Ok(())
    }
    
    /// Определяет, нужно ли обрабатывать файл объекта
    fn should_process_object_file(&self, name: &str) -> bool {
        // Обрабатываем все catalog файлы (содержат глобальные объекты и перечисления)
        // А также файлы с типами данных из catalog234
        if name.contains("catalog") && name.ends_with(".html") {
            return true;
        }
        
        // Также обрабатываем известные типы объектов
        let important_objects = [
            "String", "Number", "Date", "Boolean", "Array", "Structure", "Map",
            "Строка", "Число", "Дата", "Булево", "Массив", "Структура", "Соответствие",
            "ValueTable", "ValueList", "ValueTree", "ТаблицаЗначений", "СписокЗначений", "ДеревоЗначений",
            "Query", "QueryBuilder", "Запрос", "ПостроительЗапроса",
            "XMLReader", "XMLWriter", "ЧтениеXML", "ЗаписьXML",
            "TextDocument", "ТекстовыйДокумент"
        ];
        
        important_objects.iter().any(|obj| name.contains(obj))
    }
    
    /// Парсит HTML файл потоково с ограничением памяти
    fn parse_html_file<R: Read>(&mut self, reader: &mut R, filename: &str) -> Result<()> {
        // Читаем файл по частям с ограничением размера
        let mut content = Vec::new();
        let mut buffer = [0; BUFFER_SIZE];
        let mut total_read = 0;
        
        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            
            total_read += bytes_read;
            if total_read > MAX_HTML_SIZE {
                warn!("Файл {} слишком большой ({}MB), пропускаем", filename, total_read / 1024 / 1024);
                return Ok(());
            }
            
            content.extend_from_slice(&buffer[..bytes_read]);
        }
        
        // Конвертируем в строку и парсим
        let html_content = String::from_utf8_lossy(&content);
        let document = Html::parse_document(&html_content);
        
        if filename == "objects/Global context.html" {
            self.extract_global_functions(&document)?;
        }
        
        Ok(())
    }
    
    /// Парсит файл объекта
    fn parse_object_file<R: Read>(&mut self, reader: &mut R, filename: &str) -> Result<()> {
        let content = self.read_html_content(reader, filename)?;
        if content.is_empty() {
            return Ok(());
        }
        
        let document = Html::parse_document(&content);
        
        // Проверяем, является ли это catalog файлом
        if filename.starts_with("objects/catalog") {
            if filename.matches('/').count() == 1 {
                // Это корневой catalog файл (objects/catalog2.html)
                // catalog2.html - Системные перечисления
                // catalog234.html - Основные типы данных
                // catalog125.html - Прикладные объекты
                return self.parse_catalog_file(&document, filename);
            } else if filename.contains("catalog234/") {
                // Это объект из catalog234 - основные типы данных
                // objects/catalog234/Array.html, objects/catalog234/String.html и т.д.
                return self.parse_core_type_file(&document, filename);
            } else if filename.contains("catalog2/") {
                // Это системное перечисление из catalog2
                return self.parse_system_enum_file(&document, filename);
            }
        }
        
        // Извлекаем имя объекта из пути: objects/СправочникМенеджер.Контрагенты.html
        let object_name = self.extract_object_name_from_path(filename);
        
        // Определяем фасет
        let facet = self.detect_facet(&object_name);
        
        // Создаём ObjectInfo
        let mut object_info = ObjectInfo {
            name: object_name.clone(),
            object_type: object_name.clone(),
            description: self.extract_description(&document),
            methods: vec![],
            properties: vec![],
            constructors: vec![],
        };
        
        // Извлекаем ссылки на методы и свойства
        let link_selector = Selector::parse("a").unwrap();
        for element in document.select(&link_selector) {
            if let Some(href) = element.value().attr("href") {
                let text = element.text().collect::<String>();
                
                if href.contains("/methods/") {
                    object_info.methods.push(text);
                } else if href.contains("/properties/") {
                    object_info.properties.push(text);
                }
            }
        }
        
        self.database.global_objects.insert(object_name, object_info);
        Ok(())
    }
    
    /// Парсит файл свойства
    fn parse_property_file<R: Read>(&mut self, reader: &mut R, filename: &str) -> Result<()> {
        let content = self.read_html_content(reader, filename)?;
        if content.is_empty() {
            return Ok(());
        }
        
        let document = Html::parse_document(&content);
        
        // Извлекаем имя объекта и свойства из пути: objects/Array/properties/Count.html
        let (object_type, property_name) = self.extract_object_and_member_from_path(filename, "properties");
        
        // Определяем фасет
        let facet = self.detect_facet(&object_type);
        
        // Извлекаем тип свойства
        let property_type = self.extract_type_ref(&document, "Тип:");
        
        // Проверяем, является ли свойство readonly (ищем текст "Только чтение")
        let is_readonly = document.html().contains("Только чтение") || 
                          document.html().contains("Доступ: Чтение");
        
        let property_info = PropertyInfo {
            name: property_name.clone(),
            object_type: object_type.clone(),
            property_type,
            is_readonly,
            description: self.extract_description(&document),
            availability: self.extract_availability(&document),
            facet,
        };
        
        let key = format!("{}.{}", object_type, property_name);
        self.database.object_properties.insert(key, property_info);
        
        Ok(())
    }
    
    /// Парсит файл метода
    fn parse_method_file<R: Read>(&mut self, reader: &mut R, filename: &str) -> Result<()> {
        let content = self.read_html_content(reader, filename)?;
        if content.is_empty() {
            return Ok(());
        }
        
        let document = Html::parse_document(&content);
        
        // Извлекаем имя объекта и метода из пути: objects/Array/methods/Add.html
        let (object_type, method_name) = self.extract_object_and_member_from_path(filename, "methods");
        
        // Определяем фасет
        let facet = self.detect_facet(&object_type);
        
        // Извлекаем параметры и возвращаемый тип
        let parameters = self.extract_parameters(&document);
        let return_type = self.extract_type_ref(&document, "Возвращаемое значение:");
        
        // Извлекаем синтаксис
        let syntax = self.extract_syntax(&document);
        
        let method_info = MethodInfo {
            name: method_name.clone(),
            object_type: object_type.clone(),
            english_name: self.extract_english_name(&document),
            description: self.extract_description(&document),
            syntax,
            parameters,
            return_type,
            return_description: self.extract_return_description(&document),
            examples: self.extract_examples(&document),
            availability: self.extract_availability(&document),
            facet,
        };
        
        let key = format!("{}.{}", object_type, method_name);
        self.database.object_methods.insert(key, method_info);
        
        Ok(())
    }
    
    /// Читает HTML контент с ограничением размера
    fn read_html_content<R: Read>(&mut self, reader: &mut R, filename: &str) -> Result<String> {
        let mut content = Vec::new();
        let mut buffer = [0; BUFFER_SIZE];
        let mut total_read = 0;
        
        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            
            total_read += bytes_read;
            if total_read > MAX_HTML_SIZE {
                warn!("Файл {} слишком большой ({}MB), пропускаем", filename, total_read / 1024 / 1024);
                return Ok(String::new());
            }
            
            content.extend_from_slice(&buffer[..bytes_read]);
        }
        
        Ok(String::from_utf8_lossy(&content).to_string())
    }
    
    /// Парсит файл основного типа данных из catalog234
    fn parse_core_type_file(&mut self, document: &Html, filename: &str) -> Result<()> {
        // Извлекаем имя типа из пути: objects/catalog234/Array.html -> Array
        let type_name = filename
            .replace("objects/catalog234/", "")
            .replace(".html", "");
        
        info!("Найден основной тип данных: {}", type_name);
        
        // Создаём ObjectInfo для типа
        let mut object_info = ObjectInfo {
            name: type_name.clone(),
            object_type: type_name.clone(),
            description: self.extract_description(document),
            methods: vec![],
            properties: vec![],
            constructors: vec![],
        };
        
        // Извлекаем ссылки на методы и свойства
        let link_selector = Selector::parse("a").unwrap();
        for element in document.select(&link_selector) {
            if let Some(href) = element.value().attr("href") {
                let text = element.text().collect::<String>();
                
                if href.contains("/methods/") {
                    object_info.methods.push(text);
                } else if href.contains("/properties/") {
                    object_info.properties.push(text);
                }
            }
        }
        
        self.database.global_objects.insert(type_name, object_info);
        Ok(())
    }
    
    /// Парсит файл системного перечисления из catalog2
    fn parse_system_enum_file(&mut self, document: &Html, filename: &str) -> Result<()> {
        // Извлекаем имя перечисления из пути
        let enum_name = filename
            .replace("objects/catalog2/", "")
            .replace(".html", "");
        
        info!("Найдено системное перечисление: {}", enum_name);
        
        // Создаём EnumInfo
        let mut enum_info = EnumInfo {
            name: enum_name.clone(),
            description: self.extract_description(document),
            values: vec![],
        };
        
        // Извлекаем значения перечисления
        // В 1С значения перечислений обычно представлены как ссылки или элементы списка
        let link_selector = Selector::parse("a").unwrap();
        for element in document.select(&link_selector) {
            let text = element.text().collect::<String>();
            // Фильтруем только значения перечисления (не служебные ссылки)
            if !text.contains("Методическая") && !text.is_empty() {
                enum_info.values.push(EnumValueInfo {
                    name: text,
                    description: None,
                });
            }
        }
        
        self.database.system_enums.insert(enum_name, enum_info);
        Ok(())
    }
    
    /// Парсит catalog файл с описанием категории объектов
    fn parse_catalog_file(&mut self, document: &Html, filename: &str) -> Result<()> {
        // Извлекаем заголовок для определения типа catalog
        let title_selector = Selector::parse("h1.V8SH_pagetitle, p.V8SH_title").unwrap();
        let title = document.select(&title_selector).next()
            .map(|e| e.text().collect::<String>())
            .unwrap_or_default();
        
        debug!("Парсинг catalog файла: {}, заголовок: {}", filename, title);
        
        // Определяем тип по заголовку
        if title.contains("Системные перечисления") {
            // catalog2.html - системные перечисления
            info!("Найден каталог системных перечислений");
            // TODO: Извлечь список перечислений из содержимого
            // Пока просто помечаем, что это системные перечисления
            let enum_info = EnumInfo {
                name: "SystemEnumerations".to_string(),
                description: Some(title),
                values: vec![],
            };
            self.database.system_enums.insert("SystemEnumerations".to_string(), enum_info);
        } else if title.contains("Прикладные объекты") {
            // catalog125.html - прикладные объекты
            info!("Найден каталог прикладных объектов");
        } else if title.contains("Универсальные объекты") || title.contains("Universal objects") {
            // catalog234.html и подобные - основные типы данных
            info!("Найден каталог универсальных объектов");
        }
        
        Ok(())
    }
    
    /// Извлекает имя объекта из пути файла
    fn extract_object_name_from_path(&self, path: &str) -> String {
        // objects/СправочникМенеджер.Контрагенты.html -> СправочникМенеджер.Контрагенты
        path.replace("objects/", "")
            .replace(".html", "")
            .split('/')
            .next()
            .unwrap_or("")
            .to_string()
    }
    
    /// Извлекает имя объекта и члена из пути
    fn extract_object_and_member_from_path(&self, path: &str, member_type: &str) -> (String, String) {
        // objects/Array/properties/Count.html -> (Array, Count)
        let parts: Vec<&str> = path.split('/').collect();
        
        let object_type = parts.get(1).unwrap_or(&"").to_string();
        let member_name = parts.last()
            .unwrap_or(&"")
            .replace(".html", "");
        
        (object_type, member_name)
    }
    
    /// Извлекает описание из HTML
    fn extract_description(&self, document: &Html) -> Option<String> {
        let p_selector = Selector::parse("p").unwrap();
        
        // Берём первый параграф как описание
        document.select(&p_selector)
            .next()
            .map(|elem| elem.text().collect::<String>().trim().to_string())
            .filter(|s| !s.is_empty())
    }
    
    /// Извлекает параметры метода
    fn extract_parameters(&self, document: &Html) -> Vec<ParameterInfo> {
        let parameters = Vec::new();
        
        // Ищем секцию "Параметры:"
        let text = document.html();
        if let Some(params_start) = text.find("Параметры:") {
            // TODO: Более детальный парсинг параметров из таблицы
            // Пока возвращаем пустой вектор
        }
        
        parameters
    }
    
    /// Извлекает синтаксис метода
    fn extract_syntax(&self, document: &Html) -> Vec<String> {
        let mut syntax = Vec::new();
        
        // Ищем элементы с классом syntax или code
        let code_selector = Selector::parse("code, .syntax, .code").unwrap();
        
        for element in document.select(&code_selector) {
            let text = element.text().collect::<String>().trim().to_string();
            if !text.is_empty() {
                syntax.push(text);
            }
        }
        
        syntax
    }
    
    /// Извлекает английское название
    fn extract_english_name(&self, document: &Html) -> Option<String> {
        // Ищем в заголовке паттерн "Русское (English)"
        let h1_selector = Selector::parse("h1").unwrap();
        
        if let Some(h1) = document.select(&h1_selector).next() {
            let text = h1.text().collect::<String>();
            let (_, english) = self.parse_function_name(&text);
            return english;
        }
        
        None
    }
    
    /// Извлекает описание возвращаемого значения
    fn extract_return_description(&self, document: &Html) -> Option<String> {
        // Ищем текст после "Возвращаемое значение:"
        let text = document.html();
        if let Some(ret_start) = text.find("Возвращаемое значение:") {
            // TODO: Более точный парсинг
            None
        } else {
            None
        }
    }
    
    /// Извлекает примеры кода
    fn extract_examples(&self, document: &Html) -> Vec<String> {
        let mut examples = Vec::new();
        
        // Ищем элементы с примерами кода
        let example_selector = Selector::parse("pre, .example").unwrap();
        
        for element in document.select(&example_selector) {
            let text = element.text().collect::<String>().trim().to_string();
            if !text.is_empty() {
                examples.push(text);
            }
        }
        
        examples
    }
    
    /// Извлекает доступность (Клиент, Сервер и т.д.)
    fn extract_availability(&self, document: &Html) -> Vec<String> {
        let mut availability = Vec::new();
        
        let text = document.html();
        
        // Проверяем наличие ключевых слов
        if text.contains("Сервер") || text.contains("Server") {
            availability.push("Сервер".to_string());
        }
        if text.contains("Клиент") || text.contains("Client") {
            availability.push("Клиент".to_string());
        }
        if text.contains("МобильноеПриложение") || text.contains("MobileApp") {
            availability.push("МобильноеПриложение".to_string());
        }
        if text.contains("ВнешнееСоединение") || text.contains("ExternalConnection") {
            availability.push("ВнешнееСоединение".to_string());
        }
        
        // Если ничего не нашли, предполагаем что доступно везде
        if availability.is_empty() {
            availability.push("Клиент".to_string());
            availability.push("Сервер".to_string());
        }
        
        availability
    }
    
    /// Извлекает глобальные функции из HTML документа
    fn extract_global_functions(&mut self, document: &Html) -> Result<()> {
        let link_selector = Selector::parse("a").unwrap();
        
        // Извлекаем ссылки на методы
        for element in document.select(&link_selector) {
            if let Some(href) = element.value().attr("href") {
                if href.starts_with("Global context/methods/") {
                    let text = element.text().collect::<String>();
                    let (russian_name, english_name) = self.parse_function_name(&text);
                    
                    if !russian_name.is_empty() {
                        self.database.global_functions.insert(
                            russian_name.clone(),
                            FunctionInfo {
                                name: russian_name.clone(),
                                english_name,
                                description: Some(format!("Глобальная функция {}", russian_name)),
                                syntax: vec![],
                                parameters: vec![],
                                return_type: None,  // TODO: парсить из methods/*.html
                                return_description: None,
                                examples: vec![],
                                availability: vec!["Клиент".to_string(), "Сервер".to_string()],
                            }
                        );
                    }
                }
            }
        }
        
        debug!("Извлечено {} глобальных функций", self.database.global_functions.len());
        
        Ok(())
    }
    
    /// Парсит название функции в формате "Русское (English)"
    fn parse_function_name(&self, text: &str) -> (String, Option<String>) {
        let text = text.trim();
        
        if let Some(paren_start) = text.find('(') {
            if let Some(paren_end) = text.find(')') {
                let russian = text[..paren_start].trim().to_string();
                let english = text[paren_start + 1..paren_end].trim();
                
                if !english.is_empty() && english != russian {
                    return (russian, Some(english.to_string()));
                }
            }
        }
        
        (text.to_string(), None)
    }
    
    /// Извлекает тип из HTML элемента после "Тип:"
    fn extract_type_ref(&self, document: &Html, start_text: &str) -> Option<TypeRef> {
        // Ищем текст "Тип:" и следующую за ним ссылку
        let text_selector = Selector::parse("p, div").unwrap();
        let link_selector = Selector::parse("a").unwrap();
        
        for element in document.select(&text_selector) {
            let text = element.text().collect::<String>();
            if text.contains(start_text) {
                // Ищем ссылку внутри этого элемента
                if let Some(link) = element.select(&link_selector).next() {
                    if let Some(href) = link.value().attr("href") {
                        let link_text = link.text().collect::<String>();
                        return Some(self.parse_type_ref(href, &link_text));
                    }
                }
                // Если нет ссылки, пытаемся распарсить текст после "Тип:"
                if let Some(type_text) = text.split("Тип:").nth(1) {
                    let type_name = type_text.split('.').next()?.trim();
                    return Some(self.parse_type_by_name(type_name));
                }
            }
        }
        None
    }
    
    /// Парсит TypeRef из href и текста ссылки
    fn parse_type_ref(&self, href: &str, text: &str) -> TypeRef {
        let (name_ru, name_en) = self.parse_function_name(text);
        
        let (id, kind) = if href.contains("SyntaxHelperLanguage") {
            // Языковые типы: v8help://SyntaxHelperLanguage/def_String
            let id = href.replace("v8help://SyntaxHelperLanguage/", "language:");
            (id, TypeRefKind::Language)
        } else if href.contains("SyntaxHelperContext") {
            // Контекстные типы: v8help://SyntaxHelperContext/objects/...
            let id = href.replace("v8help://SyntaxHelperContext/", "context:");
            (id, TypeRefKind::Context)
        } else {
            // Неизвестный тип
            (format!("unknown:{}", text), TypeRefKind::MetadataRef)
        };
        
        TypeRef {
            id,
            name_ru,
            name_en,
            kind,
        }
    }
    
    /// Парсит тип по имени (для случаев без ссылки)
    fn parse_type_by_name(&self, name: &str) -> TypeRef {
        // Словарь для базовых типов
        let (id, name_en) = match name {
            "Строка" => ("language:def_String", Some("String")),
            "Число" => ("language:def_Number", Some("Number")),
            "Булево" => ("language:def_Boolean", Some("Boolean")),
            "Дата" => ("language:def_Date", Some("Date")),
            "Неопределено" => ("language:def_Undefined", Some("Undefined")),
            "Null" => ("language:def_Null", Some("Null")),
            "Тип" => ("language:def_Type", Some("Type")),
            "Массив" => ("context:objects/Array", Some("Array")),
            "Структура" => ("context:objects/Structure", Some("Structure")),
            "Соответствие" => ("context:objects/Map", Some("Map")),
            _ => {
                // Если содержит точку - это metadata ref
                if name.contains('.') {
                    return TypeRef {
                        id: format!("metadata_ref:{}", name),
                        name_ru: name.to_string(),
                        name_en: None,
                        kind: TypeRefKind::MetadataRef,
                    };
                }
                ("unknown", None)
            }
        };
        
        TypeRef {
            id: id.to_string(),
            name_ru: name.to_string(),
            name_en: name_en.map(|s| s.to_string()),
            kind: if id.starts_with("language:") {
                TypeRefKind::Language
            } else if id.starts_with("context:") {
                TypeRefKind::Context
            } else {
                TypeRefKind::MetadataRef
            },
        }
    }
    
    /// Определяет фасет по имени типа объекта
    fn detect_facet(&self, object_type: &str) -> Option<FacetKind> {
        if object_type.contains("Manager") {
            Some(FacetKind::Manager)
        } else if object_type.contains("Object") && !object_type.contains("Metadata") {
            Some(FacetKind::Object)
        } else if object_type.contains("Ref") || object_type.contains("Reference") {
            Some(FacetKind::Reference)
        } else if object_type.contains("Selection") {
            Some(FacetKind::Selection)
        } else if object_type.contains("Metadata") {
            Some(FacetKind::Metadata)
        } else if object_type == "Array" || object_type == "Structure" || object_type == "Map" {
            Some(FacetKind::Constructor)
        } else {
            None
        }
    }
    
    /// Парсит архив справки по языку
    fn parse_lang_archive(&mut self, path: &str) -> Result<()> {
        let file = File::open(path)
            .with_context(|| format!("Не удалось открыть файл: {}", path))?;
            
        let reader = BufReader::new(file);
        let mut archive = ZipArchive::new(reader)?;
        
        // Собираем информацию о ключевых словах
        let mut keyword_map: HashMap<String, KeywordInfo> = HashMap::new();
        
        // Сначала собираем английские названия из .st файлов
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let name = file.name().to_string();
            
            if name.ends_with(".st") {
                // Определяем категорию по префиксу
                let (category, keyword) = if name.starts_with("struct_") {
                    (KeywordCategory::Structure, name.replace("struct_", "").replace(".st", ""))
                } else if name.starts_with("def_") {
                    (KeywordCategory::Definition, name.replace("def_", "").replace(".st", ""))
                } else if name.starts_with("root_") {
                    (KeywordCategory::Root, name.replace("root_", "").replace(".st", ""))
                } else if name.starts_with("operator_") {
                    (KeywordCategory::Operator, name.replace("operator_", "").replace(".st", ""))
                } else if name.starts_with("Instructions_") {
                    (KeywordCategory::Instruction, name.replace("Instructions_", "").replace(".st", ""))
                } else {
                    continue; // Пропускаем файлы без известного префикса
                };
                
                // Временно создаём с одинаковыми русским и английским названиями
                keyword_map.insert(keyword.clone(), KeywordInfo {
                    russian: keyword.clone(),
                    english: keyword.clone(),
                    category,
                    description: None,
                });
            }
        }
        
        // Теперь парсим HTML файлы для получения русских названий
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name().to_string();
            
            // Ищем HTML файлы с описаниями ключевых слов
            // Обрабатываем все префиксы: struct_, def_, root_, operator_, Instructions_
            let is_keyword_html = (name.starts_with("struct_") || 
                                   name.starts_with("def_") || 
                                   name.starts_with("root_") || 
                                   name.starts_with("operator_") || 
                                   name.starts_with("Instructions_")) 
                                   && !name.ends_with(".st");
            
            if is_keyword_html {
                // Извлекаем английское название из имени файла
                let english_keyword = if name.starts_with("struct_") {
                    name.replace("struct_", "")
                } else if name.starts_with("def_") {
                    name.replace("def_", "")
                } else if name.starts_with("root_") {
                    name.replace("root_", "")
                } else if name.starts_with("operator_") {
                    name.replace("operator_", "")
                } else if name.starts_with("Instructions_") {
                    name.replace("Instructions_", "")
                } else {
                    continue;
                };
                
                if keyword_map.contains_key(&english_keyword) {
                    // Читаем содержимое HTML файла
                    let mut content = String::new();
                    if file.size() <= MAX_HTML_SIZE as u64 {
                        if let Ok(_) = file.read_to_string(&mut content) {
                            // Ищем паттерн: <H1 class=V8SH_pagetitle> или <h1 class="V8SH_pagetitle">
                            let title = if let Some(start) = content.find("V8SH_pagetitle>") {
                                // Формат без кавычек: <H1 class=V8SH_pagetitle>
                                let content_after = &content[start + 15..];
                                content_after.split("</").next().map(|s| s.to_string())
                            } else if let Some(start) = content.find("V8SH_pagetitle\">") {
                                // Формат с кавычками: <h1 class="V8SH_pagetitle">
                                let content_after = &content[start + 16..];
                                content_after.split("</").next().map(|s| s.to_string())
                            } else {
                                None
                            };
                            
                            if let Some(title) = title {
                                // Извлекаем русское и английское названия
                                if let Some(paren_pos) = title.find('(') {
                                    let russian = title[..paren_pos]
                                        .replace("&nbsp;", " ")
                                        .trim()
                                        .to_string();
                                    
                                    let english = title[paren_pos + 1..]
                                        .replace(')', "")
                                        .trim()
                                        .to_string();
                                    
                                    if !russian.is_empty() && !english.is_empty() {
                                        // Обновляем информацию о ключевом слове, сохраняя категорию
                                        if let Some(existing) = keyword_map.get(&english_keyword) {
                                            let category = existing.category.clone();
                                            keyword_map.insert(english_keyword.clone(), KeywordInfo {
                                                russian: russian.clone(),
                                                english: english.clone(),
                                                category: category.clone(),
                                                description: None,
                                            });
                                            info!("Найдено ключевое слово: {} / {} (категория: {:?})", russian, english, category);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Добавляем все ключевые слова в базу
        for (_, keyword_info) in keyword_map {
            // Проверяем, нет ли уже такого ключевого слова
            let exists = self.database.keywords.iter()
                .any(|k| k.russian == keyword_info.russian || k.english == keyword_info.english);
            
            if !exists {
                self.database.keywords.push(keyword_info);
            }
        }
        
        debug!("Извлечено {} ключевых слов", self.database.keywords.len());
        
        Ok(())
    }
    
    /// Возвращает базу знаний
    pub fn database(&self) -> &SyntaxHelperDatabase {
        &self.database
    }
    
    /// Сохраняет базу знаний в файл
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.database)?;
        std::fs::write(path, json)?;
        Ok(())
    }
    
    /// Загружает базу знаний из файла
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<SyntaxHelperDatabase> {
        let json = std::fs::read_to_string(path)?;
        let database = serde_json::from_str(&json)?;
        Ok(database)
    }
}

impl Default for SyntaxHelperParser {
    fn default() -> Self {
        Self::new()
    }
}