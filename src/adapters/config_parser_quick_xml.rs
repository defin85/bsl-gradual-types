//! Улучшенный парсер конфигурации на основе quick-xml
//! Портирован из bsl_type_safety_analyzer/src/unified_index/xml_parser.rs

use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

use crate::core::types::{
    TypeResolution, Certainty, ResolutionResult, ConcreteType, ConfigurationType,
    MetadataKind, Attribute, TabularSection, ResolutionSource, ResolutionMetadata
};

/// Улучшенный парсер конфигурации с поддержкой namespace
pub struct ConfigurationQuickXmlParser {
    config_path: PathBuf,
    metadata_cache: HashMap<String, ConfigurationMetadata>,
}

/// Метаданные объекта конфигурации
#[derive(Debug, Clone)]
pub struct ConfigurationMetadata {
    pub name: String,
    pub kind: MetadataKind,
    pub synonym: Option<String>,
    pub attributes: Vec<AttributeInfo>,
    pub tabular_sections: Vec<TabularSectionInfo>,
    pub uuid: Option<String>,
    pub generated_types: Vec<GeneratedTypeInfo>,
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

/// Информация о генерируемом типе (Object, Ref, Manager, etc.)
#[derive(Debug, Clone)]
pub struct GeneratedTypeInfo {
    pub name: String,
    pub category: String,
    pub type_id: Option<String>,
}

impl ConfigurationQuickXmlParser {
    /// Создать новый парсер
    pub fn new<P: AsRef<Path>>(config_path: P) -> Self {
        Self {
            config_path: config_path.as_ref().to_path_buf(),
            metadata_cache: HashMap::new(),
        }
    }
    
    /// Парсинг всей конфигурации
    pub fn parse_configuration(&mut self) -> Result<Vec<TypeResolution>> {
        let mut resolutions = Vec::new();
        
        println!("📁 Парсинг конфигурации: {}", self.config_path.display());
        
        // Парсим справочники
        resolutions.extend(self.parse_metadata_objects("Catalogs", MetadataKind::Catalog)?);
        
        // Парсим документы
        resolutions.extend(self.parse_metadata_objects("Documents", MetadataKind::Document)?);
        
        // Парсим регистры сведений
        resolutions.extend(self.parse_metadata_objects("InformationRegisters", MetadataKind::Register)?);
        
        // Парсим перечисления
        resolutions.extend(self.parse_metadata_objects("Enums", MetadataKind::Enum)?);
        
        println!("✅ Парсинг завершен: {} типов", resolutions.len());
        
        Ok(resolutions)
    }
    
    /// Парсинг объектов определенного типа
    fn parse_metadata_objects(&mut self, folder: &str, kind: MetadataKind) -> Result<Vec<TypeResolution>> {
        let mut resolutions = Vec::new();
        let objects_path = self.config_path.join(folder);
        
        if !objects_path.exists() {
            println!("⚠️ Папка {} не найдена", folder);
            return Ok(resolutions);
        }
        
        println!("📂 Обработка {}", folder);
        
        for entry in fs::read_dir(&objects_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().map_or(false, |ext| ext == "xml") {
                match self.parse_metadata_xml(&path, kind) {
                    Ok(metadata) => {
                        println!("   ✅ {}: {} (атрибутов: {}, табл.частей: {})", 
                            self.get_kind_display_name(kind), 
                            metadata.name,
                            metadata.attributes.len(),
                            metadata.tabular_sections.len()
                        );
                        
                        // Создаем TypeResolution для каждого фасета
                        resolutions.extend(self.create_type_resolutions(&metadata));
                        
                        // Сохраняем в кеш
                        let qualified_name = format!("{}.{}", self.get_kind_prefix(kind), metadata.name);
                        self.metadata_cache.insert(qualified_name, metadata);
                    }
                    Err(e) => {
                        println!("   ❌ Ошибка парсинга {}: {}", path.display(), e);
                    }
                }
            }
        }
        
        Ok(resolutions)
    }
    
    /// Парсинг XML файла метаданных
    pub fn parse_metadata_xml(&self, xml_path: &Path, kind: MetadataKind) -> Result<ConfigurationMetadata> {
        let content = fs::read_to_string(xml_path)
            .with_context(|| format!("Не удается прочитать файл: {}", xml_path.display()))?;
        
        let mut reader = Reader::from_str(&content);
        reader.trim_text(true);
        
        let mut metadata = ConfigurationMetadata {
            name: String::new(),
            kind,
            synonym: None,
            attributes: Vec::new(),
            tabular_sections: Vec::new(),
            uuid: None,
            generated_types: Vec::new(),
        };
        
        let mut buf = Vec::new();
        let mut in_properties = false;
        let mut in_child_objects = false;
        let mut in_internal_info = false;
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
                        "InternalInfo" => in_internal_info = true,
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
                            
                            // Извлекаем UUID из атрибутов
                            if tag == "Catalog" || tag == "Document" || tag == "InformationRegister" || tag == "Enum" {
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
                            "Name" => metadata.name = text,
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
                        "InternalInfo" => in_internal_info = false,
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
                    // Логируем ошибку, но продолжаем парсинг
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
    
    /// Создать TypeResolution для всех фасетов объекта
    fn create_type_resolutions(&self, metadata: &ConfigurationMetadata) -> Vec<TypeResolution> {
        use crate::core::types::{FacetKind};
        
        let mut resolutions = Vec::new();
        
        // Основные фасеты для каждого типа объекта
        let facets = match metadata.kind {
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
        };
        
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
                    file: Some(format!("config:{}.xml", metadata.name)),
                    line: None,
                    column: None,
                    notes: vec![
                        format!("kind:{:?}", metadata.kind),
                        format!("facet:{:?}", facet),
                        metadata.synonym.as_ref().map(|s| format!("synonym:{}", s)).unwrap_or_default(),
                    ].into_iter().filter(|s| !s.is_empty()).collect(),
                },
                active_facet: Some(facet),
                available_facets: vec![facet],
            };
            
            resolutions.push(resolution);
        }
        
        resolutions
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
    
    /// Получить кешированные метаданные
    pub fn get_metadata(&self, qualified_name: &str) -> Option<&ConfigurationMetadata> {
        self.metadata_cache.get(qualified_name)
    }
    
    /// Получить все метаданные
    pub fn get_all_metadata(&self) -> &HashMap<String, ConfigurationMetadata> {
        &self.metadata_cache
    }
}