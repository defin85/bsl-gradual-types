//! Обнаружение и навигация по структуре конфигурации

use super::parser::UniversalMetadataParser;
use super::types::{ConfigurationInfo, ConfigurationType, UniversalMetadataObject};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Результат операций discovery
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Обнаруживатель объектов конфигурации
///
/// Читает Configuration.xml и извлекает список всех ChildObjects,
/// затем парсит каждый объект через UniversalMetadataParser.
pub struct ConfigurationDiscovery {
    base_path: PathBuf,
}

impl ConfigurationDiscovery {
    /// Создать новый обнаруживатель для указанного пути конфигурации
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
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
                        in_properties = false;
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
        let config_path = config_xml.parent()
            .ok_or("Не удалось получить родительскую папку Configuration.xml")?
            .to_path_buf();

        let mut info = if is_extension {
            tracing::info!("🧩 Расширение: {} (prefix: {:?})", config_name, prefix);
            ConfigurationInfo::extension(config_path, config_name, prefix)
        } else {
            tracing::info!("📦 Основная конфигурация: {}", config_name);
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
        tracing::info!("🔍 Обнаружение всех конфигураций в {:?}", self.base_path);

        let mut configurations = Vec::new();

        // 1. Проверяем прямо в base_path (обратная совместимость)
        let direct_config = self.base_path.join("Configuration.xml");
        if direct_config.exists() {
            tracing::debug!("  📄 Configuration.xml найден напрямую в {:?}", self.base_path);

            match Self::detect_configuration_type(&direct_config) {
                Ok(info) => configurations.push(info),
                Err(e) => tracing::warn!("⚠️ Не удалось определить тип конфигурации {:?}: {}", direct_config, e),
            }
        }

        // 2. Сканируем подпапки
        tracing::debug!("  🔍 Сканирование подпапок в {:?}...", self.base_path);

        let entries = fs::read_dir(&self.base_path)
            .map_err(|e| format!("Не удалось прочитать директорию {:?}: {}", self.base_path, e))?;

        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let subdir_path = entry.path();
                    let config_in_subdir = subdir_path.join("Configuration.xml");

                    if config_in_subdir.exists() {
                        tracing::debug!("    📄 Configuration.xml найден в {:?}", subdir_path);

                        match Self::detect_configuration_type(&config_in_subdir) {
                            Ok(info) => configurations.push(info),
                            Err(e) => tracing::warn!("⚠️ Не удалось определить тип конфигурации {:?}: {}", config_in_subdir, e),
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
            ).into());
        }

        // 3. Сортировка: Base первым, Extension после
        configurations.sort_by_key(|info| match info.config_type {
            ConfigurationType::Base => 0,
            ConfigurationType::Extension => 1,
        });

        tracing::info!(
            "✅ Найдено конфигураций: {} (Base: {}, Extension: {})",
            configurations.len(),
            configurations.iter().filter(|c| c.is_base()).count(),
            configurations.iter().filter(|c| c.is_extension()).count()
        );

        for (idx, info) in configurations.iter().enumerate() {
            tracing::info!(
                "  {}. {} {} ({})",
                idx + 1,
                if info.is_base() { "📦" } else { "🧩" },
                info.name,
                info.path.display()
            );
        }

        Ok(configurations)
    }

    /// Обнаруживает объекты метаданных в конкретной конфигурации
    ///
    /// Использует ConfigurationInfo для определения пути к Configuration.xml
    /// и парсит все ChildObjects в этой конфигурации.
    pub fn discover_metadata_in_configuration(
        &self,
        config_info: &ConfigurationInfo,
    ) -> Result<Vec<UniversalMetadataObject>> {
        tracing::info!(
            "📦 Парсинг метаданных из конфигурации: {} ({:?})",
            config_info.name,
            config_info.config_type
        );

        let config_xml = config_info.path.join("Configuration.xml");

        if !config_xml.exists() {
            return Err(format!("Configuration.xml не найден: {:?}", config_xml).into());
        }

        let child_objects = self.parse_child_objects_list(&config_xml)?;
        let mut all_metadata = Vec::new();

        for (object_type, object_names) in child_objects {
            tracing::debug!(
                "  📂 Обработка типа объектов: {} ({} шт.)",
                object_type,
                object_names.len()
            );

            let folder_name = self.xml_tag_to_folder_name(&object_type);

            for object_name in object_names {
                // XML файлы могут быть в двух местах:
                // 1. Прямо в папке типа: Catalogs/Контрагенты.xml
                // 2. В подпапке: Catalogs/Контрагенты/Контрагенты.xml
                let xml_file_direct = config_info.path
                    .join(&folder_name)
                    .join(format!("{}.xml", object_name));

                let xml_file_subdir = config_info.path
                    .join(&folder_name)
                    .join(&object_name)
                    .join(format!("{}.xml", object_name));

                let xml_file = if xml_file_direct.exists() {
                    xml_file_direct
                } else {
                    xml_file_subdir
                };

                if !xml_file.exists() {
                    tracing::warn!(
                        "⚠️ XML файл не найден для объекта {}.{}: {:?}",
                        object_type,
                        object_name,
                        xml_file
                    );
                    continue;
                }

                match UniversalMetadataParser::parse_any_object(&xml_file) {
                    Ok(metadata) => {
                        tracing::trace!("    ✅ Распарсен объект: {}", metadata.name);
                        all_metadata.push(metadata);
                    }
                    Err(e) => {
                        tracing::error!(
                            "❌ Ошибка парсинга объекта {}.{}: {}",
                            object_type,
                            object_name,
                            e
                        );
                    }
                }
            }
        }

        tracing::info!(
            "✅ Обнаружено {} объектов метаданных в конфигурации {}",
            all_metadata.len(),
            config_info.name
        );

        Ok(all_metadata)
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

        tracing::info!(
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
    fn find_configuration_folder(&self) -> Result<PathBuf> {
        // Сначала проверяем прямо в base_path (обратная совместимость)
        let direct_config = self.base_path.join("Configuration.xml");
        if direct_config.exists() {
            tracing::debug!("✅ Configuration.xml найден напрямую в {:?}", self.base_path);
            return Ok(self.base_path.clone());
        }

        // Если не найден - сканируем подпапки
        tracing::debug!("🔍 Сканирование подпапок в {:?} для поиска конфигурации...", self.base_path);

        let entries = fs::read_dir(&self.base_path)
            .map_err(|e| format!("Не удалось прочитать директорию {:?}: {}", self.base_path, e))?;

        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let subdir_path = entry.path();
                    let config_in_subdir = subdir_path.join("Configuration.xml");

                    if config_in_subdir.exists() {
                        tracing::info!("✅ Найдена конфигурация в подпапке: {:?}", subdir_path);
                        return Ok(subdir_path);
                    }
                }
            }
        }

        Err(format!(
            "Configuration.xml не найден ни в {:?}, ни в подпапках. \
            Убедитесь, что указан правильный путь к выгруженной конфигурации.",
            self.base_path
        ).into())
    }

    /// УСТАРЕВШИЙ МЕТОД: Обнаруживает все объекты метаданных (только ПЕРВОЙ конфигурации)
    ///
    /// ⚠️ DEPRECATION: Используйте discover_all_configurations() + discover_metadata_in_configuration()
    ///
    /// Оставлен для обратной совместимости с существующими тестами.
    pub fn discover_all_metadata(&self) -> Result<Vec<UniversalMetadataObject>> {
        tracing::warn!("⚠️ discover_all_metadata() вызван — используйте discover_all_configurations()");

        // Обратная совместимость: возвращаем метаданные ПЕРВОЙ конфигурации
        let configurations = self.discover_all_configurations()?;

        if configurations.is_empty() {
            return Err("Не найдено ни одной конфигурации".into());
        }

        let first_config = &configurations[0];
        tracing::info!("  → Загружаем метаданные из {}", first_config.name);

        self.discover_metadata_in_configuration(first_config)
    }
}
