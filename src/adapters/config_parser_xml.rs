//! Улучшенный парсер конфигурации используя roxmltree

use anyhow::Result;
use roxmltree::{Document, Node};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::types::{
    Attribute, Certainty, ConcreteType, ResolutionMetadata, ResolutionResult,
    ResolutionSource, TypeResolution, TabularSection,
};

/// Metadata object info
#[derive(Debug, Clone)]
pub struct MetadataObject {
    pub name: String,
    pub kind: MetadataKind,
    pub synonym: Option<String>,
    pub attributes: Vec<Attribute>,
    pub tabular_sections: Vec<TabularSection>,
}

/// Metadata object kind
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MetadataKind {
    Catalog,
    Document,
    InformationRegister,
    Enum,
}

impl MetadataKind {
    pub fn to_prefix(&self) -> &str {
        match self {
            MetadataKind::Catalog => "Справочники",
            MetadataKind::Document => "Документы",
            MetadataKind::InformationRegister => "РегистрыСведений",
            MetadataKind::Enum => "Перечисления",
        }
    }
}

/// Configuration XML parser using roxmltree
#[derive(Debug)]
pub struct ConfigParserXml {
    config_path: PathBuf,
    metadata_cache: HashMap<String, MetadataObject>,
}

impl ConfigParserXml {
    pub fn new(config_path: impl AsRef<Path>) -> Self {
        Self {
            config_path: config_path.as_ref().to_path_buf(),
            metadata_cache: HashMap::new(),
        }
    }

    /// Parse configuration and return type resolutions
    pub fn parse_configuration(&mut self) -> Result<Vec<TypeResolution>> {
        let mut resolutions = Vec::new();
        
        // Parse different metadata types
        resolutions.extend(self.parse_metadata_objects("Catalogs", MetadataKind::Catalog)?);
        resolutions.extend(self.parse_metadata_objects("Documents", MetadataKind::Document)?);
        
        Ok(resolutions)
    }

    fn parse_metadata_objects(
        &mut self,
        folder: &str,
        kind: MetadataKind,
    ) -> Result<Vec<TypeResolution>> {
        let mut resolutions = Vec::new();
        let objects_path = self.config_path.join(folder);

        if !objects_path.exists() {
            return Ok(resolutions);
        }

        for entry in fs::read_dir(&objects_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().is_some_and(|ext| ext == "xml") {
                let object = self.parse_metadata_xml(&path, &kind)?;
                let qualified_name = format!("{}.{}", kind.to_prefix(), &object.name);
                self.metadata_cache.insert(qualified_name.clone(), object.clone());
                
                resolutions.push(self.create_resolution(object, &kind));
            }
        }

        Ok(resolutions)
    }

    pub fn parse_metadata_xml(&self, path: &Path, kind: &MetadataKind) -> Result<MetadataObject> {
        let content = fs::read_to_string(path)?;
        let doc = Document::parse(&content)?;
        let root = doc.root_element();
        
        let mut object = MetadataObject {
            name: String::new(),
            kind: kind.clone(),
            synonym: None,
            attributes: Vec::new(),
            tabular_sections: Vec::new(),
        };
        
        // Документ или справочник - это первый дочерний элемент root
        let metadata_element = root.children()
            .find(|n| n.is_element())
            .expect("No metadata element found");
        
        // Найдём элемент Properties внутри Document/Catalog
        if let Some(props) = metadata_element.children()
            .find(|n| n.has_tag_name("Properties")) {
            
            // Извлечём имя объекта
            if let Some(name_node) = props.children()
                .find(|n| n.has_tag_name("Name")) {
                object.name = name_node.text().unwrap_or("").to_string();
            }
            
            // Извлечём синоним
            if let Some(synonym_node) = props.children()
                .find(|n| n.has_tag_name("Synonym")) {
                if let Some(item) = synonym_node.children()
                    .find(|n| n.has_tag_name("v8:item")) {
                    if let Some(content) = item.children()
                        .find(|n| n.has_tag_name("v8:content")) {
                        object.synonym = Some(content.text().unwrap_or("").to_string());
                    }
                }
            }
        }
        
        // Найдём атрибуты и табличные части в ChildObjects
        if let Some(child_objects) = metadata_element.children()
            .find(|n| n.has_tag_name("ChildObjects")) {
            
            // Парсим каждый атрибут
            for attr_node in child_objects.children()
                .filter(|n| n.has_tag_name("Attribute")) {
                
                if let Some(attr) = self.parse_attribute_node(&attr_node) {
                    object.attributes.push(attr);
                }
            }
            
            // Парсим табличные части
            for ts_node in child_objects.children()
                .filter(|n| n.has_tag_name("TabularSection")) {
                
                if let Some(ts) = self.parse_tabular_section(&ts_node) {
                    object.tabular_sections.push(ts);
                }
            }
        }
        
        Ok(object)
    }
    
    fn parse_attribute_node(&self, attr_node: &Node) -> Option<Attribute> {
        let mut name = String::new();
        let mut type_names = Vec::new();
        
        // Найдём Properties внутри атрибута
        if let Some(props) = attr_node.children()
            .find(|n| n.has_tag_name("Properties")) {
            
            // Имя атрибута
            if let Some(name_node) = props.children()
                .find(|n| n.has_tag_name("Name")) {
                // Получаем весь текст внутри элемента Name
                let text_content: String = name_node.children()
                    .filter_map(|n| n.text())
                    .collect();
                if !text_content.is_empty() {
                    name = text_content.trim().to_string();
                }
            }
            
            // Типы атрибута
            if let Some(type_node) = props.children()
                .find(|n| n.has_tag_name("Type")) {
                
                // Собираем все элементы Type внутри Type (v8:Type в пространстве имён v8)
                for child in type_node.children() {
                    if child.is_element() && child.tag_name().name() == "Type" {
                        // Получаем текст из узла
                        let text_content: String = child.children()
                            .filter_map(|n| n.text())
                            .collect();
                        
                        if !text_content.is_empty() {
                            type_names.push(self.normalize_type_name(text_content.trim()));
                        }
                    }
                }
            }
        }
        
        if name.is_empty() {
            return None;
        }
        
        let is_composite = type_names.len() > 1;
        let type_str = if type_names.is_empty() {
            "Произвольный".to_string()
        } else {
            type_names.join(", ")
        };
        
        Some(Attribute {
            name,
            type_: type_str,
            is_composite,
            types: type_names,
        })
    }
    
    fn parse_tabular_section(&self, ts_node: &Node) -> Option<TabularSection> {
        let mut name = String::new();
        let mut synonym = None;
        let mut attributes = Vec::new();
        
        // Найдём Properties внутри TabularSection
        if let Some(props) = ts_node.children()
            .find(|n| n.has_tag_name("Properties")) {
            
            // Имя табличной части
            if let Some(name_node) = props.children()
                .find(|n| n.has_tag_name("Name")) {
                name = name_node.text().unwrap_or("").to_string();
            }
            
            // Синоним табличной части
            if let Some(synonym_node) = props.children()
                .find(|n| n.has_tag_name("Synonym")) {
                if let Some(item) = synonym_node.children()
                    .find(|n| n.has_tag_name("v8:item")) {
                    if let Some(content) = item.children()
                        .find(|n| n.has_tag_name("v8:content")) {
                        synonym = Some(content.text().unwrap_or("").to_string());
                    }
                }
            }
        }
        
        // Найдём атрибуты табличной части в ChildObjects
        if let Some(child_objects) = ts_node.children()
            .find(|n| n.has_tag_name("ChildObjects")) {
            
            // Парсим каждый атрибут табличной части
            for attr_node in child_objects.children()
                .filter(|n| n.has_tag_name("Attribute")) {
                
                if let Some(attr) = self.parse_attribute_node(&attr_node) {
                    attributes.push(attr);
                }
            }
        }
        
        if name.is_empty() {
            return None;
        }
        
        Some(TabularSection {
            name,
            synonym,
            attributes,
        })
    }
    
    /// Нормализует имя типа из XML
    fn normalize_type_name(&self, type_name: &str) -> String {
        match type_name {
            s if s.starts_with("cfg:CatalogRef.") => {
                format!("СправочникСсылка.{}", &s[15..])
            }
            s if s.starts_with("cfg:DocumentRef.") => {
                format!("ДокументСсылка.{}", &s[16..])
            }
            s if s.starts_with("cfg:EnumRef.") => {
                format!("ПеречислениеСсылка.{}", &s[12..])
            }
            s if s.starts_with("cfg:InformationRegisterRecordKey.") => {
                format!("КлючЗаписиРегистраСведений.{}", &s[34..])
            }
            "xs:string" => "Строка".to_string(),
            "xs:decimal" => "Число".to_string(),
            "xs:boolean" => "Булево".to_string(),
            "xs:dateTime" => "Дата".to_string(),
            other => other.to_string(),
        }
    }

    fn create_resolution(&self, object: MetadataObject, kind: &MetadataKind) -> TypeResolution {
        let qualified_name = format!("{}.{}", kind.to_prefix(), &object.name);

        // Map our MetadataKind to core MetadataKind
        let core_kind = match kind {
            MetadataKind::Catalog => crate::core::types::MetadataKind::Catalog,
            MetadataKind::Document => crate::core::types::MetadataKind::Document,
            MetadataKind::InformationRegister => crate::core::types::MetadataKind::Register,
            MetadataKind::Enum => crate::core::types::MetadataKind::Enum,
        };

        // Create configuration type
        let config_type = crate::core::types::ConfigurationType {
            kind: core_kind,
            name: object.name.clone(),
            attributes: object.attributes.clone(),
            tabular_sections: object.tabular_sections.clone(),
        };

        TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Configuration(config_type)),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata {
                file: Some(qualified_name),
                line: None,
                column: None,
                notes: object.synonym.map(|s| vec![s]).unwrap_or_default(),
            },
            active_facet: Some(crate::core::types::FacetKind::Manager),
            available_facets: self.get_facets_for_kind(&kind),
        }
    }
    
    fn get_facets_for_kind(&self, kind: &MetadataKind) -> Vec<crate::core::types::FacetKind> {
        use crate::core::types::FacetKind;
        
        match kind {
            MetadataKind::Catalog => vec![
                FacetKind::Manager,
                FacetKind::Object,
                FacetKind::Reference,
                FacetKind::Constructor,
            ],
            MetadataKind::Document => vec![
                FacetKind::Manager,
                FacetKind::Object,
                FacetKind::Reference,
                FacetKind::Constructor,
            ],
            MetadataKind::InformationRegister => vec![
                FacetKind::Manager,
                FacetKind::Object,    // НаборЗаписей
                FacetKind::Reference, // МенеджерЗаписи
            ],
            MetadataKind::Enum => vec![
                FacetKind::Manager,
                FacetKind::Reference,
            ],
        }
    }
    
    /// Get metadata object by qualified name (e.g., "Справочник.Контрагенты")
    pub fn get_metadata(&self, qualified_name: &str) -> Option<&MetadataObject> {
        self.metadata_cache.get(qualified_name)
    }
    
    /// Get catalog metadata
    pub fn get_catalog(&self, name: &str) -> Option<&MetadataObject> {
        let qualified_name = format!("Справочник.{}", name);
        self.get_metadata(&qualified_name)
    }
    
    /// Get document metadata
    pub fn get_document(&self, name: &str) -> Option<&MetadataObject> {
        let qualified_name = format!("Документ.{}", name);
        self.get_metadata(&qualified_name)
    }
    
    /// Get register metadata
    pub fn get_register(&self, reg_type: &str, name: &str) -> Option<&MetadataObject> {
        let qualified_name = format!("{}.{}", reg_type, name);
        self.get_metadata(&qualified_name)
    }
    
    /// Get all metadata objects
    pub fn get_all_metadata(&self) -> Vec<&MetadataObject> {
        self.metadata_cache.values().collect()
    }
    
    /// Load all metadata types from configuration
    pub fn load_all_types(&mut self) -> Result<Vec<TypeResolution>> {
        let mut all_resolutions = Vec::new();
        
        // Load catalogs
        if let Ok(resolutions) = self.parse_metadata_objects("Catalogs", MetadataKind::Catalog) {
            all_resolutions.extend(resolutions);
        }
        
        // Load documents
        if let Ok(resolutions) = self.parse_metadata_objects("Documents", MetadataKind::Document) {
            all_resolutions.extend(resolutions);
        }
        
        // Load registers
        if let Ok(resolutions) = self.parse_metadata_objects("InformationRegisters", MetadataKind::InformationRegister) {
            all_resolutions.extend(resolutions);
        }
        
        // Load enums
        if let Ok(resolutions) = self.parse_metadata_objects("Enums", MetadataKind::Enum) {
            all_resolutions.extend(resolutions);
        }
        
        Ok(all_resolutions)
    }
}