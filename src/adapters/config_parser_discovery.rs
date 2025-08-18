//! Discovery-based парсер конфигурации 1С:Предприятие
//! 
//! Основные принципы:
//! - Никаких предположений о структуре каталогов
//! - Динамическое обнаружение типов метаданных из XML
//! - Рекурсивный обход всех каталогов
//! - Автоматическое определение типа объекта по содержимому

use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::types::{
    Attribute, Certainty, ConcreteType, ConfigurationType, FacetKind,
    MetadataKind, ResolutionMetadata, ResolutionResult, ResolutionSource,
    TabularSection, TypeResolution,
};

/// Discovery-based парсер конфигурации
#[derive(Debug)]
pub struct ConfigurationDiscoveryParser {
    config_path: PathBuf,
    discovered_objects: HashMap<String, DiscoveredMetadata>,
}

/// Обнаруженные метаданные объекта
#[derive(Debug, Clone)]
pub struct DiscoveredMetadata {
    pub name: String,
    pub kind: MetadataKind,
    pub qualified_name: String,
    pub file_path: PathBuf,
    pub synonym: Option<String>,
    pub uuid: Option<String>,
    pub attributes: Vec<AttributeInfo>,
    pub tabular_sections: Vec<TabularSectionInfo>,
    pub discovery_context: DiscoveryContext,
}

/// Контекст обнаружения
#[derive(Debug, Clone)]
pub struct DiscoveryContext {
    pub discovered_from_path: String,
    pub xml_root_element: String,
    pub discovery_method: DiscoveryMethod,
}

/// Метод обнаружения
#[derive(Debug, Clone)]
pub enum DiscoveryMethod {
    /// По корневому элементу XML
    XmlRootElement,
    /// По структуре каталогов
    DirectoryStructure,
    /// По содержимому файла
    FileContent,
}

/// Информация об атрибуте
#[derive(Debug, Clone)]
pub struct AttributeInfo {
    pub name: String,
    pub type_definition: String,
    pub synonym: Option<String>,
    pub mandatory: bool,
}

/// Информация о табличной части
#[derive(Debug, Clone)]
pub struct TabularSectionInfo {
    pub name: String,
    pub synonym: Option<String>,
    pub attributes: Vec<AttributeInfo>,
}

impl ConfigurationDiscoveryParser {
    /// Создать новый discovery-based парсер
    pub fn new<P: AsRef<Path>>(config_path: P) -> Self {
        Self {
            config_path: config_path.as_ref().to_path_buf(),
            discovered_objects: HashMap::new(),
        }
    }

    /// Запустить discovery парсинг конфигурации
    pub fn discover_and_parse(&mut self) -> Result<Vec<TypeResolution>> {
        println!("🔍 Запуск Discovery-based парсинга: {}", self.config_path.display());

        // Фаза 1: Discovery - обнаружение структуры
        let discovered_files = self.discover_structure()?;
        println!("📁 Обнаружено {} XML файлов", discovered_files.len());

        // Фаза 2: Parsing - парсинг обнаруженных файлов
        let mut resolutions = Vec::new();
        for file_info in discovered_files {
            match self.parse_discovered_xml(&file_info) {
                Ok(metadata) => {
                    println!("   ✅ {}: {} ({})", 
                        self.get_kind_display_name(metadata.kind),
                        metadata.name,
                        metadata.discovery_context.xml_root_element
                    );
                    
                    // Создаем TypeResolution для всех фасетов
                    resolutions.extend(self.create_type_resolutions(&metadata));
                    
                    // Сохраняем в кеш
                    self.discovered_objects.insert(metadata.qualified_name.clone(), metadata);
                }
                Err(e) => {
                    println!("   ❌ Ошибка парсинга {}: {}", file_info.path.display(), e);
                }
            }
        }

        println!("✅ Discovery завершен: {} типов из {} объектов", 
            resolutions.len(), 
            self.discovered_objects.len()
        );

        Ok(resolutions)
    }

    /// Фаза 1: Discovery - обнаружение всех XML файлов метаданных
    fn discover_structure(&self) -> Result<Vec<DiscoveredFile>> {
        let mut discovered = Vec::new();
        
        // Рекурсивно обходим все каталоги начиная с корня конфигурации
        self.discover_recursive(&self.config_path, &mut discovered)?;
        
        Ok(discovered)
    }

    /// Рекурсивное обнаружение XML файлов
    fn discover_recursive(&self, path: &Path, discovered: &mut Vec<DiscoveredFile>) -> Result<()> {
        if !path.exists() || !path.is_dir() {
            return Ok(());
        }

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();

            if entry_path.is_dir() {
                // Рекурсивно заходим в подкаталоги
                self.discover_recursive(&entry_path, discovered)?;
            } else if entry_path.extension().map_or(false, |ext| ext == "xml") {
                // Проверяем, является ли это файлом метаданных
                if let Some(file_info) = self.analyze_xml_file(&entry_path)? {
                    discovered.push(file_info);
                }
            }
        }

        Ok(())
    }

    /// Анализ XML файла для определения типа метаданных
    fn analyze_xml_file(&self, xml_path: &Path) -> Result<Option<DiscoveredFile>> {
        // Читаем начало файла для определения типа
        let content = fs::read_to_string(xml_path)
            .with_context(|| format!("Не удается прочитать файл: {}", xml_path.display()))?;

        let mut reader = Reader::from_str(&content);
        reader.trim_text(true);
        let mut buf = Vec::new();

        // Ищем корневой элемент метаданных
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                    
                    // Определяем тип метаданных по корневому элементу
                    if let Some(kind) = self.detect_metadata_kind(&tag_name) {
                        return Ok(Some(DiscoveredFile {
                            path: xml_path.to_path_buf(),
                            detected_kind: kind,
                            root_element: tag_name,
                            discovery_method: DiscoveryMethod::XmlRootElement,
                        }));
                    }
                    
                    // Если встретили системные элементы - прекращаем анализ
                    if matches!(tag_name.as_str(), "Configuration" | "Language" | "ConfigDumpInfo") {
                        return Ok(None);
                    }
                }
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(_) => break, // Игнорируем ошибки XML парсинга на этапе discovery
            }
            buf.clear();
        }

        Ok(None)
    }

    /// Определение типа метаданных по корневому элементу XML
    fn detect_metadata_kind(&self, root_element: &str) -> Option<MetadataKind> {
        match root_element {
            "Catalog" => Some(MetadataKind::Catalog),
            "Document" => Some(MetadataKind::Document),
            "InformationRegister" => Some(MetadataKind::Register),
            "Enum" => Some(MetadataKind::Enum),
            "Report" => Some(MetadataKind::Report),
            "DataProcessor" => Some(MetadataKind::DataProcessor),
            "ChartOfAccounts" => Some(MetadataKind::ChartOfAccounts),
            "ChartOfCharacteristicTypes" => Some(MetadataKind::ChartOfCharacteristicTypes),
            
            // Исключаем системные файлы конфигурации
            "Configuration" => None,  // Корневой файл конфигурации
            "Language" => None,       // Файлы языков
            "ConfigDumpInfo" => None, // Информация о выгрузке
            _ => None,
        }
    }

    /// Фаза 2: Парсинг обнаруженного XML файла
    fn parse_discovered_xml(&self, file_info: &DiscoveredFile) -> Result<DiscoveredMetadata> {
        let content = fs::read_to_string(&file_info.path)
            .with_context(|| format!("Не удается прочитать файл: {}", file_info.path.display()))?;

        let mut reader = Reader::from_str(&content);
        reader.trim_text(true);

        let mut metadata = DiscoveredMetadata {
            name: String::new(),
            kind: file_info.detected_kind,
            qualified_name: String::new(),
            file_path: file_info.path.clone(),
            synonym: None,
            uuid: None,
            attributes: Vec::new(),
            tabular_sections: Vec::new(),
            discovery_context: DiscoveryContext {
                discovered_from_path: file_info.path.to_string_lossy().to_string(),
                xml_root_element: file_info.root_element.clone(),
                discovery_method: file_info.discovery_method.clone(),
            },
        };

        let mut buf = Vec::new();
        let mut in_properties = false;
        let mut in_child_objects = false;
        let mut current_element = String::new();
        let mut current_attribute: Option<AttributeInfo> = None;
        let mut current_tabular_section: Option<TabularSectionInfo> = None;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                    
                    match tag_name.as_str() {
                        "Properties" => in_properties = true,
                        "ChildObjects" => in_child_objects = true,
                        "Attribute" if in_child_objects => {
                            current_attribute = Some(AttributeInfo {
                                name: String::new(),
                                type_definition: String::new(),
                                synonym: None,
                                mandatory: false,
                            });
                        }
                        "TabularSection" if in_child_objects => {
                            current_tabular_section = Some(TabularSectionInfo {
                                name: String::new(),
                                synonym: None,
                                attributes: Vec::new(),
                            });
                        }
                        tag => {
                            current_element = tag.to_string();
                            
                            // Извлекаем UUID из атрибутов корневого элемента
                            if tag == &file_info.root_element {
                                for attr in e.attributes() {
                                    if let Ok(attr) = attr {
                                        if attr.key.as_ref() == b"uuid" {
                                            if let Ok(uuid_value) = attr.unescape_value() {
                                                metadata.uuid = Some(uuid_value.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(Event::Text(e)) => {
                    let text = e.unescape()?.into_owned();
                    
                    if in_properties && !text.trim().is_empty() {
                        match current_element.as_str() {
                            "Name" => {
                                // Парсим имя объекта только если еще не установлено
                                if metadata.name.is_empty() {
                                    metadata.name = text;
                                    // Формируем qualified_name
                                    metadata.qualified_name = format!("{}.{}", 
                                        self.get_kind_prefix(metadata.kind), 
                                        metadata.name
                                    );
                                }
                            }
                            _ => {}
                        }
                    } else if let Some(ref mut attr) = current_attribute {
                        match current_element.as_str() {
                            "Name" => attr.name = text,
                            _ => {}
                        }
                    } else if let Some(ref mut ts) = current_tabular_section {
                        match current_element.as_str() {
                            "Name" => ts.name = text,
                            _ => {}
                        }
                    }
                }
                Ok(Event::End(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                    
                    match tag_name.as_str() {
                        "Properties" => in_properties = false,
                        "ChildObjects" => in_child_objects = false,
                        "Attribute" if in_child_objects => {
                            if let Some(attr) = current_attribute.take() {
                                if !attr.name.is_empty() {
                                    metadata.attributes.push(attr);
                                }
                            }
                        }
                        "TabularSection" if in_child_objects => {
                            if let Some(ts) = current_tabular_section.take() {
                                if !ts.name.is_empty() {
                                    metadata.tabular_sections.push(ts);
                                }
                            }
                        }
                        _ => {}
                    }
                    current_element.clear();
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    println!("⚠️ XML parsing warning: {} at position {}", e, reader.buffer_position());
                }
                _ => {}
            }
            
            buf.clear();
        }

        // Проверяем что получили минимальные данные
        if metadata.name.is_empty() {
            return Err(anyhow::anyhow!("Не удалось извлечь имя объекта из XML"));
        }

        Ok(metadata)
    }

    /// Создание TypeResolution для всех фасетов объекта
    fn create_type_resolutions(&self, metadata: &DiscoveredMetadata) -> Vec<TypeResolution> {
        let mut resolutions = Vec::new();
        
        // Получаем фасеты для данного типа метаданных
        let facets = self.get_facets_for_kind(metadata.kind);
        
        println!("🎭 Создаем фасеты для {}: {:?}", metadata.name, facets);
        
        // Создаем TypeResolution для каждого фасета
        for facet in facets {
            let config_type = ConfigurationType {
                kind: metadata.kind,
                name: metadata.name.clone(),
                attributes: metadata.attributes.iter().map(|attr| Attribute {
                    name: attr.name.clone(),
                    type_: attr.type_definition.clone(),
                    is_composite: false,
                    types: vec![attr.type_definition.clone()],
                }).collect(),
                tabular_sections: metadata.tabular_sections.iter().map(|ts| TabularSection {
                    name: ts.name.clone(),
                    synonym: ts.synonym.clone(),
                    attributes: ts.attributes.iter().map(|attr| Attribute {
                        name: attr.name.clone(),
                        type_: attr.type_definition.clone(),
                        is_composite: false,
                        types: vec![attr.type_definition.clone()],
                    }).collect(),
                }).collect(),
            };
            
            let resolution = TypeResolution {
                certainty: Certainty::Known,
                result: ResolutionResult::Concrete(ConcreteType::Configuration(config_type)),
                source: ResolutionSource::Static,
                metadata: ResolutionMetadata {
                    file: Some(format!("discovery:{}", metadata.file_path.display())),
                    line: None,
                    column: None,
                    notes: vec![
                        format!("kind:{:?}", metadata.kind),
                        format!("facet:{:?}", facet),
                        format!("discovery_method:{:?}", metadata.discovery_context.discovery_method),
                        format!("xml_root:{}", metadata.discovery_context.xml_root_element),
                        metadata.synonym.as_ref().map(|s| format!("synonym:{}", s)).unwrap_or_default(),
                        metadata.uuid.as_ref().map(|u| format!("uuid:{}", u)).unwrap_or_default(),
                    ].into_iter().filter(|s| !s.is_empty()).collect(),
                },
                active_facet: Some(facet),
                available_facets: vec![facet],
            };
            
            resolutions.push(resolution);
        }
        
        resolutions
    }

    /// Получить фасеты для типа метаданных
    fn get_facets_for_kind(&self, kind: MetadataKind) -> Vec<FacetKind> {
        match kind {
            MetadataKind::Catalog => vec![
                FacetKind::Manager,    // Справочники.Контрагенты
                FacetKind::Object,     // СправочникОбъект.Контрагенты  
                FacetKind::Reference,  // СправочникСсылка.Контрагенты
            ],
            MetadataKind::Document => vec![
                FacetKind::Manager,    // Документы.ЗаказНаряды
                FacetKind::Object,     // ДокументОбъект.ЗаказНаряды
                FacetKind::Reference,  // ДокументСсылка.ЗаказНаряды
            ],
            MetadataKind::Register => vec![
                FacetKind::Manager,    // РегистрыСведений.ТестовыйРегистр
            ],
            MetadataKind::Enum => vec![
                FacetKind::Manager,    // Перечисления.ВидКонтрагента
            ],
            _ => vec![FacetKind::Manager], // Для остальных типов - базовый фасет
        }
    }

    /// Получить отображаемое название типа
    fn get_kind_display_name(&self, kind: MetadataKind) -> &str {
        match kind {
            MetadataKind::Catalog => "Справочник",
            MetadataKind::Document => "Документ",
            MetadataKind::Register => "Регистр сведений",
            MetadataKind::Enum => "Перечисление",
            MetadataKind::Report => "Отчет",
            MetadataKind::DataProcessor => "Обработка",
            MetadataKind::ChartOfAccounts => "План счетов",
            MetadataKind::ChartOfCharacteristicTypes => "План видов характеристик",
        }
    }
    
    /// Получить префикс для типа
    fn get_kind_prefix(&self, kind: MetadataKind) -> &str {
        match kind {
            MetadataKind::Catalog => "Справочники",
            MetadataKind::Document => "Документы",
            MetadataKind::Register => "РегистрыСведений",
            MetadataKind::Enum => "Перечисления",
            MetadataKind::Report => "Отчеты",
            MetadataKind::DataProcessor => "Обработки",
            MetadataKind::ChartOfAccounts => "ПланыСчетов",
            MetadataKind::ChartOfCharacteristicTypes => "ПланыВидовХарактеристик",
        }
    }

    /// Получить обнаруженные метаданные по qualified name
    pub fn get_discovered_metadata(&self, qualified_name: &str) -> Option<&DiscoveredMetadata> {
        self.discovered_objects.get(qualified_name)
    }

    /// Получить все обнаруженные метаданные
    pub fn get_all_discovered(&self) -> &HashMap<String, DiscoveredMetadata> {
        &self.discovered_objects
    }

    /// Статистика discovery
    pub fn get_discovery_stats(&self) -> DiscoveryStats {
        let mut stats = DiscoveryStats::default();
        
        for metadata in self.discovered_objects.values() {
            match metadata.kind {
                MetadataKind::Catalog => stats.catalogs += 1,
                MetadataKind::Document => stats.documents += 1,
                MetadataKind::Register => stats.registers += 1,
                MetadataKind::Enum => stats.enums += 1,
                MetadataKind::Report => stats.reports += 1,
                MetadataKind::DataProcessor => stats.data_processors += 1,
                MetadataKind::ChartOfAccounts => stats.chart_of_accounts += 1,
                MetadataKind::ChartOfCharacteristicTypes => stats.chart_of_characteristic_types += 1,
            }
            
            stats.total_attributes += metadata.attributes.len();
            stats.total_tabular_sections += metadata.tabular_sections.len();
        }
        
        stats.total_objects = self.discovered_objects.len();
        stats
    }
}

/// Информация об обнаруженном файле
#[derive(Debug, Clone)]
struct DiscoveredFile {
    path: PathBuf,
    detected_kind: MetadataKind,
    root_element: String,
    discovery_method: DiscoveryMethod,
}

/// Статистика discovery
#[derive(Debug, Default)]
pub struct DiscoveryStats {
    pub total_objects: usize,
    pub catalogs: usize,
    pub documents: usize,
    pub registers: usize,
    pub enums: usize,
    pub reports: usize,
    pub data_processors: usize,
    pub chart_of_accounts: usize,
    pub chart_of_characteristic_types: usize,
    pub total_attributes: usize,
    pub total_tabular_sections: usize,
}

impl DiscoveryStats {
    /// Печать статистики
    pub fn print(&self) {
        println!("📊 Статистика Discovery:");
        println!("   Всего объектов: {}", self.total_objects);
        println!("   Справочники: {}", self.catalogs);
        println!("   Документы: {}", self.documents);
        println!("   Регистры сведений: {}", self.registers);
        println!("   Перечисления: {}", self.enums);
        println!("   Отчеты: {}", self.reports);
        println!("   Обработки: {}", self.data_processors);
        println!("   Планы счетов: {}", self.chart_of_accounts);
        println!("   Планы видов характеристик: {}", self.chart_of_characteristic_types);
        println!("   Всего атрибутов: {}", self.total_attributes);
        println!("   Всего табличных частей: {}", self.total_tabular_sections);
    }
}