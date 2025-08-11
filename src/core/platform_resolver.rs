//! Platform-aware type resolver

use std::collections::HashMap;
use crate::adapters::platform_types;
use crate::adapters::config_parser_xml::ConfigParserXml;
use super::types::{TypeResolution, Certainty, ResolutionResult, ConcreteType, ResolutionMetadata, ResolutionSource, FacetKind};

/// Completion item with metadata
#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,
    pub documentation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CompletionKind {
    Global,
    Catalog,
    Document,
    Enum,
    Method,
    Property,
}

/// Resolver that knows about platform types and configuration
#[derive(Debug)]
pub struct PlatformTypeResolver {
    /// Platform global types (hardcoded for now)
    platform_globals: HashMap<String, TypeResolution>,
    
    /// Configuration types from XML parser
    config_parser: Option<ConfigParserXml>,
    
    /// Cached resolutions
    cache: HashMap<String, TypeResolution>,
}

impl PlatformTypeResolver {
    pub fn new() -> Self {
        Self {
            platform_globals: platform_types::get_platform_globals(),
            config_parser: None,
            cache: HashMap::new(),
        }
    }
    
    /// Initialize with configuration
    pub fn with_config(config_path: &str) -> anyhow::Result<Self> {
        let mut resolver = Self::new();
        let mut parser = ConfigParserXml::new(config_path);
        
        // Parse configuration to get available objects
        let config_types = parser.parse_configuration()?;
        
        // Store parser for later use
        resolver.config_parser = Some(parser);
        
        // Cache configuration types
        for type_resolution in config_types {
            if let ResolutionResult::Concrete(ConcreteType::Configuration(config)) = &type_resolution.result {
                let key = format!("{:?}.{}", config.kind, config.name);
                resolver.cache.insert(key, type_resolution);
            }
        }
        
        Ok(resolver)
    }
    
    /// Resolve a dotted expression like "Справочники.Контрагенты"
    pub fn resolve_expression(&mut self, expression: &str) -> TypeResolution {
        // Check cache first
        if let Some(cached) = self.cache.get(expression) {
            return cached.clone();
        }
        
        let parts: Vec<&str> = expression.split('.').collect();
        
        let resolution = match parts.as_slice() {
            [] => self.unknown_resolution("Empty expression"),
            
            // Single identifier - check if it's a platform global
            [name] => {
                self.platform_globals.get(*name)
                    .cloned()
                    .unwrap_or_else(|| self.unknown_resolution(&format!("Unknown identifier: {}", name)))
            }
            
            // Dotted access like "Справочники.Контрагенты"
            [base, member] => {
                self.resolve_member_access(base, member)
            }
            
            // Deeper access like "Справочники.Контрагенты.НайтиПоКоду"
            [_base, _member, _method] => {
                // TODO: Resolve method on configuration object
                self.unknown_resolution(&format!("Method resolution not implemented: {}", expression))
            }
            
            _ => self.unknown_resolution(&format!("Complex expression not supported: {}", expression))
        };
        
        // Cache the result
        self.cache.insert(expression.to_string(), resolution.clone());
        resolution
    }
    
    /// Resolve member access like "Справочники.Контрагенты"
    fn resolve_member_access(&self, base: &str, member: &str) -> TypeResolution {
        // Check if base is a known platform global
        let _base_type = match self.platform_globals.get(base) {
            Some(t) => t,
            None => return self.unknown_resolution(&format!("Unknown base type: {}", base)),
        };
        
        // For manager types, member is a configuration object name
        match base {
            "Справочники" | "Catalogs" => {
                // TODO: Check if member exists in configuration
                // For now, create a synthetic type
                self.create_catalog_resolution(member)
            }
            
            "Документы" | "Documents" => {
                self.create_document_resolution(member)
            }
            
            "Перечисления" | "Enums" => {
                self.create_enum_resolution(member)
            }
            
            _ => self.unknown_resolution(&format!("Member access not implemented for: {}", base))
        }
    }
    
    /// Create resolution for catalog manager type
    fn create_catalog_resolution(&self, name: &str) -> TypeResolution {
        // TODO: Get actual type from configuration parser
        // For now, create a synthetic catalog manager type
        
        let qualified_name = format!("Справочники.{}", name);
        
        TypeResolution {
            certainty: Certainty::Inferred(0.8), // Not 100% sure without config
            result: ResolutionResult::Concrete(ConcreteType::Configuration(
                crate::core::types::ConfigurationType {
                    kind: crate::core::types::MetadataKind::Catalog,
                    name: name.to_string(),
                    attributes: vec![], // TODO: Get from config
                    tabular_sections: vec![], // TODO: Get from config
                }
            )),
            source: ResolutionSource::Inferred,
            metadata: ResolutionMetadata {
                file: Some("platform:catalogs".to_string()),
                line: None,
                column: None,
                notes: vec![format!("Inferred catalog type: {}", qualified_name)],
            },
            // Default facet is Manager for "Справочники.X"
            active_facet: Some(FacetKind::Manager),
            available_facets: vec![
                FacetKind::Manager,    // Справочники.Контрагенты
                FacetKind::Object,     // СправочникОбъект.Контрагенты
                FacetKind::Reference,  // СправочникСсылка.Контрагенты
                FacetKind::Constructor,// Справочники.Контрагенты.СоздатьЭлемент()
            ],
        }
    }
    
    fn create_document_resolution(&self, name: &str) -> TypeResolution {
        let qualified_name = format!("Документы.{}", name);
        
        TypeResolution {
            certainty: Certainty::Inferred(0.8),
            result: ResolutionResult::Concrete(ConcreteType::Configuration(
                crate::core::types::ConfigurationType {
                    kind: crate::core::types::MetadataKind::Document,
                    name: name.to_string(),
                    attributes: vec![],
                    tabular_sections: vec![],
                }
            )),
            source: ResolutionSource::Inferred,
            metadata: ResolutionMetadata {
                file: Some("platform:documents".to_string()),
                line: None,
                column: None,
                notes: vec![format!("Inferred document type: {}", qualified_name)],
            },
            active_facet: Some(FacetKind::Manager),
            available_facets: vec![
                FacetKind::Manager,    // Документы.ЗаказПокупателя
                FacetKind::Object,     // ДокументОбъект.ЗаказПокупателя
                FacetKind::Reference,  // ДокументСсылка.ЗаказПокупателя
                FacetKind::Constructor,
            ],
        }
    }
    
    fn create_enum_resolution(&self, name: &str) -> TypeResolution {
        let qualified_name = format!("Перечисления.{}", name);
        
        TypeResolution {
            certainty: Certainty::Inferred(0.8),
            result: ResolutionResult::Concrete(ConcreteType::Configuration(
                crate::core::types::ConfigurationType {
                    kind: crate::core::types::MetadataKind::Enum,
                    name: name.to_string(),
                    attributes: vec![],
                    tabular_sections: vec![],
                }
            )),
            source: ResolutionSource::Inferred,
            metadata: ResolutionMetadata {
                file: Some("platform:enums".to_string()),
                line: None,
                column: None,
                notes: vec![format!("Inferred enum type: {}", qualified_name)],
            },
            active_facet: Some(FacetKind::Manager),
            available_facets: vec![
                FacetKind::Manager,   // Перечисления.СтатусыЗаказов
                FacetKind::Reference, // ПеречислениеСсылка.СтатусыЗаказов
            ],
        }
    }
    
    /// Create an unknown resolution with explanation
    fn unknown_resolution(&self, reason: &str) -> TypeResolution {
        TypeResolution {
            certainty: Certainty::Unknown,
            result: ResolutionResult::Dynamic,
            source: ResolutionSource::Inferred,
            metadata: ResolutionMetadata {
                file: None,
                line: None,
                column: None,
                notes: vec![reason.to_string()],
            },
            active_facet: None,
            available_facets: vec![],
        }
    }
    
    /// Switch to a different facet for a type resolution
    pub fn switch_facet(&self, mut resolution: TypeResolution, new_facet: FacetKind) -> TypeResolution {
        // Check if facet is available
        if !resolution.available_facets.contains(&new_facet) {
            // Facet not available, return unchanged
            return resolution;
        }
        
        // Switch active facet
        resolution.active_facet = Some(new_facet);
        
        // Update metadata to reflect facet change
        resolution.metadata.notes.push(format!("Switched to facet: {:?}", new_facet));
        
        // TODO: Update methods/properties based on active facet
        // This would require deeper integration with platform docs
        
        resolution
    }
    
    /// Determine facet from context (e.g., "НовыйЭлемент" -> Constructor facet)
    pub fn infer_facet_from_context(&self, expression: &str) -> Option<FacetKind> {
        // Check for constructor patterns
        if expression.contains(".СоздатьЭлемент") || expression.contains(".CreateItem") {
            return Some(FacetKind::Constructor);
        }
        
        // Check for reference patterns
        if expression.contains("Ссылка.") || expression.contains("Ref.") {
            return Some(FacetKind::Reference);
        }
        
        // Check for object patterns
        if expression.contains("Объект.") || expression.contains("Object.") {
            return Some(FacetKind::Object);
        }
        
        // Default is Manager for top-level access
        if expression.starts_with("Справочники.") || expression.starts_with("Документы.") {
            return Some(FacetKind::Manager);
        }
        
        None
    }
    
    /// Get completions for a partial expression
    pub fn get_completions(&self, prefix: &str) -> Vec<CompletionItem> {
        let mut completions = Vec::new();
        
        // Parse the prefix to understand context
        let parts: Vec<&str> = prefix.split('.').collect();
        
        match parts.as_slice() {
            // Empty or single incomplete identifier - show globals
            [] | [""] => {
                // Add all platform globals
                for (name, _) in &self.platform_globals {
                    completions.push(CompletionItem {
                        label: name.clone(),
                        kind: CompletionKind::Global,
                        detail: Some("Platform global".to_string()),
                        documentation: None,
                    });
                }
            }
            
            // After "Справочники." - show available catalogs
            ["Справочники", ""] | ["Catalogs", ""] => {
                completions.extend(self.get_catalog_completions());
            }
            
            // After "Документы." - show available documents
            ["Документы", ""] | ["Documents", ""] => {
                completions.extend(self.get_document_completions());
            }
            
            // After "Перечисления." - show enums
            ["Перечисления", ""] | ["Enums", ""] => {
                completions.extend(self.get_enum_completions());
            }
            
            // Partial match at the end
            [base, partial] if !partial.is_empty() => {
                match *base {
                    "Справочники" | "Catalogs" => {
                        completions.extend(
                            self.get_catalog_completions()
                                .into_iter()
                                .filter(|c| c.label.starts_with(partial))
                        );
                    }
                    "Документы" | "Documents" => {
                        completions.extend(
                            self.get_document_completions()
                                .into_iter()
                                .filter(|c| c.label.starts_with(partial))
                        );
                    }
                    _ => {}
                }
            }
            
            _ => {}
        }
        
        completions
    }
    
    fn get_catalog_completions(&self) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        
        // Get from configuration cache
        for (key, resolution) in &self.cache {
            if key.starts_with("Catalog.") {
                if let ResolutionResult::Concrete(ConcreteType::Configuration(config)) = &resolution.result {
                    items.push(CompletionItem {
                        label: config.name.clone(),
                        kind: CompletionKind::Catalog,
                        detail: Some("Справочник".to_string()),
                        documentation: None,
                    });
                }
            }
        }
        
        // If no configuration, add some examples
        if items.is_empty() {
            for name in &["Контрагенты", "Номенклатура", "Организации"] {
                items.push(CompletionItem {
                    label: name.to_string(),
                    kind: CompletionKind::Catalog,
                    detail: Some("Справочник (пример)".to_string()),
                    documentation: Some("Пример справочника без конфигурации".to_string()),
                });
            }
        }
        
        items
    }
    
    fn get_document_completions(&self) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        
        // Get from configuration cache
        for (key, resolution) in &self.cache {
            if key.starts_with("Document.") {
                if let ResolutionResult::Concrete(ConcreteType::Configuration(config)) = &resolution.result {
                    items.push(CompletionItem {
                        label: config.name.clone(),
                        kind: CompletionKind::Document,
                        detail: Some("Документ".to_string()),
                        documentation: None,
                    });
                }
            }
        }
        
        // If no configuration, add examples
        if items.is_empty() {
            for name in &["ЗаказПокупателя", "РеализацияТоваровУслуг", "ПоступлениеТоваров"] {
                items.push(CompletionItem {
                    label: name.to_string(),
                    kind: CompletionKind::Document,
                    detail: Some("Документ (пример)".to_string()),
                    documentation: Some("Пример документа без конфигурации".to_string()),
                });
            }
        }
        
        items
    }
    
    fn get_enum_completions(&self) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        
        // Get from configuration cache
        for (key, resolution) in &self.cache {
            if key.starts_with("Enum.") {
                if let ResolutionResult::Concrete(ConcreteType::Configuration(config)) = &resolution.result {
                    items.push(CompletionItem {
                        label: config.name.clone(),
                        kind: CompletionKind::Enum,
                        detail: Some("Перечисление".to_string()),
                        documentation: None,
                    });
                }
            }
        }
        
        items
    }
}