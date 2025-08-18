//! Configuration-guided Discovery парсер для конфигурации 1С:Предприятие
//! 
//! Использует Configuration.xml как опорный файл для получения полного списка
//! объектов метаданных, что гораздо надежнее чем рекурсивный обход каталогов

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

/// Configuration-guided Discovery парсер
#[derive(Debug)]
pub struct ConfigurationGuidedParser {
    config_path: PathBuf,
    discovered_objects: HashMap<String, DiscoveredMetadata>,
    configuration_info: Option<ConfigurationInfo>,
}

/// Информация о конфигурации из Configuration.xml
#[derive(Debug, Clone)]
pub struct ConfigurationInfo {
    pub name: String,
    pub uuid: Option<String>,
    pub version: Option<String>,
    pub metadata_objects: Vec<MetadataReference>,
}

/// Ссылка на объект метаданных из Configuration.xml
#[derive(Debug, Clone)]
pub struct MetadataReference {
    pub name: String,
    pub kind: MetadataKind,
    pub xml_tag: String,
}

/// Обнаруженные метаданные объекта
#[derive(Debug, Clone)]
pub struct DiscoveredMetadata {
    pub name: String,
    pub kind: MetadataKind,
    pub qualified_name: String,
    pub file_path: PathBuf,
    pub reference_source: ReferenceSource,
    pub synonym: Option<String>,
    pub uuid: Option<String>,
    pub attributes: Vec<AttributeInfo>,
    pub tabular_sections: Vec<TabularSectionInfo>,
}

/// Источник обнаружения ссылки
#[derive(Debug, Clone)]
pub enum ReferenceSource {
    /// Из секции ChildObjects в Configuration.xml
    ConfigurationChildObjects,
    /// Дополнительное обнаружение в каталогах
    DirectoryDiscovery,
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

/// Стандартные атрибуты объектов метаданных
#[derive(Debug, Clone, Default)]
struct StandardAttributes {
    // Справочники
    pub code_length: Option<u32>,
    pub code_type: Option<String>,
    pub description_length: Option<u32>,
    pub hierarchical: bool,
    pub owners: Vec<String>,
    
    // Документы  
    pub number_length: Option<u32>,
    pub number_type: Option<String>,
    pub number_periodicity: Option<String>,
    
    // Общие
    pub posting: Option<String>,
}

impl ConfigurationGuidedParser {
    /// Создать новый Configuration-guided парсер
    pub fn new<P: AsRef<Path>>(config_path: P) -> Self {
        Self {
            config_path: config_path.as_ref().to_path_buf(),
            discovered_objects: HashMap::new(),
            configuration_info: None,
        }
    }

    /// Запустить Configuration-guided парсинг
    pub fn parse_with_configuration_guide(&mut self) -> Result<Vec<TypeResolution>> {
        // Фаза 1: Парсинг Configuration.xml как опорного файла
        let config_xml_path = self.config_path.join("Configuration.xml");
        if !config_xml_path.exists() {
            return Err(anyhow::anyhow!("Configuration.xml не найден: {}", config_xml_path.display()));
        }

        let config_info = self.parse_configuration_xml(&config_xml_path)?;

        self.configuration_info = Some(config_info.clone());

        // Фаза 2: Парсинг объектов метаданных по ссылкам из Configuration.xml
        let mut resolutions = Vec::new();
        let mut found_count = 0;
        let mut missing_count = 0;

        for metadata_ref in &config_info.metadata_objects {
            match self.parse_metadata_by_reference(metadata_ref) {
                Ok(Some(metadata)) => {
                    resolutions.extend(self.create_type_resolutions(&metadata));
                    self.discovered_objects.insert(metadata.qualified_name.clone(), metadata);
                    found_count += 1;
                }
                Ok(None) => {
                    missing_count += 1;
                }
                Err(_) => {
                    missing_count += 1;
                }
            }
        }

        Ok(resolutions)
    }

    /// Парсинг Configuration.xml для получения списка объектов метаданных
    fn parse_configuration_xml(&self, config_xml_path: &Path) -> Result<ConfigurationInfo> {
        let content = fs::read_to_string(config_xml_path)
            .with_context(|| format!("Не удается прочитать Configuration.xml: {}", config_xml_path.display()))?;

        let mut reader = Reader::from_str(&content);
        reader.trim_text(true);

        let mut config_info = ConfigurationInfo {
            name: String::new(),
            uuid: None,
            version: None,
            metadata_objects: Vec::new(),
        };

        let mut buf = Vec::new();
        let mut in_properties = false;
        let mut in_child_objects = false;
        let mut current_element = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                    
                    match tag_name.as_str() {
                        "Configuration" => {
                            // Извлекаем UUID из атрибутов
                            for attr in e.attributes() {
                                if let Ok(attr) = attr {
                                    if attr.key.as_ref() == b"uuid" {
                                        if let Ok(uuid_value) = attr.unescape_value() {
                                            config_info.uuid = Some(uuid_value.to_string());
                                        }
                                    }
                                }
                            }
                        }
                        "Properties" => in_properties = true,
                        "ChildObjects" => in_child_objects = true,
                        tag => {
                            current_element = tag.to_string();
                            
                            // Обрабатываем объекты метаданных в ChildObjects
                            if in_child_objects {
                                if let Some(kind) = self.xml_tag_to_metadata_kind(&tag) {
                                    // Пока не знаем имя, создадим заготовку
                                    // Имя получим в Event::Text
                                }
                            }
                        }
                    }
                }
                Ok(Event::Text(e)) => {
                    let text = e.unescape()?.into_owned().trim().to_string();
                    
                    if text.is_empty() {
                        continue;
                    }
                    
                    if in_properties {
                        match current_element.as_str() {
                            "Name" => config_info.name = text,
                            _ => {}
                        }
                    } else if in_child_objects {
                        // Это имя объекта метаданных
                        if let Some(kind) = self.xml_tag_to_metadata_kind(&current_element) {
                            config_info.metadata_objects.push(MetadataReference {
                                name: text,
                                kind,
                                xml_tag: current_element.clone(),
                            });
                        }
                    }
                }
                Ok(Event::End(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                    
                    match tag_name.as_str() {
                        "Properties" => in_properties = false,
                        "ChildObjects" => in_child_objects = false,
                        _ => {}
                    }
                    current_element.clear();
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    println!("⚠️ XML parsing warning in Configuration.xml: {} at position {}", e, reader.buffer_position());
                }
                _ => {}
            }
            
            buf.clear();
        }

        if config_info.name.is_empty() {
            return Err(anyhow::anyhow!("Не удалось извлечь имя конфигурации из Configuration.xml"));
        }

        Ok(config_info)
    }

    /// Преобразование XML тега в тип метаданных (динамическое определение)
    fn xml_tag_to_metadata_kind(&self, xml_tag: &str) -> Option<MetadataKind> {
        match xml_tag {
            // Основные типы метаданных
            "Catalog" => Some(MetadataKind::Catalog),
            "Document" => Some(MetadataKind::Document),
            "InformationRegister" => Some(MetadataKind::Register),
            "AccumulationRegister" => Some(MetadataKind::Register),
            "AccountingRegister" => Some(MetadataKind::Register),
            "CalculationRegister" => Some(MetadataKind::Register),
            "Enum" => Some(MetadataKind::Enum),
            "Report" => Some(MetadataKind::Report),
            "DataProcessor" => Some(MetadataKind::DataProcessor),
            "ChartOfAccounts" => Some(MetadataKind::ChartOfAccounts),
            "ChartOfCharacteristicTypes" => Some(MetadataKind::ChartOfCharacteristicTypes),
            "ChartOfCalculationTypes" => Some(MetadataKind::ChartOfCharacteristicTypes),
            
            // Дополнительные типы, которые могут встретиться
            "BusinessProcess" => Some(MetadataKind::DataProcessor), // Бизнес-процессы как обработки
            "Task" => Some(MetadataKind::DataProcessor), // Задачи как обработки
            "FilterCriterion" => Some(MetadataKind::DataProcessor),
            "SettingsStorage" => Some(MetadataKind::DataProcessor),
            "ExchangePlan" => Some(MetadataKind::DataProcessor),
            
            // Системные элементы исключаем
            "Language" | "Configuration" | "ConfigDumpInfo" => None,
            
            // Все неизвестные теги считаем обработками (безопасная стратегия)
            _ => {
                println!("⚠️ Неизвестный тип метаданных: {}, считаем обработкой", xml_tag);
                Some(MetadataKind::DataProcessor)
            }
        }
    }

    /// Парсинг объекта метаданных по ссылке из Configuration.xml
    pub fn parse_metadata_by_reference(&self, metadata_ref: &MetadataReference) -> Result<Option<DiscoveredMetadata>> {
        // Ищем XML файл объекта динамически по всей структуре каталогов
        let xml_file_path = self.find_metadata_file_dynamically(metadata_ref)?;
        
        if let Some(file_path) = xml_file_path {
            return self.parse_metadata_from_file(&file_path, metadata_ref);
        }
        
        Ok(None)
    }

    /// Динамический поиск файла метаданных без хардкода путей
    fn find_metadata_file_dynamically(&self, metadata_ref: &MetadataReference) -> Result<Option<PathBuf>> {
        // Ищем файл с именем {metadata_ref.name}.xml рекурсивно по всем каталогам
        self.find_xml_file_recursive(&self.config_path, &metadata_ref.name)
    }

    /// Рекурсивный поиск XML файла по имени
    fn find_xml_file_recursive(&self, dir: &Path, target_name: &str) -> Result<Option<PathBuf>> {
        if !dir.is_dir() {
            return Ok(None);
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(file_name) = path.file_stem() {
                    if file_name == target_name && path.extension().map_or(false, |ext| ext == "xml") {
                        return Ok(Some(path));
                    }
                }
            } else if path.is_dir() {
                // Рекурсивно заходим в подкаталоги
                if let Some(found) = self.find_xml_file_recursive(&path, target_name)? {
                    return Ok(Some(found));
                }
            }
        }

        Ok(None)
    }

    /// Парсинг метаданных из найденного файла
    fn parse_metadata_from_file(&self, xml_file_path: &Path, metadata_ref: &MetadataReference) -> Result<Option<DiscoveredMetadata>> {

        // Парсим XML файл объекта
        let content = fs::read_to_string(&xml_file_path)
            .with_context(|| format!("Не удается прочитать файл: {}", xml_file_path.display()))?;

        let mut reader = Reader::from_str(&content);
        reader.trim_text(true);

        let mut metadata = DiscoveredMetadata {
            name: metadata_ref.name.clone(), // Имя уже известно из Configuration.xml
            kind: metadata_ref.kind,
            qualified_name: format!("{}.{}", self.get_kind_prefix(metadata_ref.kind), metadata_ref.name),
            file_path: xml_file_path.to_path_buf(),
            reference_source: ReferenceSource::ConfigurationChildObjects,
            synonym: None,
            uuid: None,
            attributes: Vec::new(),
            tabular_sections: Vec::new(),
        };

        let mut buf = Vec::new();
        let mut in_properties = false;
        let mut in_child_objects = false;
        let mut in_attribute_properties = false;
        let mut current_element = String::new();
        let mut current_attribute: Option<AttributeInfo> = None;
        let mut current_tabular_section: Option<TabularSectionInfo> = None;
        
        // Для стандартных атрибутов
        let mut standard_attributes = StandardAttributes::default();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                    
                    match tag_name.as_str() {
                        "Properties" if !in_child_objects => {
                            in_properties = true;
                        }
                        "Properties" if current_attribute.is_some() => {
                            in_attribute_properties = true;
                        }
                        "ChildObjects" => {
                            in_child_objects = true;
                        }
                        "Attribute" | "Resource" | "Dimension" if in_child_objects => {
                            current_attribute = Some(AttributeInfo {
                                name: String::new(),
                                type_definition: "xs:string".to_string(),
                                synonym: None,
                                mandatory: tag_name == "Dimension",
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
                            
                            // Извлекаем UUID из корневого элемента
                            if tag == &metadata_ref.xml_tag {
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
                    
                    if !text.trim().is_empty() {
                        if in_properties {
                            // Парсинг стандартных атрибутов из Properties
                            match current_element.as_str() {
                                "CodeLength" => {
                                    if let Ok(length) = text.parse::<u32>() {
                                        standard_attributes.code_length = Some(length);
                                    }
                                }
                                "CodeType" => standard_attributes.code_type = Some(text),
                                "DescriptionLength" => {
                                    if let Ok(length) = text.parse::<u32>() {
                                        standard_attributes.description_length = Some(length);
                                    }
                                }
                                "Hierarchical" => {
                                    standard_attributes.hierarchical = text.to_lowercase() == "true";
                                }
                                "NumberLength" => {
                                    if let Ok(length) = text.parse::<u32>() {
                                        standard_attributes.number_length = Some(length);
                                    }
                                }
                                "NumberType" => standard_attributes.number_type = Some(text),
                                "NumberPeriodicity" => standard_attributes.number_periodicity = Some(text),
                                "Posting" => standard_attributes.posting = Some(text),
                                _ => {}
                            }
                        } else if let Some(ref mut attr) = current_attribute {
                            if in_attribute_properties {
                                match current_element.as_str() {
                                    "Name" => {
                                        attr.name = text;
                                    }
                                    _ => {}
                                }
                            }
                        } else if let Some(ref mut ts) = current_tabular_section {
                            match current_element.as_str() {
                                "Name" => ts.name = text,
                                _ => {}
                            }
                        }
                    }
                }
                Ok(Event::End(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                    
                    match tag_name.as_str() {
                        "Properties" if !current_attribute.is_some() => {
                            in_properties = false;
                        }
                        "Properties" if current_attribute.is_some() => {
                            in_attribute_properties = false;
                        }
                        "ChildObjects" => {
                            in_child_objects = false;
                        }
                        "Attribute" | "Resource" | "Dimension" if in_child_objects => {
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

        // Добавляем стандартные атрибуты на основе типа метаданных
        self.add_standard_attributes(&mut metadata, &standard_attributes);

        Ok(Some(metadata))
    }

    /// Добавление стандартных атрибутов объектов метаданных
    fn add_standard_attributes(&self, metadata: &mut DiscoveredMetadata, std_attrs: &StandardAttributes) {
        match metadata.kind {
            MetadataKind::Catalog => {
                // Стандартные атрибуты справочников
                
                // Код
                if let (Some(length), code_type) = (std_attrs.code_length, &std_attrs.code_type) {
                    let code_type_str = match code_type.as_deref() {
                        Some("String") => format!("Строка({})", length),
                        Some("Number") => format!("Число({}, 0)", length),
                        _ => format!("Строка({})", length), // По умолчанию строка
                    };
                    
                    metadata.attributes.push(AttributeInfo {
                        name: "Код".to_string(),
                        type_definition: code_type_str,
                        synonym: Some("Code".to_string()),
                        mandatory: false,
                    });
                }
                
                // Наименование
                if let Some(length) = std_attrs.description_length {
                    metadata.attributes.push(AttributeInfo {
                        name: "Наименование".to_string(),
                        type_definition: format!("Строка({})", length),
                        synonym: Some("Description".to_string()),
                        mandatory: true,
                    });
                }
                
                // Родитель (если иерархический)
                if std_attrs.hierarchical {
                    let parent_type = format!("СправочникСсылка.{}", metadata.name);
                    metadata.attributes.push(AttributeInfo {
                        name: "Родитель".to_string(),
                        type_definition: parent_type,
                        synonym: Some("Parent".to_string()),
                        mandatory: false,
                    });
                }
                
                // Владелец (если есть владельцы)
                if !std_attrs.owners.is_empty() {
                    let owner_types: Vec<String> = std_attrs.owners.iter()
                        .map(|owner| format!("СправочникСсылка.{}", owner))
                        .collect();
                    
                    metadata.attributes.push(AttributeInfo {
                        name: "Владелец".to_string(),
                        type_definition: if owner_types.len() == 1 {
                            owner_types[0].clone()
                        } else {
                            format!("Составной({})", owner_types.join(", "))
                        },
                        synonym: Some("Owner".to_string()),
                        mandatory: true,
                    });
                }
            }
            
            MetadataKind::Document => {
                // Стандартные атрибуты документов
                
                // Номер
                if let (Some(length), number_type) = (std_attrs.number_length, &std_attrs.number_type) {
                    let number_type_str = match number_type.as_deref() {
                        Some("String") => format!("Строка({})", length),
                        Some("Number") => format!("Число({}, 0)", length),
                        _ => format!("Строка({})", length),
                    };
                    
                    metadata.attributes.push(AttributeInfo {
                        name: "Номер".to_string(),
                        type_definition: number_type_str,
                        synonym: Some("Number".to_string()),
                        mandatory: false,
                    });
                }
                
                // Дата
                metadata.attributes.push(AttributeInfo {
                    name: "Дата".to_string(),
                    type_definition: "Дата".to_string(),
                    synonym: Some("Date".to_string()),
                    mandatory: true,
                });
                
                // Проведен (если документ проводимый)
                if std_attrs.posting.is_some() {
                    metadata.attributes.push(AttributeInfo {
                        name: "Проведен".to_string(),
                        type_definition: "Булево".to_string(),
                        synonym: Some("Posted".to_string()),
                        mandatory: false,
                    });
                }
            }
            
            MetadataKind::Register => {
                // Стандартные атрибуты регистров
                
                // Период (для регистров сведений)
                metadata.attributes.push(AttributeInfo {
                    name: "Период".to_string(),
                    type_definition: "ДатаВремя".to_string(),
                    synonym: Some("Period".to_string()),
                    mandatory: true,
                });
                
                // Активность (для регистров сведений)
                metadata.attributes.push(AttributeInfo {
                    name: "Активность".to_string(),
                    type_definition: "Булево".to_string(),
                    synonym: Some("Active".to_string()),
                    mandatory: false,
                });
            }
            
            _ => {
                // Для других типов метаданных стандартные атрибуты пока не добавляем
            }
        }
    }


    /// Создание TypeResolution для всех фасетов объекта
    fn create_type_resolutions(&self, metadata: &DiscoveredMetadata) -> Vec<TypeResolution> {
        let mut resolutions = Vec::new();
        
        // Получаем фасеты для данного типа метаданных
        let facets = self.get_facets_for_kind(metadata.kind);
        
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
                    file: Some(format!("guided:{}", metadata.file_path.display())),
                    line: None,
                    column: None,
                    notes: vec![
                        format!("kind:{:?}", metadata.kind),
                        format!("facet:{:?}", facet),
                        format!("source:{:?}", metadata.reference_source),
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

    /// Получить информацию о конфигурации
    pub fn get_configuration_info(&self) -> Option<&ConfigurationInfo> {
        self.configuration_info.as_ref()
    }

    /// Получить обнаруженные метаданные по qualified name
    pub fn get_discovered_metadata(&self, qualified_name: &str) -> Option<&DiscoveredMetadata> {
        self.discovered_objects.get(qualified_name)
    }

    /// Получить все обнаруженные метаданные
    pub fn get_all_discovered(&self) -> &HashMap<String, DiscoveredMetadata> {
        &self.discovered_objects
    }

    /// Статистика guided discovery
    pub fn get_guided_discovery_stats(&self) -> GuidedDiscoveryStats {
        let mut stats = GuidedDiscoveryStats::default();
        
        if let Some(config_info) = &self.configuration_info {
            stats.configuration_name = config_info.name.clone();
            stats.total_references = config_info.metadata_objects.len();
        }
        
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
        
        stats.found_objects = self.discovered_objects.len();
        stats.missing_objects = stats.total_references - stats.found_objects;
        stats
    }
}

/// Статистика Configuration-guided discovery
#[derive(Debug, Default)]
pub struct GuidedDiscoveryStats {
    pub configuration_name: String,
    pub total_references: usize,
    pub found_objects: usize,
    pub missing_objects: usize,
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

impl GuidedDiscoveryStats {
    /// Печать статистики
    pub fn print(&self) {
        println!("📊 Статистика Configuration-guided Discovery:");
        println!("   Конфигурация: {}", self.configuration_name);
        println!("   Ссылок в Configuration.xml: {}", self.total_references);
        println!("   Найдено объектов: {}", self.found_objects);
        println!("   Пропущено объектов: {}", self.missing_objects);
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