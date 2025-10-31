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
use scraper::Html;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tracing::{debug, info, warn};

// Импорт модулей парсера
use super::syntax_helper::*;

// Импорт типов из shared
use bsl_shared::domain::types::FacetKind;

// Импорт структур прогресса
use super::progress::{IndexingPhase, ProgressUpdate};

// ============================================================================
// Структуры данных импортируются из syntax_helper/types.rs
// ============================================================================

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

    /// Парсер документов (содержит html_extractor внутри)
    document_parser: DocumentParser,

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
            document_parser: DocumentParser::new(),
            settings,
            processed_files: Arc::new(AtomicUsize::new(0)),
            error_count: Arc::new(AtomicUsize::new(0)),
            total_files: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Парсит синтаксис-помощник, автоматически определяя структуру
    pub fn parse_syntax_helper<P: AsRef<Path>>(&mut self, base_path: P) -> Result<()> {
        let base_path = base_path.as_ref();
        info!("🚀 Начинаем парсинг синтаксис-помощника из {:?}", base_path);

        // Проверяем стандартную структуру синтаксис-помощника 1С
        let context_help_path = base_path.join("rebuilt.shcntx_ru");
        let language_help_path = base_path.join("rebuilt.shlang_ru");

        let mut parsed_something = false;

        // Парсим контекстную справку (объекты, методы, свойства)
        if context_help_path.exists() {
            info!("📚 Найдена контекстная справка (shcntx_ru), запускаем парсинг...");
            match self.parse_directory(&context_help_path, None::<fn(ProgressUpdate)>) {
                Ok(()) => {
                    info!("✅ Парсинг контекстной справки завершен");
                    parsed_something = true;
                }
                Err(e) => {
                    warn!("⚠️ Ошибка парсинга контекстной справки: {}", e);
                }
            }
        }

        // Парсим справку по языку (примитивные типы, операторы)
        if language_help_path.exists() {
            info!("📖 Найдена справка по языку (shlang_ru), запускаем парсинг...");
            match self.parse_directory(&language_help_path, None::<fn(ProgressUpdate)>) {
                Ok(()) => {
                    info!("✅ Парсинг справки по языку завершен");
                    parsed_something = true;
                }
                Err(e) => {
                    warn!("⚠️ Ошибка парсинга справки по языку: {}", e);
                }
            }
        }

        // Fallback: если стандартных папок нет, парсим как единую папку
        if !parsed_something && base_path.exists() {
            info!("📚 Стандартные папки не найдены, парсим как единую папку...");
            self.parse_directory(base_path, None::<fn(ProgressUpdate)>)?;
        }

        if !parsed_something {
            warn!(
                "📁 Не найдено подходящих файлов для парсинга в {:?}",
                base_path
            );
        }

        Ok(())
    }

    /// Парсит каталог с прогресс-баром и опциональным callback
    pub fn parse_directory<P, F>(
        &mut self,
        base_path: P,
        progress_callback: Option<F>,
    ) -> Result<()>
    where
        P: AsRef<Path>,
        F: Fn(ProgressUpdate) + Send + Sync + Clone + 'static,
    {
        let base_path = base_path.as_ref();
        info!("🚀 Начинаем оптимизированный парсинг из {:?}", base_path);

        // Фаза 1: Collecting Files (0-10%)
        if let Some(ref callback) = progress_callback {
            callback(ProgressUpdate::new(
                IndexingPhase::CollectingFiles,
                0,
                100, // Placeholder, так как не знаем сколько файлов
                Some("Начинаем сканирование...".to_string()),
            ));
        }

        let start = std::time::Instant::now();
        let html_files = self.collect_html_files(base_path)?;
        let file_count = html_files.len();
        self.total_files.store(file_count, Ordering::Relaxed);

        // Завершение фазы сбора
        if let Some(ref callback) = progress_callback {
            callback(ProgressUpdate::new(
                IndexingPhase::CollectingFiles,
                file_count,
                file_count,
                Some(format!("Найдено {} HTML файлов", file_count)),
            ));
        }

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

        // Фаза 2: Parsing Files (10-70%)
        let parse_start = std::time::Instant::now();

        files_to_process
            .par_chunks(self.settings.batch_size)
            .for_each(|batch| {
                self.process_batch(batch, &main_progress, &progress_callback);
            });

        if let Some(pb) = main_progress {
            pb.finish_with_message(format!(
                "✅ Парсинг завершён за {:?}",
                parse_start.elapsed()
            ));
        }

        // Фаза 3: Linking Categories (70-90%)
        info!("🔗 Связываем типы с категориями...");
        if let Some(ref callback) = progress_callback {
            callback(ProgressUpdate::new(
                IndexingPhase::LinkingCategories,
                0,
                self.categories.len(),
                Some("Начинаем связывание категорий...".to_string()),
            ));
        }

        self.link_types_to_categories();

        // Завершение связывания
        if let Some(ref callback) = progress_callback {
            callback(ProgressUpdate::new(
                IndexingPhase::LinkingCategories,
                self.categories.len(),
                self.categories.len(),
                Some("Категории связаны".to_string()),
            ));
        }

        // Фаза 4: Building Indexes (90-100%)
        let index_start = std::time::Instant::now();
        if let Some(ref callback) = progress_callback {
            callback(ProgressUpdate::new(
                IndexingPhase::BuildingIndexes,
                0,
                self.nodes.len(),
                Some("Построение индексов...".to_string()),
            ));
        }

        let index = if self.settings.parallel_indexing {
            IndexBuilder::build_indexes_parallel(&self.nodes)
        } else {
            IndexBuilder::build_indexes(&self.nodes)
        };
        self.type_index.insert("main".to_string(), index);

        info!("📚 Индексы построены за {:?}", index_start.elapsed());

        // Завершение индексации
        if let Some(ref callback) = progress_callback {
            callback(ProgressUpdate::new(
                IndexingPhase::BuildingIndexes,
                self.nodes.len(),
                self.nodes.len(),
                Some(format!("Индексы построены ({} типов)", self.nodes.len())),
            ));
        }

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

                    // Фильтруем HTML файлы и файлы без расширения, которые содержат HTML
                    if path.is_file() {
                        let extension = path.extension().and_then(|s| s.to_str());
                        if extension == Some("html") {
                            // Обычные .html файлы
                            Some(path.to_path_buf())
                        } else if extension.is_none() {
                            // Файлы без расширения - проверяем, содержат ли HTML
                            if let Ok(first_line) = std::fs::read_to_string(path)
                                .map(|content| content.lines().next().unwrap_or("").to_lowercase())
                            {
                                if first_line.contains("<html") || first_line.contains("<!doctype")
                                {
                                    Some(path.to_path_buf())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
            })
            .collect();

        Ok(files)
    }

    /// Обрабатывает батч файлов
    fn process_batch<F>(
        &self,
        batch: &[PathBuf],
        progress: &Option<ProgressBar>,
        progress_callback: &Option<F>,
    ) where
        F: Fn(ProgressUpdate) + Send + Sync,
    {
        // Параллельная обработка внутри батча
        batch.par_iter().for_each(|file_path| {
            match self.parse_html_file(file_path) {
                Ok(node) => {
                    // Сохраняем узел
                    self.save_node(node.clone());
                    let count = self.processed_files.fetch_add(1, Ordering::Relaxed) + 1;

                    // ✅ НОВОЕ: Отправляем прогресс каждые 10 файлов
                    if let Some(ref callback) = progress_callback {
                        if count % 10 == 0 {
                            let total = self.total_files.load(Ordering::Relaxed);

                            // Извлекаем имя типа из узла
                            let type_name = match &node {
                                SyntaxNode::Type(type_info) => {
                                    type_info.identity.russian_name.clone()
                                }
                                SyntaxNode::Method(method) => {
                                    format!("Метод: {}", method.name)
                                }
                                SyntaxNode::Property(prop) => {
                                    format!("Свойство: {}", prop.name)
                                }
                                SyntaxNode::Category(cat) => {
                                    format!("Категория: {}", cat.name)
                                }
                                SyntaxNode::Constructor(_) => "Конструктор".to_string(),
                                SyntaxNode::GlobalFunction(func) => {
                                    format!("Функция: {}", func.name)
                                }
                            };

                            callback(ProgressUpdate::new(
                                IndexingPhase::ParsingFiles,
                                count,
                                total,
                                Some(type_name),
                            ));
                        }
                    }
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
        let file_type = utils::detect_file_type(path, &document);

        match file_type {
            FileType::Type => {
                let type_info = self.document_parser.parse_type(path, &document)?;
                Ok(SyntaxNode::Type(type_info))
            }
            FileType::Method => {
                let method_info = self.document_parser.parse_method(&document)?;
                Ok(SyntaxNode::Method(method_info))
            }
            FileType::Property => {
                let property_info = self.document_parser.parse_property(&document)?;
                Ok(SyntaxNode::Property(property_info))
            }
            FileType::Category => {
                let category_info = self.document_parser.parse_category(path, &document)?;
                Ok(SyntaxNode::Category(category_info))
            }
            FileType::Constructor => {
                let constructor_info = self.document_parser.parse_constructor(&document)?;
                Ok(SyntaxNode::Constructor(constructor_info))
            }
            FileType::GlobalFunction => {
                let global_func_info = self
                    .document_parser
                    .parse_global_function(path, &document)?;
                Ok(SyntaxNode::GlobalFunction(global_func_info))
            }
        }
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

    // =========================================================================
    // Публичный API
    // =========================================================================

    /// Парсит синтаксис-помощник с отправкой прогресса через callback
    ///
    /// # Arguments
    /// * `progress_callback` - Функция для получения обновлений прогресса
    ///
    /// # Example
    /// ```no_run
    /// use bsl_backend::data::loaders::syntax_helper_parser::SyntaxHelperParser;
    /// use bsl_backend::data::loaders::progress::ProgressUpdate;
    /// use std::path::Path;
    ///
    /// let mut parser = SyntaxHelperParser::new();
    /// let path = Path::new("examples/syntax_helper");
    /// parser.parse_with_progress(path, |update: ProgressUpdate| {
    ///     println!("[{:?}] {:.1}%", update.phase, update.percentage);
    /// }).unwrap();
    /// ```
    pub fn parse_with_progress<P, F>(&mut self, base_path: P, progress_callback: F) -> Result<()>
    where
        P: AsRef<Path>,
        F: Fn(ProgressUpdate) + Send + Sync + Clone + 'static,
    {
        // Используем parse_syntax_helper для стандартной логики определения структуры,
        // но парсим через parse_directory с callback
        let base_path = base_path.as_ref();

        // Проверяем стандартную структуру
        let context_help_path = base_path.join("rebuilt.shcntx_ru");
        let language_help_path = base_path.join("rebuilt.shlang_ru");

        let mut parsed_something = false;

        // Парсим контекстную справку
        if context_help_path.exists() {
            info!("📚 Найдена контекстная справка (shcntx_ru), запускаем парсинг...");
            match self.parse_directory(&context_help_path, Some(progress_callback.clone())) {
                Ok(()) => {
                    info!("✅ Парсинг контекстной справки завершен");
                    parsed_something = true;
                }
                Err(e) => {
                    warn!("⚠️ Ошибка парсинга контекстной справки: {}", e);
                }
            }
        }

        // Парсим справку по языку
        if language_help_path.exists() {
            info!("📖 Найдена справка по языку (shlang_ru), запускаем парсинг...");
            match self.parse_directory(&language_help_path, Some(progress_callback.clone())) {
                Ok(()) => {
                    info!("✅ Парсинг справки по языку завершен");
                    parsed_something = true;
                }
                Err(e) => {
                    warn!("⚠️ Ошибка парсинга справки по языку: {}", e);
                }
            }
        }

        // Fallback
        if !parsed_something && base_path.exists() {
            info!("📚 Стандартные папки не найдены, парсим как единую папку...");
            self.parse_directory(base_path, Some(progress_callback))?;
        }

        if !parsed_something {
            warn!(
                "📁 Не найдено подходящих файлов для парсинга в {:?}",
                base_path
            );
        }

        Ok(())
    }

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
        parser
            .parse_directory(&test_dir, None::<fn(ProgressUpdate)>)
            .unwrap();

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
                        enum_values: Vec::new(),
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

    #[test]
    fn test_parse_with_progress_callback() {
        use super::super::progress::{IndexingPhase, ProgressUpdate};
        use std::sync::{Arc, Mutex};

        // Создаём небольшую тестовую директорию
        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path().join("test");
        fs::create_dir(&test_dir).unwrap();

        // Создаём несколько HTML файлов для теста
        for i in 0..15 {
            let html = format!(
                r#"
                <html>
                <head><title>TestType{} (TestType{})</title></head>
                <body>
                    <h1 class="V8SH_pagetitle">TestType{} (TestType{})</h1>
                    <p>Test description {}</p>
                </body>
                </html>
            "#,
                i, i, i, i, i
            );

            let file_path = test_dir.join(format!("type_{}.html", i));
            fs::write(file_path, html).unwrap();
        }

        // Собираем прогресс в вектор
        let progress_updates = Arc::new(Mutex::new(Vec::new()));
        let progress_clone = progress_updates.clone();

        let callback = move |update: ProgressUpdate| {
            progress_clone.lock().unwrap().push(update);
        };

        // Парсим с callback
        let settings = OptimizationSettings {
            max_threads: Some(2),
            batch_size: 5,
            show_progress: false,
            ..Default::default()
        };

        let mut parser = SyntaxHelperParser::with_settings(settings);
        parser.parse_directory(&test_dir, Some(callback)).unwrap();

        // Проверяем что прогресс был отправлен
        let updates = progress_updates.lock().unwrap();

        // Должны быть обновления для всех 4 фаз
        let phases: Vec<IndexingPhase> = updates.iter().map(|u| u.phase).collect();
        assert!(
            phases.contains(&IndexingPhase::CollectingFiles),
            "Нет фазы CollectingFiles"
        );
        assert!(
            phases.contains(&IndexingPhase::ParsingFiles),
            "Нет фазы ParsingFiles"
        );
        assert!(
            phases.contains(&IndexingPhase::LinkingCategories),
            "Нет фазы LinkingCategories"
        );
        assert!(
            phases.contains(&IndexingPhase::BuildingIndexes),
            "Нет фазы BuildingIndexes"
        );

        // Проверяем что последнее обновление - 100%
        let last = updates.last().unwrap();
        assert_eq!(last.percentage, 100.0, "Последний процент должен быть 100%");
        assert_eq!(
            last.phase,
            IndexingPhase::BuildingIndexes,
            "Последняя фаза должна быть BuildingIndexes"
        );
    }

    #[test]
    fn test_parse_without_callback_still_works() {
        // Проверяем что парсинг без callback продолжает работать
        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path().join("test");
        fs::create_dir(&test_dir).unwrap();

        // Создаём тестовый файл
        let html = r#"
            <html>
            <head><title>TestType (TestType)</title></head>
            <body>
                <h1 class="V8SH_pagetitle">TestType (TestType)</h1>
                <p>Test description</p>
            </body>
            </html>
        "#;
        fs::write(test_dir.join("test.html"), html).unwrap();

        // Парсим БЕЗ callback (старый API)
        let settings = OptimizationSettings {
            show_progress: false,
            ..Default::default()
        };

        let mut parser = SyntaxHelperParser::with_settings(settings);
        let result = parser.parse_directory(&test_dir, None::<fn(ProgressUpdate)>);

        // Должно работать без ошибок
        assert!(result.is_ok(), "Парсинг без callback должен работать");

        // Проверяем что файл обработан
        let stats = parser.get_stats();
        assert_eq!(stats.processed_files, 1, "Должен быть обработан 1 файл");
    }
}
