//! Platform types repository using syntax helper data
//!  
//! Uses optimized syntax helper parser to extract platform types from documentation

use super::syntax_helper_parser::{
    OptimizationSettings, SyntaxHelperDatabase, SyntaxHelperParser, SyntaxNode, TypeIndex, TypeInfo,
};
use crate::domain::types::{
    Certainty, ConcreteType, Method, Parameter, PlatformType, Property, ResolutionMetadata,
    ResolutionResult, ResolutionSource, TypeResolution,
};
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

/// Platform types repository using syntax helper data
pub struct PlatformTypesRepository {
    /// Парсер документации
    parser: SyntaxHelperParser,
    /// База данных типов
    database: Option<SyntaxHelperDatabase>,
    /// Индексы для быстрого поиска
    type_index: Option<TypeIndex>,
}

impl PlatformTypesRepository {
    /// Creates new resolver with default settings
    pub fn new() -> Self {
        Self {
            parser: SyntaxHelperParser::new(),
            database: None,
            type_index: None,
        }
    }

    /// Creates resolver with custom optimization settings
    pub fn with_settings(settings: OptimizationSettings) -> Self {
        Self {
            parser: SyntaxHelperParser::with_settings(settings),
            database: None,
            type_index: None,
        }
    }

    /// Loads syntax helper data from directory
    pub fn load_from_directory<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        // Парсим директорию с документацией
        self.parser.parse_directory(path)?;

        // Экспортируем базу данных и индексы
        self.database = Some(self.parser.export_database());
        self.type_index = Some(self.parser.export_index());

        Ok(())
    }

    /// Loads syntax helper data from saved JSON file
    pub fn load_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let json_str = std::fs::read_to_string(path)?;
        let database: SyntaxHelperDatabase = serde_json::from_str(&json_str)?;
        self.database = Some(database);

        // Перестраиваем индексы
        self.rebuild_indexes();

        Ok(())
    }

    /// Saves database to JSON file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        if let Some(ref database) = self.database {
            let json_str = serde_json::to_string_pretty(database)?;
            std::fs::write(path, json_str)?;
        }
        Ok(())
    }

    /// Rebuilds indexes from database
    fn rebuild_indexes(&mut self) {
        if let Some(ref database) = self.database {
            let mut index = TypeIndex::default();

            for (path, node) in &database.nodes {
                if let SyntaxNode::Type(type_info) = node {
                    // Индекс по русскому имени
                    index
                        .by_russian
                        .insert(type_info.identity.russian_name.clone(), path.clone());

                    // Индекс по английскому имени
                    if !type_info.identity.english_name.is_empty() {
                        index
                            .by_english
                            .insert(type_info.identity.english_name.clone(), path.clone());
                    }

                    // Индекс по фасетам
                    for facet in &type_info.metadata.available_facets {
                        index.by_facet.entry(*facet).or_default().push(path.clone());
                    }
                }
            }

            self.type_index = Some(index);
        }
    }

    /// Returns global functions from syntax helper
    pub fn get_global_functions(&self) -> HashMap<String, TypeResolution> {
        let mut functions = HashMap::new();

        if let Some(ref database) = self.database {
            for method in database.methods.values() {
                let resolution = self.method_to_resolution(method.name.clone());
                functions.insert(method.name.clone(), resolution);
            }
        }

        functions
    }

    /// Returns global objects/types
    pub fn get_global_objects(&self) -> HashMap<String, TypeResolution> {
        let mut objects = HashMap::new();

        if let Some(ref database) = self.database {
            for node in database.nodes.values() {
                if let SyntaxNode::Type(type_info) = node {
                    let resolution = self.type_to_resolution(type_info);

                    // Добавляем по русскому имени
                    if !type_info.identity.russian_name.is_empty() {
                        objects.insert(type_info.identity.russian_name.clone(), resolution.clone());
                    }

                    // Добавляем по английскому имени
                    if !type_info.identity.english_name.is_empty() {
                        objects.insert(type_info.identity.english_name.clone(), resolution);
                    }
                }
            }
        }

        objects
    }

    /// Resolves type by name
    pub fn resolve(&self, type_name: &str) -> TypeResolution {
        // Ищем тип в индексе
        if let Some(type_info) = self.find_type(type_name) {
            return self.type_to_resolution(type_info);
        }

        // Проверяем стандартные типы
        match type_name {
            "Строка" | "String" => self.create_standard_type("String"),
            "Число" | "Number" => self.create_standard_type("Number"),
            "Булево" | "Boolean" => self.create_standard_type("Boolean"),
            "Дата" | "Date" => self.create_standard_type("Date"),
            _ => {
                let mut resolution = TypeResolution::unknown();
                resolution
                    .metadata
                    .notes
                    .push(format!("Unknown type: {}", type_name));
                resolution
            }
        }
    }

    /// Finds type in database
    fn find_type(&self, name: &str) -> Option<&TypeInfo> {
        let index = self.type_index.as_ref()?;
        let database = self.database.as_ref()?;

        // Ищем по русскому имени
        if let Some(path) = index.by_russian.get(name) {
            if let Some(SyntaxNode::Type(type_info)) = database.nodes.get(path) {
                return Some(type_info);
            }
        }

        // Ищем по английскому имени
        if let Some(path) = index.by_english.get(name) {
            if let Some(SyntaxNode::Type(type_info)) = database.nodes.get(path) {
                return Some(type_info);
            }
        }

        None
    }

    /// Converts TypeInfo to TypeResolution
    fn type_to_resolution(&self, type_info: &TypeInfo) -> TypeResolution {
        let platform_type = PlatformType {
            name: type_info.identity.russian_name.clone(),
            methods: self.extract_methods(type_info),
            properties: self.extract_properties(type_info),
        };

        TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Platform(platform_type)),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata {
                file: None,
                line: None,
                column: None,
                notes: vec![format!(
                    "From syntax helper: {}",
                    type_info.identity.catalog_path
                )],
            },
            active_facet: type_info.metadata.default_facet,
            available_facets: type_info.metadata.available_facets.clone(),
        }
    }

    /// Extracts methods from TypeInfo
    fn extract_methods(&self, type_info: &TypeInfo) -> Vec<Method> {
        let mut methods = Vec::new();

        for method_name in &type_info.structure.methods {
            if let Some(ref database) = self.database {
                let key = format!("method_{}", method_name);
                if let Some(method_info) = database.methods.get(&key) {
                    methods.push(Method {
                        name: method_info.name.clone(),
                        parameters: self.extract_parameters(&method_info.parameters),
                        return_type: method_info.return_type.clone(),
                        is_function: method_info.return_type.is_some(),
                    });
                }
            }
        }

        methods
    }

    /// Extracts properties from TypeInfo
    fn extract_properties(&self, type_info: &TypeInfo) -> Vec<Property> {
        let mut properties = Vec::new();

        for prop_name in &type_info.structure.properties {
            if let Some(ref database) = self.database {
                let key = format!("property_{}", prop_name);
                if let Some(prop_info) = database.properties.get(&key) {
                    properties.push(Property {
                        name: prop_info.name.clone(),
                        type_: prop_info
                            .property_type
                            .clone()
                            .unwrap_or_else(|| "Unknown".to_string()),
                        readonly: prop_info.is_readonly,
                    });
                }
            }
        }

        properties
    }

    /// Extracts parameters
    fn extract_parameters(
        &self,
        params: &[super::syntax_helper_parser::ParameterInfo],
    ) -> Vec<Parameter> {
        params
            .iter()
            .map(|p| Parameter {
                name: p.name.clone(),
                type_: p.type_name.clone(),
                optional: p.is_optional,
                by_value: true, // По умолчанию параметры передаются по значению
            })
            .collect()
    }

    /// Creates resolution for method
    fn method_to_resolution(&self, name: String) -> TypeResolution {
        let platform_type = PlatformType {
            name: name.clone(),
            methods: Vec::new(),
            properties: Vec::new(),
        };

        TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Platform(platform_type)),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata {
                file: None,
                line: None,
                column: None,
                notes: vec![format!("Global function: {}", name)],
            },
            active_facet: None,
            available_facets: vec![],
        }
    }

    /// Creates standard type resolution
    fn create_standard_type(&self, type_name: &str) -> TypeResolution {
        use crate::domain::types::PrimitiveType;

        let primitive = match type_name {
            "String" | "Строка" => PrimitiveType::String,
            "Number" | "Число" => PrimitiveType::Number,
            "Boolean" | "Булево" => PrimitiveType::Boolean,
            "Date" | "Дата" => PrimitiveType::Date,
            _ => PrimitiveType::String, // Default to string
        };

        TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Primitive(primitive)),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata {
                file: None,
                line: None,
                column: None,
                notes: vec![format!("Standard type: {}", type_name)],
            },
            active_facet: None,
            available_facets: vec![],
        }
    }

    /// Gets object methods by type name
    pub fn get_object_methods(&self, type_name: &str) -> Vec<Method> {
        if let Some(type_info) = self.find_type(type_name) {
            return self.extract_methods(type_info);
        }
        Vec::new()
    }

    /// Gets object properties by type name
    pub fn get_object_properties(&self, type_name: &str) -> Vec<Property> {
        if let Some(type_info) = self.find_type(type_name) {
            return self.extract_properties(type_info);
        }
        Vec::new()
    }

    /// Gets platform globals (for compatibility)
    pub fn get_platform_globals(&self) -> HashMap<String, TypeResolution> {
        let mut globals = self.get_global_functions();
        let objects = self.get_global_objects();
        globals.extend(objects);
        globals
    }

    /// Gets resolver statistics
    pub fn get_stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();

        if let Some(ref database) = self.database {
            stats.insert("total_nodes".to_string(), database.nodes.len());
            stats.insert("methods".to_string(), database.methods.len());
            stats.insert("properties".to_string(), database.properties.len());

            let types_count = database
                .nodes
                .values()
                .filter(|n| matches!(n, SyntaxNode::Type(_)))
                .count();
            stats.insert("types".to_string(), types_count);
        }

        if let Some(ref index) = self.type_index {
            stats.insert("indexed_russian".to_string(), index.by_russian.len());
            stats.insert("indexed_english".to_string(), index.by_english.len());
        }

        stats
    }

    /// Enhanced version: Gets platform globals with hardcoded managers
    pub fn get_platform_globals_enhanced(&self) -> HashMap<String, TypeResolution> {
        let mut globals = self.get_platform_globals();

        // Add hardcoded platform managers if not loaded from file
        if !globals.contains_key("Справочники") {
            Self::add_platform_managers(&mut globals);
        }

        globals
    }

    /// Add hardcoded platform managers
    fn add_platform_managers(globals: &mut HashMap<String, TypeResolution>) {
        // Russian names
        globals.insert(
            "Справочники".to_string(),
            Self::create_manager_type("Справочники"),
        );
        globals.insert(
            "Документы".to_string(),
            Self::create_manager_type("Документы"),
        );
        globals.insert(
            "Перечисления".to_string(),
            Self::create_manager_type("Перечисления"),
        );
        globals.insert(
            "РегистрыСведений".to_string(),
            Self::create_manager_type("РегистрыСведений"),
        );
        globals.insert(
            "РегистрыНакопления".to_string(),
            Self::create_manager_type("РегистрыНакопления"),
        );
        globals.insert(
            "РегистрыБухгалтерии".to_string(),
            Self::create_manager_type("РегистрыБухгалтерии"),
        );
        globals.insert(
            "РегистрыРасчета".to_string(),
            Self::create_manager_type("РегистрыРасчета"),
        );

        // English names
        globals.insert(
            "Catalogs".to_string(),
            Self::create_manager_type("Catalogs"),
        );
        globals.insert(
            "Documents".to_string(),
            Self::create_manager_type("Documents"),
        );
        globals.insert("Enums".to_string(), Self::create_manager_type("Enums"));
        globals.insert(
            "InformationRegisters".to_string(),
            Self::create_manager_type("InformationRegisters"),
        );
        globals.insert(
            "AccumulationRegisters".to_string(),
            Self::create_manager_type("AccumulationRegisters"),
        );
        globals.insert(
            "AccountingRegisters".to_string(),
            Self::create_manager_type("AccountingRegisters"),
        );
        globals.insert(
            "CalculationRegisters".to_string(),
            Self::create_manager_type("CalculationRegisters"),
        );
    }

    /// Create a manager type resolution
    fn create_manager_type(name: &str) -> TypeResolution {
        TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
                name: name.to_string(),
                methods: vec![],
                properties: vec![],
            })),
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

    /// Enhanced resolve method that supports expressions
    pub fn resolve_expression(&mut self, expression: &str) -> TypeResolution {
        // Try direct type resolution first
        let direct_result = self.resolve(expression);
        if !matches!(direct_result.certainty, Certainty::Unknown) {
            return direct_result;
        }

        // Try expression parsing (simplified version)
        if let Some((base, member)) = self.parse_member_access(expression) {
            return self.resolve_member_access(&base, &member);
        }

        // Fallback to unknown
        TypeResolution::unknown()
    }

    /// Parse member access like "Справочники.Контрагенты"
    fn parse_member_access(&self, expression: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = expression.split('.').collect();
        if parts.len() == 2 {
            Some((parts[0].to_string(), parts[1].to_string()))
        } else {
            None
        }
    }

    /// Resolve member access like "Справочники.Контрагенты"  
    fn resolve_member_access(&self, base: &str, member: &str) -> TypeResolution {
        use crate::domain::types::{ConfigurationType, FacetKind, MetadataKind};

        match base {
            "Справочники" | "Catalogs" => {
                let qualified_name = format!("Справочники.{}", member);
                TypeResolution {
                    certainty: Certainty::Inferred(0.8),
                    result: ResolutionResult::Concrete(ConcreteType::Configuration(
                        ConfigurationType {
                            kind: MetadataKind::Catalog,
                            name: member.to_string(),
                            attributes: vec![],
                            tabular_sections: vec![],
                        },
                    )),
                    source: ResolutionSource::Inferred,
                    metadata: ResolutionMetadata {
                        file: Some("platform:catalogs".to_string()),
                        line: None,
                        column: None,
                        notes: vec![format!("Inferred catalog type: {}", qualified_name)],
                    },
                    active_facet: Some(FacetKind::Manager),
                    available_facets: vec![
                        FacetKind::Manager,
                        FacetKind::Object,
                        FacetKind::Reference,
                        FacetKind::Constructor,
                    ],
                }
            }
            "Документы" | "Documents" => {
                let qualified_name = format!("Документы.{}", member);
                TypeResolution {
                    certainty: Certainty::Inferred(0.8),
                    result: ResolutionResult::Concrete(ConcreteType::Configuration(
                        ConfigurationType {
                            kind: MetadataKind::Document,
                            name: member.to_string(),
                            attributes: vec![],
                            tabular_sections: vec![],
                        },
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
                        FacetKind::Manager,
                        FacetKind::Object,
                        FacetKind::Reference,
                        FacetKind::Constructor,
                    ],
                }
            }
            "Перечисления" | "Enums" => {
                let qualified_name = format!("Перечисления.{}", member);
                TypeResolution {
                    certainty: Certainty::Inferred(0.8),
                    result: ResolutionResult::Concrete(ConcreteType::Configuration(
                        ConfigurationType {
                            kind: MetadataKind::Enum,
                            name: member.to_string(),
                            attributes: vec![],
                            tabular_sections: vec![],
                        },
                    )),
                    source: ResolutionSource::Inferred,
                    metadata: ResolutionMetadata {
                        file: Some("platform:enums".to_string()),
                        line: None,
                        column: None,
                        notes: vec![format!("Inferred enum type: {}", qualified_name)],
                    },
                    active_facet: Some(FacetKind::Manager),
                    available_facets: vec![FacetKind::Manager, FacetKind::Reference],
                }
            }
            _ => {
                let mut resolution = TypeResolution::unknown();
                resolution
                    .metadata
                    .notes
                    .push(format!("Unknown base type: {}", base));
                resolution
            }
        }
    }
}

impl Default for PlatformTypesRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolver_creation() {
        let resolver = PlatformTypesRepository::new();
        assert!(resolver.database.is_none());
        assert!(resolver.type_index.is_none());
    }

    #[test]
    fn test_standard_types() {
        let resolver = PlatformTypesRepository::new();

        let string_type = resolver.resolve("Строка");
        assert_eq!(string_type.certainty, Certainty::Known);

        let number_type = resolver.resolve("Number");
        assert_eq!(number_type.certainty, Certainty::Known);
    }

    #[test]
    fn test_custom_settings() {
        let settings = OptimizationSettings {
            max_threads: Some(2),
            batch_size: 10,
            show_progress: false,
            ..Default::default()
        };

        let resolver = PlatformTypesRepository::with_settings(settings);
        assert!(resolver.database.is_none());
    }
}

// РЕФАКТОРИНГ: Дополнительные методы для совместимости с UnifiedTypeSystem
impl PlatformTypesRepository {
    /// Получить количество платформенных глобальных типов
    pub fn get_platform_globals_count(&self) -> usize {
        self.get_platform_globals_enhanced().len()
    }

    /// Проверить, есть ли платформенный глобальный объект
    pub fn has_platform_global(&self, key: &str) -> bool {
        self.get_platform_globals_enhanced().contains_key(key)
    }

    /// Получить автодополнение для выражения (базовая реализация)
    pub fn get_completions(&self, _expression: &str) -> Vec<crate::domain::CompletionItem> {
        // Базовая реализация - возвращаем все глобальные объекты как варианты
        let globals = self.get_platform_globals_enhanced();
        globals
            .keys()
            .map(|name| crate::domain::CompletionItem {
                label: name.clone(),
                kind: crate::domain::CompletionKind::Global,
                detail: Some("Платформенный объект".to_string()),
                documentation: None,
                insert_text: Some(name.clone()),
                filter_text: Some(name.clone()),
                sort_text: Some(name.clone()),
            })
            .collect()
    }

    /// Получить базу данных синтакс-помощника для построения иерархий
    pub fn get_syntax_helper_database(&self) -> Option<&SyntaxHelperDatabase> {
        self.database.as_ref()
    }
}
