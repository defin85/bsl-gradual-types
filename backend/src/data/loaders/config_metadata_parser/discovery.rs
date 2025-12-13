//! Обнаружение и навигация по структуре конфигурации

use super::form_parser::FormParser;
use super::form_types::FormMetadata;
use super::parser::UniversalMetadataParser;
use super::types::{ConfigurationInfo, ConfigurationType, UniversalMetadataObject};
use indicatif::{ProgressBar, ProgressStyle};
use quick_xml::events::Event;
use quick_xml::Reader;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::warn;

// Импорт структур прогресса
use crate::data::loaders::progress::{IndexingPhase, ProgressUpdate};

/// Результат операций discovery
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Обнаруживатель объектов конфигурации
///
/// Читает Configuration.xml и извлекает список всех ChildObjects,
/// затем парсит каждый объект через UniversalMetadataParser.
pub struct ConfigurationDiscovery {
    base_path: PathBuf,
    /// Флаг отображения прогресс-бара в терминале (для CLI/Web сервера)
    show_progress: bool,
}

impl ConfigurationDiscovery {
    /// Создать новый обнаруживатель для указанного пути конфигурации
    ///
    /// # Параметры
    /// - `base_path` - путь к папке с конфигурацией
    /// - `show_progress` - показывать ли прогресс-бар в терминале (true для CLI/Web, false для LSP)
    pub fn new(base_path: PathBuf, show_progress: bool) -> Self {
        Self {
            base_path,
            show_progress,
        }
    }

    /// Конвертирует XML тег из Configuration.xml в название папки
    /// Пример: "Catalog" -> "Catalogs", "Document" -> "Documents"
    fn xml_tag_to_folder_name(&self, xml_tag: &str) -> String {
        match xml_tag {
            "Catalog" => "Catalogs".to_string(),
            "Document" => "Documents".to_string(),
            "Enum" => "Enums".to_string(),
            "InformationRegister" => "InformationRegisters".to_string(),
            "AccumulationRegister" => "AccumulationRegisters".to_string(),
            "AccountingRegister" => "AccountingRegisters".to_string(),
            "CalculationRegister" => "CalculationRegisters".to_string(),
            "ChartOfAccounts" => "ChartsOfAccounts".to_string(),
            "ChartOfCharacteristicTypes" => "ChartsOfCharacteristicTypes".to_string(),
            "ChartOfCalculationTypes" => "ChartsOfCalculationTypes".to_string(),
            "Report" => "Reports".to_string(),
            "DataProcessor" => "DataProcessors".to_string(),
            "BusinessProcess" => "BusinessProcesses".to_string(),
            "Task" => "Tasks".to_string(),
            "ExchangePlan" => "ExchangePlans".to_string(),
            "Constant" => "Constants".to_string(),
            "Role" => "Roles".to_string(),
            "CommonModule" => "CommonModules".to_string(),
            "Subsystem" => "Subsystems".to_string(),
            "Language" => "Languages".to_string(),
            _ => xml_tag.to_string(), // Fallback для неизвестных типов
        }
    }

    /// Парсит только Properties секцию Configuration.xml для быстрого определения типа
    ///
    /// Извлекает:
    /// - <ObjectBelonging> — ГЛАВНЫЙ маркер расширения
    /// - <ConfigurationExtensionPurpose> — дополнительный маркер
    /// - <Name> — название конфигурации
    /// - <NamePrefix> — префикс расширения
    ///
    /// Останавливается после закрытия </Properties> — минимальная нагрузка на I/O
    fn detect_configuration_type(config_xml: &Path) -> Result<ConfigurationInfo> {
        tracing::debug!("🔍 Определение типа конфигурации: {:?}", config_xml);

        let content = fs::read_to_string(config_xml)?;
        let mut reader = Reader::from_str(&content);
        reader.trim_text(true);

        let mut buf = Vec::new();
        let mut in_properties = false;
        let mut current_tag: Option<String> = None;

        // Извлекаемые данные
        let mut has_object_belonging_adopted = false;
        let mut has_config_extension_purpose = false;
        let mut name: Option<String> = None;
        let mut prefix: Option<String> = None;
        let mut uuid: Option<String> = None;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    if tag_name == "Properties" {
                        in_properties = true;
                        tracing::trace!("  → Вошли в <Properties>");
                    } else if in_properties {
                        current_tag = Some(tag_name.clone());
                    } else if tag_name == "Configuration" {
                        // Извлекаем UUID из атрибута
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"uuid" {
                                uuid = Some(String::from_utf8_lossy(&attr.value).to_string());
                            }
                        }
                    }
                }
                Ok(Event::Text(e)) => {
                    if let Some(ref tag) = current_tag {
                        let text = e.unescape()?.trim().to_string();

                        match tag.as_str() {
                            "ObjectBelonging" if text == "Adopted" => {
                                has_object_belonging_adopted = true;
                                tracing::debug!("  ✅ Найден маркер: <ObjectBelonging>Adopted</ObjectBelonging>");
                            }
                            "ConfigurationExtensionPurpose" => {
                                has_config_extension_purpose = true;
                                tracing::debug!("  ✅ Найден маркер: <ConfigurationExtensionPurpose>{}</ConfigurationExtensionPurpose>", text);
                            }
                            "Name" if name.is_none() => {
                                name = Some(text.clone());
                                tracing::debug!("  📝 Имя конфигурации: {}", text);
                            }
                            "NamePrefix" if !text.is_empty() => {
                                prefix = Some(text.clone());
                                tracing::debug!("  🏷️ Префикс: {}", text);
                            }
                            _ => {}
                        }
                    }
                }
                Ok(Event::End(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    if tag_name == "Properties" {
                        // Завершаем парсинг при выходе из блока Properties
                        tracing::trace!("  → Вышли из </Properties> — завершаем парсинг");
                        break; // ⚡ ОПТИМИЗАЦИЯ: не читаем весь файл!
                    } else if current_tag.as_ref() == Some(&tag_name) {
                        current_tag = None;
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    tracing::error!("❌ Ошибка парсинга Configuration.xml: {:?}", e);
                    return Err(Box::new(e));
                }
                _ => {}
            }
            buf.clear();
        }

        // Определяем тип конфигурации
        let is_extension = has_object_belonging_adopted || has_config_extension_purpose;

        let config_name = name.ok_or("Не найден тег <Name> в Configuration.xml")?;
        let config_path = config_xml
            .parent()
            .ok_or("Не удалось получить родительскую папку Configuration.xml")?
            .to_path_buf();

        let mut info = if is_extension {
            tracing::debug!("🧩 Расширение: {} (prefix: {:?})", config_name, prefix);
            ConfigurationInfo::extension(config_path, config_name, prefix)
        } else {
            tracing::debug!("📦 Основная конфигурация: {}", config_name);
            ConfigurationInfo::base(config_path, config_name)
        };

        info.uuid = uuid;

        Ok(info)
    }

    /// Обнаруживает ВСЕ конфигурации (основную + расширения) в указанной директории
    ///
    /// Алгоритм:
    /// 1. Проверяет Configuration.xml прямо в base_path (обратная совместимость)
    /// 2. Сканирует все подпапки на наличие Configuration.xml
    /// 3. Для каждого найденного определяет тип (Base или Extension)
    /// 4. Сортирует результат: Base первым, Extension после
    ///
    /// Возвращает:
    /// - `Vec<ConfigurationInfo>` — отсортированный список (база → расширения)
    pub fn discover_all_configurations(&self) -> Result<Vec<ConfigurationInfo>> {
        tracing::debug!("🔍 Обнаружение всех конфигураций в {:?}", self.base_path);

        let mut configurations = Vec::new();

        // 1. Проверяем прямо в base_path (обратная совместимость)
        let direct_config = self.base_path.join("Configuration.xml");
        if direct_config.exists() {
            tracing::debug!(
                "  📄 Configuration.xml найден напрямую в {:?}",
                self.base_path
            );

            match Self::detect_configuration_type(&direct_config) {
                Ok(info) => configurations.push(info),
                Err(e) => tracing::debug!(
                    "⚠️ Не удалось определить тип конфигурации {:?}: {}",
                    direct_config,
                    e
                ),
            }
        }

        // 2. Сканируем подпапки
        tracing::debug!("  🔍 Сканирование подпапок в {:?}...", self.base_path);

        let entries = fs::read_dir(&self.base_path).map_err(|e| {
            format!(
                "Не удалось прочитать директорию {:?}: {}",
                self.base_path, e
            )
        })?;

        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let subdir_path = entry.path();
                    let config_in_subdir = subdir_path.join("Configuration.xml");

                    if config_in_subdir.exists() {
                        tracing::debug!("    📄 Configuration.xml найден в {:?}", subdir_path);

                        match Self::detect_configuration_type(&config_in_subdir) {
                            Ok(info) => configurations.push(info),
                            Err(e) => tracing::debug!(
                                "⚠️ Не удалось определить тип конфигурации {:?}: {}",
                                config_in_subdir,
                                e
                            ),
                        }
                    }
                }
            }
        }

        if configurations.is_empty() {
            return Err(format!(
                "Configuration.xml не найден ни в {:?}, ни в подпапках. \
                Убедитесь, что указан правильный путь к выгруженной конфигурации.",
                self.base_path
            )
            .into());
        }

        // 3. Сортировка: Base первым, Extension после
        configurations.sort_by_key(|info| match info.config_type {
            ConfigurationType::Base => 0,
            ConfigurationType::Extension => 1,
        });

        tracing::debug!(
            "✅ Найдено конфигураций: {} (Base: {}, Extension: {})",
            configurations.len(),
            configurations.iter().filter(|c| c.is_base()).count(),
            configurations.iter().filter(|c| c.is_extension()).count()
        );

        for (idx, info) in configurations.iter().enumerate() {
            tracing::debug!(
                "  {}. {} {} ({})",
                idx + 1,
                if info.is_base() { "📦" } else { "🧩" },
                info.name,
                info.path.display()
            );
        }

        Ok(configurations)
    }

    /// Обнаруживает объекты метаданных в конкретной конфигурации (с прогрессом через 4 фазы)
    ///
    /// Использует ConfigurationInfo для определения пути к Configuration.xml
    /// и парсит все ChildObjects в этой конфигурации параллельно через rayon.
    ///
    /// Новая версия с поддержкой 4 фаз: Discovery → Parsing → Linking → Finalizing
    ///
    /// # Параметры
    /// - `config_info`: Информация о конфигурации (путь, имя, тип)
    /// - `progress_callback`: Callback для отслеживания прогресса (обязательный для новых фаз)
    pub fn discover_metadata_in_configuration_with_progress<F>(
        &self,
        config_info: &ConfigurationInfo,
        progress_callback: F,
    ) -> Result<Vec<UniversalMetadataObject>>
    where
        F: Fn(ProgressUpdate) + Send + Sync + Clone + 'static,
    {
        tracing::debug!(
            "📦 [4-Phase] Парсинг метаданных из конфигурации: {} ({:?})",
            config_info.name,
            config_info.config_type
        );

        let config_xml = config_info.path.join("Configuration.xml");

        if !config_xml.exists() {
            return Err(format!("Configuration.xml не найден: {:?}", config_xml).into());
        }

        // === PHASE 1: Discovery (0-5%) ===
        progress_callback(ProgressUpdate::new(
            IndexingPhase::ConfigurationDiscovery,
            0,
            1,
            Some(format!(
                "Сканирование структуры конфигурации {}",
                config_info.name
            )),
        ));

        let child_objects = self.parse_child_objects_list(&config_xml)?;

        // Собираем все задачи парсинга в один вектор
        let parsing_tasks: Vec<_> = child_objects
            .iter()
            .flat_map(|(object_type, object_names)| {
                object_names
                    .iter()
                    .map(move |name| (object_type.clone(), name.clone()))
            })
            .collect();

        let total_objects = parsing_tasks.len();
        tracing::debug!(
            "📊 Найдено {} объектов для парсинга в конфигурации {}",
            total_objects,
            config_info.name
        );

        progress_callback(ProgressUpdate::new(
            IndexingPhase::ConfigurationDiscovery,
            1,
            1,
            Some(format!("Обнаружено {} объектов", total_objects)),
        ));

        // === PHASE 2: Parsing (5-85%) ===
        progress_callback(ProgressUpdate::new(
            IndexingPhase::ConfigurationParsing,
            0,
            total_objects,
            Some("Начинаем парсинг XML файлов...".to_string()),
        ));

        // Создаём терминальный прогресс-бар если show_progress == true
        let terminal_progress = if self.show_progress {
            let pb = ProgressBar::new(total_objects as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg} [{per_sec}]")
                    .unwrap()
                    .progress_chars("##-"),
            );
            pb.set_message(format!("Парсинг {}", config_info.name));
            Some(pb)
        } else {
            None
        };

        // Счётчик обработанных объектов (потокобезопасный)
        let processed = Arc::new(AtomicUsize::new(0));

        // Параллельный парсинг через rayon::par_iter
        let all_metadata: Vec<_> = parsing_tasks
            .par_iter()
            .filter_map(|(object_type, object_name)| {
                // Определяем путь к XML файлу
                let folder_name = self.xml_tag_to_folder_name(object_type);

                // XML файлы могут быть в двух местах:
                // 1. Прямо в папке типа: Catalogs/Контрагенты.xml
                // 2. В подпапке: Catalogs/Контрагенты/Контрагенты.xml
                let xml_file_direct = config_info
                    .path
                    .join(&folder_name)
                    .join(format!("{}.xml", object_name));

                let xml_file_subdir = config_info
                    .path
                    .join(&folder_name)
                    .join(object_name)
                    .join(format!("{}.xml", object_name));

                let xml_file = if xml_file_direct.exists() {
                    xml_file_direct
                } else {
                    xml_file_subdir
                };

                if !xml_file.exists() {
                    tracing::debug!(
                        "⚠️ XML файл не найден для объекта {}.{}: {:?}",
                        object_type,
                        object_name,
                        xml_file
                    );
                    return None;
                }

                // Парсинг объекта
                match UniversalMetadataParser::parse_any_object(&xml_file) {
                    Ok(mut metadata) => {
                        // Увеличиваем счётчик обработанных объектов
                        let count = processed.fetch_add(1, Ordering::Relaxed) + 1;

                        // Обновляем терминальный прогресс-бар (thread-safe)
                        if let Some(ref pb) = terminal_progress {
                            pb.inc(1);
                            pb.set_message(object_name.to_string());
                        }

                        tracing::trace!(
                            "    ✅ Распарсен объект: {} ({}/{})",
                            metadata.name,
                            count,
                            total_objects
                        );

                        // Парсим формы для Document и Catalog
                        if matches!(object_type.as_str(), "Document" | "Catalog") {
                            // Создаём временный discovery для поиска форм (с корректным base_path)
                            // show_progress = false чтобы не создавать вложенные прогресс-бары
                            let forms_discovery = ConfigurationDiscovery::new(config_info.path.clone(), false);

                            if let Ok(forms) = forms_discovery.discover_forms(&folder_name, object_name) {
                                metadata.forms = forms;
                                if !metadata.forms.is_empty() {
                                    tracing::trace!(
                                        "      📋 Обнаружено {} форм для {}.{}",
                                        metadata.forms.len(),
                                        object_type,
                                        object_name
                                    );
                                }
                            }

                            // Парсим модули объекта (ObjectModule, ManagerModule)
                            let (object_mod, manager_mod, record_set_mod) =
                                forms_discovery.discover_object_modules(&folder_name, object_name);

                            metadata.object_module_path = object_mod;
                            metadata.manager_module_path = manager_mod;
                            metadata.record_set_module_path = record_set_mod;

                            if metadata.object_module_path.is_some()
                                || metadata.manager_module_path.is_some()
                                || metadata.record_set_module_path.is_some()
                            {
                                tracing::trace!(
                                    "      📦 Обнаружены модули для {}.{} (object: {}, manager: {}, record_set: {})",
                                    object_type,
                                    object_name,
                                    metadata.object_module_path.is_some(),
                                    metadata.manager_module_path.is_some(),
                                    metadata.record_set_module_path.is_some()
                                );
                            }
                        }

                        // Throttling: отправляем прогресс каждые 5 объектов или на последнем
                        if count.is_multiple_of(5) || count == total_objects {
                            progress_callback(ProgressUpdate::new(
                                IndexingPhase::ConfigurationParsing,
                                count,
                                total_objects,
                                Some(format!("Файл {}/{}: {}", count, total_objects, object_name)),
                            ));
                        }

                        Some(metadata)
                    }
                    Err(e) => {
                        tracing::error!(
                            "❌ Ошибка парсинга объекта {}.{}: {}",
                            object_type,
                            object_name,
                            e
                        );
                        None
                    }
                }
            })
            .collect();

        // Завершаем терминальный прогресс-бар
        if let Some(pb) = terminal_progress {
            pb.finish_with_message(format!(
                "Парсинг {} завершён ({} объектов)",
                config_info.name,
                all_metadata.len()
            ));
        }

        // === PHASE 3: Linking (85-95%) ===
        progress_callback(ProgressUpdate::new(
            IndexingPhase::ConfigurationLinking,
            0,
            1,
            Some("Построение связей между типами...".to_string()),
        ));

        // Здесь можно добавить логику связывания типов (пока просто сообщаем о завершении)
        progress_callback(ProgressUpdate::new(
            IndexingPhase::ConfigurationLinking,
            1,
            1,
            Some(format!("Обработано {} типов", all_metadata.len())),
        ));

        // === PHASE 4: Finalizing (95-100%) ===
        progress_callback(ProgressUpdate::new(
            IndexingPhase::ConfigurationFinalizing,
            0,
            1,
            Some(format!(
                "Финализация загрузки конфигурации {}",
                config_info.name
            )),
        ));

        tracing::debug!(
            "✅ Обнаружено {} объектов метаданных в конфигурации {}",
            all_metadata.len(),
            config_info.name
        );

        progress_callback(ProgressUpdate::new(
            IndexingPhase::ConfigurationFinalizing,
            1,
            1,
            Some(format!(
                "Загружено {} типов конфигурации",
                all_metadata.len()
            )),
        ));

        Ok(all_metadata)
    }

    /// Обнаруживает объекты метаданных в конкретной конфигурации БЕЗ прогресса (обратная совместимость)
    ///
    /// Делегирует вызов методу с прогрессом, передавая no-op callback.
    ///
    /// # Параметры
    /// - `config_info`: Информация о конфигурации (путь, имя, тип)
    /// - `progress_callback`: Опциональный callback для отслеживания прогресса (каждые 5 объектов)
    pub fn discover_metadata_in_configuration<F>(
        &self,
        config_info: &ConfigurationInfo,
        progress_callback: Option<F>,
    ) -> Result<Vec<UniversalMetadataObject>>
    where
        F: Fn(ProgressUpdate) + Send + Sync + Clone + 'static,
    {
        // ✅ Делегируем методу с прогрессом, передавая callback (или no-op если None)
        if let Some(callback) = progress_callback {
            self.discover_metadata_in_configuration_with_progress(config_info, callback)
        } else {
            // No-op callback для обратной совместимости
            self.discover_metadata_in_configuration_with_progress(config_info, |_| {
                // Ничего не делаем - обратная совместимость для кода без прогресса
            })
        }
    }

    /// Парсит Configuration.xml и извлекает список ChildObjects
    ///
    /// Возвращает HashMap: тип объекта -> список имён объектов
    /// Пример: {"Catalog": ["Контрагенты", "Номенклатура"], "Document": ["РеализацияТоваров"]}
    pub fn parse_child_objects_list(
        &self,
        config_xml: &Path,
    ) -> Result<HashMap<String, Vec<String>>> {
        tracing::debug!(
            "📄 Парсинг ChildObjects из Configuration.xml: {:?}",
            config_xml
        );

        let content = fs::read_to_string(config_xml)?;
        let mut reader = Reader::from_str(&content);
        reader.trim_text(true);

        let mut child_objects: HashMap<String, Vec<String>> = HashMap::new();
        let mut buf = Vec::new();
        let mut in_child_objects = false;
        let mut current_object_type: Option<String> = None;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    if tag_name == "ChildObjects" {
                        in_child_objects = true;
                        tracing::trace!("🔍 Вошли в секцию <ChildObjects>");
                    } else if in_child_objects && current_object_type.is_none() {
                        // Это тег типа объекта (Catalog, Document, etc.)
                        current_object_type = Some(tag_name.clone());
                        tracing::trace!("📦 Обнаружен тип объекта: {}", tag_name);
                    }
                }
                Ok(Event::Text(e)) => {
                    if let Some(ref obj_type) = current_object_type {
                        let obj_name = e.unescape()?.trim().to_string();
                        if !obj_name.is_empty() {
                            child_objects
                                .entry(obj_type.clone())
                                .or_default()
                                .push(obj_name.clone());
                            tracing::trace!("  ➕ Объект: {} (тип: {})", obj_name, obj_type);
                        }
                    }
                }
                Ok(Event::End(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    if tag_name == "ChildObjects" {
                        in_child_objects = false;
                        tracing::debug!("✅ Завершён парсинг ChildObjects");
                    } else if current_object_type.as_ref() == Some(&tag_name) {
                        current_object_type = None;
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    tracing::error!("❌ Ошибка парсинга XML: {:?}", e);
                    return Err(Box::new(e));
                }
                _ => {}
            }
            buf.clear();
        }

        tracing::debug!(
            "📊 Найдено {} типов объектов, всего {} объектов",
            child_objects.len(),
            child_objects.values().map(|v| v.len()).sum::<usize>()
        );

        Ok(child_objects)
    }

    /// Автоматически обнаруживает папку с конфигурацией
    ///
    /// Если Configuration.xml находится прямо в base_path - возвращает base_path.
    /// Иначе сканирует подпапки и возвращает первую подпапку с Configuration.xml.
    #[allow(dead_code)] // Вспомогательный метод для будущего использования
    fn find_configuration_folder(&self) -> Result<PathBuf> {
        // Сначала проверяем прямо в base_path (обратная совместимость)
        let direct_config = self.base_path.join("Configuration.xml");
        if direct_config.exists() {
            tracing::debug!(
                "✅ Configuration.xml найден напрямую в {:?}",
                self.base_path
            );
            return Ok(self.base_path.clone());
        }

        // Если не найден - сканируем подпапки
        tracing::debug!(
            "🔍 Сканирование подпапок в {:?} для поиска конфигурации...",
            self.base_path
        );

        let entries = fs::read_dir(&self.base_path).map_err(|e| {
            format!(
                "Не удалось прочитать директорию {:?}: {}",
                self.base_path, e
            )
        })?;

        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let subdir_path = entry.path();
                    let config_in_subdir = subdir_path.join("Configuration.xml");

                    if config_in_subdir.exists() {
                        tracing::debug!("✅ Найдена конфигурация в подпапке: {:?}", subdir_path);
                        return Ok(subdir_path);
                    }
                }
            }
        }

        Err(format!(
            "Configuration.xml не найден ни в {:?}, ни в подпапках. \
            Убедитесь, что указан правильный путь к выгруженной конфигурации.",
            self.base_path
        )
        .into())
    }

    /// УСТАРЕВШИЙ МЕТОД: Обнаруживает все объекты метаданных (только ПЕРВОЙ конфигурации)
    ///
    /// ⚠️ DEPRECATION: Используйте discover_all_configurations() + discover_metadata_in_configuration()
    ///
    /// Оставлен для обратной совместимости с существующими тестами.
    pub fn discover_all_metadata<F>(
        &self,
        progress_callback: Option<F>,
    ) -> Result<Vec<UniversalMetadataObject>>
    where
        F: Fn(ProgressUpdate) + Send + Sync + Clone + 'static,
    {
        tracing::debug!(
            "⚠️ discover_all_metadata() вызван — используйте discover_all_configurations()"
        );

        // Обратная совместимость: возвращаем метаданные ПЕРВОЙ конфигурации
        let configurations = self.discover_all_configurations()?;

        if configurations.is_empty() {
            return Err("Не найдено ни одной конфигурации".into());
        }

        let first_config = &configurations[0];
        tracing::debug!("  → Загружаем метаданные из {}", first_config.name);

        self.discover_metadata_in_configuration(first_config, progress_callback)
    }

    /// Обнаруживает модули объекта (ObjectModule, ManagerModule, RecordSetModule)
    ///
    /// Проверяет наличие модулей в папке Ext объекта метаданных.
    ///
    /// # Параметры
    ///
    /// - `object_type` - тип объекта в множественном числе ("Documents", "Catalogs")
    /// - `object_name` - имя объекта ("ЗаказНаряды", "Контрагенты")
    ///
    /// # Возвращает
    ///
    /// Кортеж из трёх опциональных PathBuf:
    /// - ObjectModule.bsl
    /// - ManagerModule.bsl
    /// - RecordSetModule.bsl
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let (object_mod, manager_mod, record_set_mod) =
    ///     discovery.discover_object_modules("Catalogs", "Контрагенты");
    /// assert!(object_mod.is_some());
    /// ```
    pub fn discover_object_modules(
        &self,
        object_type: &str,  // "Documents", "Catalogs"
        object_name: &str,  // "ЗаказНаряды", "Контрагенты"
    ) -> (Option<PathBuf>, Option<PathBuf>, Option<PathBuf>) {
        let base_dir = self.base_path
            .join(object_type)
            .join(object_name)
            .join("Ext");

        let object_module = base_dir.join("ObjectModule.bsl");
        let manager_module = base_dir.join("ManagerModule.bsl");
        let record_set_module = base_dir.join("RecordSetModule.bsl");

        tracing::trace!(
            "🔍 Scanning for modules in {:?} (object_type: {}, object_name: {})",
            base_dir,
            object_type,
            object_name
        );

        (
            if object_module.exists() {
                tracing::trace!("  ✅ Found ObjectModule.bsl");
                Some(object_module)
            } else {
                None
            },
            if manager_module.exists() {
                tracing::trace!("  ✅ Found ManagerModule.bsl");
                Some(manager_module)
            } else {
                None
            },
            if record_set_module.exists() {
                tracing::trace!("  ✅ Found RecordSetModule.bsl");
                Some(record_set_module)
            } else {
                None
            },
        )
    }

    /// Обнаруживает формы для объекта метаданных
    ///
    /// Сканирует папку Forms внутри объекта и парсит все найденные Form.xml файлы.
    ///
    /// # Параметры
    ///
    /// - `object_type` - тип объекта в множественном числе ("Documents", "Catalogs")
    /// - `object_name` - имя объекта ("ЗаказНаряды", "Контрагенты")
    ///
    /// # Возвращает
    ///
    /// Вектор `FormMetadata` с информацией о найденных формах
    ///
    /// # Errors
    ///
    /// Возвращает ошибку при проблемах с чтением файловой системы
    pub fn discover_forms(
        &self,
        object_type: &str,  // "Documents", "Catalogs"
        object_name: &str,  // "ЗаказНаряды", "Контрагенты"
    ) -> Result<Vec<FormMetadata>> {
        let forms_dir = self.base_path
            .join(object_type)
            .join(object_name)
            .join("Forms");

        if !forms_dir.exists() {
            tracing::trace!("  ℹ️ Forms directory not found: {:?}", forms_dir);
            return Ok(Vec::new());
        }

        let mut forms = Vec::new();

        for entry in fs::read_dir(&forms_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }

            let form_name = entry.file_name().to_string_lossy().to_string();
            let form_xml = entry.path().join("Ext").join("Form.xml");

            if form_xml.exists() {
                // Преобразуем тип объекта: Documents -> Document
                let owner_type_singular = object_type.trim_end_matches('s');
                let owner_type = format!("{}.{}", owner_type_singular, object_name);

                match FormParser::parse_form_xml(&form_xml, &owner_type, &form_name) {
                    Ok(form) => {
                        tracing::trace!("    ✅ Parsed form: {}", form_name);
                        forms.push(form);
                    }
                    Err(e) => {
                        warn!("⚠️ Failed to parse form {}: {}", form_name, e);
                    }
                }
            }
        }

        if !forms.is_empty() {
            tracing::debug!("📋 Discovered {} forms for {}.{}", forms.len(), object_type, object_name);
        }

        Ok(forms)
    }
}
