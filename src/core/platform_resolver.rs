//! Platform-aware type resolver

use std::collections::HashMap;
use crate::adapters::platform_types_v2::PlatformTypesResolverV2;
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
    GlobalFunction,
    Variable,
    Function,
}

/// Resolver that knows about platform types and configuration
pub struct PlatformTypeResolver {
    /// Platform types resolver v2 with syntax helper data
    platform_resolver: PlatformTypesResolverV2,
    
    /// Platform global types
    platform_globals: HashMap<String, TypeResolution>,
    
    /// Configuration types from XML parser
    config_parser: Option<ConfigParserXml>,
    
    /// Cached resolutions
    cache: HashMap<String, TypeResolution>,
}

impl Default for PlatformTypeResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformTypeResolver {
    pub fn new() -> Self {
        let mut platform_resolver = PlatformTypesResolverV2::new();
        
        // Try to load syntax helper data from HTML directory
        let html_dir_path = "examples/syntax_helper/rebuilt.shcntx_ru";
        let absolute_path = std::path::Path::new(&std::env::current_dir().unwrap_or_default()).join(html_dir_path);
        
        println!("🔍 Checking HTML directory: {}", absolute_path.display());
        
        if absolute_path.exists() {
            println!("✅ Found HTML directory, loading...");
            match platform_resolver.load_from_directory(absolute_path.to_str().unwrap()) {
                Ok(_) => println!("✅ HTML directory loaded successfully"),
                Err(e) => println!("❌ Error loading HTML directory: {}", e),
            }
        } else if std::path::Path::new(html_dir_path).exists() {
            println!("✅ Found relative HTML directory, loading...");
            match platform_resolver.load_from_directory(html_dir_path) {
                Ok(_) => println!("✅ Relative HTML directory loaded successfully"),
                Err(e) => println!("❌ Error loading relative HTML directory: {}", e),
            }
        } else {
            println!("⚠️ HTML directory not found, falling back to JSON");
            // Fallback to JSON if HTML directory not found
            let json_path = "examples/syntax_helper/syntax_database.json";
            let json_absolute_path = std::path::Path::new(&std::env::current_dir().unwrap_or_default()).join(json_path);
            if json_absolute_path.exists() {
                println!("✅ Found absolute JSON file, loading...");
                match platform_resolver.load_from_file(json_absolute_path.to_str().unwrap()) {
                    Ok(_) => println!("✅ JSON file loaded successfully"),
                    Err(e) => println!("❌ Error loading JSON file: {}", e),
                }
            } else if std::path::Path::new(json_path).exists() {
                println!("✅ Found relative JSON file, loading...");
                match platform_resolver.load_from_file(json_path) {
                    Ok(_) => println!("✅ Relative JSON file loaded successfully"),
                    Err(e) => println!("❌ Error loading relative JSON file: {}", e),
                }
            } else {
                println!("❌ No data source found!");
            }
        }
        
        let mut platform_globals = platform_resolver.get_platform_globals();
        
        println!("📊 Loaded {} platform globals", platform_globals.len());
        
        // Add hardcoded platform managers if not loaded from file
        if !platform_globals.contains_key("Справочники") {
            Self::add_platform_managers(&mut platform_globals);
        }
        
        Self {
            platform_resolver,
            platform_globals,
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
    
    /// Add hardcoded platform managers
    fn add_platform_managers(globals: &mut HashMap<String, TypeResolution>) {
        // Russian names
        globals.insert("Справочники".to_string(), Self::create_manager_type("Справочники"));
        globals.insert("Документы".to_string(), Self::create_manager_type("Документы"));
        globals.insert("Перечисления".to_string(), Self::create_manager_type("Перечисления"));
        globals.insert("РегистрыСведений".to_string(), Self::create_manager_type("РегистрыСведений"));
        globals.insert("РегистрыНакопления".to_string(), Self::create_manager_type("РегистрыНакопления"));
        globals.insert("РегистрыБухгалтерии".to_string(), Self::create_manager_type("РегистрыБухгалтерии"));
        globals.insert("РегистрыРасчета".to_string(), Self::create_manager_type("РегистрыРасчета"));
        
        // English names
        globals.insert("Catalogs".to_string(), Self::create_manager_type("Catalogs"));
        globals.insert("Documents".to_string(), Self::create_manager_type("Documents"));
        globals.insert("Enums".to_string(), Self::create_manager_type("Enums"));
        globals.insert("InformationRegisters".to_string(), Self::create_manager_type("InformationRegisters"));
        globals.insert("AccumulationRegisters".to_string(), Self::create_manager_type("AccumulationRegisters"));
        globals.insert("AccountingRegisters".to_string(), Self::create_manager_type("AccountingRegisters"));
        globals.insert("CalculationRegisters".to_string(), Self::create_manager_type("CalculationRegisters"));
    }
    
    /// Create a manager type resolution
    fn create_manager_type(name: &str) -> TypeResolution {
        TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Platform(
                crate::core::types::PlatformType {
                    name: name.to_string(),
                    methods: vec![],
                    properties: vec![],
                }
            )),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata {
                file: Some("platform:managers".to_string()),
                line: None,
                column: None,
                notes: vec![format!("Platform manager type: {}", name)],
            },
            active_facet: None,
            available_facets: vec![],
        }
    }
    
    /// Get count of loaded platform globals (for debugging)
    pub fn get_platform_globals_count(&self) -> usize {
        self.platform_globals.len()
    }
    
    /// Check if a specific global is loaded (for debugging)
    pub fn has_platform_global(&self, key: &str) -> bool {
        self.platform_globals.contains_key(key)
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
                // Add all platform globals (managers and global functions)
                for name in self.platform_globals.keys() {
                    let (kind, detail) = if name.contains("Справочники") || name.contains("Catalogs") ||
                                           name.contains("Документы") || name.contains("Documents") ||
                                           name.contains("Перечисления") || name.contains("Enums") ||
                                           name.contains("РегистрыСведений") || name.contains("InformationRegisters") {
                        (CompletionKind::Global, "Менеджер объектов конфигурации")
                    } else {
                        // Это глобальная функция из синтакс-помощника
                        (CompletionKind::GlobalFunction, "Глобальная функция")
                    };
                    
                    completions.push(CompletionItem {
                        label: name.clone(),
                        kind,
                        detail: Some(detail.to_string()),
                        documentation: self.get_function_documentation(name),
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
            
            // Single partial identifier - filter globals
            [partial] if !partial.is_empty() => {
                for name in self.platform_globals.keys() {
                    // Case-insensitive starts_with for Russian and English
                    if name.to_lowercase().starts_with(&partial.to_lowercase()) {
                        let (kind, detail) = if name.contains("Справочники") || name.contains("Catalogs") ||
                                               name.contains("Документы") || name.contains("Documents") ||
                                               name.contains("Перечисления") || name.contains("Enums") ||
                                               name.contains("РегистрыСведений") || name.contains("InformationRegisters") {
                            (CompletionKind::Global, "Менеджер объектов конфигурации")
                        } else {
                            (CompletionKind::Method, "Глобальная функция")
                        };
                        
                        completions.push(CompletionItem {
                            label: name.clone(),
                            kind,
                            detail: Some(detail.to_string()),
                            documentation: self.get_function_documentation(name),
                        });
                    }
                }
            }
            
            // Partial match at the end after dot
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
                    // Методы и свойства объектов
                    "Массив" | "Array" | "Строка" | "String" | 
                    "Структура" | "Structure" | "Соответствие" | "Map" => {
                        completions.extend(
                            self.get_object_member_completions(base)
                                .into_iter()
                                .filter(|c| c.label.to_lowercase().starts_with(&partial.to_lowercase()))
                        );
                    }
                    _ => {}
                }
            }
            
            // Object methods/properties after dot (e.g., "Массив.", "Строка.")
            [base, ""] => {
                // Check if base is a known object type
                if matches!(*base, "Массив" | "Array" | "Строка" | "String" | 
                           "Структура" | "Structure" | "Соответствие" | "Map" |
                           "ТаблицаЗначений" | "ValueTable" | "СписокЗначений" | "ValueList") {
                    completions.extend(self.get_object_member_completions(base));
                } else {
                    // Check for configuration managers
                    match *base {
                        "Справочники" | "Catalogs" => {
                            completions.extend(self.get_catalog_completions());
                        }
                        "Документы" | "Documents" => {
                            completions.extend(self.get_document_completions());
                        }
                        "Перечисления" | "Enums" => {
                            completions.extend(self.get_enum_completions());
                        }
                        _ => {}
                    }
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
    
    /// Получает документацию для глобальной функции
    fn get_function_documentation(&self, name: &str) -> Option<String> {
        // Можно расширить для получения документации из синтакс-помощника
        match name {
            "Сообщить" => Some("Выводит сообщение пользователю".to_string()),
            "Тип" => Some("Возвращает тип значения".to_string()),
            "ТипЗнч" => Some("Возвращает тип значения".to_string()),
            "XMLСтрока" => Some("Преобразует значение в строку XML".to_string()),
            "XMLЗначение" => Some("Преобразует строку XML в значение".to_string()),
            _ => None,
        }
    }
    
    /// Получает автодополнение для членов объекта (методы и свойства)
    fn get_object_member_completions(&self, object_name: &str) -> Vec<CompletionItem> {
        let mut completions = Vec::new();
        
        // Получаем методы из PlatformTypesResolverV2
        let methods = self.platform_resolver.get_object_methods(object_name);
        for method in methods {
            let params_str = method.parameters.iter()
                .map(|p| format!("{}: {}", 
                    p.name, 
                    p.type_.as_deref().unwrap_or("Произвольный")))
                .collect::<Vec<_>>()
                .join(", ");
                
            let detail = if !params_str.is_empty() {
                format!("Метод({})", params_str)
            } else {
                "Метод()".to_string()
            };
            
            completions.push(CompletionItem {
                label: method.name.clone(),
                kind: CompletionKind::Method,
                detail: Some(detail),
                documentation: method.return_type.map(|rt| format!("Возвращает: {}", rt)),
            });
        }
        
        // Получаем свойства из PlatformTypesResolverV2
        let properties = self.platform_resolver.get_object_properties(object_name);
        for property in properties {
            let detail = format!("Свойство: {}{}", 
                property.type_, 
                if property.readonly { " (только чтение)" } else { "" });
                
            completions.push(CompletionItem {
                label: property.name.clone(),
                kind: CompletionKind::Property,
                detail: Some(detail),
                documentation: None,
            });
        }
        
        completions
    }
}