//! Platform types resolver using syntax helper data
//! 
//! Replaces hardcoded platform types with real data from 1C syntax helper

use std::collections::HashMap;
use std::path::Path;
use anyhow::Result;
use crate::core::types::{
    Certainty, ConcreteType, PlatformType, Method, Property, Parameter,
    ResolutionResult, ResolutionSource, TypeResolution, ResolutionMetadata,
    FacetKind,
};
use super::syntax_helper_parser::{SyntaxHelperParser, SyntaxHelperDatabase};

/// Enhanced platform types resolver using syntax helper data
pub struct PlatformTypesResolverV2 {
    syntax_database: Option<SyntaxHelperDatabase>,
}

impl PlatformTypesResolverV2 {
    /// Creates new resolver
    pub fn new() -> Self {
        Self {
            syntax_database: None,
        }
    }
    
    /// Loads syntax helper data from archives
    pub fn load_from_archives<P1: AsRef<Path>, P2: AsRef<Path>>(
        &mut self, 
        context_path: P1, 
        lang_path: P2
    ) -> Result<()> {
        let mut parser = SyntaxHelperParser::new()
            .with_context_archive(context_path)
            .with_lang_archive(lang_path);
            
        parser.parse()?;
        self.syntax_database = Some(parser.database().clone());
        Ok(())
    }
    
    /// Loads syntax helper data from saved JSON file
    pub fn load_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let database = SyntaxHelperParser::load_from_file(path)?;
        self.syntax_database = Some(database);
        Ok(())
    }
    
    /// Returns global functions from syntax helper
    pub fn get_global_functions(&self) -> HashMap<String, TypeResolution> {
        let mut functions = HashMap::new();
        
        if let Some(ref db) = self.syntax_database {
            for (name, func_info) in &db.global_functions {
                // Convert syntax helper function to TypeResolution
                let platform_type = PlatformType {
                    name: format!("GlobalFunction_{}", name),
                    methods: vec![], // Global functions are not methods
                    properties: vec![],
                };
                
                functions.insert(name.clone(), TypeResolution {
                    certainty: Certainty::Known,
                    result: ResolutionResult::Concrete(ConcreteType::Platform(platform_type)),
                    source: ResolutionSource::Static,
                    metadata: ResolutionMetadata {
                        file: Some("syntax_helper:global_functions".to_string()),
                        line: None,
                        column: None,
                        notes: vec![
                            format!("Global function from syntax helper: {}", name),
                            func_info.description.clone().unwrap_or_default(),
                        ],
                    },
                    active_facet: None,
                    available_facets: vec![],
                });
                
                // Also register by English name if available
                if let Some(ref eng_name) = func_info.english_name {
                    functions.insert(eng_name.clone(), functions[name].clone());
                }
            }
        }
        
        functions
    }
    
    /// Returns enhanced platform globals (still includes hardcoded ones for compatibility)
    pub fn get_platform_globals(&self) -> HashMap<String, TypeResolution> {
        let mut globals = HashMap::new();
        
        // Add hardcoded fallback platform globals
        globals.extend(self.get_fallback_platform_globals());
        
        // Add global functions from syntax helper
        globals.extend(self.get_global_functions());
        
        globals
    }
    
    /// Returns primitive types (enhanced with syntax helper data)
    pub fn get_primitive_types(&self) -> HashMap<String, TypeResolution> {
        let mut types = HashMap::new();
        
        // Start with fallback primitives for compatibility
        types.extend(self.get_fallback_primitive_types());
        
        // TODO: Enhance with syntax helper enum types
        if let Some(ref db) = self.syntax_database {
            for (name, enum_info) in &db.system_enums {
                let platform_type = PlatformType {
                    name: format!("SystemEnum_{}", name),
                    methods: vec![],
                    properties: vec![], // TODO: Convert enum values to properties
                };
                
                types.insert(name.clone(), TypeResolution {
                    certainty: Certainty::Known,
                    result: ResolutionResult::Concrete(ConcreteType::Platform(platform_type)),
                    source: ResolutionSource::Static,
                    metadata: ResolutionMetadata {
                        file: Some("syntax_helper:system_enums".to_string()),
                        line: None,
                        column: None,
                        notes: vec![
                            format!("System enum from syntax helper: {}", name),
                            enum_info.description.clone().unwrap_or_default(),
                        ],
                    },
                    active_facet: None,
                    available_facets: vec![],
                });
            }
        }
        
        types
    }
    
    /// Returns collection types (enhanced with syntax helper data)
    pub fn get_collection_types(&self) -> HashMap<String, TypeResolution> {
        let mut types = HashMap::new();
        
        // Start with fallback collections for compatibility  
        types.extend(self.get_fallback_collection_types());
        
        // Дополняем типами коллекций из синтакс-помощника
        if let Some(ref db) = self.syntax_database {
            // Обрабатываем объекты-коллекции (Массив, Структура, Соответствие и т.д.)
            for (name, object_info) in &db.global_objects {
                if name == "Массив" || name == "Array" ||
                   name == "Структура" || name == "Structure" ||
                   name == "Соответствие" || name == "Map" ||
                   name == "СписокЗначений" || name == "ValueList" ||
                   name == "ТаблицаЗначений" || name == "ValueTable" {
                    
                    // Создаём методы и свойства из данных синтакс-помощника
                    let mut methods = Vec::new();
                    let mut properties = Vec::new();
                    
                    // Добавляем методы объекта
                    for method_name in &object_info.methods {
                        let key = format!("{}.{}", name, method_name);
                        if let Some(method_info) = db.object_methods.get(&key) {
                            methods.push(Method {
                                name: method_info.name.clone(),
                                parameters: method_info.parameters.iter().map(|p| Parameter {
                                    name: p.name.clone(),
                                    type_: p.type_ref.as_ref().map(|t| t.name_ru.clone()),
                                    optional: p.is_optional,
                                }).collect(),
                                return_type: method_info.return_type.as_ref().map(|t| t.name_ru.clone()),
                            });
                        }
                    }
                    
                    // Добавляем свойства объекта
                    for property_name in &object_info.properties {
                        let key = format!("{}.{}", name, property_name);
                        if let Some(property_info) = db.object_properties.get(&key) {
                            properties.push(Property {
                                name: property_info.name.clone(),
                                type_: property_info.property_type.as_ref()
                                    .map(|t| t.name_ru.clone())
                                    .unwrap_or_else(|| "Произвольный".to_string()),
                                readonly: property_info.is_readonly,
                            });
                        }
                    }
                    
                    let platform_type = PlatformType {
                        name: name.clone(),
                        methods,
                        properties,
                    };
                    
                    types.insert(name.clone(), TypeResolution {
                        certainty: Certainty::Known,
                        result: ResolutionResult::Concrete(ConcreteType::Platform(platform_type)),
                        source: ResolutionSource::Static,
                        metadata: ResolutionMetadata {
                            file: Some("syntax_helper:collection_types".to_string()),
                            line: None,
                            column: None,
                            notes: vec![
                                format!("Collection type from syntax helper: {}", name),
                                object_info.description.clone().unwrap_or_default(),
                            ],
                        },
                        active_facet: Some(FacetKind::Constructor),
                        available_facets: vec![FacetKind::Constructor, FacetKind::Collection],
                    });
                }
            }
        }
        
        types
    }
    
    /// Returns keywords from syntax helper
    pub fn get_keywords(&self) -> Vec<String> {
        if let Some(ref db) = self.syntax_database {
            // Возвращаем русские названия ключевых слов
            db.keywords.iter()
                .map(|k| k.russian.clone())
                .collect()
        } else {
            vec![]
        }
    }
    
    /// Returns operators from syntax helper
    pub fn get_operators(&self) -> Vec<String> {
        if let Some(ref db) = self.syntax_database {
            db.operators.iter().map(|op| op.symbol.clone()).collect()
        } else {
            vec![]
        }
    }
    
    /// Returns statistics about loaded syntax helper data
    pub fn get_statistics(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();
        
        if let Some(ref db) = self.syntax_database {
            stats.insert("global_functions".to_string(), db.global_functions.len());
            stats.insert("global_objects".to_string(), db.global_objects.len());
            stats.insert("object_methods".to_string(), db.object_methods.len());
            stats.insert("object_properties".to_string(), db.object_properties.len());
            stats.insert("system_enums".to_string(), db.system_enums.len());
            stats.insert("keywords".to_string(), db.keywords.len());
            stats.insert("operators".to_string(), db.operators.len());
        } else {
            stats.insert("status".to_string(), 0); // Not loaded
        }
        
        stats
    }
    
    /// Checks if syntax helper data is loaded
    pub fn is_loaded(&self) -> bool {
        self.syntax_database.is_some()
    }
    
    /// Заполняет FacetRegistry данными из синтакс-помощника
    pub fn populate_facet_registry(&self, registry: &mut crate::core::facets::FacetRegistry) {
        if let Some(ref db) = self.syntax_database {
            // Обрабатываем все объекты
            for (object_name, object_info) in &db.global_objects {
                // Определяем фасет по имени объекта
                if let Some(facet_kind) = self.detect_facet_from_object_name(object_name) {
                    // Создаём методы для фасета
                    let mut methods = Vec::new();
                    for method_name in &object_info.methods {
                        let key = format!("{}.{}", object_name, method_name);
                        if let Some(method_info) = db.object_methods.get(&key) {
                            methods.push(Method {
                                name: method_info.name.clone(),
                                parameters: method_info.parameters.iter().map(|p| Parameter {
                                    name: p.name.clone(),
                                    type_: p.type_ref.as_ref().map(|t| t.name_ru.clone()),
                                    optional: p.is_optional,
                                }).collect(),
                                return_type: method_info.return_type.as_ref().map(|t| t.name_ru.clone()),
                            });
                        }
                    }
                    
                    // Создаём свойства для фасета
                    let mut properties = Vec::new();
                    for property_name in &object_info.properties {
                        let key = format!("{}.{}", object_name, property_name);
                        if let Some(property_info) = db.object_properties.get(&key) {
                            properties.push(Property {
                                name: property_info.name.clone(),
                                type_: property_info.property_type.as_ref()
                                    .map(|t| t.name_ru.clone())
                                    .unwrap_or_else(|| "Произвольный".to_string()),
                                readonly: property_info.is_readonly,
                            });
                        }
                    }
                    
                    // Извлекаем базовый тип из имени объекта
                    // Например: СправочникМенеджер.Контрагенты -> Контрагенты
                    let base_type = if object_name.contains('.') {
                        object_name.split('.').nth(1).unwrap_or(object_name).to_string()
                    } else {
                        object_name.clone()
                    };
                    
                    // Регистрируем фасет в registry
                    registry.register_facet(&base_type, facet_kind, methods, properties);
                }
            }
        }
    }
    
    /// Возвращает методы для объекта
    pub fn get_object_methods(&self, object_name: &str) -> Vec<Method> {
        let mut methods = Vec::new();
        
        if let Some(ref db) = self.syntax_database {
            // Проверяем глобальные объекты
            if let Some(object_info) = db.global_objects.get(object_name) {
                for method_name in &object_info.methods {
                    let key = format!("{}.{}", object_name, method_name);
                    if let Some(method_info) = db.object_methods.get(&key) {
                        methods.push(Method {
                            name: method_info.name.clone(),
                            parameters: method_info.parameters.iter().map(|p| Parameter {
                                name: p.name.clone(),
                                type_: p.type_ref.as_ref().map(|t| t.name_ru.clone()),
                                optional: p.is_optional,
                            }).collect(),
                            return_type: method_info.return_type.as_ref().map(|t| t.name_ru.clone()),
                        });
                    }
                }
            }
        }
        
        // Если методы не найдены в синтакс-помощнике, возвращаем fallback методы
        if methods.is_empty() {
            methods.extend(self.get_fallback_object_methods(object_name));
        }
        
        methods
    }
    
    /// Возвращает свойства для объекта
    pub fn get_object_properties(&self, object_name: &str) -> Vec<Property> {
        let mut properties = Vec::new();
        
        if let Some(ref db) = self.syntax_database {
            // Проверяем глобальные объекты
            if let Some(object_info) = db.global_objects.get(object_name) {
                for property_name in &object_info.properties {
                    let key = format!("{}.{}", object_name, property_name);
                    if let Some(property_info) = db.object_properties.get(&key) {
                        properties.push(Property {
                            name: property_info.name.clone(),
                            type_: property_info.property_type.as_ref()
                                .map(|t| t.name_ru.clone())
                                .unwrap_or_else(|| "Произвольный".to_string()),
                            readonly: property_info.is_readonly,
                        });
                    }
                }
            }
        }
        
        properties
    }
    
    /// Определяет фасет по имени объекта из синтакс-помощника
    fn detect_facet_from_object_name(&self, object_name: &str) -> Option<FacetKind> {
        if object_name.contains("Manager") || object_name.contains("Менеджер") {
            Some(FacetKind::Manager)
        } else if object_name.contains("Object") || object_name.contains("Объект") {
            Some(FacetKind::Object)
        } else if object_name.contains("Ref") || object_name.contains("Ссылка") {
            Some(FacetKind::Reference)
        } else if object_name.contains("Metadata") || object_name.contains("Метаданные") {
            Some(FacetKind::Metadata)
        } else if object_name == "Array" || object_name == "Массив" ||
                  object_name == "Structure" || object_name == "Структура" ||
                  object_name == "Map" || object_name == "Соответствие" {
            Some(FacetKind::Constructor)
        } else {
            None
        }
    }
}

impl Default for PlatformTypesResolverV2 {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to create resolver with syntax helper data loaded
/// Returns fallback to hardcoded types if syntax helper files not found
pub fn create_platform_resolver_with_syntax_helper() -> PlatformTypesResolverV2 {
    let mut resolver = PlatformTypesResolverV2::new();
    
    // Try to load from saved JSON file first (faster)
    let json_path = "examples/syntax_helper/syntax_database.json";
    if Path::new(json_path).exists() {
        if let Err(e) = resolver.load_from_file(json_path) {
            eprintln!("Warning: Failed to load syntax database from {}: {}", json_path, e);
        }
    } else {
        // Try to load from archives
        let context_path = "examples/syntax_helper/rebuilt.shcntx_ru.zip";
        let lang_path = "examples/syntax_helper/rebuilt.shlang_ru.zip";
        
        if Path::new(context_path).exists() && Path::new(lang_path).exists() {
            if let Err(e) = resolver.load_from_archives(context_path, lang_path) {
                eprintln!("Warning: Failed to load syntax helper from archives: {}", e);
            }
        } else {
            eprintln!("Warning: Syntax helper files not found, using hardcoded types only");
        }
    }
    
    resolver
}

// ====== Fallback функции (заменяют platform_types.rs) ======

impl PlatformTypesResolverV2 {
    /// Fallback platform globals for when syntax helper is not loaded
    fn get_fallback_platform_globals(&self) -> HashMap<String, TypeResolution> {
        let mut globals = HashMap::new();
        
        // Справочники (Catalogs manager)
        globals.insert("Справочники".to_string(), self.create_catalogs_manager());
        globals.insert("Catalogs".to_string(), self.create_catalogs_manager());
        
        // Документы (Documents manager)
        globals.insert("Документы".to_string(), self.create_documents_manager());
        globals.insert("Documents".to_string(), self.create_documents_manager());
        
        // Перечисления (Enums manager)
        globals.insert("Перечисления".to_string(), self.create_enums_manager());
        globals.insert("Enums".to_string(), self.create_enums_manager());
        
        // РегистрыСведений (Information registers manager)
        globals.insert("РегистрыСведений".to_string(), self.create_info_registers_manager());
        globals.insert("InformationRegisters".to_string(), self.create_info_registers_manager());
        
        globals
    }
    
    fn create_catalogs_manager(&self) -> TypeResolution {
        let platform_type = PlatformType {
            name: "CatalogsManager".to_string(),
            methods: vec![
                Method {
                    name: "НайтиПоНаименованию".to_string(),
                    parameters: vec![
                        Parameter {
                            name: "Наименование".to_string(),
                            type_: Some("Строка".to_string()),
                            optional: false,
                        },
                    ],
                    return_type: Some("СправочникСсылка".to_string()),
                },
                Method {
                    name: "НайтиПоКоду".to_string(),
                    parameters: vec![
                        Parameter {
                            name: "Код".to_string(),
                            type_: Some("Строка,Число".to_string()),
                            optional: false,
                        },
                    ],
                    return_type: Some("СправочникСсылка".to_string()),
                },
            ],
            properties: vec![],
        };
        
        TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Platform(platform_type)),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata {
                file: Some("platform:globals".to_string()),
                line: None,
                column: None,
                notes: vec!["Fallback platform type".to_string()],
            },
            active_facet: None,
            available_facets: vec![],
        }
    }
    
    fn create_documents_manager(&self) -> TypeResolution {
        let platform_type = PlatformType {
            name: "DocumentsManager".to_string(),
            methods: vec![
                Method {
                    name: "НайтиПоНомеру".to_string(),
                    parameters: vec![
                        Parameter {
                            name: "Номер".to_string(),
                            type_: Some("Строка".to_string()),
                            optional: false,
                        },
                        Parameter {
                            name: "Дата".to_string(),
                            type_: Some("Дата".to_string()),
                            optional: true,
                        },
                    ],
                    return_type: Some("ДокументСсылка".to_string()),
                },
            ],
            properties: vec![],
        };
        
        TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Platform(platform_type)),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata {
                file: Some("platform:globals".to_string()),
                line: None,
                column: None,
                notes: vec!["Fallback platform type".to_string()],
            },
            active_facet: None,
            available_facets: vec![],
        }
    }
    
    fn create_enums_manager(&self) -> TypeResolution {
        let platform_type = PlatformType {
            name: "EnumsManager".to_string(),
            methods: vec![],
            properties: vec![],
        };
        
        TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Platform(platform_type)),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata {
                file: Some("platform:globals".to_string()),
                line: None,
                column: None,
                notes: vec!["Fallback platform type".to_string()],
            },
            active_facet: None,
            available_facets: vec![],
        }
    }
    
    fn create_info_registers_manager(&self) -> TypeResolution {
        let platform_type = PlatformType {
            name: "InformationRegistersManager".to_string(),
            methods: vec![
                Method {
                    name: "СоздатьМенеджерЗаписи".to_string(),
                    parameters: vec![],
                    return_type: Some("РегистрСведенийМенеджерЗаписи".to_string()),
                },
                Method {
                    name: "СоздатьНаборЗаписей".to_string(),
                    parameters: vec![],
                    return_type: Some("РегистрСведенийНаборЗаписей".to_string()),
                },
            ],
            properties: vec![],
        };
        
        TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Platform(platform_type)),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata {
                file: Some("platform:globals".to_string()),
                line: None,
                column: None,
                notes: vec!["Fallback platform type".to_string()],
            },
            active_facet: None,
            available_facets: vec![],
        }
    }
    
    /// Fallback primitive types
    fn get_fallback_primitive_types(&self) -> HashMap<String, TypeResolution> {
        let mut types = HashMap::new();
        
        for (name, eng_name) in &[
            ("Строка", "String"),
            ("Число", "Number"),
            ("Булево", "Boolean"),
            ("Дата", "Date"),
            ("Неопределено", "Undefined"),
            ("Null", "Null"),
            ("Тип", "Type"),
        ] {
            let platform_type = PlatformType {
                name: eng_name.to_string(),
                methods: vec![],
                properties: vec![],
            };
            
            types.insert(name.to_string(), TypeResolution {
                certainty: Certainty::Known,
                result: ResolutionResult::Concrete(ConcreteType::Platform(platform_type)),
                source: ResolutionSource::Static,
                metadata: ResolutionMetadata {
                    file: Some("platform:primitives".to_string()),
                    line: None,
                    column: None,
                    notes: vec!["Fallback primitive type".to_string()],
                },
                active_facet: None,
                available_facets: vec![],
            });
        }
        
        types
    }
    
    /// Fallback методы для объектов
    fn get_fallback_object_methods(&self, object_name: &str) -> Vec<Method> {
        match object_name {
            "Массив" | "Array" => vec![
                Method {
                    name: "Добавить".to_string(),
                    parameters: vec![
                        Parameter {
                            name: "Значение".to_string(),
                            type_: None,
                            optional: false,
                        },
                    ],
                    return_type: None,
                },
                Method {
                    name: "Вставить".to_string(),
                    parameters: vec![
                        Parameter {
                            name: "Индекс".to_string(),
                            type_: Some("Число".to_string()),
                            optional: false,
                        },
                        Parameter {
                            name: "Значение".to_string(),
                            type_: None,
                            optional: false,
                        },
                    ],
                    return_type: None,
                },
                Method {
                    name: "Количество".to_string(),
                    parameters: vec![],
                    return_type: Some("Число".to_string()),
                },
                Method {
                    name: "Очистить".to_string(),
                    parameters: vec![],
                    return_type: None,
                },
                Method {
                    name: "Удалить".to_string(),
                    parameters: vec![
                        Parameter {
                            name: "Индекс".to_string(),
                            type_: Some("Число".to_string()),
                            optional: false,
                        },
                    ],
                    return_type: None,
                },
                Method {
                    name: "Найти".to_string(),
                    parameters: vec![
                        Parameter {
                            name: "Значение".to_string(),
                            type_: None,
                            optional: false,
                        },
                    ],
                    return_type: Some("Число".to_string()),
                },
            ],
            "Строка" | "String" => vec![
                Method {
                    name: "Длина".to_string(),
                    parameters: vec![],
                    return_type: Some("Число".to_string()),
                },
                Method {
                    name: "НайтиПервое".to_string(),
                    parameters: vec![
                        Parameter {
                            name: "Подстрока".to_string(),
                            type_: Some("Строка".to_string()),
                            optional: false,
                        },
                    ],
                    return_type: Some("Число".to_string()),
                },
            ],
            _ => vec![],
        }
    }
    
    /// Fallback collection types
    fn get_fallback_collection_types(&self) -> HashMap<String, TypeResolution> {
        let mut types = HashMap::new();
        
        let array_type = PlatformType {
            name: "Array".to_string(),
            methods: vec![
                Method {
                    name: "Добавить".to_string(),
                    parameters: vec![
                        Parameter {
                            name: "Значение".to_string(),
                            type_: None,
                            optional: false,
                        },
                    ],
                    return_type: None,
                },
                Method {
                    name: "Количество".to_string(),
                    parameters: vec![],
                    return_type: Some("Число".to_string()),
                },
                Method {
                    name: "Очистить".to_string(),
                    parameters: vec![],
                    return_type: None,
                },
            ],
            properties: vec![],
        };
        
        types.insert("Массив".to_string(), TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Platform(array_type)),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata {
                file: Some("platform:collections".to_string()),
                line: None,
                column: None,
                notes: vec!["Fallback collection type".to_string()],
            },
            active_facet: None,
            available_facets: vec![],
        });
        
        types
    }
}