//! Обработчики файлов и батчей для загрузчика синтакс-помощника

use anyhow::{Context, Result};
use indicatif::ProgressBar;
use rayon::prelude::*;
use scraper::Html;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::debug;

use super::super::progress::{IndexingPhase, ProgressUpdate};
use super::loader::SyntaxHelperLoader;
use super::types::{CategoryInfo, PropertySourceKind, SyntaxNode};
use super::utils;
use super::utils::FileType;

impl SyntaxHelperLoader {
    /// Собирает все HTML файлы рекурсивно (параллельно)
    pub(crate) fn collect_html_files(&self, base_path: &Path) -> Result<Vec<PathBuf>> {
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
                            // Файлы без расширения - проверяем первую строку на HTML
                            if let Ok(file) = std::fs::File::open(path) {
                                let mut reader = BufReader::new(file);
                                let mut first_line = String::new();
                                if reader.read_line(&mut first_line).is_ok() {
                                    let first_line_lower = first_line.to_lowercase();
                                    if first_line_lower.contains("<!doctype html")
                                        || first_line_lower.contains("<html")
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
                    } else {
                        None
                    }
                })
            })
            .collect();

        Ok(files)
    }

    /// Обрабатывает батч файлов
    pub(crate) fn process_batch<F>(
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

                    // Отправляем прогресс по времени, чтобы не "схлопывался" на быстрых машинах
                    if let Some(ref callback) = progress_callback {
                        let total = self.total_files.load(Ordering::Relaxed);
                        if total == 0 {
                            return;
                        }

                        let now_ms = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;
                        let should_report = if count == total {
                            self.last_progress_at.store(now_ms, Ordering::Relaxed);
                            self.last_reported_count.store(count, Ordering::Relaxed);
                            true
                        } else {
                            const PROGRESS_MIN_INTERVAL_MS: u64 = 75;
                            let last_ms = self.last_progress_at.load(Ordering::Relaxed);
                            let last_count = self.last_reported_count.load(Ordering::Relaxed);
                            let enough_time =
                                now_ms.saturating_sub(last_ms) >= PROGRESS_MIN_INTERVAL_MS;
                            let advanced = count > last_count;
                            if enough_time && advanced {
                                self.last_progress_at
                                    .compare_exchange(
                                        last_ms,
                                        now_ms,
                                        Ordering::Relaxed,
                                        Ordering::Relaxed,
                                    )
                                    .is_ok()
                            } else {
                                false
                            }
                        };

                        if should_report {
                            self.last_reported_count.store(count, Ordering::Relaxed);
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
    pub(crate) fn parse_html_file(&self, path: &Path) -> Result<SyntaxNode> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Не удалось прочитать файл {:?}", path))?;
        let document = Html::parse_document(&content);
        if self.settings.collect_keywords && is_language_help_path(path) {
            let keywords = self
                .document_parser
                .html_extractor()
                .extract_keywords(&document);
            for keyword in keywords {
                self.keywords.insert(keyword);
            }
        }

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
                let property_info = self.document_parser.parse_property(path, &document)?;
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
    pub(crate) fn link_types_to_categories(&self) {
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
                    let Some(start) = path.find(&top_level_pattern) else {
                        continue;
                    };
                    let after_pattern = &path[start + top_level_pattern.len()..];

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
    pub(crate) fn save_node(&self, node: SyntaxNode) {
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
                if prop.source_kind == PropertySourceKind::GlobalContextProperty {
                    let source_key = prop.source_key.clone().unwrap_or_else(|| key.clone());
                    self.global_context_properties
                        .insert(source_key.clone(), prop.clone());
                    self.nodes.insert(source_key, SyntaxNode::Property(prop));
                } else {
                    self.nodes.insert(key, SyntaxNode::Property(prop));
                }
            }
            SyntaxNode::GlobalFunction(func) => {
                let key = format!("global_function_{}", func.name);
                self.global_functions.insert(key.clone(), func.clone());
                // Добавим в nodes для общего доступа
                self.nodes.insert(key, SyntaxNode::GlobalFunction(func));
            }
            SyntaxNode::Constructor(cons) => {
                let key = format!("constructor_{}", self.nodes.len());
                self.nodes.insert(key, SyntaxNode::Constructor(cons));
            }
        }
    }
}

fn is_language_help_path(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    path_str.contains("rebuilt.shlang_ru")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_node_indexes_global_context_property_by_source_key() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let html_path = root.join(
            "examples/syntax_helper/rebuilt.shcntx_ru/objects/Global context/properties/Metadata974.html",
        );
        let loader = SyntaxHelperLoader::new();
        let node = loader
            .parse_html_file(&html_path)
            .expect("failed to parse Metadata974.html");

        loader.save_node(node);
        let db = loader.export_database();

        let property = db
            .global_context_properties
            .get("objects/Global context/properties/Metadata974")
            .expect("global-context property should be indexed by source key");

        assert_eq!(property.normalized_global_context_key(), "метаданные");
        assert_eq!(
            property.normalized_global_context_english_key().as_deref(),
            Some("metadata")
        );
        assert!(
            db.properties
                .contains_key("property_Глобальный контекст.Метаданные"),
            "legacy property key is preserved for compatibility"
        );
        assert!(
            !db.nodes
                .contains_key("property_Глобальный контекст.Метаданные"),
            "global-context classification must not depend on legacy property_<name> node key"
        );
    }
}
