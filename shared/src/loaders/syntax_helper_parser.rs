//! Парсер синтакс-помощника 1С для извлечения информации о типах платформы
//!
//! Единственная актуальная версия парсера с поддержкой:
//! - Многопоточной обработки через rayon
//! - Lock-free структур данных через DashMap
//! - Полной информации о типах, методах, свойствах
//! - Двуязычности (русский/английский)
//! - Построения индексов для быстрого поиска

use anyhow::{Context, Result};
use dashmap::DashMap;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tracing::{debug, info, warn};

use bsl_shared::domain::types::FacetKind;

// ============================================================================
// Структуры данных
// ============================================================================

/// Узел в иерархии синтакс-помощника
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyntaxNode {
    /// Категория типов (например "Таблица значений")
    Category(CategoryInfo),
    /// Конкретный тип данных (например "ТаблицаЗначений")
    Type(TypeInfo),
    /// Метод типа
    Method(MethodInfo),
    /// Свойство типа  
    Property(PropertyInfo),
    /// Конструктор типа
    Constructor(ConstructorInfo),
    /// Глобальная функция (например "Мин", "Макс")
    GlobalFunction(GlobalFunctionInfo),
}

/// Информация о категории типов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryInfo {
    pub name: String,
    pub catalog_path: String,
    pub description: String,
    pub related_links: Vec<String>,
    pub types: Vec<String>,
}

/// Полная информация о типе
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInfo {
    pub identity: TypeIdentity,
    pub documentation: TypeDocumentation,
    pub structure: TypeStructure,
    pub metadata: TypeMetadata,
}

/// Идентификация типа
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeIdentity {
    pub russian_name: String,
    pub english_name: String,
    pub catalog_path: String,
    pub aliases: Vec<String>,
    pub category_path: String,
}

/// Документация типа
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDocumentation {
    pub category_description: Option<String>,
    pub type_description: String,
    pub examples: Vec<CodeExample>,
    pub availability: Vec<String>,
    pub since_version: String,
}

/// Структура типа
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeStructure {
    pub collection_element: Option<String>,
    pub methods: Vec<String>,
    pub properties: Vec<String>,
    pub constructors: Vec<String>,
    pub iterable: bool,
    pub indexable: bool,
}

/// Метаданные типа
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeMetadata {
    pub available_facets: Vec<FacetKind>,
    pub default_facet: Option<FacetKind>,
    pub serializable: bool,
    pub exchangeable: bool,
    pub xdto_namespace: Option<String>,
    pub xdto_type: Option<String>,
}

/// Пример кода
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExample {
    pub description: Option<String>,
    pub code: String,
    pub language: String,
}

/// Информация о методе
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodInfo {
    pub name: String,
    pub english_name: Option<String>,
    pub description: Option<String>,
    pub parameters: Vec<ParameterInfo>,
    pub return_type: Option<String>,
    pub return_description: Option<String>,
}

/// Информация о свойстве
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyInfo {
    pub name: String,
    pub property_type: Option<String>,
    pub is_readonly: bool,
    pub description: Option<String>,
}

/// Информация о конструкторе
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstructorInfo {
    pub name: String,
    pub parameters: Vec<ParameterInfo>,
    pub description: Option<String>,
}

/// Информация о параметре
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterInfo {
    pub name: String,
    pub type_name: Option<String>,
    pub is_optional: bool,
    pub default_value: Option<String>,
    pub description: Option<String>,
}

/// Информация о глобальной функции
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalFunctionInfo {
    pub name: String,
    pub english_name: Option<String>,
    pub description: Option<String>,
    pub parameters: Vec<ParameterInfo>,
    pub return_type: Option<String>,
    pub return_description: Option<String>,
    pub polymorphic: bool,
    pub pure: bool,
    pub contexts: Vec<String>,    // Where available (Server, Client, etc.)
    pub category: Option<String>, // Category name like "Прочие процедуры и функции"
}

/// База данных синтакс-помощника
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyntaxHelperDatabase {
    pub nodes: HashMap<String, SyntaxNode>,
    pub methods: HashMap<String, MethodInfo>,
    pub properties: HashMap<String, PropertyInfo>,
    pub categories: HashMap<String, CategoryInfo>,
}

/// Индексы для поиска типов
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TypeIndex {
    pub by_russian: HashMap<String, String>,
    pub by_english: HashMap<String, String>,
    pub by_any_name: HashMap<String, Vec<String>>,
    pub by_category: HashMap<String, Vec<String>>,
    pub by_facet: HashMap<FacetKind, Vec<String>>,
}

/// Настройки оптимизации
#[derive(Debug, Clone)]
pub struct OptimizationSettings {
    /// Максимальное количество потоков
    pub max_threads: Option<usize>,
    /// Размер батча для параллельной обработки
    pub batch_size: usize,
    /// Показывать прогресс-бар
    pub show_progress: bool,
    /// Лимит файлов для обработки (для тестирования)
    pub file_limit: Option<usize>,
    /// Пропускать определённые каталоги
    pub skip_dirs: Vec<String>,
    /// Использовать параллельное построение индексов
    pub parallel_indexing: bool,
}

impl Default for OptimizationSettings {
    fn default() -> Self {
        Self {
            max_threads: None, // Использовать все доступные ядра
            batch_size: 50,    // Оптимальный размер батча
            show_progress: true,
            file_limit: None,
            skip_dirs: vec![
                "tables".to_string(), // Большие таблицы можно пропустить
                "IndexPackLookup".to_string(),
            ],
            parallel_indexing: true,
        }
    }
}

/// Парсер синтакс-помощника с поддержкой многопоточности
pub struct SyntaxHelperParser {
    /// База данных с узлами (lock-free concurrent hashmap)
    pub(crate) nodes: Arc<DashMap<String, SyntaxNode>>,
    /// Методы (lock-free)
    methods: Arc<DashMap<String, MethodInfo>>,
    /// Свойства (lock-free)
    properties: Arc<DashMap<String, PropertyInfo>>,
    /// Категории (lock-free)
    categories: Arc<DashMap<String, CategoryInfo>>,

    /// Индексы для поиска (собираются после парсинга)
    type_index: Arc<DashMap<String, TypeIndex>>,

    /// Настройки оптимизации
    settings: OptimizationSettings,

    /// Счётчик обработанных файлов
    processed_files: Arc<AtomicUsize>,
    /// Счётчик ошибок парсинга
    error_count: Arc<AtomicUsize>,
    /// Общее количество файлов
    total_files: Arc<AtomicUsize>,
}

impl SyntaxHelperParser {
    /// Создаёт новый оптимизированный парсер
    pub fn new() -> Self {
        Self::with_settings(OptimizationSettings::default())
    }

    /// Создаёт парсер с настройками
    pub fn with_settings(settings: OptimizationSettings) -> Self {
        // Настраиваем rayon thread pool
        if let Some(threads) = settings.max_threads {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build_global()
                .ok();
        }

        Self {
            nodes: Arc::new(DashMap::new()),
            methods: Arc::new(DashMap::new()),
            properties: Arc::new(DashMap::new()),
            categories: Arc::new(DashMap::new()),
            type_index: Arc::new(DashMap::new()),
            settings,
            processed_files: Arc::new(AtomicUsize::new(0)),
            error_count: Arc::new(AtomicUsize::new(0)),
            total_files: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Парсит каталог с прогресс-баром
    pub fn parse_directory<P: AsRef<Path>>(&mut self, base_path: P) -> Result<()> {
        let base_path = base_path.as_ref();
        info!("🚀 Начинаем оптимизированный парсинг из {:?}", base_path);

        // Фаза 1: Собираем все HTML файлы
        let start = std::time::Instant::now();
        let html_files = self.collect_html_files(base_path)?;
        let file_count = html_files.len();
        self.total_files.store(file_count, Ordering::Relaxed);

        info!(
            "📊 Найдено {} HTML файлов за {:?}",
            file_count,
            start.elapsed()
        );

        // Применяем лимит если установлен
        let files_to_process = if let Some(limit) = self.settings.file_limit {
            &html_files[..limit.min(file_count)]
        } else {
            &html_files
        };

        info!(
            "⚡ Обрабатываем {} файлов с {} потоками",
            files_to_process.len(),
            rayon::current_num_threads()
        );

        // Создаём мульти-прогресс для детального отображения
        let multi_progress = if self.settings.show_progress {
            Some(MultiProgress::new())
        } else {
            None
        };

        // Основной прогресс-бар
        let main_progress = if let Some(ref mp) = multi_progress {
            let pb = mp.add(ProgressBar::new(files_to_process.len() as u64));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template(
                        "[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg} [{per_sec}]",
                    )?
                    .progress_chars("##-"),
            );
            pb.set_message("Парсинг HTML файлов");
            Some(pb)
        } else {
            None
        };

        // Фаза 2: Параллельная обработка файлов
        let parse_start = std::time::Instant::now();

        files_to_process
            .par_chunks(self.settings.batch_size)
            .for_each(|batch| {
                self.process_batch(batch, &main_progress);
            });

        if let Some(pb) = main_progress {
            pb.finish_with_message(format!(
                "✅ Парсинг завершён за {:?}",
                parse_start.elapsed()
            ));
        }

        // Фаза 3: Связываем типы с категориями
        info!("🔗 Связываем типы с категориями...");
        self.link_types_to_categories();

        // Фаза 4: Параллельное построение индексов
        let index_start = std::time::Instant::now();

        if self.settings.parallel_indexing {
            self.build_indexes_parallel();
        } else {
            self.build_indexes();
        }

        info!("📚 Индексы построены за {:?}", index_start.elapsed());

        // Выводим финальную статистику
        let processed = self.processed_files.load(Ordering::Relaxed);
        let errors = self.error_count.load(Ordering::Relaxed);
        let total_time = start.elapsed();

        info!("✨ Обработано {} файлов за {:?}", processed, total_time);
        info!(
            "📈 Скорость: {:.2} файлов/сек",
            processed as f64 / total_time.as_secs_f64()
        );

        if errors > 0 {
            warn!("⚠️ Произошло {} ошибок при парсинге", errors);
        }

        Ok(())
    }

    /// Собирает все HTML файлы рекурсивно (параллельно)
    fn collect_html_files(&self, base_path: &Path) -> Result<Vec<PathBuf>> {
        use walkdir::WalkDir;

        let files: Vec<PathBuf> = WalkDir::new(base_path)
            .into_iter()
            .par_bridge() // Параллельный обход
            .filter_map(|entry| {
                entry.ok().and_then(|e| {
                    let path = e.path();

                    // Проверяем, нужно ли пропустить директорию
                    if path.is_dir() {
                        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                            if self.settings.skip_dirs.contains(&dir_name.to_string()) {
                                return None;
                            }
                        }
                    }

                    // Фильтруем только HTML файлы
                    if path.extension().and_then(|s| s.to_str()) == Some("html") {
                        Some(path.to_path_buf())
                    } else {
                        None
                    }
                })
            })
            .collect();

        Ok(files)
    }

    /// Обрабатывает батч файлов
    fn process_batch(&self, batch: &[PathBuf], progress: &Option<ProgressBar>) {
        // Параллельная обработка внутри батча
        batch.par_iter().for_each(|file_path| {
            match self.parse_html_file(file_path) {
                Ok(node) => {
                    self.save_node(node);
                    self.processed_files.fetch_add(1, Ordering::Relaxed);
                }
                Err(e) => {
                    debug!("Ошибка парсинга {:?}: {}", file_path, e);
                    self.error_count.fetch_add(1, Ordering::Relaxed);
                }
            }

            if let Some(pb) = progress {
                pb.inc(1);
            }
        });
    }

    /// Парсит один HTML файл
    fn parse_html_file(&self, path: &Path) -> Result<SyntaxNode> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Не удалось прочитать файл {:?}", path))?;
        let document = Html::parse_document(&content);

        // Определяем тип файла по содержимому и пути
        let file_type = self.detect_file_type(path, &document);

        match file_type {
            FileType::Type => {
                let type_info = self.parse_type_from_document(path, &document)?;
                Ok(SyntaxNode::Type(type_info))
            }
            FileType::Method => {
                let method_info = self.parse_method_from_document(&document)?;
                Ok(SyntaxNode::Method(method_info))
            }
            FileType::Property => {
                let property_info = self.parse_property_from_document(&document)?;
                Ok(SyntaxNode::Property(property_info))
            }
            FileType::Category => {
                let category_info = self.parse_category_from_document(path, &document)?;
                Ok(SyntaxNode::Category(category_info))
            }
            FileType::Constructor => {
                let constructor_info = self.parse_constructor_from_document(&document)?;
                Ok(SyntaxNode::Constructor(constructor_info))
            }
            FileType::GlobalFunction => {
                let global_func_info = self.parse_global_function_from_document(path, &document)?;
                Ok(SyntaxNode::GlobalFunction(global_func_info))
            }
        }
    }

    /// Определяет тип файла
    fn detect_file_type(&self, path: &Path, document: &Html) -> FileType {
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
                    if title.starts_with("Глобальный контекст.")
                        || title.starts_with("Global context.")
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

    /// Связывает типы с категориями на основе путей файлов
    fn link_types_to_categories(&self) {
        // Получаем все категории
        let categories_snapshot: Vec<(String, CategoryInfo)> = self
            .categories
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();

        // Сначала обрабатываем категории верхнего уровня
        for (catalog_id, category) in &categories_snapshot {
            debug!("Обработка категории {}: {}", catalog_id, category.name);

            // Паттерн для категорий верхнего уровня (например, objects/catalog234/)
            let top_level_pattern = format!("/objects/{}/", catalog_id);

            // Находим все типы непосредственно в этой категории
            for mut entry in self.nodes.iter_mut() {
                let path = entry.key();

                // Проверяем, находится ли тип непосредственно в каталоге категории
                // но не в подкаталогах вида catalogXXX
                if path.contains(&top_level_pattern) {
                    // Проверяем, что это не вложенная категория
                    let after_pattern =
                        &path[path.find(&top_level_pattern).unwrap() + top_level_pattern.len()..];

                    // Если после паттерна нет другого catalog*, значит это тип в основной категории
                    if !after_pattern.starts_with("catalog") {
                        if let SyntaxNode::Type(ref mut type_info) = entry.value_mut() {
                            type_info.identity.category_path = category.name.clone();
                            debug!(
                                "  Связал тип {} с категорией {}",
                                type_info.identity.russian_name, category.name
                            );
                        }
                    }
                }
            }
        }

        // Затем обрабатываем подкатегории (catalogXXX внутри catalogYYY)
        for mut entry in self.nodes.iter_mut() {
            let path = entry.key().clone(); // Клонируем ключ чтобы избежать проблем с заимствованием
            if let SyntaxNode::Type(ref mut type_info) = entry.value_mut() {
                // Если категория еще не установлена
                if type_info.identity.category_path.is_empty() {
                    // Ищем родительскую категорию для вложенных типов
                    for (catalog_id, category) in &categories_snapshot {
                        let pattern = format!("/{}/", catalog_id);
                        if path.contains(&pattern) {
                            // Для вложенных категорий используем основную категорию
                            type_info.identity.category_path = category.name.clone();
                            debug!(
                                "  Связал вложенный тип {} с категорией {}",
                                type_info.identity.russian_name, category.name
                            );
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Парсит тип из документа
    fn parse_type_from_document(&self, path: &Path, document: &Html) -> Result<TypeInfo> {
        let title = self.extract_title(document);
        let (russian, english) = self.parse_title(&title);
        let description = self.extract_description(document);

        Ok(TypeInfo {
            identity: TypeIdentity {
                russian_name: russian.clone(),
                english_name: english,
                catalog_path: self.build_path(path),
                category_path: self.extract_category_path(path),
                aliases: self.extract_aliases(document),
            },
            documentation: TypeDocumentation {
                category_description: None,
                type_description: description.clone(),
                examples: self.extract_examples(document),
                availability: self.extract_availability(document),
                since_version: self.extract_version(document),
            },
            structure: TypeStructure {
                collection_element: self.extract_collection_element(document),
                methods: Vec::new(),      // Будут заполнены позже
                properties: Vec::new(),   // Будут заполнены позже
                constructors: Vec::new(), // Будут заполнены позже
                iterable: self.is_iterable(&description),
                indexable: self.is_indexable(&description),
            },
            metadata: TypeMetadata {
                available_facets: self.detect_facets(&russian, &description),
                default_facet: None,
                serializable: self.is_serializable(document),
                exchangeable: self.is_exchangeable(document),
                xdto_namespace: None,
                xdto_type: None,
            },
        })
    }

    /// Парсит метод из документа
    fn parse_method_from_document(&self, document: &Html) -> Result<MethodInfo> {
        let name = self.extract_title(document);
        let description = self.extract_description(document);
        let parameters = self.extract_parameters(document);
        let (return_type, return_description) = self.extract_return_info(document);

        Ok(MethodInfo {
            name: name.clone(),
            english_name: self.extract_english_name(document),
            description: Some(description),
            parameters,
            return_type,
            return_description,
        })
    }

    /// Парсит глобальную функцию из документа
    fn parse_global_function_from_document(
        &self,
        path: &Path,
        document: &Html,
    ) -> Result<GlobalFunctionInfo> {
        // Извлекаем категорию из пути
        // Путь вида: .../Global context/methods/catalog1762/Min962.html
        // Нужно найти catalog ID и загрузить название категории
        let category = self.extract_function_category(path);
        // Извлекаем заголовок, например "Глобальный контекст.Мин (Global context.Min)"
        let title = self.extract_title(document);

        // Извлекаем имя функции из заголовка
        let (russian_name, english_name) = if title.contains('.') {
            // Убираем "Глобальный контекст." или "Global context."
            let parts: Vec<&str> = title.split('.').collect();
            if parts.len() >= 2 {
                // Берём всё после первой точки
                let func_part = parts[1..].join(".");
                // Извлекаем русское и английское названия
                if func_part.contains(" (") {
                    // Разделяем по " (" чтобы получить русское имя и английскую часть
                    let russian = func_part.split(" (").next().unwrap_or(&func_part).trim();
                    let english = func_part.split(" (").nth(1).and_then(|s| {
                        // Убираем "Global context." из английской части
                        if s.contains('.') {
                            s.split('.').nth(1).and_then(|name| {
                                // Убираем закрывающую скобку
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
            } else {
                // Если нет точки, пробуем извлечьь из скобок
                if title.contains(" (") {
                    let russian = title.split(" (").next().unwrap_or(&title).trim();
                    (russian.to_string(), None)
                } else {
                    (title.clone(), None)
                }
            }
        } else {
            // Если нет точки, пробуем извлечьь из скобок
            if title.contains(" (") {
                let russian = title.split(" (").next().unwrap_or(&title).trim();
                (russian.to_string(), None)
            } else {
                (title.clone(), None)
            }
        };

        let description = self.extract_description(document);
        let parameters = self.extract_parameters(document);
        let (return_type, return_description) = self.extract_return_info(document);

        // Определяем, является ли функция полиморфной
        let polymorphic = self.is_polymorphic_function(&russian_name, &english_name);

        // Определяем, является ли функция чистой (без побочных эффектов)
        let pure = self.is_pure_function(&russian_name, &english_name);

        // Извлекаем контексты доступности
        let contexts = self.extract_availability_contexts(document);

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

    /// Определяет, является ли функция полиморфной
    fn is_polymorphic_function(&self, russian_name: &str, english_name: &Option<String>) -> bool {
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
    fn is_pure_function(&self, russian_name: &str, english_name: &Option<String>) -> bool {
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
    fn extract_function_category(&self, path: &Path) -> Option<String> {
        // Путь вида: .../Global context/methods/catalog1762/Min962.html
        // Извлекаем catalog ID (например, catalog1762)
        let path_str = path.to_string_lossy();

        // Ищем паттерн "methods\catalogXXX\" или "methods/catalogXXX/" в зависимости от ОС
        let pattern = if cfg!(windows) {
            r"methods\catalog"
        } else {
            "methods/catalog"
        };
        let separator = if cfg!(windows) { '\\' } else { '/' };

        if let Some(start) = path_str.find(pattern) {
            let skip_len = if cfg!(windows) { 8 } else { 8 }; // Skip "methods\" or "methods/"
            let catalog_part = &path_str[start + skip_len..];
            if let Some(end) = catalog_part.find(separator) {
                let catalog_id = &catalog_part[..end];

                // Строим путь к файлу категории
                let category_file = path
                    .parent()
                    .and_then(|p| p.parent())
                    .map(|p| p.join(format!("{}.html", catalog_id)));

                // Пытаемся прочитать название категории из файла
                if let Some(category_path) = category_file {
                    if category_path.exists() {
                        if let Ok(content) = std::fs::read_to_string(&category_path) {
                            // Извлекаем название из <h1 class="V8SH_pagetitle">
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
    fn extract_availability_contexts(&self, document: &Html) -> Vec<String> {
        let availability_selector = Selector::parse("p.V8SH_chapter + p").unwrap();

        for element in document.select(&availability_selector) {
            let text = element.text().collect::<String>();
            if text.contains("клиент") || text.contains("сервер") || text.contains("соединение")
            {
                // Парсим список контекстов
                return text
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }

        Vec::new()
    }

    /// Парсит свойство из документа
    fn parse_property_from_document(&self, document: &Html) -> Result<PropertyInfo> {
        let name = self.extract_title(document);
        let description = self.extract_description(document);
        let property_type = self.extract_property_type(document);
        let is_readonly = self.is_readonly(document);

        Ok(PropertyInfo {
            name,
            property_type,
            is_readonly,
            description: Some(description),
        })
    }

    /// Парсит категорию из документа
    fn parse_category_from_document(&self, path: &Path, document: &Html) -> Result<CategoryInfo> {
        let name = self.extract_title(document);
        let description = self.extract_description(document);
        let related_links = self.extract_links(document);
        let types = self.extract_type_list(document);

        // Строим catalog_path относительно objects
        // Для корневых категорий: catalog234
        // Для вложенных: catalog234/catalog236
        let catalog_id = if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
            // Проверяем путь относительно objects
            let path_str = path.to_string_lossy();
            if let Some(objects_pos) = path_str.rfind("objects") {
                let after_objects = &path_str[objects_pos + 8..]; // 8 = len("objects/")
                                                                  // Убираем расширение .html и слеши в начале
                let clean_path = after_objects
                    .trim_start_matches('/')
                    .trim_start_matches('\\')
                    .trim_end_matches(".html");

                // Если это файл в корне objects - возвращаем только имя
                // Если вложенный - сохраняем структуру каталогов
                if !clean_path.contains('/') && !clean_path.contains('\\') {
                    file_stem.to_string()
                } else {
                    // Заменяем обратные слеши на прямые и берём путь до файла
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
    fn parse_constructor_from_document(&self, document: &Html) -> Result<ConstructorInfo> {
        let description = self.extract_description(document);
        let parameters = self.extract_parameters(document);

        Ok(ConstructorInfo {
            name: self.extract_title(document),
            description: Some(description),
            parameters,
        })
    }

    /// Сохраняет узел в базу данных (lock-free)
    fn save_node(&self, node: SyntaxNode) {
        match node {
            SyntaxNode::Category(cat) => {
                let path = cat.catalog_path.clone();
                self.categories.insert(path.clone(), cat.clone());
                self.nodes.insert(path, SyntaxNode::Category(cat));
            }
            SyntaxNode::Type(type_info) => {
                let path = type_info.identity.catalog_path.clone();
                self.nodes.insert(path, SyntaxNode::Type(type_info));
            }
            SyntaxNode::Method(method) => {
                let key = format!("method_{}", method.name);
                self.methods.insert(key.clone(), method.clone());
                self.nodes.insert(key, SyntaxNode::Method(method));
            }
            SyntaxNode::Property(prop) => {
                let key = format!("property_{}", prop.name);
                self.properties.insert(key.clone(), prop.clone());
                self.nodes.insert(key, SyntaxNode::Property(prop));
            }
            SyntaxNode::GlobalFunction(func) => {
                let key = format!("global_function_{}", func.name);
                // Добавим в nodes для общего доступа
                self.nodes.insert(key, SyntaxNode::GlobalFunction(func));
            }
            SyntaxNode::Constructor(cons) => {
                let key = format!("constructor_{}", self.nodes.len());
                self.nodes.insert(key, SyntaxNode::Constructor(cons));
            }
        }
    }

    /// Строит индексы после парсинга (однопоточно)
    fn build_indexes(&self) {
        let mut index = TypeIndex::default();

        for entry in self.nodes.iter() {
            let (path, node) = entry.pair();

            if let SyntaxNode::Type(type_info) = node {
                // Индекс по русскому имени
                index
                    .by_russian
                    .insert(type_info.identity.russian_name.clone(), path.clone());

                // Индекс по английскому имени
                if !type_info.identity.english_name.is_empty() {
                    index
                        .by_english
                        .insert(type_info.identity.english_name.clone(), path.clone());
                }

                // Индекс по фасетам
                for facet in &type_info.metadata.available_facets {
                    index.by_facet.entry(*facet).or_default().push(path.clone());
                }

                // Индекс по категориям
                if !type_info.identity.category_path.is_empty() {
                    index
                        .by_category
                        .entry(type_info.identity.category_path.clone())
                        .or_default()
                        .push(path.clone());
                }
            }
        }

        self.type_index.insert("main".to_string(), index);
    }

    /// Параллельное построение индексов
    fn build_indexes_parallel(&self) {
        use dashmap::DashMap;

        // Создаём параллельные индексы
        let by_russian = Arc::new(DashMap::new());
        let by_english = Arc::new(DashMap::new());
        let by_facet = Arc::new(DashMap::new());
        let by_category = Arc::new(DashMap::new());

        // Параллельно обрабатываем все узлы
        self.nodes.iter().par_bridge().for_each(|entry| {
            let (path, node) = entry.pair();

            if let SyntaxNode::Type(type_info) = node {
                // Индекс по русскому имени
                by_russian.insert(type_info.identity.russian_name.clone(), path.clone());

                // Индекс по английскому имени
                if !type_info.identity.english_name.is_empty() {
                    by_english.insert(type_info.identity.english_name.clone(), path.clone());
                }

                // Индекс по фасетам
                for facet in &type_info.metadata.available_facets {
                    by_facet
                        .entry(*facet)
                        .or_insert_with(Vec::new)
                        .push(path.clone());
                }

                // Индекс по категориям
                if !type_info.identity.category_path.is_empty() {
                    by_category
                        .entry(type_info.identity.category_path.clone())
                        .or_insert_with(Vec::new)
                        .push(path.clone());
                }
            }
        });

        // Конвертируем в обычный индекс
        let mut index = TypeIndex::default();

        for entry in by_russian.iter() {
            index
                .by_russian
                .insert(entry.key().clone(), entry.value().clone());
        }

        for entry in by_english.iter() {
            index
                .by_english
                .insert(entry.key().clone(), entry.value().clone());
        }

        for entry in by_facet.iter() {
            index.by_facet.insert(*entry.key(), entry.value().clone());
        }

        for entry in by_category.iter() {
            index
                .by_category
                .insert(entry.key().clone(), entry.value().clone());
        }

        self.type_index.insert("main".to_string(), index);
    }

    // =========================================================================
    // Вспомогательные методы для извлечения данных
    // =========================================================================

    fn extract_title(&self, document: &Html) -> String {
        self.extract_element_text(document, "h1.V8SH_pagetitle")
            .or_else(|| self.extract_element_text(document, "h1"))
            .unwrap_or_default()
    }

    fn parse_title(&self, title: &str) -> (String, String) {
        if let Some(open) = title.find('(') {
            if let Some(close) = title.find(')') {
                let russian = title[..open].trim().to_string();
                let english = title[open + 1..close].trim().to_string();
                return (russian, english);
            }
        }
        (title.trim().to_string(), String::new())
    }

    fn extract_element_text(&self, document: &Html, selector_str: &str) -> Option<String> {
        Selector::parse(selector_str).ok().and_then(|selector| {
            document
                .select(&selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
        })
    }

    fn extract_description(&self, document: &Html) -> String {
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

    fn extract_examples(&self, document: &Html) -> Vec<CodeExample> {
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

    fn extract_parameters(&self, document: &Html) -> Vec<ParameterInfo> {
        let mut parameters = Vec::new();

        // Ищем таблицу параметров
        if let Ok(selector) = Selector::parse("table.V8SH_params tr, table tr") {
            for row in document.select(&selector).skip(1) {
                // Пропускаем заголовок
                let cells: Vec<String> = Selector::parse("td")
                    .ok()
                    .map(|s| {
                        row.select(&s)
                            .map(|cell| cell.text().collect::<String>().trim().to_string())
                            .collect()
                    })
                    .unwrap_or_default();

                if cells.len() >= 2 {
                    parameters.push(ParameterInfo {
                        name: cells[0].clone(),
                        type_name: Some(cells[1].clone()),
                        is_optional: cells
                            .get(2)
                            .map(|s| s.contains("Необязательный") || s.contains("Optional"))
                            .unwrap_or(false),
                        default_value: cells.get(3).cloned(),
                        description: cells.get(4).cloned(),
                    });
                }
            }
        }

        parameters
    }

    fn extract_return_info(&self, document: &Html) -> (Option<String>, Option<String>) {
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

    #[allow(dead_code)]
    fn extract_return_type(&self, document: &Html) -> String {
        self.extract_return_info(document).0.unwrap_or_default()
    }

    fn extract_property_type(&self, document: &Html) -> Option<String> {
        self.extract_element_text(document, "span.V8SH_type, span.type")
    }

    fn extract_english_name(&self, document: &Html) -> Option<String> {
        self.extract_element_text(document, "span.V8SH_english, span.english")
    }

    fn extract_availability(&self, document: &Html) -> Vec<String> {
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

    fn extract_version(&self, document: &Html) -> String {
        self.extract_element_text(document, "span.V8SH_version, span.version")
            .unwrap_or_else(|| "8.3.0+".to_string())
    }

    fn extract_aliases(&self, _document: &Html) -> Vec<String> {
        // Извлекаем альтернативные имена из текста
        Vec::new() // TODO: Implement alias extraction
    }

    fn extract_collection_element(&self, _document: &Html) -> Option<String> {
        // Извлекаем тип элемента коллекции
        None // TODO: Implement collection element extraction
    }

    fn extract_links(&self, document: &Html) -> Vec<String> {
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

    fn extract_type_list(&self, document: &Html) -> Vec<String> {
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

    fn extract_category_path(&self, _path: &Path) -> String {
        // Пока возвращаем пустую строку, так как категории будут установлены
        // позже в link_types_to_categories на основе каталогов категорий
        String::new()
    }

    fn is_readonly(&self, document: &Html) -> bool {
        let text = document.root_element().text().collect::<String>();
        text.contains("Только чтение") || text.contains("Read only")
    }

    fn is_iterable(&self, description: &str) -> bool {
        description.contains("Для каждого")
            || description.contains("For each")
            || description.contains("итерация")
            || description.contains("iteration")
    }

    fn is_indexable(&self, description: &str) -> bool {
        description.contains("индекс")
            || description.contains("index")
            || description.contains("[]")
    }

    fn is_serializable(&self, document: &Html) -> bool {
        let text = document.root_element().text().collect::<String>();
        text.contains("Сериализуемый")
            || text.contains("Serializable")
            || text.contains("XML")
            || text.contains("JSON")
    }

    fn is_exchangeable(&self, document: &Html) -> bool {
        let text = document.root_element().text().collect::<String>();
        text.contains("Обмен данными") || text.contains("Data exchange") || text.contains("XDTO")
    }

    fn detect_facets(&self, type_name: &str, description: &str) -> Vec<FacetKind> {
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

    fn build_path(&self, path: &Path) -> String {
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

    // =========================================================================
    // Публичный API
    // =========================================================================

    /// Получить статистику парсинга
    pub fn get_stats(&self) -> ParsingStats {
        ParsingStats {
            total_files: self.total_files.load(Ordering::Relaxed),
            processed_files: self.processed_files.load(Ordering::Relaxed),
            error_count: self.error_count.load(Ordering::Relaxed),
            total_nodes: self.nodes.len(),
            types_count: self
                .nodes
                .iter()
                .filter(|entry| matches!(entry.value(), SyntaxNode::Type(_)))
                .count(),
            methods_count: self.methods.len(),
            properties_count: self.properties.len(),
            categories_count: self.categories.len(),
            index_size: self
                .type_index
                .get("main")
                .map(|idx| idx.by_russian.len() + idx.by_english.len())
                .unwrap_or(0),
        }
    }

    /// Экспортировать базу данных
    pub fn export_database(&self) -> SyntaxHelperDatabase {
        let mut db = SyntaxHelperDatabase::default();

        // Копируем все узлы
        for entry in self.nodes.iter() {
            db.nodes.insert(entry.key().clone(), entry.value().clone());
        }

        // Копируем методы
        for entry in self.methods.iter() {
            db.methods
                .insert(entry.key().clone(), entry.value().clone());
        }

        // Копируем свойства
        for entry in self.properties.iter() {
            db.properties
                .insert(entry.key().clone(), entry.value().clone());
        }

        // Копируем категории
        for entry in self.categories.iter() {
            db.categories
                .insert(entry.key().clone(), entry.value().clone());
        }

        db
    }

    /// Экспортировать индексы
    pub fn export_index(&self) -> TypeIndex {
        self.type_index
            .get("main")
            .map(|idx| idx.clone())
            .unwrap_or_default()
    }

    /// Поиск типа по имени
    pub fn find_type(&self, name: &str) -> Option<TypeInfo> {
        // Сначала ищем в индексе
        if let Some(index) = self.type_index.get("main") {
            // Ищем по русскому имени
            if let Some(path) = index.by_russian.get(name) {
                if let Some(node) = self.nodes.get(path) {
                    if let SyntaxNode::Type(type_info) = node.value() {
                        return Some(type_info.clone());
                    }
                }
            }

            // Ищем по английскому имени
            if let Some(path) = index.by_english.get(name) {
                if let Some(node) = self.nodes.get(path) {
                    if let SyntaxNode::Type(type_info) = node.value() {
                        return Some(type_info.clone());
                    }
                }
            }
        }

        None
    }

    /// Получить все типы с определённым фасетом
    pub fn get_types_by_facet(&self, facet: FacetKind) -> Vec<TypeInfo> {
        let mut types = Vec::new();

        if let Some(index) = self.type_index.get("main") {
            if let Some(paths) = index.by_facet.get(&facet) {
                for path in paths {
                    if let Some(node) = self.nodes.get(path) {
                        if let SyntaxNode::Type(type_info) = node.value() {
                            types.push(type_info.clone());
                        }
                    }
                }
            }
        }

        types
    }
}

/// Статистика парсинга
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsingStats {
    pub total_files: usize,
    pub processed_files: usize,
    pub error_count: usize,
    pub total_nodes: usize,
    pub types_count: usize,
    pub methods_count: usize,
    pub properties_count: usize,
    pub categories_count: usize,
    pub index_size: usize,
}

/// Тип файла для парсинга
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileType {
    Type,
    Method,
    Property,
    Category,
    Constructor,
    GlobalFunction,
}

impl Default for SyntaxHelperParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parallel_parsing() {
        // Создаём временную директорию с тестовыми HTML файлами
        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path().join("test");
        fs::create_dir(&test_dir).unwrap();

        // Создаём несколько тестовых HTML файлов
        for i in 0..10 {
            let html = format!(
                r#"
                <html>
                <body>
                    <h1 class="V8SH_pagetitle">TestType{} (TestType{})</h1>
                    <p>Test description {}</p>
                </body>
                </html>
            "#,
                i, i, i
            );

            let file_path = test_dir.join(format!("type_{}.html", i));
            fs::write(file_path, html).unwrap();
        }

        // Парсим с многопоточностью
        let settings = OptimizationSettings {
            max_threads: Some(4),
            batch_size: 2,
            show_progress: false,
            ..Default::default()
        };

        let mut parser = SyntaxHelperParser::with_settings(settings);
        parser.parse_directory(&test_dir).unwrap();

        // Проверяем результаты
        let stats = parser.get_stats();
        assert_eq!(stats.processed_files, 10);
        assert_eq!(stats.types_count, 10);
        assert_eq!(stats.error_count, 0);
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let parser = Arc::new(SyntaxHelperParser::new());
        let mut handles = vec![];

        // Создаём несколько потоков для одновременного доступа
        for i in 0..10 {
            let parser_clone = Arc::clone(&parser);
            let handle = thread::spawn(move || {
                // Симулируем сохранение узла
                let type_info = TypeInfo {
                    identity: TypeIdentity {
                        russian_name: format!("Тип{}", i),
                        english_name: format!("Type{}", i),
                        catalog_path: format!("path_{}", i),
                        category_path: String::new(),
                        aliases: Vec::new(),
                    },
                    documentation: TypeDocumentation {
                        category_description: None,
                        type_description: format!("Description {}", i),
                        examples: Vec::new(),
                        availability: vec!["Сервер".to_string()],
                        since_version: "8.3.0".to_string(),
                    },
                    structure: TypeStructure {
                        collection_element: None,
                        methods: Vec::new(),
                        properties: Vec::new(),
                        constructors: Vec::new(),
                        iterable: false,
                        indexable: false,
                    },
                    metadata: TypeMetadata {
                        available_facets: vec![],
                        default_facet: None,
                        serializable: true,
                        exchangeable: true,
                        xdto_namespace: None,
                        xdto_type: None,
                    },
                };

                parser_clone.save_node(SyntaxNode::Type(type_info));
            });

            handles.push(handle);
        }

        // Ждём завершения всех потоков
        for handle in handles {
            handle.join().unwrap();
        }

        // Проверяем, что все узлы сохранены
        assert_eq!(parser.nodes.len(), 10);
    }
}
