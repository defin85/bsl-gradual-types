//! Обнаружение и навигация по структуре конфигурации

use super::parser::UniversalMetadataParser;
use super::types::UniversalMetadataObject;
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
                                .or_insert_with(Vec::new)
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

    /// Обнаруживает все объекты метаданных в конфигурации
    ///
    /// Читает Configuration.xml, извлекает ChildObjects,
    /// затем парсит каждый объект через UniversalMetadataParser.
    pub fn discover_all_metadata(&self) -> Result<Vec<UniversalMetadataObject>> {
        tracing::info!("🔍 Начало обнаружения метаданных в {:?}", self.base_path);

        let config_xml = self.base_path.join("Configuration.xml");
        if !config_xml.exists() {
            tracing::error!("❌ Configuration.xml не найден: {:?}", config_xml);
            return Err(format!("Configuration.xml не найден: {:?}", config_xml).into());
        }

        let child_objects = self.parse_child_objects_list(&config_xml)?;
        let mut all_metadata = Vec::new();

        for (object_type, object_names) in child_objects {
            tracing::debug!(
                "📦 Обработка типа объектов: {} ({} шт.)",
                object_type,
                object_names.len()
            );

            // Маппинг XML тегов на названия папок в файловой системе
            let folder_name = self.xml_tag_to_folder_name(&object_type);

            for object_name in object_names {
                // XML файлы могут быть в двух местах:
                // 1. Прямо в папке типа: Catalogs/Контрагенты.xml
                // 2. В подпапке: Catalogs/Контрагенты/Контрагенты.xml
                let xml_file_direct = self
                    .base_path
                    .join(&folder_name)
                    .join(format!("{}.xml", object_name));

                let xml_file_subdir = self
                    .base_path
                    .join(&folder_name)
                    .join(&object_name)
                    .join(format!("{}.xml", object_name));

                let xml_file = if xml_file_direct.exists() {
                    xml_file_direct
                } else {
                    xml_file_subdir
                };

                if !xml_file.exists() {
                    eprintln!(
                        "⚠️ XML файл не найден для объекта {}.{}: {:?}",
                        object_type, object_name, xml_file
                    );
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
                        tracing::trace!("  ✅ Распарсен объект: {}", metadata.name);
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

        tracing::info!("✅ Обнаружено {} объектов метаданных", all_metadata.len());
        Ok(all_metadata)
    }
}
